use crate::source::SourceExt;

use std::path::Path;
use std::fmt::Debug;
use std::io;
use std::fs;
//use std::marker::Sized;

use same_file::Handle;

use crate::dent::DirEntry;
use crate::Ancestor;

#[derive(Debug, Clone)]
pub struct DirEntryUnixExt {
    /// The underlying inode number (Unix only).
    ino: u64,
}

impl<E: SourceExt> SourceDirEntryExt<E> for DirEntryUnixExt {
    fn from_entry(ent: &fs::DirEntry) -> io::Result<Self> {
        use std::os::unix::fs::DirEntryExt;
        Ok(Self { ino: ent.ino() })
    }

    fn from_metadata(md: fs::Metadata) -> Self {
        Self { ino: md.ino() }
    }

    fn read_dir<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::ReadDir> {
        fs::read_dir(path.as_ref())
    }

}



pub struct WalkDirUnixExt {
}

impl WalkDirSourceExt for WalkDirUnixExt {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = Nil;
    type DirEntryExt = DirEntryUnixExt;

    type PathBuf = path::PathBuf;
    type Path = path::Path;

    fn new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    fn device_num<P: AsRef<Path>>(path: P) -> io::Result<u64> {
        use std::os::unix::fs::MetadataExt;
    
        path.as_ref().metadata().map(|md| md.dev())
    }
} 

