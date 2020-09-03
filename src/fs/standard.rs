use super::{FsError, FsFileType, FsMetadata, FsReadDir, FsDirEntry};
use super::path::FsPaths;

impl FsError for std::io::Error {
    type Inner = Self;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_fserror(error: Self::Inner) -> Self {
        error
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

pub struct StandardReadDir {
    inner:      std::fs::ReadDir,
}

impl StandardReadDir {
    fn inner(&self) -> &std::fs::ReadDir {
        &self.inner
    }
}

/// Functions for FsReadDir
impl FsReadDir for StandardReadDir {
    type Error      = std::io::Error;
    type DirEntry   = StandardDirEntry;
}

impl Iterator for StandardReadDir {
    type Item = StandardDirEntry;

    fn next(&self) -> Option<Self::Item> {
        let dent = self.inner.next()?;
        let pathbuf = dent.path();
        StandardDirEntry {
            pathbuf,
            inner: dent,
        }
    }
}

pub struct StandardDirEntry {
    pathbuf:    std::path::PathBuf,
    inner:      std::fs::DirEntry,
}

impl StandardDirEntry {
    fn inner(&self) -> &std::fs::DirEntry {
        &self.inner
    }
}

/// Functions for FsDirEntry
impl FsDirEntry for StandardDirEntry {
    type Context    = ();

    type Path       = std::path::Path;
    type PathBuf    = std::path::PathBuf;

    type Error      = std::io::Error;
    type FileType   = std::fs::FileType;
    type Metadata   = std::fs::Metadata;
    type ReadDir    = StandardReadDir;

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
        std::fs::DirEntry::file_type(self)    
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
            inner: std::fs::read_dir(&self.pathbuf),
        }
    }

    /// Read dir
    fn read_dir_from_path(
        path: &Self::Path,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error> {
        StandardReadDir {
            inner: std::fs::read_dir(path),
        }
    }
}

