use crate::source::{SourceExt, SourceIntoIterExt, SourceAncestorExt, SourceDirEntryExt};

use std::fmt::Debug;
use std::io;
use std::fs;
//use std::marker::Sized;

use same_file::Handle;

use crate::dent::DirEntry;
use crate::Ancestor;

/// Useful stub for nothing
#[derive(Debug, Clone, Default)]
pub struct Nil {}

impl<E: SourceExt> SourceIntoIterExt<E> for Nil {
    #[allow(unused_variables)]
    fn new(ext: E) -> Self {
        Self {}
    }
}

impl<E: SourceExt> SourceAncestorExt<E> for Nil {
    #[allow(unused_variables)]
    fn new(dent: &DirEntry<E>) -> io::Result<Self> {
        Ok(Self {})
    }

    #[allow(unused_variables)]
    fn is_same(&self, ancestor: &Ancestor<E>, child: &Handle) -> io::Result<bool> {
        Ok(false)
    }
}

impl<E: SourceExt> SourceDirEntryExt<E> for Nil {
    fn metadata<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path.as_ref())
    }

    fn symlink_metadata(&self, entry: &DirEntry<E>) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(entry.path())
    }

    fn read_dir<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::ReadDir> {
        fs::read_dir(path.as_ref())
    }


    #[allow(unused_variables)]
    fn from_entry(ent: &fs::DirEntry) -> io::Result<Self> {
        Ok(Self {})
    }

    #[allow(unused_variables)]
    fn from_metadata(md: fs::Metadata) -> Self {
        Self {}
    }
}

impl SourceExt for Nil {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = Nil;
    type DirEntryExt = Nil;

    type PathBuf = std::path::PathBuf;
    type Path = std::path::Path;

    #[allow(unused_variables)]
    fn new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "walkdir: same_file_system option not supported on this platform",
        ))
    }

}


