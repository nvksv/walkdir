use std::cmp::Ordering;
use std::io;
use std::vec;


use crate::wd::{self, ContentFilter, ContentOrder, Position, FnCmp};
use crate::dent::DirEntry;
#[cfg(unix)]
use crate::dent::DirEntryExt;
use crate::source;
//use crate::source::{SourceFsFileType, SourceFsMetadata, SourcePath};
use crate::walk::WalkDirOptionsImmut;



/////////////////////////////////////////////////////////////////////////
//// ReadDir

/// A sequence of unconsumed directory entries.
///
/// This represents the opened or closed state of a directory handle. When
/// open, future entries are read by iterating over the raw `fs::ReadDir`.
/// When closed, all future entries are read into memory. Iteration then
/// proceeds over a [`Vec<fs::DirEntry>`].
///
/// [`fs::ReadDir`]: https://doc.rust-lang.org/stable/std/fs/struct.ReadDir.html
/// [`Vec<fs::DirEntry>`]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html
#[derive(Debug)]
enum ReadDir<E: source::SourceExt> {

    /// The single item (used for root)
    Once { item: Option<DirEntry<E>> },

    /// An opened handle.
    ///
    /// This includes the depth of the handle itself.
    ///
    /// If there was an error with the initial [`fs::read_dir`] call, then it
    /// is stored here. (We use an [`Option<...>`] to make yielding the error
    /// exactly once simpler.)
    ///
    /// [`fs::read_dir`]: https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
    /// [`Option<...>`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
    Opened { rd: E::FsReadDir },

    /// A closed handle.
    ///
    /// All remaining directory entries are read into memory.
    Closed,

    /// Error on handle creating
    Error( Option<wd::ErrorInner<E>> ),
}

impl<E: source::SourceExt> ReadDir<E> {
    fn new_once( dent: DirEntry<E> ) -> Self {
        Self::Once { 
            item: Some(dent),
        }
    }

    fn new( rd: wd::ResultInner<E::FsReadDir, E> ) -> Self {
        match rd {
            Ok(rd) => Self::Opened { rd },
            Err(err) => Self::Error( Some(err) ),
        }        
    }

    fn collect_all<T>(&mut self, process_fsdent: &impl (Fn(wd::ResultInner<DirEntry<E>, E>) -> Option<T>) ) -> Vec<T> {
        match *self {
            ReadDir::Opened { ref mut rd } => {
                let entries = rd.map(Self::process_next).map(process_fsdent).filter_map(|opt| opt).collect();
                *self = ReadDir::<E>::Closed;
                entries
            },
            ReadDir::Once { ref mut item } => {
                let entries = match item.take() {
                    Some(dent) => match process_fsdent(Ok(dent)) {
                        Some(t) => vec![t],
                        None => vec![],
                    },
                    None => vec![],
                };
                *self = ReadDir::<E>::Closed;
                entries
            },
            ReadDir::Closed => {
                vec![]
            },
            ReadDir::Error( ref mut oerr ) => {
                match oerr.take() {
                    Some(err) => match process_fsdent(Err(err)) {
                        Some(e) => vec![e],
                        None => vec![],
                    },
                    None => vec![],
                }
            },
        }
    }

    fn process_next( r_ent: io::Result<E::FsDirEntry> ) -> wd::ResultInner<DirEntry<E>, E> {
        match r_ent {
            Ok(ent) => {
                DirEntry::<E>::from_entry( &ent )
            },
            Err(err) => {
                Err(wd::ErrorInner::from_io( err ))
            },
        }
    }
}

impl<E: source::SourceExt> Iterator for ReadDir<E> {
    type Item = wd::ResultInner<DirEntry<E>, E>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            ReadDir::Once { ref mut item } => {
                item.take().map(Ok)
            },
            ReadDir::Opened { ref mut rd } => {
                rd.next().map(Self::process_next)
            },
            ReadDir::Closed => {
                None
            },
            ReadDir::Error( ref mut err ) => {
                err.take().map(Err)
            },
        }
    }
}




/////////////////////////////////////////////////////////////////////////
//// 

pub struct ProcessedDirEntry<E: source::SourceExt> {
    pub dent: DirEntry<E>,
    pub is_dir: bool,
}




/////////////////////////////////////////////////////////////////////////
//// DirEntryRecord

#[derive(Debug)]
pub(crate) struct DirEntryRecord<E: source::SourceExt> {
    /// Value from ReadDir
    dent: wd::ResultInner<DirEntry<E>, E>,
    /// This entry is a dir and will be walked recursive
    is_dir: bool,
    /// This entry must be yielded first according to opts.content_order
    first_pass: bool,
    /// This entry will not be yielded according to opts.content_filter
    hidden: bool,
}

impl<E: source::SourceExt> DirEntryRecord<E> {
    fn new( rdr: wd::ResultInner<DirEntry<E>, E>, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) -> Option<Self> {
        let rdr_flatten = match rdr {
            Ok(raw_dent) => match process_dent(raw_dent) {
                Some(pdent) => pdent,
                None => return None,
            },
            Err(e) => Err(e),
        };

        let this = match rdr_flatten {
            Ok(ProcessedDirEntry{ dent, is_dir }) => {
                let first_pass = match opts_immut.content_order {
                    ContentOrder::None => false,
                    ContentOrder::DirsFirst => is_dir,
                    ContentOrder::FilesFirst => !is_dir,
                };

                let hidden = match opts_immut.content_filter {
                    ContentFilter::None => false,
                    ContentFilter::DirsOnly => !is_dir,
                    ContentFilter::FilesOnly => is_dir,
                };
                
                Self {
                    dent: Ok(dent),
                    is_dir,
                    first_pass,
                    hidden,
                }
            },
            Err(err) => {
                Self {
                    dent: Err(err),
                    is_dir: false,
                    first_pass: false,
                    hidden: false,
                }
            }
        };

        Some(this)
    }
}




/////////////////////////////////////////////////////////////////////////
//// DirState

#[derive(Debug)]
pub struct DirContent<E: source::SourceExt> {
    /// Source of not consumed DirEntries
    rd: ReadDir<E>,
    /// A list of already consumed DirEntries
    content: Vec<DirEntryRecord<E>>,
    /// Count of consumed entries = position of unconsumed in content
    current_pos: Option<usize>,
}

impl<E: source::SourceExt> DirContent<E> {
    /// New DirContent from alone DirEntry
    pub(crate) fn new_once( raw_dent: DirEntry<E> ) -> Self {
        Self {
            rd: ReadDir::<E>::new_once( raw_dent ),
            content: vec![],
            current_pos: None,
        }
    }

    /// New DirContent from FsReadDir
    pub(crate) fn new( rd: wd::ResultInner<E::FsReadDir, E> ) -> Self {
        Self {
            rd: ReadDir::<E>::new( rd ),
            content: vec![],
            current_pos: None,
        }
    }

    /// Load all remaining DirEntryRecord into tail of self.content.
    /// Doesn't change position.
    pub(crate) fn load_all(&mut self, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) {
        let mut collected = self.rd.collect_all(& |r_ent| Self::new_rec( r_ent, opts_immut, process_dent ));

        if self.content.is_empty() {
            self.content = collected;
        } else {
            self.content.append(&mut collected);
        }
    }

    /// Makes new DirEntryRecord from processed Result<DirEntry> or rejects it. 
    /// Doesn't change position.
    fn new_rec(r_ent: wd::ResultInner<DirEntry<E>, E>, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) -> Option<DirEntryRecord<E>> {
        let rec = DirEntryRecord::<E>::new( r_ent, opts_immut, process_dent )?;

        // if let Ok(ref mut dent) = rec.dent {
        //     dent.set_depth_mut( depth );
        // };

        Some(rec)
    }

    /// Shifts to next record (and loads it when necessary) -- without any passes, content filters and so on.
    /// Updates current position on success.
    pub(crate) fn get_next_rec(&mut self, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>)) -> Option<(bool, bool)> {
        loop {
            // Check for already loaded entry
            let next_pos = if let Some(pos) = self.current_pos {pos + 1} else {0};
            if let Some(rec) = self.content.get(next_pos) {
                self.current_pos = Some(next_pos);
                return Some((rec.first_pass, rec.hidden));
            }

            if let Some(r_ent) = self.rd.next() {
                let rec = match Self::new_rec(r_ent, opts_immut, process_dent) {
                    Some(rec) => rec,
                    None => continue,
                };
                self.content.push(rec);
                self.current_pos = Some(self.content.len()-1);

                let last = self.content.last();
                assert!( last.is_some() );
                let rec = last.unwrap();
                return Some((rec.first_pass, rec.hidden));
            }

            break;
        }

        None
    }

    /// Rewind current position: now we stand before beginning.
    pub(crate) fn rewind(&mut self) {
        self.current_pos = None;
    }

    /// Gets record at current position
    /// Doesn't change position.
    pub(crate) fn get_current_rec(&mut self) -> Option<&mut DirEntryRecord<E>> {
        match self.current_pos {
            Some(pos) => self.content.get_mut(pos),
            None => None,
        }
    }

    /// Sorts all loaded content.
    /// Changes current position.
    fn sort_content_and_rewind(&mut self, cmp: &mut FnCmp<E>) {
        self.content.sort_by(|a, b| {
                match (&a.dent, &b.dent) {
                    (&Ok(ref a), &Ok(ref b)) => cmp(a, b),
                    (&Err(_), &Err(_)) => Ordering::Equal,
                    (&Ok(_), &Err(_)) => Ordering::Greater,
                    (&Err(_), &Ok(_)) => Ordering::Less,
                }
            }
        );
        self.current_pos = None;
    }

    /// Sorts all loaded content.
    /// Changes current position.
    pub(crate) fn load_all_and_sort(&mut self, opts_immut: &WalkDirOptionsImmut<E>, cmp: &mut FnCmp<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>)) {
        self.load_all( opts_immut, process_dent );
        self.sort_content_and_rewind( cmp );
    }

    pub(crate) fn iter_content<'s, F, T>(&'s self, f: F) -> std::iter::FilterMap<std::slice::Iter<'s, DirEntryRecord<E>>, F> where F: FnMut(&'s DirEntryRecord<E>) -> Option<T> {
        self.content.iter().filter_map( f )
    }
}



/////////////////////////////////////////////////////////////////////////
//// DirState

#[derive(Debug, PartialEq, Eq)]
enum DirPass {
    Entire,
    First,
    Second
}

#[derive(Debug)]
pub struct DirState<E: source::SourceExt> {
    /// The depth of this dir
    depth: usize,
    /// Content of this dir
    content: DirContent<E>,
    /// Current pass
    pass: DirPass,
    /// Current position
    position: Position<(), ()>,
}

impl<E: source::SourceExt> DirState<E> {
    fn get_initial_pass( opts_immut: &WalkDirOptionsImmut<E> ) -> DirPass {
        if opts_immut.content_order == ContentOrder::None {
            DirPass::Entire
        } else {
            DirPass::First
        }
    }

    fn init(&mut self, opts_immut: &WalkDirOptionsImmut<E>, sorter: &mut Option<FnCmp<E>>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) {

        if let Some(cmp) = sorter {
            // let opts_immut = &opts.immut;
            // let cmp = if let Some(ref mut cmp) = &mut opts.sorter {cmp} else {unreachable!()};

            self.content.load_all_and_sort(opts_immut, cmp, process_dent);
        }

    }

    /// New DirState from alone DirEntry
    pub fn new_once( raw_dent: DirEntry<E>, depth: usize, opts_immut: &WalkDirOptionsImmut<E>, sorter: &mut Option<FnCmp<E>>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) -> Self {
        let mut this = Self {
            depth,
            content: DirContent::<E>::new_once(raw_dent),
            pass: Self::get_initial_pass(opts_immut),
            position: Position::BeforeContent,
        };
        this.init(opts_immut, sorter, process_dent);
        this
    }

    /// New DirState from FsReadDir
    pub fn new( rd: wd::ResultInner<E::FsReadDir, E>, depth: usize, opts_immut: &WalkDirOptionsImmut<E>, sorter: &mut Option<FnCmp<E>>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) -> Self {
        let mut this = Self {
            depth,
            content: DirContent::<E>::new(rd),
            pass: Self::get_initial_pass(opts_immut),
            position: Position::BeforeContent,
        };
        this.init(opts_immut, sorter, process_dent);
        this
    }

    /// Load all remaining DirEntryRecord into tail of self.content.
    /// Doesn't change position.
    pub(crate) fn load_all(&mut self, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>) ) {
        self.content.load_all(opts_immut, process_dent)
    }

    /// Gets next record (according to content order and filter).
    /// Updates current position.
    fn next(&mut self, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>)) -> bool {
        loop {
            if let Some((first_pass, hidden)) = self.content.get_next_rec(opts_immut, process_dent) {
                let valid = match self.pass {
                    DirPass::Entire => true,
                    DirPass::First => first_pass,
                    DirPass::Second => !first_pass,
                };

                if valid && !hidden {
                    return true;
                };

                continue;
            };

            match self.pass {
                DirPass::Entire | DirPass::Second => {
                    self.position = Position::AfterContent;
                    return false;
                },
                DirPass::First => {
                    self.pass = DirPass::Second;
                    self.content.rewind();
                    continue;
                },
            };
        }
    }

    /// Next.
    /// Updates current position.
    pub fn next_position(&mut self, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>)) {
        if self.position == Position::AfterContent {
            return;
        };

        if self.next( opts_immut, process_dent ) {
            // Remember: at this state current rec must exist
            self.position = Position::Entry(());
        } else {
            self.position = Position::AfterContent;
        };
    }

    /// Get current state.
    /// Doesn't change position.
    pub fn get_current_position(&mut self) -> Position<ProcessedDirEntry<E>, wd::Error<E>> {
        match self.position {
            Position::BeforeContent => Position::BeforeContent,
            Position::Entry(_) => {
                // At this state current rec must exist
                let rec = self.content.get_current_rec().unwrap();
                match &mut rec.dent {
                    Ok(dent) => {
                        Position::Entry(ProcessedDirEntry{ dent: dent.clone(), is_dir: rec.is_dir })
                    },
                    Err(err) => {
                        Position::Error(wd::Error::from_inner(err.take(), self.depth))
                    },
                }
            },
            Position::AfterContent => Position::AfterContent,
            _ => unreachable!()
        }
    }

    /// Gets copy of entire dir, loading all remaining content if necessary (not considering content order).
    /// Doesn't change position.
    pub fn clone_all_content(&mut self, filter: ContentFilter, opts_immut: &WalkDirOptionsImmut<E>, process_dent: &impl (Fn(DirEntry<E>) -> Option<wd::ResultInner<ProcessedDirEntry<E>, E>>)) -> Vec<DirEntry<E>> {

        self.content.load_all(opts_immut, process_dent);
        
        match filter {
            ContentFilter::None => self.content.iter_content(|rec| rec.dent.as_ref().ok()).cloned().collect(),
            ContentFilter::DirsOnly => self.content.iter_content(|rec| rec.dent.as_ref().ok().filter(|_| rec.is_dir)).cloned().collect(),
            ContentFilter::FilesOnly => self.content.iter_content(|rec| rec.dent.as_ref().ok().filter(|_| !rec.is_dir)).cloned().collect(),
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}
