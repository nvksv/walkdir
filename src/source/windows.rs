use crate::source::{SourceExt, Nil};

use std::path;
use std::fmt::Debug;
use std::io;
use std::fs;

use same_file;

use crate::dent::DirEntry;

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
pub struct WalkDirWindowsExt {
}

impl SourceExt for WalkDirWindowsExt {
    type OptionsExt = Nil;
    type IntoIterExt = Nil;
    type AncestorExt = AncestorWindowsExt;
    type DirEntryExt = DirEntryWindowsExt;

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

    fn get_handle<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::SameFileHandle> {
        same_file::Handle::from_path(path)
    }

    fn ancestor_new(dent: &DirEntry<Self>) -> io::Result<Self::AncestorExt> {
        let handle = same_file::Handle::from_path(dent.path())?;
        Ok(Self::AncestorExt { handle })
    }

    #[allow(unused_variables)]
    fn is_same(ancestor_path: &Self::PathBuf, ancestor_ext: &Self::AncestorExt, child: &Self::SameFileHandle) -> io::Result<bool> {
        Ok(child == &ancestor_ext.handle)
    }

    fn metadata<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::FsMetadata> {
        fs::metadata(path)
    }

    /// Get metadata for symlink
    fn symlink_metadata<P: AsRef<Self::Path>>(path: P) -> io::Result<Self::FsMetadata> {
        fs::symlink_metadata(path)
    }

    /// Get metadata for symlink
    fn symlink_metadata_internal(dent: &DirEntry<Self>) -> io::Result<Self::FsMetadata> {
        Ok(dent.ext.metadata.clone())
    }

    #[allow(unused_variables)]
    fn read_dir<P: AsRef<Self::Path>>(dent: &DirEntry<Self>, path: P) -> io::Result<Self::FsReadDir> {
        fs::read_dir(path.as_ref())
    }

    /// This works around a bug in Rust's standard library:
    /// https://github.com/rust-lang/rust/issues/46484
    #[allow(unused_variables)]
    fn is_dir(dent: &DirEntry<Self>) -> bool {
        use std::os::windows::fs::MetadataExt;
        use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;

        dent.ext.metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
    }

    fn dent_from_fsentry(ent: &Self::FsDirEntry) -> io::Result<Self::DirEntryExt> {
        Ok(Self::DirEntryExt { metadata: ent.metadata()? })
    }

    fn dent_from_metadata(md: Self::FsMetadata) -> Self::DirEntryExt {
        Self::DirEntryExt { metadata: md }
    }


    #[allow(unused_variables)]
    fn walkdir_new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        use winapi_util::{file, Handle};
    
        let h = Handle::from_path_any(path)?;
        file::information(h).map(|info| info.volume_serial_number())
    }
    
    fn get_file_name(path: &Self::PathBuf) -> &Self::FsFileName {
        path.file_name().unwrap_or_else(|| path.as_os_str())
    }
} 


