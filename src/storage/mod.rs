/*!
Source-specific extensions for directory walking
*/
use crate::error;

mod standard;
#[cfg(unix)]
mod unix;
mod util;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::WalkDirUnixExt;
pub use util::Nil;
#[cfg(windows)]
pub use windows::WalkDirWindowsExt;

#[cfg(not(any(unix, windows)))]
/// Default storage-specific type.
pub type DefaultStorageExt = Nil;
#[cfg(unix)]
/// Default source-specific type.
pub type DefaultStorageExt = WalkDirUnixExt;
#[cfg(windows)]
/// Default source-specific type.
pub type DefaultStorageExt = WalkDirWindowsExt;

use std::cmp::Ord;
use std::convert::AsRef;
use std::fmt;
use std::marker::Send;
use std::ops::Deref;

/// Functions for StorageExt::Path
pub trait StoragePath<PathBuf> {
    /// Copy to owned
    fn to_path_buf(&self) -> PathBuf;
}

/// Functions for StorageExt::PathBuf
pub trait StoragePathBuf<'s> {
    /// Intermediate object
    type Display: 's + fmt::Display;

    /// Create intermediate object which can Display
    fn display(&'s self) -> Self::Display;
}

/// Functions for FsDirEntry
pub trait StorageDirEntry<E: StorageExt>: fmt::Debug + Sized {
    /// Get path of this entry
    fn path(&self) -> E::PathBuf;
    /// Get type of this entry
    fn file_type(&self) -> Result<E::FileType, E::Error>;
}

/// Functions for FsFileType
pub trait StorageFileType: Clone + Copy + fmt::Debug {
    /// Is it dir?
    fn is_dir(&self) -> bool;
    /// Is it file
    fn is_file(&self) -> bool;
    /// Is it symlink
    fn is_symlink(&self) -> bool;
}

/// Functions for FsMetadata
pub trait StorageMetadata<E: StorageExt>: fmt::Debug {
    /// Get type of this entry
    fn file_type(&self) -> E::FileType;
}

/// Functions for FsReadDir
pub trait StorageReadDir<E: StorageExt>:
    fmt::Debug + Iterator<Item = Result<E::DirEntry, E::Error>>
{
}

/// Functions for FsMetadata
pub trait StorageError<E: StorageExt>: 'static + std::error::Error + fmt::Debug {
    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn new(kind: std::io::ErrorKind, error: error::Error<E>) -> E::Error;
    /// Returns the corresponding ErrorKind for this error.
    fn kind(&self) -> std::io::ErrorKind;
}

/// Trait for source-specific extensions
pub trait StorageExt: fmt::Debug + Clone + Send + Sync + Sized {
    /// Context
    type BuilderCtx: fmt::Debug + Default;

    /// Extension for WalkDirOptions
    type OptionsExt: fmt::Debug + Default;
    /// Extension for IntoIter
    type IteratorExt: fmt::Debug;
    /// Extension for Ancestor
    type AncestorExt: fmt::Debug + Sized;
    /// Extension for RawDirEntry
    type RawDirEntryExt: fmt::Debug;
    /// Extension for DirEntry
    type DirEntryExt: fmt::Debug + Clone;

    /// io::Error
    type Error: StorageError<Self>;
    /// Wrapper for fs::DirEntry
    type DirEntry: StorageDirEntry<Self>;
    /// Wrapper for fs::ReadDir
    type ReadDir: StorageReadDir<Self>;
    /// fs::Metadata
    type Metadata: StorageMetadata<Self>;
    /// fs::FileType
    type FileType: StorageFileType;
    /// ffi::OsStr
    type FileName: ?Sized;

    /// std::path::Path
    type Path: ?Sized + Ord + StoragePath<Self::PathBuf> + AsRef<Self::Path>;
    /// std::path::PathBuf
    type PathBuf: fmt::Debug
        + Clone
        + Send
        + Sync
        + Deref<Target = Self::Path>
        + AsRef<Self::Path>
        + for<'s> StoragePathBuf<'s>;

    /// Handle to determine the sameness of two dirs
    type SameFileHandle: Eq;
    /// Handle to determine the sameness of file systems
    type DeviceNum: Eq + Clone + Copy;

    /// Make new builder
    fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self;

    /// Make new ancestor
    fn ancestor_new<P: AsRef<Self::Path>>(
        path: P,
        dent: Option<&Self::DirEntry>,
        raw_ext: &Self::RawDirEntryExt,
    ) -> Result<Self::AncestorExt, Self::Error>;

    /// Make new
    fn iterator_new(self) -> Self::IteratorExt;

    /// Create extension from DirEntry
    fn rawdent_from_fsentry(ent: &Self::DirEntry) -> Result<Self::RawDirEntryExt, Self::Error>;

    /// Create extension from metadata
    fn rawdent_from_path<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        md: Self::Metadata,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::RawDirEntryExt, Self::Error>;

    /// Create extension for DirEntry
    fn dent_new<P: AsRef<Self::Path>>(
        path: P,
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt,
    ) -> Self::DirEntryExt;

    /// Get metadata
    fn metadata<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        raw_ext: Option<&Self::RawDirEntryExt>,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::Metadata, Self::Error>;

    /// Get metadata for symlink
    fn read_dir<P: AsRef<Self::Path>>(
        path: P,
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::ReadDir, Self::Error>;

    /// Check if this entry is a directory
    #[allow(unused_variables)]
    fn is_dir(dent: &Self::DirEntry, raw_ext: &Self::RawDirEntryExt) -> bool {
        match dent.file_type() {
            Ok(ty) => ty.is_dir(),
            Err(_) => false,
        }
    }

    /// Get metadata
    fn dent_metadata<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        ext: &Self::DirEntryExt,
    ) -> Result<Self::Metadata, Self::Error>;

    /// Return the unique handle
    fn get_handle<P: AsRef<Self::Path>>(path: P) -> Result<Self::SameFileHandle, Self::Error>;

    /// Check if this entry and child is same
    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> Result<bool, Self::Error> {
        Ok(child == &Self::get_handle(ancestor_path)?)
    }

    /// device_num
    fn device_num<P: AsRef<Self::Path>>(path: P) -> Result<Self::DeviceNum, Self::Error>;

    /// file_name
    fn get_file_name(path: &Self::Path) -> &Self::FileName;
}
