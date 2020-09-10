use std::ops::Deref;

mod path;
mod standard;
mod windows;

use crate::wd::{IntoSome, IntoErr};
pub use self::path::{FsPath, FsPathBuf};

/// Functions for FsMetadata
pub trait FsError: 'static + std::error::Error + std::fmt::Debug {
    type Inner: std::error::Error;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_inner(error: Self::Inner) -> Self;
}

/// Functions for FsFileType
pub trait FsFileType: Clone + Copy + std::fmt::Debug {
    /// Is it dir?
    fn is_dir(&self) -> bool;
    /// Is it file
    fn is_file(&self) -> bool;
    /// Is it symlink
    fn is_symlink(&self) -> bool;
}

/// Functions for FsMetadata
pub trait FsMetadata: std::fmt::Debug + Clone {
    type FileType: FsFileType;

    /// Get type of this entry
    fn file_type(&self) -> Self::FileType;
}

pub trait FsReadDirIterator: std::fmt::Debug + Sized {
    type Error: std::error::Error;
    type DirEntry;

    fn next_entry(&mut self) -> Option<Result<Self::DirEntry, Self::Error>>;
}

/// Functions for FsReadDir
pub trait FsReadDir: std::fmt::Debug + Sized {
    type Inner: FsReadDirIterator;
    type Error: FsError<Inner = <Self::Inner as FsReadDirIterator>::Error>;
    type DirEntry: FsDirEntry<Error = Self::Error>;

    fn inner_mut(&mut self) -> &mut Self::Inner;
    fn process_inner_entry(&mut self, inner_entry: <Self::Inner as FsReadDirIterator>::DirEntry) -> Result<Self::DirEntry, Self::Error>;

    fn next_fsentry(&mut self) -> Option<Result<Self::DirEntry, Self::Error>> {
        match self.inner_mut().next_entry()? {
            Ok(inner_entry) => self.process_inner_entry(inner_entry),
            Err(err)        => Self::Error::from_inner(err).into_err(),
        }.into_some()
    }
}

impl<RD> FsReadDirIterator for RD where RD: FsReadDir {
    type Error      = RD::Error;
    type DirEntry   = RD::DirEntry;

    fn next_entry(&mut self) -> Option<Result<Self::DirEntry, Self::Error>> {
        self.next_fsentry()
    }
}

pub trait FsDirFingerprint: std::fmt::Debug {
    fn is_same(&self, rhs: &Self) -> bool;
}

/// Functions for FsDirEntry
pub trait FsDirEntry: std::fmt::Debug + Sized {
    type Context;

    type Path: FsPath<PathBuf = Self::PathBuf, FileName = Self::FileName> + AsRef<Self::Path> + ?Sized;
    type PathBuf: for<'p> FsPathBuf<'p> + AsRef<Self::Path> + Deref<Target = Self::Path> + Sized;
    type FileName: Sized;

    type Error:    FsError;
    type FileType: FsFileType;
    type Metadata: FsMetadata<FileType=Self::FileType>;
    type ReadDir:  FsReadDirIterator<DirEntry=Self, Error=Self::Error> + Iterator<Item = Result<Self, Self::Error>>;
    type DirFingerprint: FsDirFingerprint;
    type DeviceNum: Eq + Clone + Copy;

    /// Get path of this entry
    fn path(&self) -> &Self::Path;
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf;
    /// Get canonical path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error>;
    /// Get bare name of this entry withot any leading path components
    fn file_name(&self) -> Self::FileName;

    /// Get type of this entry
    fn file_type(&self) -> Self::FileType;

    fn is_dir(&self) -> bool;
    fn metadata_is_dir(metadata: &Self::Metadata) -> bool;

    fn file_name_from_path(
        path: &Self::Path,
    ) -> Result<Self::FileName, Self::Error>;

    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<Self::Metadata, Self::Error>;

    /// Get metadata
    fn metadata_from_path(
        path: &Self::Path,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<Self::Metadata, Self::Error>;

    /// Read dir
    fn read_dir(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error>;

    /// Read dir
    fn read_dir_from_path(
        path: &Self::Path,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error>;

    /// Return the unique handle
    fn fingerprint(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::DirFingerprint, Self::Error>;

    /// Return the unique handle
    fn fingerprint_from_path(
        path: &Self::Path,
        ctx: &mut Self::Context,
    ) -> Result<Self::DirFingerprint, Self::Error>;

    /// device_num
    fn device_num(&self) -> Result<Self::DeviceNum, Self::Error>;
}
