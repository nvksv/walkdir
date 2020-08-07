use std::cmp;
use std::io;
use std::vec;


use crate::wd::{self, ContentFilter, Position, DeviceNum};
//use crate::rawdent::RawDirEntry;
use crate::rawdent::RawDirEntry;
use crate::dent::DirEntry;
#[cfg(unix)]
use crate::dent::DirEntryExt;
use crate::error::ErrorInner;
use crate::source::{self, SourceFsFileType, SourceFsMetadata, SourcePath};
use crate::dir::{DirState, FlatDirEntry};
use crate::opts::{WalkDirOptions, WalkDirOptionsImmut};

/// Like try, but for iterators that return [`Option<Result<_, _>>`].
///
/// [`Option<Result<_, _>>`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
macro_rules! ortry {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Some(Err(From::from(err))),
        }
    };
}

/// Like try, but for iterators that return [`Option<Result<_, _>>`].
///
/// [`Option<Result<_, _>>`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
macro_rules! rtry {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Err(From::from(err)),
        }
    };
}

macro_rules! process_dent {
    ($self:expr, $depth:expr) => {
        ((|depth, opts_immut, root_device, ancestors| move |raw_dent: RawDirEntry<E>| Self::process_rawdent(raw_dent, depth, opts_immut, root_device, ancestors))($depth, &$self.opts.immut, &$self.root_device, &$self.ancestors))
    };
}




/////////////////////////////////////////////////////////////////////////
//// Ancestor

/// An ancestor is an item in the directory tree traversed by walkdir, and is
/// used to check for loops in the tree when traversing symlinks.
#[derive(Debug)]
struct Ancestor<E: source::SourceExt> {
    /// The path of this ancestor.
    path: E::PathBuf,
    /// Extension part
    ext: E::AncestorExt,
}

impl<E: source::SourceExt> Ancestor<E> {
    /// Create a new ancestor from the given directory path.
    fn new(raw_dent: &RawDirEntry<E>) -> wd::ResultInner<Self, E> {
        Ok(Self {
            path: raw_dent.path().to_path_buf(),
            ext: E::ancestor_new(raw_dent).map_err(|err| ErrorInner::<E>::from_io(err))?,
        })
    }

    /// Returns true if and only if the given open file handle corresponds to
    /// the same directory as this ancestor.
    fn is_same(&self, child: &E::SameFileHandle) -> io::Result<bool> {
        E::is_same(&self.path, &self.ext, child)
    }
}








/////////////////////////////////////////////////////////////////////////
//// IntoIter

#[derive(Debug, PartialEq, Eq)]
enum TransitionState {
    None,
    BeforePushDown,
    BeforePopUp,
    AfterPopUp,
}

/// An iterator for recursively descending into a directory.
///
/// A value with this type must be constructed with the [`WalkDir`] type, which
/// uses a builder pattern to set options such as min/max depth, max open file
/// descriptors and whether the iterator should follow symbolic links. After
/// constructing a `WalkDir`, call [`.into_iter()`] at the end of the chain.
///
/// The order of elements yielded by this iterator is unspecified.
///
/// [`WalkDir`]: struct.WalkDir.html
/// [`.into_iter()`]: struct.WalkDir.html#into_iter.v
#[derive(Debug)]
pub struct WalkDirIterator<E: source::SourceExt = source::DefaultSourceExt> {
    /// Options specified in the builder. Depths, max fds, etc.
    opts: WalkDirOptions<E>,
    /// The start path.
    ///
    /// This is only `Some(...)` at the beginning. After the first iteration,
    /// this is always `None`.
    start: Option<E::PathBuf>,
    /// A stack of open (up to max fd) or closed handles to directories.
    /// An open handle is a plain [`fs::ReadDir`] while a closed handle is
    /// a `Vec<fs::DirEntry>` corresponding to the as-of-yet consumed entries.
    ///
    /// [`fs::ReadDir`]: https://doc.rust-lang.org/stable/std/fs/struct.ReadDir.html
    states: Vec<DirState<E>>,
    /// before push down / after pop up
    transition_state: TransitionState,
    /// A stack of file paths.
    ///
    /// This is *only* used when [`follow_links`] is enabled. In all other
    /// cases this stack is empty.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    ancestors: Vec<Ancestor<E>>,
    /// An index into `states` that points to the oldest open directory
    /// handle. If the maximum fd limit is reached and a new directory needs to
    /// be read, the handle at this index is closed before the new directory is
    /// opened.
    oldest_opened: usize,
    /// The current depth of iteration (the length of the stack at the
    /// beginning of each iteration).
    depth: usize,
    /// The device of the root file path when the first call to `next` was
    /// made.
    ///
    /// If the `same_file_system` option isn't enabled, then this is always
    /// `None`. Conversely, if it is enabled, this is always `Some(...)` after
    /// handling the root path.
    root_device: Option<DeviceNum>,
    /// Extension part.
    ext: E::IntoIterExt,
}

impl<E: source::SourceExt> WalkDirIterator<E> {
    /// Make new
    pub fn new( opts: WalkDirOptions<E>, root: E::PathBuf, ext: E ) -> Self {
        Self {
            opts: opts,
            start: Some(root),
            states: vec![],
            transition_state: TransitionState::None,
            ancestors: vec![],
            oldest_opened: 0,
            depth: 0,
            root_device: None,
            ext: E::intoiter_new(ext),
        }
    }

    // Follow symlinks and check same_file_system. Also determine is_dir flag.
    // - Some(Ok((dent, is_dir))) -- normal entry to yielding
    // - Some(Err(_)) -- some error occured
    // - None -- entry must be ignored
    fn process_rawdent(raw_dent: RawDirEntry<E>, depth: usize, opts_immut: &WalkDirOptionsImmut<E>, root_device: &Option<DeviceNum>, ancestors: &Vec<Ancestor<E>>) -> Option<wd::ResultInner<FlatDirEntry<E>, E>> {
        
        let (new_raw_dent, loop_link, follow_link) = if raw_dent.file_type().is_symlink() && opts_immut.follow_links {
            let (new_raw_dent, loop_link) = ortry!(Self::follow(raw_dent, ancestors));
            (new_raw_dent, loop_link, true)
        } else {
            (raw_dent, None, false)
        };

        let mut is_normal_dir = !new_raw_dent.file_type().is_symlink() && new_raw_dent.is_dir();

        if is_normal_dir {
            if opts_immut.same_file_system && depth > 0 {
                if ! ortry!(Self::is_same_file_system(root_device, &new_raw_dent)) {
                    return None;
                };
            };
        } else if depth == 0 && new_raw_dent.file_type().is_symlink() {
            // As a special case, if we are processing a root entry, then we
            // always follow it even if it's a symlink and follow_links is
            // false. We are careful to not let this change the semantics of
            // the DirEntry however. Namely, the DirEntry should still respect
            // the follow_links setting. When it's disabled, it should report
            // itself as a symlink. When it's enabled, it should always report
            // itself as the target.
            let md = ortry!(E::metadata(new_raw_dent.path()).map_err(|err| {
                ErrorInner::<E>::from_path(new_raw_dent.path().to_path_buf(), err)
            }));
            is_normal_dir = md.file_type().is_dir();
        };

        Some(Ok(FlatDirEntry{ 
            raw: new_raw_dent, 
            is_dir: is_normal_dir, 
            follow_link,
            loop_link, 
        }))
    }

    fn init(&mut self, root: E::PathBuf) -> wd::ResultInner<(), E> {
        if self.opts.immut.same_file_system {
            let result = E::device_num(&root)
                .map_err(|e| ErrorInner::<E>::from_path(root.clone(), e));
            self.root_device = Some(rtry!(result));
        }
        let raw_dent = rtry!(RawDirEntry::<E>::from_path(root, false));

        self.push_root(raw_dent, 0)?;

        Ok(())
    }

    fn push_root(&mut self, dent: RawDirEntry<E>, new_depth: usize) -> wd::ResultInner<(), E> {

        let state = DirState::<E>::new_once( dent.clone(), new_depth, &self.opts.immut, &mut self.opts.sorter, &process_dent!(self, new_depth) );

        self.states.push(state);

        Ok(())
    }

    fn push_dir(&mut self, dent: DirEntry<E>, new_depth: usize) -> wd::ResultInner<(), E>  {

        let flat = dent.into_flat();

        // flat_ref is ref to current position of current state, so we can update if safely

        // This is safe as we makes any changes strictly AFTER using dent_ptr.
        // Neither E::read_dir nor Ancestor::new

        assert!(flat.loop_link.is_none());

        // Make room for another open file descriptor if we've hit the max.
        let free = self.states.len().checked_sub(self.oldest_opened).unwrap();
        if free == self.opts.immut.max_open {
            let state = self.states.get_mut(self.oldest_opened).unwrap();
            state.load_all(&self.opts.immut, &process_dent!(self, new_depth) );
        }

        // Open a handle to reading the directory's entries.
        let rd = E::read_dir(&flat.raw, flat.raw.path()).map_err(|err| ErrorInner::<E>::from_path(flat.raw.path().to_path_buf(), err));
        let state = DirState::<E>::new( rd, new_depth, &self.opts.immut, &mut self.opts.sorter, &process_dent!(self, new_depth) );

        if self.opts.immut.follow_links {
            let ancestor = Ancestor::new(&flat.raw)?;
            self.ancestors.push(ancestor);
        };

        self.states.push(state);

        // If we had to close out a previous directory stream, then we need to
        // increment our index the oldest still-open stream. We do this only
        // after adding to our stack, in order to ensure that the oldest_opened
        // index remains valid. The worst that can happen is that an already
        // closed stream will be closed again, which is a no-op.
        //
        // We could move the close of the stream above into this if-body, but
        // then we would have more than the maximum number of file descriptors
        // open at a particular point in time.
        if free == self.opts.immut.max_open {
            // Unwrap is safe here because self.oldest_opened is guaranteed to
            // never be greater than `self.stack_list.len()`, which implies
            // that the subtraction won't underflow and that adding 1 will
            // never overflow.
            self.oldest_opened = self.oldest_opened.checked_add(1).unwrap();
        };

        Ok(())
    }

    fn pop_dir(&mut self) {
        self.states.pop().expect("BUG: cannot pop from empty stack");
        if self.opts.immut.follow_links {
            self.ancestors.pop().expect("BUG: list/path stacks out of sync");
        }
        // If everything in the stack is already closed, then there is
        // room for at least one more open descriptor and it will
        // always be at the top of the stack.
        self.oldest_opened = cmp::min(self.oldest_opened, self.states.len());
    }

    /// Skips the current directory.
    ///
    /// This causes the iterator to stop traversing the contents of the least
    /// recently yielded directory. This means any remaining entries in that
    /// directory will be skipped (including sub-directories).
    ///
    /// Note that the ergonomics of this method are questionable since it
    /// borrows the iterator mutably. Namely, you must write out the looping
    /// condition manually. For example, to skip hidden entries efficiently on
    /// unix systems:
    ///
    /// ```no_run
    /// use walkdir::{DirEntry, WalkDir, WalkDirIter, ClassicWalkDirIter};
    ///
    /// fn is_hidden(entry: &DirEntry) -> bool {
    ///     entry.file_name()
    ///          .to_str()
    ///          .map(|s| s.starts_with("."))
    ///          .unwrap_or(false)
    /// }
    ///
    /// let mut it = WalkDir::new("foo").into_classic();
    /// loop {
    ///     let entry = match it.next() {
    ///         None => break,
    ///         Some(Err(err)) => panic!("ERROR: {}", err),
    ///         Some(Ok(entry)) => entry,
    ///     };
    ///     if is_hidden(&entry) {
    ///         if entry.file_type().is_dir() {
    ///             it.skip_current_dir();
    ///         }
    ///         continue;
    ///     }
    ///     println!("{}", entry.path().display());
    /// }
    /// ```
    ///
    /// You may find it more convenient to use the [`filter_entry`] iterator
    /// adapter. (See its documentation for the same example functionality as
    /// above.)
    ///
    /// [`filter_entry`]: #method.filter_entry
    pub fn skip_current_dir(&mut self) {
        if let Some(cur_state) = self.states.last_mut() {
            cur_state.skip_all();
            self.transition_state = TransitionState::None;
        }
    }



    fn follow(raw_dent: RawDirEntry<E>, ancestors: &Vec<Ancestor<E>>) -> wd::ResultInner<(RawDirEntry<E>, Option<usize>), E> {
        let dent = RawDirEntry::<E>::from_path(
            raw_dent.path().to_path_buf(),
            true,
        )?;

        let loop_link = if dent.is_dir() && !ancestors.is_empty(){
            Self::check_loop(dent.path(), ancestors)?
        } else {
            None
        };

        Ok((dent, loop_link))
    }

    fn check_loop<P: AsRef<E::Path>>(child: P, ancestors: &Vec<Ancestor<E>>) -> wd::ResultInner<Option<usize>, E> {
        
        let hchild = E::get_handle(&child).map_err(ErrorInner::<E>::from_io)?;

        for (index, ancestor) in ancestors.iter().enumerate().rev() {
            let is_same = ancestor.is_same(&hchild).map_err(ErrorInner::<E>::from_io)?;
            if is_same {
                return Ok(Some(index));
            }
        }

        Ok(None)

    }

    fn make_loop_error<P: AsRef<E::Path>>(ancestors: &Vec<Ancestor<E>>, index: usize, child: P) -> ErrorInner<E> {
        
        let ancestor = ancestors.get(index).unwrap();
        
        ErrorInner::<E>::from_loop(
            &ancestor.path,
            child.as_ref()
        )

    }

    fn is_same_file_system(root_device: &Option<DeviceNum>, dent: &RawDirEntry<E>) -> wd::ResultInner<bool, E> {
        let dent_device = E::device_num(dent.path())
            .map_err(|err| ErrorInner::<E>::from_entry(dent, err))?;
        Ok(root_device
            .map(|d| d == dent_device)
            .expect("BUG: called is_same_file_system without root device"))
    }


    /// Gets content of current dir
    pub fn get_current_dir_content(&mut self, filter: ContentFilter) -> Option<Vec<DirEntry<E>>> {
        let cur_state = match self.states.last_mut() {
            Some(state) => state,
            None => return None,
        };

        let content = cur_state.clone_all_content(filter, &self.opts.immut, &process_dent!(self, cur_state.depth()) );
        
        Some(content)
    }

}


impl<E: source::SourceExt> Iterator for WalkDirIterator<E> {
    type Item = Position<DirEntry<E>, DirEntry<E>, wd::Error<E>>;
    /// Advances the iterator and returns the next value.
    ///
    /// # Errors
    ///
    /// If the iterator fails to retrieve the next value, this method returns
    /// an error value. The error will be wrapped in an Option::Some.
    fn next(&mut self) -> Option<Self::Item> {

        fn get_parent_dent<E>(this: &mut WalkDirIterator<E>, cur_depth: usize) -> DirEntry<E> where E: source::SourceExt {
            let prev_state = this.states.get_mut(cur_depth-1).unwrap();
            match prev_state.get_current_position() {
                Position::Entry(rflat) => {
                    return rflat.into_dent();
                },
                _ => unreachable!(),
            }
        }

        // Initial actions
        if let Some(start) = self.start.take() {
            if let Err(e) = self.init(start) {
                return Some(Position::Error(wd::Error::from_inner(e, 0)));
                // Here self.states is empty, so next call will always return None.
            };
        }

        loop {
            let cur_depth = match self.states.len() {
                0 => unreachable!(),
                len @ _ => (len-1),
            }; 

            let cur_state = self.states.get_mut(cur_depth).unwrap();

            match cur_state.get_current_position() {
                Position::BeforeContent(_) => {
                    assert!( self.transition_state == TransitionState::None );
                    
                    cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );

                    if cur_depth == 0 {
                        continue;
                    }

                    return Some(Position::BeforeContent(get_parent_dent(self, cur_depth)));
                }, 
                Position::Entry(rflat) => {
                    let allow_yield = !rflat.hidden() && (cur_depth >= self.opts.immut.min_depth) && (if rflat.loop_link().is_some() {self.opts.immut.yield_loop_links} else {true});

                    if rflat.is_dir() {
                        let allow_push = cur_depth < self.opts.immut.max_depth;

                        match self.transition_state {
                            TransitionState::None => {
                                if allow_push {
                                    self.transition_state = TransitionState::BeforePushDown;
                                } else {
                                    self.transition_state = TransitionState::AfterPopUp;
                                }

                                if !self.opts.immut.contents_first && allow_yield {
                                    return Some(Position::Entry(rflat.into_dent()));
                                };
                            },
                            TransitionState::BeforePushDown => {
                                self.transition_state = TransitionState::None;

                                if let Some(loop_depth) = rflat.loop_link() {
                                    self.transition_state = TransitionState::AfterPopUp;
                                    if !self.opts.immut.yield_loop_links {
                                        let err = Self::make_loop_error(&self.ancestors, loop_depth, rflat.path());
                                        return Some(Position::Error(wd::Error::from_inner(err, cur_depth)));
                                    }
                                    continue
                                }

                                let dent = rflat.into_dent();
                                match self.push_dir( dent, cur_depth+1 ) {
                                    Ok(_) => {},
                                    Err(err) => {
                                        self.transition_state = TransitionState::AfterPopUp;
                                        return Some(Position::Error(wd::Error::from_inner(err, cur_depth)));
                                    }
                                }
                            },
                            TransitionState::AfterPopUp => {
                                self.transition_state = TransitionState::None;

                                if self.opts.immut.contents_first && allow_yield {
                                    let dent = rflat.into_dent();
                                    cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );
                                    return Some(Position::Entry(dent));
                                } else {
                                    cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );
                                };
                            },
                            _ => unreachable!(),
                        };

                    } else {
                        assert!( self.transition_state == TransitionState::None );

                        if allow_yield {
                            let dent = rflat.into_dent();
                            cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );
                            return Some(Position::Entry(dent));
                        } else {
                            cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );
                        }
                    }
                },
                Position::Error(rerr) => {
                    assert!( self.transition_state == TransitionState::None );

                    let err = rerr.into_error();
                    cur_state.next_position( &self.opts.immut, &process_dent!(self, cur_depth) );
                    return Some(Position::Error(err));
                },
                Position::AfterContent => {
                    if cur_depth == 0 {
                        return None;
                    }

                    match self.transition_state {
                        TransitionState::None => {
                            self.transition_state = TransitionState::BeforePopUp;
                            return Some(Position::AfterContent);
                        },
                        TransitionState::BeforePopUp => {
                            self.pop_dir();
                            self.transition_state = TransitionState::AfterPopUp;
                        },
                        _ => unreachable!()
                    }
                }
            }
        }

    }
}





