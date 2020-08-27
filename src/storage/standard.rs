use crate::storage::{util::Nil, StorageExt};
use crate::wd::IntoOk;

use std::fs;
use std::io;

impl StorageExt for Nil {
    type BuilderCtx = Nil;

    type OptionsExt = Nil;
    type IteratorExt = Nil;
    type AncestorExt = Nil;
    type RawDirEntryExt = Nil;
    type DirEntryExt = Nil;

    type Error = std::io::Error;
    type FileName = std::ffi::OsStr;
    type DirEntry = std::fs::DirEntry;
    type ReadDir = std::fs::ReadDir;
    type FileType = std::fs::FileType;
    type Metadata = std::fs::Metadata;

    type Path = std::path::Path;
    type PathBuf = std::path::PathBuf;

    type SameFileHandle = ();
    type DeviceNum = ();

    #[allow(unused_variables)]
    fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self {
        Self {}
    }

    #[allow(unused_variables)]
    fn ancestor_new<P: AsRef<Self::Path>>(
        path: P,
        dent: Option<&Self::DirEntry>,
        raw_ext: &Self::RawDirEntryExt,
    ) -> Result<Self::AncestorExt, Self::Error> {
        (Self::AncestorExt {}).into_ok()
    }

    #[allow(unused_variables)]
    fn iterator_new(self) -> Self::IteratorExt {
        Self {}
    }

    #[allow(unused_variables)]
    fn dent_new<P: AsRef<Self::Path>>(
        path: P,
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt,
    ) -> Self::DirEntryExt {
        Self::DirEntryExt {}
    }

    /// Create extension from DirEntry
    #[allow(unused_variables)]
    fn rawdent_from_fsentry(ent: &Self::DirEntry) -> Result<Self::RawDirEntryExt, Self::Error> {
        (Self::RawDirEntryExt {}).into_ok()
    }

    /// Create extension from metadata
    #[allow(unused_variables)]
    fn rawdent_from_path<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        md: Self::Metadata,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::RawDirEntryExt, Self::Error> {
        (Self::RawDirEntryExt {}).into_ok()
    }

    #[allow(unused_variables)]
    fn metadata<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        raw_ext: Option<&Self::RawDirEntryExt>,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::Metadata, Self::Error> {
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
    ) -> Result<Self::ReadDir, Self::Error> {
        fs::read_dir(path.as_ref())
    }

    /// Get metadata
    #[allow(unused_variables)]
    fn dent_metadata<P: AsRef<Self::Path>>(
        path: P,
        follow_link: bool,
        ext: &Self::DirEntryExt,
    ) -> Result<Self::Metadata, Self::Error> {
        if follow_link {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        }
    }

    #[allow(unused_variables)]
    fn get_handle<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::SameFileHandle> {
        ().into_ok()
    }

    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> io::Result<bool> {
        false.into_ok()
    }

    #[allow(unused_variables)]
    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::DeviceNum> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "walkdir: same_file_system option not supported on this platform",
        ))
    }

    fn get_file_name(path: &Self::Path) -> &Self::FileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
