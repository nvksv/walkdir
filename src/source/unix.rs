use crate::source::{Nil, SourceExt};

use std::fmt::Debug;
use std::fs;
use std::io;
use std::path;

use same_file;

use crate::dent::DirEntry;

#[derive(Debug, Clone)]
pub struct RawDirEntryUnixExt {
    /// The underlying inode number (Unix only).
    pub(crate) ino: u64,
}

/// Unix-specific extensions
#[derive(Debug, Clone)]
pub struct WalkDirUnixExt {}

impl SourceExt for WalkDirUnixExt {
    type BuilderCtx = Nil;

    type OptionsExt = Nil;
    type IteratorExt = Nil;
    type AncestorExt = Nil;
    type DirEntryExt = DirEntryUnixExt;
    type RawDirEntryExt = DirEntryUnixExt;

    type FsError = std::io::Error;
    type FsFileName = std::ffi::OsStr;
    type FsDirEntry = std::fs::DirEntry;
    type FsReadDir = std::fs::ReadDir;
    type FsFileType = std::fs::FileType;
    type FsMetadata = std::fs::Metadata;

    type Path = path::Path;
    type PathBuf = path::PathBuf;

    type SameFileHandle = same_file::Handle;

    /// Make new builder
    #[allow(unused_variables)]
    fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self {
        Self {}
    }

    /// Make new ancestor
    fn ancestor_new(dent: &Self::FsDirEntry) -> Result<Self::AncestorExt, Self::FsError> {
        Ok(Self::AncestorExt {})
    }

    #[allow(unused_variables)]
    fn iterator_new(self) -> Self::IteratorExt {
        Self::IteratorExt {}
    }

    /// Create extension from DirEntry
    fn rawdent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        (Self::RawDirEntryExt { ino: ent.ino() }).into_ok()
    }

    /// Create extension from metadata
    fn rawdent_from_path<P: AsRef<Self::Path>>( path: P, follow_link: bool, md: Self::FsMetadata, ctx: &mut Self::IteratorExt ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        Self::RawDirEntryExt { ino: md.ino() }
    }

    fn metadata<P: AsRef<Self::Path>>(
        path: P, 
        follow_link: bool, 
        raw_ext: Option<&Self::RawDirEntryExt>,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::FsMetadata, Self::FsError> {
        if follow_link {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        }
    }

    #[allow(unused_variables)]
    fn read_dir<P: AsRef<Self::Path>>(
        path: P,
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::FsReadDir, Self::FsError> {
        fs::read_dir(path.as_ref())
    }

    fn get_handle<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::SameFileHandle> {
        same_file::Handle::from_path(path)
    }

    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> io::Result<bool> {
        Ok(child == &Self::get_handle(ancestor_path)?)
    }

    #[allow(unused_variables)]
    fn dent_from_rawdent(
        raw: &Self::RawDirEntryExt,
    ) -> Self::DirEntryExt {
        raw
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        use std::os::unix::fs::MetadataExt;

        path.as_ref().metadata().map(|md| md.dev())
    }

    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
