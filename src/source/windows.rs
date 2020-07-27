use crate::source::{SourceExt, SourceAncestorExt, SourceDirEntryExt, Nil};

use std::path;
use std::fmt::Debug;
use std::io;
use std::fs;
//use std::marker::Sized;

use same_file::Handle;

use crate::dent::DirEntry;
use crate::Ancestor;

#[derive(Debug)]
pub struct AncestorWindowsExt {
    /// An open file to this ancesor. This is only used on Windows where
    /// opening a file handle appears to be quite expensive, so we choose to
    /// cache it. This comes at the cost of not respecting the file descriptor
    /// limit set by the user.
    handle: Handle,
}

impl<E: SourceExt> SourceAncestorExt<E> for AncestorWindowsExt {

    fn new(dent: &DirEntry<E>) -> io::Result<Self> {
        let handle = Handle::from_path(dent.path())?;
        Ok(Self { handle })
    }

    #[allow(unused_variables)]
    fn is_same(&self, ancestor: &Ancestor<E>, child: &Handle) -> io::Result<bool> {
        Ok(child == &self.handle)
    }

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

impl<E: SourceExt> SourceDirEntryExt<E> for DirEntryWindowsExt {

    fn metadata<P: AsRef<E::Path>>(&self, path: P) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }

    #[allow(unused_variables)]
    fn symlink_metadata(&self, entry: &DirEntry<E>) -> io::Result<fs::Metadata> {
        Ok(self.metadata.clone())
    }

    /// This works around a bug in Rust's standard library:
    /// https://github.com/rust-lang/rust/issues/46484
    #[allow(unused_variables)]
    fn is_dir(&self, entry: &DirEntry<E>) -> bool {
        use std::os::windows::fs::MetadataExt;
        use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;

        self.metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
    }

    fn from_entry(ent: &fs::DirEntry) -> io::Result<Self> {
        Ok(Self { metadata: ent.metadata()? })
    }

    fn from_metadata(md: fs::Metadata) -> Self {
        Self { metadata: md }
    }
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

    type PathBuf = path::PathBuf;
    type Path = path::Path;

    #[allow(unused_variables)]
    fn new<P: AsRef<Self::Path>>(root: P) -> Self {
        Self {}
    }

    fn device_num<P: AsRef<Self::Path>>(path: P) -> io::Result<u64> {
        use winapi_util::{file, Handle};
    
        let h = Handle::from_path_any(path)?;
        file::information(h).map(|info| info.volume_serial_number())
    }
    
} 


