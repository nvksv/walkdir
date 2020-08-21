use crate::source::{SourceExt, util::Nil};
use crate::wd::IntoOk;

use std::fs;
use std::io;

impl SourceExt for Nil {
    type BuilderCtx = Nil;
    
    type OptionsExt = Nil;
    type IteratorExt = Nil;
    type AncestorExt = Nil;
    type RawDirEntryExt = Nil;
    type DirEntryExt = Nil;

    type FsError = std::io::Error;
    type FsFileName = std::ffi::OsStr;
    type FsDirEntry = std::fs::DirEntry;
    type FsReadDir = std::fs::ReadDir;
    type FsFileType = std::fs::FileType;
    type FsMetadata = std::fs::Metadata;

    type Path = std::path::Path;
    type PathBuf = std::path::PathBuf;

    type SameFileHandle = ();

    #[allow(unused_variables)]
    fn builder_new<P: AsRef<Self::Path>>(root: P, ctx: Option<Self::BuilderCtx>) -> Self {
        Self {}
    }

    #[allow(unused_variables)]
    fn ancestor_new<P: AsRef<Self::Path>>(
        path: P, 
        dent: Option<&Self::FsDirEntry>, 
        raw_ext: &Self::RawDirEntryExt,
    ) -> Result<Self::AncestorExt, Self::FsError> {
        (Self::AncestorExt {}).into_ok()
    }

    #[allow(unused_variables)]
    fn iterator_new(self) -> Self::IteratorExt {
        Self {}
    }

    fn dent_new<P: AsRef<Self::Path>>( 
        path: P, 
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt, 
    ) -> Self::DirEntryExt {
        Self::DirEntryExt {}
    }

    /// Create extension from DirEntry
    fn rawdent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        (Self::RawDirEntryExt {}).into_ok()
    }

    /// Create extension from metadata
    fn rawdent_from_path<P: AsRef<Self::Path>>( path: P, follow_link: bool, md: Self::FsMetadata, ctx: &mut Self::IteratorExt ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        (Self::RawDirEntryExt {}).into_ok()
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

    /// Get metadata 
    fn dent_metadata<P: AsRef<Self::Path>>(
        path: P, 
        follow_link: bool, 
        ext: &Self::DirEntryExt,
    ) -> Result<Self::FsMetadata, Self::FsError> {
        if follow_link {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        }
    }

    #[allow(unused_variables)]
    fn get_handle<P: AsRef<Self::Path>>(
        path: P,
    ) -> io::Result<Self::SameFileHandle> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn is_same(
        ancestor_path: &Self::PathBuf,
        ancestor_ext: &Self::AncestorExt,
        child: &Self::SameFileHandle,
    ) -> io::Result<bool> {
        Ok(false)
    }


    #[allow(unused_variables)]
    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "walkdir: same_file_system option not supported on this platform",
        ))
    }

    fn get_file_name(path: &Self::Path) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
