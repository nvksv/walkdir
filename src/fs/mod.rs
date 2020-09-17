use std::ops::Deref;
use std::fmt::Debug;

mod path;
mod standard;
mod windows;

use crate::wd::{IntoSome, IntoErr};
pub use self::path::{FsPath, FsPathBuf};

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsMetadata
pub trait FsError: 'static + std::error::Error + Debug {
    type Inner: std::error::Error;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_inner(error: Self::Inner) -> Self;
}

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsFileType
pub trait FsFileType: Clone + Copy + Debug {
    /// Is it dir?
    fn is_dir(&self) -> bool;
    /// Is it file
    fn is_file(&self) -> bool;
    /// Is it symlink
    fn is_symlink(&self) -> bool;
}

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsMetadata
pub trait FsMetadata: Debug + Clone {
    type FileType: FsFileType;

    /// Get type of this entry
    fn file_type(&self) -> Self::FileType;

    /// Is it dir?
    fn is_dir(&self) -> bool;
    /// Is it symlink
    fn is_symlink(&self) -> bool;
}

///////////////////////////////////////////////////////////////////////////////////////////////

pub trait FsReadDirIterator: Debug + Sized {
    type Context;

    type Error: std::error::Error;
    type DirEntry;

    fn next_entry(
        &mut self, 
        ctx: &mut Self::Context,
    ) -> Option<Result<Self::DirEntry, Self::Error>>;
}

/// Functions for FsReadDir
pub trait FsReadDir: Debug + Sized {
    type Context;
    type Inner: FsReadDirIterator<Context = Self::Context>;
    type Error: FsError<Inner = <Self::Inner as FsReadDirIterator>::Error>;
    type DirEntry: FsDirEntry<Context = Self::Context, Error = Self::Error>;

    fn inner_mut(&mut self) -> &mut Self::Inner;
    fn process_inner_entry(&mut self, inner_entry: <Self::Inner as FsReadDirIterator>::DirEntry) -> Result<Self::DirEntry, Self::Error>;

    fn next_fsentry(
        &mut self,
        ctx: &mut Self::Context,
    ) -> Option<Result<Self::DirEntry, Self::Error>> {
        match self.inner_mut().next_entry(ctx)? {
            Ok(inner_entry) => self.process_inner_entry(inner_entry),
            Err(err)        => Self::Error::from_inner(err).into_err(),
        }.into_some()
    }
}

impl<RD> FsReadDirIterator for RD where RD: FsReadDir {
    type Context    = RD::Context;
    type Error      = RD::Error;
    type DirEntry   = RD::DirEntry;

    fn next_entry(
        &mut self,
        ctx: &mut Self::Context,
    ) -> Option<Result<Self::DirEntry, Self::Error>> {
        self.next_fsentry(ctx)
    }
}

impl<RD, DE, E> FsReadDirIterator for RD where 
    RD: Iterator<Item=Result<DE, E>>,
{
    type Context    = ();
    type Error      = E;
    type DirEntry   = DE;

    fn next_entry(
        &mut self,
        ctx: &mut Self::Context,
    ) -> Option<Result<Self::DirEntry, Self::Error>> {
        self.next()
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsDirEntry
pub trait FsDirEntry: Debug + Sized {
    type Context;

    type Path: FsPath<PathBuf = Self::PathBuf, FileName = Self::FileName> + AsRef<Self::Path> + ?Sized;
    type PathBuf: for<'p> FsPathBuf<'p> + AsRef<Self::Path> + Deref<Target = Self::Path> + Sized;
    type FileName: Sized;

    type Error:    FsError;
    type FileType: FsFileType;
    type Metadata: FsMetadata<FileType=Self::FileType>;
    type ReadDir:  FsReadDirIterator<Context=Self::Context, DirEntry=Self, Error=Self::Error>;
    type DirFingerprint: Debug + Eq;
    type DeviceNum: Eq + Clone + Copy;
    type RootDirEntry: FsRootDirEntry<Context=Self::Context, DirEntry=Self>;

    /// Get path of this entry
    fn path(&self) -> &Self::Path;
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf;
    /// Get canonical path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error>;
    /// Get bare name of this entry withot any leading path components
    fn file_name(&self) -> Self::FileName;

    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<Self::Metadata, Self::Error>;

    /// Read dir (always follow symlink!)
    fn read_dir(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error>;

    /// Return the unique handle (always follow symlink!)
    fn fingerprint(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::DirFingerprint, Self::Error>;

    fn is_same(
        lhs: (&Self::Path, &Self::DirFingerprint),
        rhs: (&Self::Path, &Self::DirFingerprint),
    ) -> bool;

    /// device_num (always follow symlink!)
    fn device_num(
        &self
    ) -> Result<Self::DeviceNum, Self::Error>;
}

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsRootDirEntry
pub trait FsRootDirEntry: Debug + Sized {
    type Context;
    type DirEntry: FsDirEntry<Context=Self::Context, RootDirEntry=Self>;

    /// Get path of this entry
    fn path(&self) -> &<Self::DirEntry as FsDirEntry>::Path;
    /// Get path of this entry
    fn pathbuf(&self) -> <Self::DirEntry as FsDirEntry>::PathBuf;
    /// Get canonical path of this entry
    fn canonicalize(&self) -> Result<<Self::DirEntry as FsDirEntry>::PathBuf, <Self::DirEntry as FsDirEntry>::Error>;
    /// Get bare name of this entry withot any leading path components
    fn file_name(&self) -> <Self::DirEntry as FsDirEntry>::FileName;

    fn from_path(
        path: &<Self::DirEntry as FsDirEntry>::Path,
        ctx: &mut Self::Context,
    ) -> Result<(Self, <Self::DirEntry as FsDirEntry>::Metadata), <Self::DirEntry as FsDirEntry>::Error>;

    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::Metadata, <Self::DirEntry as FsDirEntry>::Error>;

    /// Read dir
    fn read_dir(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::ReadDir, <Self::DirEntry as FsDirEntry>::Error>;

    /// Return the unique handle
    fn fingerprint(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::DirFingerprint, <Self::DirEntry as FsDirEntry>::Error>;

    /// device_num
    fn device_num(&self) -> Result<<Self::DirEntry as FsDirEntry>::DeviceNum, <Self::DirEntry as FsDirEntry>::Error>;
}
