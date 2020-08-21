use crate::source::{SourceExt, util::Nil};
use crate::wd::IntoOk;

use std::fmt::Debug;
use std::fs;
use std::io;
use std::path;

use same_file;

#[derive(Debug)]
pub struct AncestorWindowsExt {
    /// An open file to this ancesor. This is only used on Windows where
    /// opening a file handle appears to be quite expensive, so we choose to
    /// cache it. This comes at the cost of not respecting the file descriptor
    /// limit set by the user.
    handle: same_file::Handle,
}

#[derive(Debug, Clone)]
pub struct DirEntryWindowsExt {
    /// The underlying metadata (Windows only). We store this on Windows
    /// because this comes for free while reading a directory.
    ///
    /// We use this to determine whether an entry is a directory or not, which
    /// works around a bug in Rust's standard library:
    /// https://github.com/rust-lang/rust/issues/46484
    metadata: fs::Metadata,
}

/// Windows-specific extensions
#[derive(Debug, Clone)]
pub struct WalkDirWindowsExt {}

impl SourceExt for WalkDirWindowsExt {
    type BuilderCtx = Nil;

    type OptionsExt = Nil;
    type IteratorExt = Nil;
    type AncestorExt = AncestorWindowsExt;
    type RawDirEntryExt = DirEntryWindowsExt;
    type DirEntryExt = DirEntryWindowsExt;

    type FsError = std::io::Error;
    type FsFileName = std::ffi::OsStr;
    type FsDirEntry = std::fs::DirEntry;
    type FsReadDir = std::fs::ReadDir;
    type FsFileType = std::fs::FileType;
    type FsMetadata = std::fs::Metadata;

    type Path = path::Path;
    type PathBuf = path::PathBuf;

    type SameFileHandle = same_file::Handle;

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
        let handle = same_file::Handle::from_path(path)?;
        (Self::AncestorExt { handle }).into_ok()
    }

    #[allow(unused_variables)]
    fn iterator_new(self) -> Self::IteratorExt {
        Self::IteratorExt {}
    }

    fn dent_new<P: AsRef<Self::Path>>( 
        path: P, 
        raw_ext: &Self::RawDirEntryExt,
        ctx: &mut Self::IteratorExt, 
    ) -> Self::DirEntryExt {
        raw_ext.clone()
    }

    /// Create extension from DirEntry
    fn rawdent_from_fsentry(
        ent: &Self::FsDirEntry,
    ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        Self::RawDirEntryExt { metadata: ent.metadata()? }
            .into_ok()
    }

    /// Create extension from metadata
    fn rawdent_from_path<P: AsRef<Self::Path>>( 
        path: P, 
        follow_link: bool, 
        md: Self::FsMetadata, 
        ctx: &mut Self::IteratorExt 
    ) -> Result<Self::RawDirEntryExt, Self::FsError> {
        Self::RawDirEntryExt { metadata: md }
            .into_ok()
    }

    fn metadata<P: AsRef<Self::Path>>(
        path: P, 
        follow_link: bool, 
        raw_ext: Option<&Self::RawDirEntryExt>,
        ctx: &mut Self::IteratorExt,
    ) -> Result<Self::FsMetadata, Self::FsError> {
        if let Some(raw_ext) = raw_ext {
            return raw_ext.metadata.clone().into_ok() 
        };

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

    /// This works around a bug in Rust's standard library:
    /// https://github.com/rust-lang/rust/issues/46484
    #[allow(unused_variables)]
    fn is_dir(dent: &Self::FsDirEntry, raw_ext: &Self::RawDirEntryExt) -> bool {
        use std::os::windows::fs::MetadataExt;
        use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;

        raw_ext.metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
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
        Ok(child == &ancestor_ext.handle)
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        use winapi_util::{file, Handle};

        let h = Handle::from_path_any(path)?;
        file::information(h).map(|info| info.volume_serial_number())
    }

    fn get_file_name(path: &Self::Path) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
}
