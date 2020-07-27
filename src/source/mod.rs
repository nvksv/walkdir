/*!
Source-specific extensions for directory walking
*/
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

use std::fmt::Debug;
use std::ops::Deref;
use std::convert::AsRef;
use std::io;
use std::fs;
//use std::marker::Sized;

use same_file::Handle;

use crate::dent::DirEntry;
use crate::Ancestor;

pub trait SourcePath<PathBuf> {
    fn to_path_buf(&self) -> PathBuf;
}


impl SourcePath<std::path::PathBuf> for std::path::Path {
    #[inline(always)]
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.to_path_buf()
    }
}

impl SourcePath<std::string::String> for str {
    #[inline(always)]
    fn to_path_buf(&self) -> std::string::String {
        self.to_string()
    }
}


/// Extension for IntoIter
pub trait SourceIntoIterExt<E: SourceExt> {
    /// Make new
    fn new(ext: E) -> Self;
}

/// Extension for Ancestor
pub trait SourceAncestorExt<E: SourceExt>: Debug + Sized {
    /// Make new
    fn new(dent: &DirEntry<E>) -> io::Result<Self>;
    /// Check if this entry and child is same
    fn is_same(&self, ancestor: &Ancestor<E>, child: &Handle) -> io::Result<bool> {
        Ok(child == &Handle::from_path(&ancestor.path)?)
    }
}

/// Extensions for DirEntry
pub trait SourceDirEntryExt<E: SourceExt>: Debug + Clone {
    /// Get metadata for symlink
    fn metadata<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::Metadata>;

    /// Get metadata for symlink
    fn symlink_metadata(&self, entry: &DirEntry<E>) -> io::Result<fs::Metadata>;

    /// Get metadata for symlink
    fn read_dir<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::ReadDir>;

    /// Check if this entry is a directory
    fn is_dir(&self, entry: &DirEntry<E>) -> bool {
        entry.file_type().is_dir()
    }

    /// Create extension from DirEntry
    fn from_entry(ent: &fs::DirEntry) -> io::Result<Self>;
    /// Create extension from metadata
    fn from_metadata(md: fs::Metadata) -> Self;
}

/// Trait for source-specific extensions
pub trait SourceExt: Debug + Clone {
    /// Extension for WalkDirOptions
    type OptionsExt: Default + Debug;
    /// Extension for IntoIter
    type IntoIterExt: SourceIntoIterExt<Self>;
    /// Extension for Ancestor
    type AncestorExt: SourceAncestorExt<Self>;
    /// Extension for DirEntry
    type DirEntryExt: SourceDirEntryExt<Self>;

    type PathBuf: Debug + Clone + Deref<Target = Self::Path> + AsRef<Self::Path>;
    type Path: ?Sized + SourcePath<Self::PathBuf> + AsRef<Self::Path>;

    /// Make new 
    fn new<P: AsRef<Self::Path>>(root: P) -> Self;

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64>;
}
