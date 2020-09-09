use std::cmp::Ord;
use std::convert::AsRef;
use std::fmt;
use std::marker::Send;
use std::ops::Deref;

/// Functions for StorageExt::Path
pub trait FsPath: Ord
{
    type PathBuf: Deref<Target = Self> + AsRef<Self>;

    /// Copy to owned
    fn to_path_buf(&self) -> Self::PathBuf;
}

/// Functions for StorageExt::PathBuf
pub trait FsPathBuf<'s>: Sized 
+ fmt::Debug
+ Clone
+ Send
+ Sync
{
    /// Intermediate object
    type Display: 's + fmt::Display;

    /// Create intermediate object which can Display
    fn display(&'s self) -> Self::Display;
}

pub trait FsFileName: FsPath {
    type FileName: ?Sized;

    /// file_name
    fn file_name(&self) -> &Self::FileName;
}

//////////////////////////////////////////////////////////////////////////////////////

impl FsPath for std::path::Path {
    type PathBuf = std::path::PathBuf;

    #[inline(always)]
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.to_path_buf()
    }
}

impl FsFileName for std::path::Path {
    type FileName = std::ffi::OsStr;

    fn file_name(&self) -> &Self::FileName {
        std::path::Path::file_name(self).unwrap_or_else(|| std::path::Path::as_os_str(self))
    }
}

impl<'s> FsPathBuf<'s> for std::path::PathBuf {
    type Display = std::path::Display<'s>;

    #[inline(always)]
    fn display(&'s self) -> Self::Display {
        std::path::Path::display(self)
    }
}

//////////////////////////////////////////////////////////////////////////////////////

impl FsPath for str {
    type PathBuf = std::string::String;

    #[inline(always)]
    fn to_path_buf(&self) -> std::string::String {
        self.to_string()
    }
}

pub struct StringDisplay<'s> {
    inner: &'s std::string::String,
}

impl<'s> std::fmt::Display for StringDisplay<'s> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.inner, f)
    }
}

impl<'s> FsPathBuf<'s> for std::string::String {
    type Display = StringDisplay<'s>;

    #[inline(always)]
    fn display(&'s self) -> Self::Display {
        StringDisplay { inner: self }
    }
}