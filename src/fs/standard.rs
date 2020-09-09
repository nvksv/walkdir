use super::{FsError, FsFileType, FsMetadata, FsReadDir, FsReadDirIterator, FsDirEntry, FsDirFingerprint};
use crate::wd::IntoOk;

use same_file;

impl FsError for std::io::Error {
    type Inner = Self;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_inner(inner: Self::Inner) -> Self {
        inner
    }
}

/// Functions for FsFileType
impl FsFileType for std::fs::FileType {
    /// Is it dir?
    fn is_dir(&self) -> bool {
        std::fs::FileType::is_dir(self)
    }
    /// Is it file
    fn is_file(&self) -> bool {
        std::fs::FileType::is_file(self)
    }
    /// Is it symlink
    fn is_symlink(&self) -> bool {
        std::fs::FileType::is_symlink(self)
    }
}

/// Functions for FsMetadata
impl FsMetadata for std::fs::Metadata {
    type FileType = std::fs::FileType;

    /// Get type of this entry
    fn file_type(&self) -> std::fs::FileType {
        std::fs::Metadata::file_type(self)    
    }
}

impl FsReadDirIterator for std::fs::ReadDir {
    type Error      = std::io::Error;
    type DirEntry   = std::fs::DirEntry;

    fn next_entry(&mut self) -> Option<Result<Self::DirEntry, Self::Error>> {
        self.next()
    }
}

#[derive(Debug)]
pub struct StandardReadDir {
    inner:      std::fs::ReadDir,
}

impl StandardReadDir {
    pub fn inner(&self) -> &std::fs::ReadDir {
        &self.inner
    }
}

/// Functions for FsReadDir
impl FsReadDir for StandardReadDir {
    type Inner      = std::fs::ReadDir;
    type Error      = std::io::Error;
    type DirEntry   = StandardDirEntry;

    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.inner
    }

    fn process_inner_entry(&mut self, inner_entry: std::fs::DirEntry) -> Result<Self::DirEntry, Self::Error> {
        Self::DirEntry::from_inner(inner_entry).into_ok()    
    }
}

impl Iterator for StandardReadDir {
    type Item = Result<StandardDirEntry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_fsentry()
    }
}

#[derive(Debug)]
pub struct StandardDirEntry {
    pathbuf:    std::path::PathBuf,
    inner:      std::fs::DirEntry,
}

impl StandardDirEntry {
    pub fn inner(&self) -> &std::fs::DirEntry {
        &self.inner
    }

    pub fn from_inner(inner: std::fs::DirEntry) -> Result<Self, std::io::Error> {
        let pathbuf = inner.path().to_path_buf();
        Self {
            pathbuf,
            inner
        }.into_ok()
    }
}

/// Functions for FsDirEntry
impl FsDirEntry for StandardDirEntry {
    type Context        = ();

    type Path           = std::path::Path;
    type PathBuf        = std::path::PathBuf;

    type Error          = std::io::Error;
    type FileType       = std::fs::FileType;
    type Metadata       = std::fs::Metadata;
    type ReadDir        = StandardReadDir;
    type DirFingerprint = StandardDirFingerprint;
    type DeviceNum      = ();

    /// Get path of this entry
    fn path(&self) -> &Self::Path {
        &self.pathbuf    
    }
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf {
        self.pathbuf.clone()
    }

    /// Get type of this entry
    fn file_type(&self) -> Result<Self::FileType, Self::Error> {
        std::fs::DirEntry::file_type(&self.inner)    
    }

    /// Get path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error> {
        std::fs::canonicalize(self.path())
    }


    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<Self::Metadata, Self::Error> {
        if follow_link {
            std::fs::metadata(&self.pathbuf)    
        } else {
            std::fs::symlink_metadata(&self.pathbuf)    
        }
    }

    /// Read dir
    fn read_dir(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error> {
        StandardReadDir {
            inner: std::fs::read_dir(self.path())?,
        }.into_ok()
    }

    /// Read dir
    fn read_dir_from_path(
        path: &Self::Path,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error> {
        StandardReadDir {
            inner: std::fs::read_dir(path)?,
        }.into_ok()
    }

    /// Return the unique handle
    fn fingerprint(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::DirFingerprint, Self::Error> {
        StandardDirFingerprint {
            handle: same_file::Handle::from_path(self.path())?
        }.into_ok()
    }

    /// device_num
    fn device_num(&self) -> Result<Self::DeviceNum, Self::Error> {
        ().into_ok()
    }
}

#[derive(Debug)]
pub struct StandardDirFingerprint {
    handle: same_file::Handle,
}

impl FsDirFingerprint for StandardDirFingerprint {
    fn is_same(&self, rhs: &Self) -> bool {
        self.handle == rhs.handle
    }
}
