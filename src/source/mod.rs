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

use std::fmt;
use std::ops::Deref;
use std::convert::AsRef;
use std::marker::Send;
use std::io;
use std::cmp::Ord;

use crate::dent::DirEntry;
use crate::Ancestor;

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
pub trait SourceFsDirEntry<E: SourceExt> {
    /// Get path of this entry
    fn path(&self) -> E::PathBuf;
    /// Get type of this entry
    fn file_type(&self) -> io::Result<E::FsFileType>;
}

/// Functions for FsFileType
pub trait SourceFsFileType: Clone + Copy {
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
pub trait SourceFsReadDir<E: SourceExt>: fmt::Debug + Iterator<Item=io::Result<E::FsDirEntry>> {}

/// Trait for source-specific extensions
pub trait SourceExt: fmt::Debug + Clone + Send + Sync {
    /// Extension for WalkDirOptions
    type OptionsExt: fmt::Debug + Default;
    /// Extension for IntoIter
    type IntoIterExt: fmt::Debug;
    /// Extension for Ancestor
    type AncestorExt: fmt::Debug + Sized;
    /// Extension for DirEntry
    type DirEntryExt: fmt::Debug + Clone;

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
    type PathBuf: fmt::Debug + Clone + Send + Sync + Deref<Target = Self::Path> + AsRef<Self::Path> + for<'s> SourcePathBuf<'s>;

    /// Handle to determine the sameness of two dirs
    type SameFileHandle: Eq;

    /// Make new
    fn intoiter_new(self) -> Self::IntoIterExt;

    /// Return the unique handle 
    fn get_handle<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::SameFileHandle>;

    /// Make new
    fn ancestor_new(dent: &DirEntry<Self>) -> io::Result<Self::AncestorExt>;
    
    /// Check if this entry and child is same
    fn is_same(ancestor: &Ancestor<Self>, child: &Self::SameFileHandle) -> io::Result<bool> {
        Ok(child == &Self::get_handle(&ancestor.path)?)
    }

    /// Get metadata for symlink
    fn metadata<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::FsMetadata>;

    /// Get metadata for symlink
    fn symlink_metadata<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::FsMetadata>;

    /// Get metadata for symlink
    fn symlink_metadata_internal(dent: &DirEntry<Self>) -> io::Result<Self::FsMetadata>;

    /// Get metadata for symlink
    fn read_dir<P: AsRef<Self::Path>>(dent: &DirEntry<Self>, path: P) -> io::Result<Self::FsReadDir>;

    /// Check if this entry is a directory
    fn is_dir(dent: &DirEntry<Self>) -> bool {
        dent.file_type().is_dir()
    }

    /// Create extension from DirEntry
    fn dent_from_fsentry(ent: &Self::FsDirEntry) -> io::Result<Self::DirEntryExt>;
    /// Create extension from metadata
    fn dent_from_metadata(md: Self::FsMetadata) -> Self::DirEntryExt;

    /// Make new 
    fn walkdir_new<P: AsRef<Self::Path>>(root: P) -> Self;

    /// device_num
    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64>;

    /// file_name
    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName;
}
