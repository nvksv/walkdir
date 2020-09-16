use super::{FsError, FsFileType, FsMetadata, FsReadDir, FsReadDirIterator, FsDirEntry, FsRootDirEntry};
use crate::wd::{IntoOk};

use same_file;

///////////////////////////////////////////////////////////////////////////////////////////////

impl FsError for std::io::Error {
    type Inner = Self;

    /// Creates a new I/O error from a known kind of error as well as an arbitrary error payload.
    fn from_inner(inner: Self::Inner) -> Self {
        inner
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////

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

///////////////////////////////////////////////////////////////////////////////////////////////

/// Functions for FsMetadata
impl FsMetadata for std::fs::Metadata {
    type FileType = std::fs::FileType;

    /// Get type of this entry
    fn file_type(&self) -> std::fs::FileType {
        std::fs::Metadata::file_type(self)    
    }

    /// Is it dir?
    fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }
    /// Is it symlink
    fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////

// impl FsReadDirIterator for std::fs::ReadDir {
//     type Context    = ();
//     type Error      = std::io::Error;
//     type DirEntry   = std::fs::DirEntry;

//     fn next_entry(
//         &mut self,
//         ctx: &mut Self::Context,
//     ) -> Option<Result<Self::DirEntry, Self::Error>> {
//         self.next()
//     }
// }

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
    type Context    = ();
    type Inner      = std::fs::ReadDir;
    type Error      = std::io::Error;
    type DirEntry   = StandardDirEntry;

    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.inner
    }

    fn process_inner_entry(&mut self, inner_entry: std::fs::DirEntry) -> Result<Self::DirEntry, Self::Error> {
        Self::DirEntry::from_inner(inner_entry)    
    }
}


// impl Iterator for StandardReadDir {
//     type Item = Result<StandardDirEntry, std::io::Error>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.next_fsentry(&mut ())
//     }
// }

///////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct StandardDirEntry {
    pathbuf:    std::path::PathBuf,
    ty:         std::fs::FileType,
    inner:      std::fs::DirEntry,
}

impl StandardDirEntry {
    pub fn inner(&self) -> &std::fs::DirEntry {
        &self.inner
    }

    pub fn from_inner(inner: std::fs::DirEntry) -> Result<Self, std::io::Error> {
        let pathbuf = inner.path().to_path_buf();
        let ty      = inner.file_type()?;
        Self {
            pathbuf,
            ty,
            inner
        }.into_ok()
    }
}

impl StandardDirEntry {

    pub fn canonicalize_from_path(
        path: &<Self as FsDirEntry>::Path
    ) -> Result<<Self as FsDirEntry>::PathBuf, <Self as FsDirEntry>::Error> {
        std::fs::canonicalize(path)
    }

    pub fn file_name_from_path(
        path: &<Self as FsDirEntry>::Path,
    ) -> <Self as FsDirEntry>::FileName {
        match path.file_name() {
            Some(n) => n.to_os_string(),
            None => panic!("Wrong path!"),
        } 
    }

    /// Get metadata
    pub fn metadata_from_path(
        path: &<Self as FsDirEntry>::Path,
        follow_link: bool,
        ctx: &mut <Self as FsDirEntry>::Context,
    ) -> Result<<Self as FsDirEntry>::Metadata, <Self as FsDirEntry>::Error> {
        if follow_link {
            std::fs::metadata(path)    
        } else {
            std::fs::symlink_metadata(path)    
        }
    }

    /// Read dir
    pub fn read_dir_from_path(
        path: &<Self as FsDirEntry>::Path,
        ctx: &mut <Self as FsDirEntry>::Context,
    ) -> Result<<Self as FsDirEntry>::ReadDir, <Self as FsDirEntry>::Error> {
        StandardReadDir {
            inner: std::fs::read_dir(path)?,
        }.into_ok()
    }

    /// Return the unique handle
    pub fn fingerprint_from_path(
        path: &<Self as FsDirEntry>::Path,
        ctx: &mut <Self as FsDirEntry>::Context,
    ) -> Result<<Self as FsDirEntry>::DirFingerprint, <Self as FsDirEntry>::Error> {
        StandardDirFingerprint {
            handle: same_file::Handle::from_path(path)?
        }.into_ok()
    }

    /// device_num
    pub fn device_num_from_path(
        path: &<Self as FsDirEntry>::Path,
    ) -> Result<<Self as FsDirEntry>::DeviceNum, <Self as FsDirEntry>::Error> {
        ().into_ok()
    }

}

/// Functions for FsDirEntry
impl FsDirEntry for StandardDirEntry {
    type Context        = ();

    type Path           = std::path::Path;
    type PathBuf        = std::path::PathBuf;
    type FileName       = std::ffi::OsString;

    type Error          = std::io::Error;
    type FileType       = std::fs::FileType;
    type Metadata       = std::fs::Metadata;
    type ReadDir        = StandardReadDir;
    type DirFingerprint = StandardDirFingerprint;
    type DeviceNum      = ();
    type RootDirEntry   = StandardRootDirEntry;

    /// Get path of this entry
    fn path(&self) -> &Self::Path {
        &self.pathbuf    
    }
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf {
        self.pathbuf.clone()
    }
    /// Get path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error> {
        Self::canonicalize_from_path(self.path())
    }
    fn file_name(&self) -> Self::FileName {
        self.inner.file_name()
    }

    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut Self::Context,
    ) -> Result<Self::Metadata, Self::Error> {
        Self::metadata_from_path( &self.pathbuf, follow_link, ctx )
    }

    /// Read dir
    fn read_dir(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::ReadDir, Self::Error> {
        Self::read_dir_from_path( self.path(), ctx )
    }

    /// Return the unique handle
    fn fingerprint(
        &self,
        ctx: &mut Self::Context,
    ) -> Result<Self::DirFingerprint, Self::Error> {
        Self::fingerprint_from_path( self.path(), ctx )
    }

    /// device_num
    fn device_num(&self) -> Result<Self::DeviceNum, Self::Error> {
        Self::device_num_from_path( self.path() )
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub struct StandardDirFingerprint {
    handle: same_file::Handle,
}

////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct StandardRootDirEntry {
    pathbuf:    std::path::PathBuf,
    metadata:   std::fs::Metadata,
}

impl StandardRootDirEntry {
    pub fn from_path(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let pathbuf  = path.to_path_buf();
        let metadata = path.metadata()?;
        Self {
            pathbuf,
            metadata,
        }.into_ok()
    }
}

/// Functions for FsDirEntry
impl FsRootDirEntry for StandardRootDirEntry {
    type DirEntry = StandardDirEntry;


    /// Get path of this entry
    fn path(&self) -> &<Self::DirEntry as FsDirEntry>::Path {
        &self.pathbuf    
    }
    /// Get path of this entry
    fn pathbuf(&self) -> <Self::DirEntry as FsDirEntry>::PathBuf {
        self.pathbuf.clone()
    }
    /// Get path of this entry
    fn canonicalize(&self) -> Result<<Self::DirEntry as FsDirEntry>::PathBuf, <Self::DirEntry as FsDirEntry>::Error> {
        StandardDirEntry::canonicalize_from_path( self.path() )
    }

    fn file_name(
        &self
    ) -> <Self::DirEntry as FsDirEntry>::FileName {
        StandardDirEntry::file_name_from_path( self.path() )
    }

    /// Get metadata
    fn metadata(
        &self,
        follow_link: bool,
        ctx: &mut <Self::DirEntry as FsDirEntry>::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::Metadata, <Self::DirEntry as FsDirEntry>::Error> {
        StandardDirEntry::metadata_from_path( self.path(), follow_link, ctx )
    }

    /// Read dir
    fn read_dir(
        &self,
        ctx: &mut <Self::DirEntry as FsDirEntry>::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::ReadDir, <Self::DirEntry as FsDirEntry>::Error> {
        StandardDirEntry::read_dir_from_path( self.path(), ctx )
    }

    /// Return the unique handle
    fn fingerprint(
        &self,
        ctx: &mut <Self::DirEntry as FsDirEntry>::Context,
    ) -> Result<<Self::DirEntry as FsDirEntry>::DirFingerprint, <Self::DirEntry as FsDirEntry>::Error> {
        StandardDirEntry::fingerprint_from_path( self.path(), ctx )
    }

    /// device_num
    fn device_num(&self) -> Result<<Self::DirEntry as FsDirEntry>::DeviceNum, <Self::DirEntry as FsDirEntry>::Error> {
        StandardDirEntry::device_num_from_path( self.path() )
    }
}
