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

use std::path::Path;
use std::fmt::Debug;
use std::io;
use std::fs;
//use std::marker::Sized;

use same_file::Handle;

use crate::dent::DirEntry;
use crate::Ancestor;

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

pub trait SourceDirEntryExt<E: SourceExt>: Debug + Clone {
    fn symlink_metadata(&self, entry: &DirEntry<E>) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(entry.path())
    }

    fn is_dir(&self, entry: &DirEntry<E>) -> bool {
        entry.file_type().is_dir()
    }

    fn from_entry(ent: &fs::DirEntry) -> io::Result<Self>;
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

    /// Make new 
    fn new<P: AsRef<Path>>(root: P) -> Self;
}
