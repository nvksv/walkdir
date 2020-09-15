use crate::error::{into_io_err, into_path_err, ErrorInner};
use crate::fs;
use crate::fs::{FsMetadata, FsFileType, FsPath};
use crate::wd::{self, FnCmp, IntoErr, IntoOk, IntoSome, Depth};
use crate::cp::ContentProcessor;

#[derive(Debug)]
enum RawDirEntryKind<E: fs::FsDirEntry> {
    FromPath { 
        path: E::PathBuf,
        metadata: E::Metadata,
    },
    FromFsDirEntry { 
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
}

impl<E: fs::FsDirEntry> RawDirEntry<E> {
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
            RawDirEntryKind::FromPath { path, .. } => path,
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => fsdent.path(),
        }
    }

    /// The full path that this entry represents.
    ///
    /// Analogous to [`path`], but moves ownership of the path.
    ///
    /// [`path`]: struct.DirEntry.html#method.path
    pub fn pathbuf(&self) -> E::PathBuf {
        match &self.kind {
            RawDirEntryKind::FromPath { path, .. } => path.clone(),
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => fsdent.pathbuf(),
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
        ctx: &mut E::Context
    ) -> wd::ResultInner<E::Metadata, E> {
        match &self.kind {
            RawDirEntryKind::FromPath { metadata, .. } => {
                metadata.clone().into_ok()
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.metadata( self.follow_link, ctx )
            },
        }.map_err(into_io_err)
    }

    pub(crate) fn metadata_follow(
        &self,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<E::Metadata, E> {
        match &self.kind {
            RawDirEntryKind::FromPath { path, metadata, .. } => {
                if self.follow_link {
                    metadata.clone().into_ok()
                } else {
                    E::metadata_from_path( &path, true, ctx )
                }
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
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
    pub fn file_type(&self) -> E::FileType {
        match &self.kind {
            RawDirEntryKind::FromPath { metadata, .. } => {
                metadata.file_type()
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.file_type()
            },
        }
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
        self.file_type().is_symlink()
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
            RawDirEntryKind::FromPath { path, .. } => {
                E::file_name_from_path( &path ).unwrap_or_else( |_| path.file_name().unwrap() )
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.file_name()
            },
        }
    }

    /// Returns true if and only if this entry points to a directory.
    pub fn is_dir(&self) -> bool {
        match &self.kind {
            RawDirEntryKind::FromPath { metadata, .. } => {
                E::metadata_is_dir( &metadata )
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.is_dir()
            },
        }
    }

    fn from_fsentry(fsdent: E) -> wd::ResultInner<Self, E> {
        Self { 
            kind: RawDirEntryKind::FromFsDirEntry { fsdent },
            follow_link: false, 
        }.into_ok()
    }

    fn from_path_internal(
        path: &E::Path,
        ctx: &mut E::Context,
        follow_link: bool,
    ) -> wd::ResultInner<Self, E> {
        let md = E::metadata_from_path( path, follow_link, ctx ).map_err(|e| into_path_err(path, e))?;
        let pb = path.as_ref().to_path_buf();

        Self { 
            kind: RawDirEntryKind::FromPath { 
                path: pb,  
                metadata: md,
            },
            follow_link, 
        }.into_ok()
    }

    pub fn from_path(
        path: &E::Path,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<ReadDir<E>, E> {
        let rawdent = Self::from_path_internal( path, ctx, false )?;
        ReadDir::<E>::new_once(rawdent).into_ok()
    }

    pub fn read_dir(
        &self, 
        ctx: &mut E::Context
    ) -> wd::ResultInner<ReadDir<E>, E> {
        let rd = match &self.kind {
            RawDirEntryKind::FromPath { path, .. } => {
                E::read_dir_from_path( &path, ctx )
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.read_dir( ctx )
            },
        }.map_err(into_io_err)?;
        ReadDir::<E>::new(rd).into_ok()
    }

    pub fn follow(&self, ctx: &mut E::Context) -> wd::ResultInner<Self, E> {
        Self::from_path_internal( self.path(), ctx, true )
    }

    fn get_fs_dir_entry(&self) -> Option<&E> {
        match &self.kind {
            RawDirEntryKind::FromFsDirEntry { ref fsdent, .. } => Some(fsdent),
            RawDirEntryKind::FromPath { .. } => None,
        }
    }

    pub fn call_cmp(a: &Self, b: &Self, cmp: &mut FnCmp<E>) -> std::cmp::Ordering {
        let fs_a = a.get_fs_dir_entry().unwrap();
        let fs_b = b.get_fs_dir_entry().unwrap();
        cmp(fs_a, fs_b)
    }

    pub fn make_content_item<CP: ContentProcessor<E>>(
        &self,
        content_processor: &CP,
        is_dir: bool,
        depth: Depth,
    ) -> Option<CP::Item> {
        match &self.kind {
            RawDirEntryKind::FromPath { path, .. } => {
                content_processor.process_direntry_from_path( &path, is_dir, self.follow_link, depth )
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                content_processor.process_direntry( fsdent, is_dir, self.follow_link, depth )
            },
        }
    }

    pub fn error_inner_from_entry(&self, err: E::Error) -> ErrorInner<E> {
        ErrorInner::<E>::from_entry(self.get_fs_dir_entry().unwrap(), err)
    }

    fn fingerprint(
        &self,
        ctx: &mut E::Context,
    ) -> wd::ResultInner<E::DirFingerprint, E> {
        match &self.kind {
            RawDirEntryKind::FromPath { path, .. } => {
                E::fingerprint_from_path( path, ctx )
            },
            RawDirEntryKind::FromFsDirEntry { fsdent, .. } => {
                fsdent.fingerprint()
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
    fn new_once(raw_dent: RawDirEntry<E>) -> Self {
        Self::Once { item: raw_dent.into_some() }
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
        process_rawdent: &mut impl (FnMut(wd::ResultInner<RawDirEntry<E>, E>) -> Option<T>),
    ) -> Vec<T> {
        match *self {
            ReadDir::Opened { ref mut rd } => {
                let entries = rd
                    .map(Self::fsdent_into_raw)
                    .map(process_rawdent)
                    .filter_map(|opt| opt)
                    .collect();
                *self = ReadDir::<E>::Closed;
                entries
            }
            ReadDir::Once { ref mut item } => {
                let entries = match item.take() {
                    Some(raw_dent) => match process_rawdent(Ok(raw_dent)) {
                        Some(t) => vec![t],
                        None => vec![],
                    },
                    None => vec![],
                };
                *self = ReadDir::<E>::Closed;
                entries
            }
            ReadDir::Closed => vec![],
            ReadDir::Error(ref mut oerr) => match oerr.take() {
                Some(err) => match process_rawdent(Err(err)) {
                    Some(e) => vec![e],
                    None => vec![],
                },
                None => vec![],
            },
        }
    }

    fn fsdent_into_raw(r_ent: Result<E, E::Error>) -> wd::ResultInner<RawDirEntry<E>, E> {
        match r_ent {
            Ok(ent) => RawDirEntry::<E>::from_fsentry(ent),
            Err(err) => into_io_err(err).into_err(),
        }
    }
}

impl<E: fs::FsDirEntry> Iterator for ReadDir<E> {
    type Item = wd::ResultInner<RawDirEntry<E>, E>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            ReadDir::Once { ref mut item } => item.take().map(Ok),
            ReadDir::Opened { ref mut rd } => rd.next().map(Self::fsdent_into_raw),
            ReadDir::Closed => None,
            ReadDir::Error(ref mut err) => err.take().map(Err),
        }
    }
}
