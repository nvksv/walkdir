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
}



pub struct WalkDirUnixExt {
}

impl WalkDirSourceExt for WalkDirUnixExt {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = Nil;
    type DirEntryExt = DirEntryUnixExt;

    fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {}
    }
} 

