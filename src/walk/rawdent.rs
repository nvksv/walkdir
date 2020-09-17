use crate::error::{into_io_err, into_path_err, ErrorInner};
use crate::fs;
use crate::fs::{FsMetadata, FsFileType, FsPath, FsRootDirEntry, FsReadDirIterator};
use crate::wd::{self, FnCmp, IntoOk, IntoSome, Depth};
use crate::cp::ContentProcessor;

#[derive(Debug)]
enum RawDirEntryKind<E: fs::FsDirEntry> {
    Root { 
        fsdent: <E as fs::FsDirEntry>::RootDirEntry, 
    },
    DirEntry { 
        fsdent: E 
    },
}

/// A directory entry.
///
/// This is the type of value that is yielded from the iterators defined in
/// this crate.
///
/// On Unix systems, this type implements the [`DirEntryExt`] trait, which
/// provides efficient access to the inode number of the directory entry.
///
/// # Differences with `std::fs::DirEntry`
///
/// This type mostly mirrors the type by the same name in [`std::fs`]. There
/// are some differences however:
///
/// * All recursive directory iterators must inspect the entry's type.
/// Therefore, the value is stored and its access is guaranteed to be cheap and
/// successful.
/// * [`path`] and [`file_name`] return borrowed variants.
/// * If [`follow_links`] was enabled on the originating iterator, then all
/// operations except for [`path`] operate on the link target. Otherwise, all
/// operations operate on the symbolic link.
///
/// [`std::fs`]: https://doc.rust-lang.org/stable/std/fs/index.html
/// [`path`]: #method.path
/// [`file_name`]: #method.file_name
/// [`follow_links`]: struct.WalkDir.html#method.follow_links
/// [`DirEntryExt`]: trait.DirEntryExt.html
#[derive(Debug)]
pub struct RawDirEntry<E: fs::FsDirEntry> {
    /// Kind of this entry
    kind: RawDirEntryKind<E>,
    /// Is set when this entry was created from a symbolic link and the user
    /// expects to follow symbolic links.
    follow_link: bool,
    /// Cached metadata 
    metadata: E::Metadata,
}

impl<E: fs::FsDirEntry> RawDirEntry<E> {

    pub fn from_path(
        path: &E::Path,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<Self, E> {
        let (fsdent, metadata) = E::RootDirEntry::from_path( path, ctx )
            .map_err(|err| into_path_err(path, err))?;
        Self {
            kind: RawDirEntryKind::<E>::Root{ fsdent },
            follow_link: false,
            metadata,
        }.into_ok()
    }

    pub fn from_fsdent(
        fsdent: E,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<Self, E> {
        let metadata = fsdent.metadata( false, ctx )
            .map_err(into_io_err)?;
        Self {
            kind: RawDirEntryKind::<E>::DirEntry{ fsdent },
            follow_link: false,
            metadata,
        }.into_ok()
    }

    pub fn follow(self, ctx: &mut E::Context) -> wd::ResultInner<Self, E> {
        Self {
            kind:           self.kind,
            follow_link:    true,
            metadata:       self.metadata_follow(ctx)?,
        }.into_ok()
    }

    /// The full path that this entry represents.
    ///
    /// The full path is created by joining the parents of this entry up to the
    /// root initially given to [`WalkDir::new`] with the file name of this
    /// entry.
    ///
    /// Note that this *always* returns the path reported by the underlying
    /// directory entry, even when symbolic links are followed. To get the
    /// target path, use [`path_is_symlink`] to (cheaply) check if this entry
    /// corresponds to a symbolic link, and [`std::fs::read_link`] to resolve
    /// the target.
    ///
    /// [`WalkDir::new`]: struct.WalkDir.html#method.new
    /// [`path_is_symlink`]: struct.DirEntry.html#method.path_is_symlink
    /// [`std::fs::read_link`]: https://doc.rust-lang.org/stable/std/fs/fn.read_link.html
    pub fn path(&self) -> &E::Path {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. }        => fsdent.path(),
            RawDirEntryKind::DirEntry { fsdent, .. }    => fsdent.path(),
        }
    }

    /// The full path that this entry represents.
    ///
    /// Analogous to [`path`], but moves ownership of the path.
    ///
    /// [`path`]: struct.DirEntry.html#method.path
    pub fn pathbuf(&self) -> E::PathBuf {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. }        => fsdent.pathbuf(),
            RawDirEntryKind::DirEntry { fsdent, .. }    => fsdent.pathbuf(),
        }
    }

    /// Return the metadata for the file that this entry points to.
    ///
    /// This will follow symbolic links if and only if the [`WalkDir`] value
    /// has [`follow_links`] enabled.
    ///
    /// # Platform behavior
    ///
    /// This always calls [`std::fs::symlink_metadata`].
    ///
    /// If this entry is a symbolic link and [`follow_links`] is enabled, then
    /// [`std::fs::metadata`] is called instead.
    ///
    /// # Errors
    ///
    /// Similar to [`std::fs::metadata`], returns errors for path values that
    /// the program does not have permissions to access or if the path does not
    /// exist.
    ///
    /// [`WalkDir`]: struct.WalkDir.html
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    /// [`std::fs::metadata`]: https://doc.rust-lang.org/std/fs/fn.metadata.html
    /// [`std::fs::symlink_metadata`]: https://doc.rust-lang.org/stable/std/fs/fn.symlink_metadata.html
    pub fn metadata(
        &self, 
    ) -> &E::Metadata {
        &self.metadata
    }

    pub(crate) fn metadata_follow(
        &self,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<E::Metadata, E> {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. } => {
                fsdent.metadata( true, ctx )
            },
            RawDirEntryKind::DirEntry { fsdent, .. } => {
                fsdent.metadata( true, ctx )
            },
        }.map_err(into_io_err)
    }

    // fn metadata_internal(&self, follow_link: bool) -> wd::ResultInner<E::Metadata, E> {
    //     if follow_link {
    //         E::metadata(&self.path)
    //     } else {
    //         E::symlink_metadata_internal(self, &self.ext)
    //     }
    //     .map_err(ErrorInner::<E>::from_io)
    // }

    /// Return the file type for the file that this entry points to.
    ///
    /// If this is a symbolic link and [`follow_links`] is `true`, then this
    /// returns the type of the target.
    ///
    /// This never makes any system calls.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    pub fn file_type(
        &self
    ) -> E::FileType {
        self.metadata.file_type()
    }

    /// Return the file type for the file that this entry points to.
    ///
    /// If this is a symbolic link and [`follow_links`] is `true`, then this
    /// returns the type of the target.
    ///
    /// This never makes any system calls.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }

    /// Return the file type for the file that this entry points to.
    ///
    /// If this is a symbolic link and [`follow_links`] is `true`, then this
    /// returns the type of the target.
    ///
    /// This never makes any system calls.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn follow_link(&self) -> bool {
        self.follow_link
    }

    /// Return the file name of this entry.
    ///
    /// If this entry has no file name (e.g., `/`), then the full path is
    /// returned.
    pub fn file_name(&self) -> E::FileName {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. } => {
                fsdent.file_name()
            },
            RawDirEntryKind::DirEntry { fsdent, .. } => {
                fsdent.file_name()
            },
        }
    }

    // fn from_path_internal(
    //     path: &E::Path,
    //     ctx: &mut E::Context,
    //     follow_link: bool,
    // ) -> wd::ResultInner<Self, E> {
    //     let md = E::metadata_from_path( path, follow_link, ctx ).map_err(|e| into_path_err(path, e))?;
    //     let pb = path.as_ref().to_path_buf();

    //     Self { 
    //         kind: RawDirEntryKind::Root { 
    //             path: pb,  
    //             metadata: md,
    //         },
    //         follow_link, 
    //     }.into_ok()
    // }

    // pub fn from_path(
    //     path: &E::Path,
    //     ctx: &mut E::Context,
    // ) -> wd::ResultInner<ReadDir<E>, E> {
    //     let rawdent = Self::from_path_internal( path, ctx, false )?;
    //     ReadDir::<E>::new_once(rawdent).into_ok()
    // }

    pub fn read_dir(
        &self, 
        ctx: &mut E::Context
    ) -> wd::ResultInner<ReadDir<E>, E> {
        let rd = match &self.kind {
            RawDirEntryKind::Root { fsdent, .. } => {
                fsdent.read_dir( ctx )
            },
            RawDirEntryKind::DirEntry { fsdent, .. } => {
                fsdent.read_dir( ctx )
            },
        }.map_err(into_io_err)?;
        ReadDir::<E>::new(rd).into_ok()
    }

    fn as_fsdent_md(&self) -> Option<(&E, &E::Metadata)> {
        match &self.kind {
            RawDirEntryKind::Root { .. } => None,
            RawDirEntryKind::DirEntry { ref fsdent, .. } => (fsdent, &self.metadata).into_some(),
        }
    }

    pub fn call_cmp(a: &Self, b: &Self, cmp: &mut FnCmp<E>) -> std::cmp::Ordering {
        let ap = a.as_fsdent_md().unwrap();
        let bp = b.as_fsdent_md().unwrap();
        cmp(ap, bp)
    }

    pub fn make_content_item<CP: ContentProcessor<E>>(
        &self,
        content_processor: &CP,
        is_dir: bool,
        depth: Depth,
    ) -> Option<CP::Item> {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. } => {
                content_processor.process_root_direntry( fsdent, is_dir, self.follow_link, depth )
            },
            RawDirEntryKind::DirEntry { fsdent, .. } => {
                content_processor.process_direntry( fsdent, is_dir, self.follow_link, depth )
            },
        }
    }

    // pub fn error_inner_from_entry(&self, err: E::Error) -> ErrorInner<E> {
    //     ErrorInner::<E>::from_entry(self.get_fs_dir_entry().unwrap(), err)
    // }

    pub fn fingerprint(
        &self,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<E::DirFingerprint, E> {
        match &self.kind {
            RawDirEntryKind::Root { fsdent, .. } => {
                fsdent.fingerprint( ctx )
            },
            RawDirEntryKind::DirEntry { fsdent, .. } => {
                fsdent.fingerprint( ctx )
            },
        }.map_err(into_io_err)
    }

}

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
pub enum ReadDir<E: fs::FsDirEntry> {
    /// The single item (used for root)
    Once { item: Option<RawDirEntry<E>> },

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
    Opened { rd: E::ReadDir },

    /// A closed handle.
    ///
    /// All remaining directory entries are read into memory.
    Closed,

    /// Error on handle creating
    Error(Option<ErrorInner<E>>),
}

impl<E: fs::FsDirEntry> ReadDir<E> {
    
    pub fn new_once(
        path: &E::Path,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<Self, E> {
        let raw_dent = RawDirEntry::<E>::from_path( path, ctx )?;
        Self::Once { 
            item: raw_dent.into_some() 
        }.into_ok()
    }

    fn new(rd: E::ReadDir) -> Self {
        // match rd {
        //     Ok(rd) => Self::Opened { rd },
        //     Err(err) => Self::Error( Some(err) ),
        // }
        Self::Opened { rd }
    }

    pub fn collect_all<T>(
        &mut self,
        process_rawdent: &mut impl (FnMut(wd::ResultInner<RawDirEntry<E>, E>, &mut E::Context) -> Option<T>),
        ctx: &mut E::Context,
    ) -> Vec<T> {
        match *self {
            ReadDir::Opened { rd } => {
                let entries = ReadDirOpenedIterator::new( rd, process_rawdent, ctx )
                    .filter_map(|opt| opt)
                    .collect();
                *self = ReadDir::<E>::Closed;
                entries
            },
            ReadDir::Once { item } => {
                let entries = match item {
                    Some(raw_dent) => match process_rawdent(Ok(raw_dent), ctx) {
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
            ReadDir::Error(ref mut oerr) => { 
                match oerr.take() {
                    Some(err) => match process_rawdent(Err(err), ctx) {
                        Some(e) => vec![e],
                        None => vec![],
                    },
                    None => vec![],
                }
            },
        }
    }

    #[inline(always)]
    pub fn next(
        &mut self,
        ctx: &mut E::Context,
    ) -> Option<wd::ResultInner<RawDirEntry<E>, E>> {
        match *self {
            ReadDir::Once { ref mut item } => {
                item.take().map(Ok)
            },
            ReadDir::Opened { ref mut rd } => {
                match rd.next_entry(ctx)? {
                    Ok(fsdent)  => RawDirEntry::<E>::from_fsdent( fsdent, ctx ),
                    Err(e)      => Err(into_io_err(e)),
                }.into_some()
            },
            ReadDir::Closed => {
                None
            },
            ReadDir::Error(ref mut err) => {
                err.take().map(Err)
            },
        }
    }
}

/////////////////////////////////////////////////////////////////////////
//// ReadDirOpenedIterator

struct ReadDirOpenedIterator<'c, E, P, T> 
where
    E: fs::FsDirEntry,
    P: (FnMut(wd::ResultInner<RawDirEntry<E>, E>, &mut E::Context) -> Option<T>),
{
    rd: E::ReadDir,
    process_rawdent: &'c mut P,
    ctx: &'c mut E::Context,
}

impl<'c, E, P, T> ReadDirOpenedIterator<'c, E, P, T> 
where
    E: fs::FsDirEntry,
    P: (FnMut(wd::ResultInner<RawDirEntry<E>, E>, &mut E::Context) -> Option<T>),
{
    fn new(
        rd: E::ReadDir,
        process_rawdent: &'c mut P,
        ctx: &'c mut E::Context,
    ) -> Self {
        Self {
            rd,
            process_rawdent,
            ctx,
        }
    }
}

impl<'c, E, P, T> Iterator for ReadDirOpenedIterator<'c, E, P, T> 
where
    E: fs::FsDirEntry,
    P: (FnMut(wd::ResultInner<RawDirEntry<E>, E>, &mut E::Context) -> Option<T>),
{
    type Item = Option<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let rrawdent = match self.rd.next_entry(self.ctx)? {
            Ok(fsdent)  => RawDirEntry::<E>::from_fsdent( fsdent, self.ctx ),
            Err(e)      => Err(into_io_err(e)),
        };
        
        let t = (self.process_rawdent)( rrawdent, self.ctx );

        Some(t)
    }
}
