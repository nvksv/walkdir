use crate::source::{SourceExt, SourceIntoIterExt, SourceAncestorExt, SourceDirEntryExt};

use std::path::Path;
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

    #[allow(unused_variables)]
    fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {}
    }
}


