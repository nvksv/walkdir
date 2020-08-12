/*!
Source-specific extensions for directory walking
*/
mod util;
mod stub;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use stub::Nil;
#[cfg(unix)]
pub use unix::WalkDirUnixExt;
#[cfg(windows)]
pub use windows::WalkDirWindowsExt;

#[cfg(not(any(unix, windows)))]
/// Default source-specific type.
pub type DefaultSourceExt = Nil;
#[cfg(unix)]
/// Default source-specific type.
pub type DefaultSourceExt = WalkDirUnixExt;
#[cfg(windows)]
/// Default source-specific type.
pub type DefaultSourceExt = WalkDirWindowsExt;

use std::cmp::Ord;
use std::convert::AsRef;
use std::fmt;
use std::io;
use std::marker::Send;
use std::ops::Deref;

/// Functions for SourceExt::Path
pub trait SourcePath<PathBuf> {
    /// Copy to owned
    fn to_path_buf(&self) -> PathBuf;
}

/// Functions for SourceExt::PathBuf
pub trait SourcePathBuf<'s> {
    /// Intermediate object
    type Display: 's + fmt::Display;

    /// Create intermediate object which can Display
    fn display(&'s self) -> Self::Display;
}

/// Functions for FsDirEntry
pub trait SourceFsDirEntry<E: SourceExt>: fmt::Debug + Sized {
    /// Get path of this entry
    fn path(&self) -> E::PathBuf;
    /// Get type of this entry
    fn file_type(&self) -> Result<E::FsFileType, E::FsError>;
}

/// Functions for FsFileType
pub trait SourceFsFileType: Clone + Copy + fmt::Debug {
    /// Is it dir?
    fn is_dir(&self) -> bool;
    /// Is it file
    fn is_file(&self) -> bool;
    /// Is it symlink
    fn is_symlink(&self) -> bool;
}

/// Functions for FsMetadata
pub trait SourceFsMetadata<E: SourceExt> {
    /// Get type of this entry
    fn file_type(&self) -> E::FsFileType;
}

/// Functions for FsReadDir
pub trait SourceFsReadDir<E: SourceExt>:
    fmt::Debug + Iterator<Item = Result<E::FsDirEntry, E::FsError>>
{
}

/// Trait for source-specific extensions
pub trait SourceExt: fmt::Debug + Clone + Send + Sync + Sized {
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

    /// io::Error
    type FsError: std::error::Error + fmt::Debug;
    /// ffi::OsStr
    type FsFileName: ?Sized;
    /// fs::DirEntry
    type FsDirEntry: SourceFsDirEntry<Self>;
    /// fs::ReadDir
    type FsReadDir: SourceFsReadDir<Self>;
    /// fs::FileType
    type FsFileType: SourceFsFileType;
    /// fs::Metadata
    type FsMetadata: SourceFsMetadata<Self>;

    /// std::path::Path
    type Path: ?Sized + Ord + SourcePath<Self::PathBuf> + AsRef<Self::Path>;
    /// std::path::PathBuf
    type PathBuf: fmt::Debug
        + Clone
        + Send
        + Sync
        + Deref<Target = Self::Path>
        + AsRef<Self::Path>
        + for<'s> SourcePathBuf<'s>;

    /// Handle to determine the sameness of two dirs
    type SameFileHandle: Eq;

    /// Make new builder
    fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self;

    /// Make new
    fn ancestor_new(dent: &Self::FsDirEntry) -> Result<Self::AncestorExt, Self::FsError>;

    /// Make new
    fn iterator_new(self) -> Self::IteratorExt;

    // /// Create FsDirEntry from path
    // fn fsentry_from_path<P: AsRef<Self::Path>>(
    //     path: P,
    //     ctx: Self::IteratorExt,
    // ) -> io::Result<Self::FsDirEntry>;

    /// Create extension from DirEntry
    fn rawdent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> io::Result<Self::RawDirEntryExt>;

    /// Create extension from metadata
    fn rawdent_from_path( path: &Self::PathBuf, md: &Self::FsMetadata, ctx: &mut Self::IteratorExt ) -> Result<Self::RawDirEntryExt, Self::FsError>;


    /// Return the unique handle
    fn get_handle<P: AsRef<Self::Path>>(
        path: P,
    ) -> Result<Self::SameFileHandle, Self::FsError>;

    /// Check if this entry and child is same
    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> Result<bool, Self::FsError> {
        Ok(child == &Self::get_handle(ancestor_path)?)
    }

    /// Get metadata for symlink
    fn metadata<P: AsRef<Self::Path>>(path: P)
        -> Result<Self::FsMetadata, Self::FsError>;

    // /// Get metadata for symlink
    // fn symlink_metadata<P: AsRef<Self::Path>>(
    //     path: P,
    // ) -> io::Result<Self::FsMetadata>;

    // /// Get metadata for symlink
    // fn symlink_metadata_internal(
    //     dent: &Self::FsDirEntry,
    //     raw_dent_ext: &Self::RawDirEntryExt,
    // ) -> io::Result<Self::FsMetadata>;

    /// Get metadata for symlink
    fn read_dir<P: AsRef<Self::Path>>(
        dent: &Self::FsDirEntry,
        path: P,
    ) -> Result<Self::FsReadDir, Self::FsError>;

    /// Check if this entry is a directory
    #[allow(unused_variables)]
    fn is_dir(dent: &Self::FsDirEntry, raw_dent_ext: &Self::RawDirEntryExt) -> bool {
        dent.file_type().is_dir()
    }

    /// device_num
    fn device_num<P: AsRef<Self::Path>>(path: P) -> Result<u64, Self::FsError>;

    /// file_name
    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName;
}
