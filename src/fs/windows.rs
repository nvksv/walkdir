use crate::fs::standard::{StandardDirEntry, StandardDirFingerprint, StandardReadDir};
use crate::fs::{FsDirEntry, FsReadDir, FsDirFingerprint};
use crate::wd::IntoOk;

use std::fmt::Debug;
use std::fs;
use std::io;
use std::path;

use same_file;


// impl StorageExt for WalkDirWindowsExt {
//     type BuilderCtx = Nil;

//     type OptionsExt = Nil;
//     type IteratorExt = Nil;
//     type AncestorExt = AncestorWindowsExt;
//     type RawDirEntryExt = DirEntryWindowsExt;
//     type DirEntryExt = DirEntryWindowsExt;

//     type Error = std::io::Error;
//     type FileName = std::ffi::OsStr;
//     type DirEntry = std::fs::DirEntry;
//     type ReadDir = std::fs::ReadDir;
//     type FileType = std::fs::FileType;
//     type Metadata = std::fs::Metadata;

//     type Path = path::Path;
//     type PathBuf = path::PathBuf;

//     type SameFileHandle = same_file::Handle;
//     type DeviceNum = u64;

//     #[allow(unused_variables)]
//     fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self {
//         Self {}
//     }

//     #[allow(unused_variables)]
//     fn ancestor_new<P: AsRef<Self::Path>>(
//         path: P,
//         dent: Option<&Self::DirEntry>,
//         raw_ext: &Self::RawDirEntryExt,
//     ) -> Result<Self::AncestorExt, Self::Error> {
//         let handle = same_file::Handle::from_path(path)?;
//         (Self::AncestorExt { handle }).into_ok()
//     }

//     #[allow(unused_variables)]
//     fn iterator_new(self) -> Self::IteratorExt {
//         Self::IteratorExt {}
//     }

//     #[allow(unused_variables)]
//     fn dent_new<P: AsRef<Self::Path>>(
//         path: P,
//         raw_ext: &Self::RawDirEntryExt,
//         ctx: &mut Self::IteratorExt,
//     ) -> Self::DirEntryExt {
//         raw_ext.clone()
//     }

//     /// Create extension from DirEntry
//     fn rawdent_from_fsentry(ent: &Self::DirEntry) -> Result<Self::RawDirEntryExt, Self::Error> {
//         Self::RawDirEntryExt { metadata: ent.metadata()? }.into_ok()
//     }

//     /// Create extension from metadata
//     #[allow(unused_variables)]
//     fn rawdent_from_path<P: AsRef<Self::Path>>(
//         path: P,
//         follow_link: bool,
//         md: Self::Metadata,
//         ctx: &mut Self::IteratorExt,
//     ) -> Result<Self::RawDirEntryExt, Self::Error> {
//         Self::RawDirEntryExt { metadata: md }.into_ok()
//     }

//     #[allow(unused_variables)]
//     fn metadata<P: AsRef<Self::Path>>(
//         path: P,
//         follow_link: bool,
//         raw_ext: Option<&Self::RawDirEntryExt>,
//         ctx: &mut Self::IteratorExt,
//     ) -> Result<Self::Metadata, Self::Error> {
//         if let Some(raw_ext) = raw_ext {
//             return raw_ext.metadata.clone().into_ok();
//         };

//         if follow_link {
//             fs::metadata(path)
//         } else {
//             fs::symlink_metadata(path)
//         }
//     }

//     #[allow(unused_variables)]
//     fn read_dir<P: AsRef<Self::Path>>(
//         path: P,
//         raw_ext: &Self::RawDirEntryExt,
//         ctx: &mut Self::IteratorExt,
//     ) -> Result<Self::ReadDir, Self::Error> {
//         fs::read_dir(path.as_ref())
//     }

//     #[allow(unused_variables)]
//     fn dent_metadata<P: AsRef<Self::Path>>(
//         path: P,
//         follow_link: bool,
//         ext: &Self::DirEntryExt,
//     ) -> Result<Self::Metadata, Self::Error> {
//         if follow_link {
//             fs::metadata(path)
//         } else {
//             fs::symlink_metadata(path)
//         }
//     }

//     /// This works around a bug in Rust's standard library:
//     /// https://github.com/rust-lang/rust/issues/46484
//     #[allow(unused_variables)]
//     fn is_dir(dent: &Self::DirEntry, raw_ext: &Self::RawDirEntryExt) -> bool {
//         use std::os::windows::fs::MetadataExt;
//         use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;

//         raw_ext.metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
//     }

//     fn get_handle<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::SameFileHandle> {
//         same_file::Handle::from_path(path)
//     }

//     #[allow(unused_variables)]
//     fn is_same(
//         ancestor_path: &Self::PathBuf,
//         ancestor_ext: &Self::AncestorExt,
//         child: &Self::SameFileHandle,
//     ) -> io::Result<bool> {
//         Ok(child == &ancestor_ext.handle)
//     }

//     fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::DeviceNum> {
//         use winapi_util::{file, Handle};

//         let h = Handle::from_path_any(path)?;
//         file::information(h).map(|info| info.volume_serial_number())
//     }

//     fn get_file_name(path: &Self::Path) -> &Self::FileName {
//         path.file_name().unwrap_or_else(|| path.as_os_str())
//     }
// }

#[derive(Debug)]
pub struct WindowsReadDir {
    standard: StandardReadDir,
}

impl WindowsReadDir {
    fn standard(&self) -> &std::fs::ReadDir {
        self.standard()
    }
    // fn standard(&self) -> &StandardReadDir {
    //     &self.standard
    // }
}

/// Functions for FsReadDir
impl FsReadDir for WindowsReadDir {
    type Inner      = StandardReadDir;
    type Error      = std::io::Error;
    type DirEntry   = WindowsDirEntry;

    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.standard
    }

    fn process_inner_entry(&mut self, inner_entry: StandardDirEntry) -> Result<Self::DirEntry, Self::Error> {
        Self::DirEntry::from_inner(inner_entry)
    }
}

impl Iterator for WindowsReadDir {
    type Item = Result<WindowsDirEntry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_fsentry()
    }
}

#[derive(Debug)]
pub struct WindowsDirEntry {
    standard: StandardDirEntry,
    /// The underlying metadata (Windows only). We store this on Windows
    /// because this comes for free while reading a directory.
    ///
    /// We use this to determine whether an entry is a directory or not, which
    /// works around a bug in Rust's standard library:
    /// https://github.com/rust-lang/rust/issues/46484
    metadata: fs::Metadata,
}

impl WindowsDirEntry {
    pub fn inner(&self) -> &std::fs::DirEntry {
        self.standard.inner()
    }

    pub fn standard(&self) -> &StandardDirEntry {
        &self.standard
    }

    pub fn from_inner(inner: StandardDirEntry) -> Result<Self, std::io::Error> {
        let metadata = inner.inner().metadata()?;
        Self {
            metadata,
            standard: inner,
        }.into_ok()
    }
}

/// Functions for FsDirEntry
impl FsDirEntry for WindowsDirEntry {
    type Context        = <StandardDirEntry as FsDirEntry>::Context;

    type Path           = <StandardDirEntry as FsDirEntry>::Path;
    type PathBuf        = <StandardDirEntry as FsDirEntry>::PathBuf;

    type Error          = <StandardDirEntry as FsDirEntry>::Error;
    type FileType       = <StandardDirEntry as FsDirEntry>::FileType;
    type Metadata       = std::fs::Metadata;
    type ReadDir        = WindowsReadDir;
    type DirFingerprint = <StandardDirEntry as FsDirEntry>::DirFingerprint;
    type DeviceNum      = <StandardDirEntry as FsDirEntry>::DeviceNum;

    /// Get path of this entry
    fn path(&self) -> &Self::Path {
        self.standard.path()
    }
    /// Get path of this entry
    fn pathbuf(&self) -> Self::PathBuf {
        self.standard.pathbuf()
    }

    /// Get type of this entry
    fn file_type(&self) -> Result<Self::FileType, Self::Error> {
        self.standard.file_type()
    }

    /// Get path of this entry
    fn canonicalize(&self) -> Result<Self::PathBuf, Self::Error> {
        self.standard.canonicalize()
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
        WindowsReadDir {
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
struct WindowsDirFingerprint {
    handle: same_file::Handle,
}

impl FsDirFingerprint for WindowsDirFingerprint {
    fn is_same(&self, rhs: &Self) -> bool {
        self.handle == rhs.handle
    }
}
