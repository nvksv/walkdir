use crate::source::SourceExt;

use std::fmt::Debug;
use std::fs;
use std::io;

use crate::rawdent::RawDirEntry;

/// Useful stub for nothing
#[derive(Debug, Clone, Default)]
pub struct Nil {}

impl SourceExt for Nil {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = Nil;
    type RawDirEntryExt = Nil;

    type FsFileName = std::ffi::OsStr;
    type FsDirEntry = std::fs::DirEntry;
    type FsReadDir = std::fs::ReadDir;
    type FsFileType = std::fs::FileType;
    type FsMetadata = std::fs::Metadata;

    type Path = std::path::Path;
    type PathBuf = std::path::PathBuf;

    type SameFileHandle = ();

    #[allow(unused_variables)]
    fn intoiter_new(self) -> Self::IntoIterExt {
        Self {}
    }

    #[allow(unused_variables)]
    fn get_handle<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::SameFileHandle> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn ancestor_new(dent: &RawDirEntry<Self>) -> io::Result<Self::AncestorExt> {
        Ok(Self {})
    }

    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> io::Result<bool> {
        Ok(false)
    }

    fn metadata<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::FsMetadata> {
        fs::metadata(path.as_ref())
    }

    /// Get metadata for symlink
    fn symlink_metadata<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::FsMetadata> {
        fs::symlink_metadata(path.as_ref())
    }

    /// Get metadata for symlink
    #[allow(unused_variables)]
    fn symlink_metadata_internal(
        raw_dent: &RawDirEntry<Self>,
        raw_dent_ext: &Self::RawDirEntryExt,
    ) -> io::Result<Self::FsMetadata> {
        Self::symlink_metadata(raw_dent.path())
    }

    #[allow(unused_variables)]
    fn read_dir<P: AsRef<Self::Path>>(
        dent: &RawDirEntry<Self>,
        path: P,
    ) -> io::Result<Self::FsReadDir> {
        fs::read_dir(path.as_ref())
    }

    #[allow(unused_variables)]
    fn rawdent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> io::Result<Self::RawDirEntryExt> {
        Ok(Self::RawDirEntryExt {})
    }

    #[allow(unused_variables)]
    fn rawdent_from_metadata(md: Self::FsMetadata) -> Self::RawDirEntryExt {
        Self::RawDirEntryExt {}
    }

    #[allow(unused_variables)]
    fn walkdir_new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    #[allow(unused_variables)]
    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "walkdir: same_file_system option not supported on this platform",
        ))
    }

    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
