mod path;
mod standard;
mod windows;

pub use path::{FsPath, FsPathBuf};

/// Functions for FsMetadata
pub trait FsError: 'static + std::error::Error + std::fmt::Debug {
    type Inner: std::error::Error;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_fserror(error: Self::Inner) -> Self;
}

/// Functions for FsDirEntry
pub trait FsDirEntry: std::fmt::Debug + Sized {
    type Context;

    type Path: FsPath + ?Sized;
    type PathBuf: for<'p> FsPathBuf<'p>;

    type Error:    FsError;
    type FileType: FsFileType;
    type Metadata: FsMetadata<Filetype=Self::FileType>;
    type ReadDir:  FsReadDir<DirEntry=Self, Error=Self::Error>;
    type DirFingerprint: FsDirFingerprint;
    type DeviceNum: Eq + Clone + Copy;

    /// Get path of this entry
    fn path(&self) -> &Self::Path;
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf;
    /// Get canonical path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error>;
    
    /// Get type of this entry
    fn file_type(&self) -> Result<Self::FileType, Self::Error>;

    /// Get metadata
    fn metadata(
        &self,
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

    /// device_num
    fn device_num(&self) -> Result<Self::DeviceNum, Self::Error>;
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
pub trait FsMetadata: std::fmt::Debug {
    type FileType: FsFileType;

    /// Get type of this entry
    fn file_type(&self) -> Self::FileType;
}

/// Functions for FsReadDir
pub trait FsReadDir: std::fmt::Debug {
    type Error;
    type DirEntry: FsDirEntry<Error=Self::Error>;

    fn next_fsentry<E>(&self) -> Option<Result<Self::DirEntry, E>>;
}

impl<RD: FsReadDir> Iterator for RD {
    type Item = Result<Self::DirEntry, Self::Error>;

    fn next(&self) -> Option<Self::Item> {
        match self.next_fsentry() {
            Some(Err(e)) => Some(Err(Self::DirEntry::Error::from_fserror(e))),
            v @ _ => v,
        }
    }
}

pub trait FsDirFingerprint: std::fmt::Debug {
    type Path: FsPath + ?Sized;

    fn is_same(&self, rhs: &Self) -> bool;
}