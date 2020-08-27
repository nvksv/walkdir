use crate::error::{into_io_err, into_path_err, ErrorInner};
use crate::storage;
use crate::storage::{StorageDirEntry, StorageFileType, StorageMetadata, StoragePath};
use crate::wd::{self, FnCmp, IntoErr, IntoOk, IntoSome};

#[derive(Debug)]
enum RawDirEntryKind<E: storage::StorageExt> {
    FromPath { path: E::PathBuf },
    FromFsDirEntry { fsdent: E::DirEntry, path: E::PathBuf },
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
pub struct RawDirEntry<E: storage::StorageExt = storage::DefaultStorageExt> {
    // /// The path as reported by the [`fs::ReadDir`] iterator (even if it's a
    // /// symbolic link).
    // ///
    // /// [`fs::ReadDir`]: https://doc.rust-lang.org/stable/std/fs/struct.ReadDir.html
    // path: E::PathBuf,
    /// Is set when this entry was created from a symbolic link and the user
    /// expects to follow symbolic links.
    follow_link: bool,
    /// The file type. Necessary for recursive iteration, so store it.
    ty: E::FileType,
    /// Kind of this entry
    kind: RawDirEntryKind<E>,
    /// The source-specific part.
    ext: E::RawDirEntryExt,
}

impl<E: storage::StorageExt> RawDirEntry<E> {
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
        match self.kind {
            RawDirEntryKind::FromPath { ref path, .. } => path,
            RawDirEntryKind::FromFsDirEntry { ref path, .. } => path,
        }
    }

    /// The full path that this entry represents.
    ///
    /// Analogous to [`path`], but moves ownership of the path.
    ///
    /// [`path`]: struct.DirEntry.html#method.path
    pub fn into_path(self) -> E::PathBuf {
        match self.kind {
            RawDirEntryKind::FromPath { path, .. } => path,
            RawDirEntryKind::FromFsDirEntry { path, .. } => path,
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
    pub fn metadata(&self, ctx: &mut E::IteratorExt) -> wd::ResultInner<E::Metadata, E> {
        E::metadata(self.path(), self.follow_link, Some(&self.ext), ctx).map_err(into_io_err)
    }

    pub(crate) fn metadata_follow(
        &self,
        ctx: &mut E::IteratorExt,
    ) -> wd::ResultInner<E::Metadata, E> {
        E::metadata(self.path(), true, None, ctx).map_err(into_io_err)
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
        self.ty
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
        self.ty.is_symlink()
    }

    pub fn follow_link(&self) -> bool {
        self.follow_link
    }

    /// Return the file name of this entry.
    ///
    /// If this entry has no file name (e.g., `/`), then the full path is
    /// returned.
    pub fn file_name(&self) -> &E::FileName {
        E::get_file_name(self.path())
    }

    /// Returns true if and only if this entry points to a directory.
    pub fn is_dir(&self) -> bool {
        match self.get_fs_dir_entry() {
            Some(fsdent) => E::is_dir(fsdent, &self.ext),
            None => self.file_type().is_dir(),
        }
    }

    fn from_fsentry(fsdent: E::DirEntry) -> wd::ResultInner<Self, E> {
        let path = fsdent.path();
        let ty = fsdent.file_type().map_err(|err| into_path_err(&path, err))?;
        let ext = E::rawdent_from_fsentry(&fsdent).map_err(into_io_err)?;

        Self { follow_link: false, ty, kind: RawDirEntryKind::FromFsDirEntry { path, fsdent }, ext }
            .into_ok()
    }

    fn from_path_internal<P: AsRef<E::Path> + Copy>(
        path: P,
        ctx: &mut E::IteratorExt,
        follow_link: bool,
    ) -> wd::ResultInner<Self, E> {
        let md = E::metadata(path, follow_link, None, ctx).map_err(|e| into_path_err(path, e))?;
        let ty = md.file_type().clone();
        let ext = E::rawdent_from_path(path, follow_link, md, ctx)
            .map_err(|err| into_path_err(path, err))?;
        let pb = path.as_ref().to_path_buf();

        Self { follow_link, ty, kind: RawDirEntryKind::FromPath { path: pb }, ext }.into_ok()
    }

    pub fn from_path<P: AsRef<E::Path> + Copy>(
        path: P,
        ctx: &mut E::IteratorExt,
    ) -> wd::ResultInner<ReadDir<E>, E> {
        let rawdent = Self::from_path_internal(path, ctx, false)?;
        ReadDir::<E>::new_once(rawdent).into_ok()
    }

    pub fn read_dir(&self, ctx: &mut E::IteratorExt) -> wd::ResultInner<ReadDir<E>, E> {
        let rd = E::read_dir(self.path(), &self.ext, ctx).map_err(into_io_err)?;
        ReadDir::<E>::new(rd).into_ok()
    }

    pub fn follow(&self, ctx: &mut E::IteratorExt) -> wd::ResultInner<Self, E> {
        Self::from_path_internal(self.path(), ctx, true)
    }

    fn get_fs_dir_entry(&self) -> Option<&E::DirEntry> {
        match &self.kind {
            RawDirEntryKind::FromFsDirEntry { ref fsdent, .. } => Some(fsdent),
            RawDirEntryKind::FromPath { .. } => None,
        }
    }

    pub fn ancestor_new_ext(&self) -> wd::ResultInner<E::AncestorExt, E> {
        E::ancestor_new(self.path(), self.get_fs_dir_entry(), &self.ext).map_err(into_io_err)
    }

    pub fn call_cmp(a: &Self, b: &Self, cmp: &mut FnCmp<E>) -> std::cmp::Ordering {
        let fs_a = a.get_fs_dir_entry().unwrap();
        let fs_b = b.get_fs_dir_entry().unwrap();
        cmp(fs_a, fs_b)
    }

    pub fn clone_dent_parts(
        &self,
        ctx: &mut E::IteratorExt,
    ) -> (E::PathBuf, E::FileType, bool, E::DirEntryExt) {
        let path = self.path().to_path_buf();
        let dent_ext = E::dent_new(&path, &self.ext, ctx);

        (path, self.ty, self.follow_link, dent_ext)
    }

    pub fn error_inner_from_entry(&self, err: E::Error) -> ErrorInner<E> {
        ErrorInner::<E>::from_entry(self.get_fs_dir_entry().unwrap(), err)
    }

    // pub(crate) fn from_path(
    //     pb: E::PathBuf,
    // ) -> wd::ResultInner<Self, E> {
    //     let md = if follow_link {
    //         E::metadata(&pb)
    //             .map_err(|err| ErrorInner::<E>::from_path(pb.clone(), err))?
    //     } else {
    //         E::symlink_metadata(&pb)
    //             .map_err(|err| ErrorInner::<E>::from_path(pb.clone(), err))?
    //     };

    //     Self {
    //         path: pb,
    //         ty: md.file_type(),
    //         ext: E::rawdent_from_metadata(md),
    //     }.into_ok()
    // }
}

// impl<E: storage::StorageExt> Clone for RawDirEntry<E> {
//     fn clone(&self) -> Self {
//         Self {
//             path: self.path.clone(),
//             ty: self.ty,
//             ext: self.ext.clone(),
//         }
//     }
// }

// impl<E: storage::StorageExt> fmt::Debug for RawDirEntry<E> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("RawDirEntry")
//             .field("path",        &self.path)
//             .field("follow_link", &self.follow_link)
//             .field("ty",          &self.ty)
//             .field("ext",         &self.ext)
//             .finish()
//     }
// }

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
pub enum ReadDir<E: storage::StorageExt> {
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
    Error(Option<wd::ErrorInner<E>>),
}

impl<E: storage::StorageExt> ReadDir<E> {
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

    fn fsdent_into_raw(r_ent: Result<E::DirEntry, E::Error>) -> wd::ResultInner<RawDirEntry<E>, E> {
        match r_ent {
            Ok(ent) => RawDirEntry::<E>::from_fsentry(ent),
            Err(err) => into_io_err(err).into_err(),
        }
    }
}

impl<E: storage::StorageExt> Iterator for ReadDir<E> {
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
