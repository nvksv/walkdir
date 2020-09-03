use std::cmp::Ord;
use std::convert::AsRef;
use std::fmt;
use std::marker::Send;
use std::ops::Deref;

/// Functions for StorageExt::Path
pub trait FsPath<PathBuf> {
    /// Copy to owned
    fn to_path_buf(&self) -> PathBuf;
}

/// Functions for StorageExt::PathBuf
pub trait FsPathBuf<'s> {
    /// Intermediate object
    type Display: 's + fmt::Display;

    /// Create intermediate object which can Display
    fn display(&'s self) -> Self::Display;
}

pub trait FsPaths {
    type Path: ?Sized 
        + Ord 
        + FsPath<Self::PathBuf> 
        + AsRef<Self::Path>;
    type PathBuf: Sized 
        + fmt::Debug
        + Clone
        + Send
        + Sync
        + Deref<Target = Self::Path>
        + AsRef<Self::Path>
        + for<'s> FsPathBuf<'s>;
}

