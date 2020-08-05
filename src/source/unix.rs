use crate::source::{Nil, SourceExt};

use std::fmt::Debug;
use std::fs;
use std::io;
use std::path;

use same_file;

use crate::dent::DirEntry;

#[derive(Debug, Clone)]
pub struct DirEntryUnixExt {
    /// The underlying inode number (Unix only).
    pub(crate) ino: u64,
}

/// Unix-specific extensions
#[derive(Debug, Clone)]
pub struct WalkDirUnixExt {}

impl SourceExt for WalkDirUnixExt {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = Nil;
    type DirEntryExt = DirEntryUnixExt;
    type RawDirEntryExt = DirEntryUnixExt;

    type FsFileName = std::ffi::OsStr;
    type FsDirEntry = std::fs::DirEntry;
    type FsReadDir = std::fs::ReadDir;
    type FsFileType = std::fs::FileType;
    type FsMetadata = std::fs::Metadata;

    type Path = path::Path;
    type PathBuf = path::PathBuf;

    type SameFileHandle = same_file::Handle;

    #[allow(unused_variables)]
    fn intoiter_new(self) -> Self::IntoIterExt {
        Self::IntoIterExt {}
    }

    fn get_handle<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::SameFileHandle> {
        same_file::Handle::from_path(path)
    }

    #[allow(unused_variables)]
    fn ancestor_new(dent: &DirEntry<Self>) -> io::Result<Self::AncestorExt> {
        Ok(Self::AncestorExt {})
    }

    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> io::Result<bool> {
        Ok(child == &Self::get_handle(ancestor_path)?)
    }

    fn metadata<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::FsMetadata> {
        fs::metadata(path)
    }

    /// Get metadata for symlink
    fn symlink_metadata<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::FsMetadata> {
        fs::symlink_metadata(path)
    }

    /// Get metadata for symlink
    fn symlink_metadata_internal(
        dent: &DirEntry<Self>,
    ) -> io::Result<Self::FsMetadata> {
        Self::symlink_metadata(&dent.path())
    }

    #[allow(unused_variables)]
    fn read_dir<P: AsRef<Self::Path>>(
        dent: &DirEntry<Self>,
        path: P,
    ) -> io::Result<Self::FsReadDir> {
        fs::read_dir(path.as_ref())
    }

    fn dent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> io::Result<Self::DirEntryExt> {
        use std::os::unix::fs::DirEntryExt;
        Ok(Self::DirEntryExt { ino: ent.ino() })
    }

    fn dent_from_metadata(md: Self::FsMetadata) -> Self::DirEntryExt {
        use std::os::unix::fs::MetadataExt;
        Self::DirEntryExt { ino: md.ino() }
    }

    #[allow(unused_variables)]
    fn dent_from_rawdent(
        raw: &Self::RawDirEntryExt,
    ) -> Self::DirEntryExt {
        raw
    }

    #[allow(unused_variables)]
    fn walkdir_new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        use std::os::unix::fs::MetadataExt;

        path.as_ref().metadata().map(|md| md.dev())
    }

    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
