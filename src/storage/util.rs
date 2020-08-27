use crate::storage::{
    StorageDirEntry, StorageError, StorageExt, StorageFileType, StorageMetadata, StoragePath,
    StoragePathBuf, StorageReadDir,
};

use crate::error;

/// Useful stub for nothing
#[derive(Debug, Clone, Default)]
pub struct Nil {}

impl StoragePath<std::path::PathBuf> for std::path::Path {
    #[inline(always)]
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.to_path_buf()
    }
}

impl StoragePath<std::string::String> for str {
    #[inline(always)]
    fn to_path_buf(&self) -> std::string::String {
        self.to_string()
    }
}

impl<'s> StoragePathBuf<'s> for std::path::PathBuf {
    type Display = std::path::Display<'s>;

    #[inline(always)]
    fn display(&'s self) -> Self::Display {
        std::path::Path::display(self)
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

impl<'s> StoragePathBuf<'s> for std::string::String {
    type Display = StringDisplay<'s>;

    #[inline(always)]
    fn display(&'s self) -> Self::Display {
        StringDisplay { inner: self }
    }
}

impl<E> StorageDirEntry<E> for std::fs::DirEntry
where
    E: 'static
        + StorageExt<
            Path = std::path::Path,
            PathBuf = std::path::PathBuf,
            Error = std::io::Error,
            FileType = std::fs::FileType,
        >,
{
    #[inline(always)]
    fn path(&self) -> E::PathBuf {
        self.path()
    }

    #[inline(always)]
    fn file_type(&self) -> Result<E::FileType, E::Error> {
        std::fs::DirEntry::file_type(self)
    }
}

impl<E> StorageReadDir<E> for std::fs::ReadDir where
    E: 'static
        + StorageExt<
            Path = std::path::Path,
            PathBuf = std::path::PathBuf,
            Error = std::io::Error,
            DirEntry = std::fs::DirEntry,
            FileType = std::fs::FileType,
        >
{
}

impl StorageFileType for std::fs::FileType {
    #[inline(always)]
    fn is_dir(&self) -> bool {
        std::fs::FileType::is_dir(self)
    }

    #[inline(always)]
    fn is_file(&self) -> bool {
        std::fs::FileType::is_file(self)
    }

    #[inline(always)]
    fn is_symlink(&self) -> bool {
        std::fs::FileType::is_symlink(self)
    }
}

impl<E> StorageMetadata<E> for std::fs::Metadata
where
    E: StorageExt<FileType = std::fs::FileType>,
{
    #[inline(always)]
    fn file_type(&self) -> E::FileType {
        std::fs::Metadata::file_type(self)
    }
}

impl<E> StorageError<E> for std::io::Error
where
    E: 'static + StorageExt<Error = std::io::Error>,
{
    #[inline(always)]
    fn new(kind: std::io::ErrorKind, error: error::Error<E>) -> Self {
        std::io::Error::new(kind, error)
    }

    #[inline(always)]
    fn kind(&self) -> std::io::ErrorKind {
        self.kind()
    }
}
