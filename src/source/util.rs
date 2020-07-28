use crate::source::{
    SourceExt, SourceFsDirEntry, SourceFsFileType, SourceFsMetadata,
    SourceFsReadDir, SourcePath, SourcePathBuf,
};

impl SourcePath<std::path::PathBuf> for std::path::Path {
    #[inline(always)]
    fn to_path_buf(&self) -> std::path::PathBuf {
        self.to_path_buf()
    }
}

impl SourcePath<std::string::String> for str {
    #[inline(always)]
    fn to_path_buf(&self) -> std::string::String {
        self.to_string()
    }
}

impl<'s> SourcePathBuf<'s> for std::path::PathBuf {
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

impl<'s> SourcePathBuf<'s> for std::string::String {
    type Display = StringDisplay<'s>;

    #[inline(always)]
    fn display(&'s self) -> Self::Display {
        StringDisplay { inner: self }
    }
}

impl<E> SourceFsDirEntry<E> for std::fs::DirEntry
where
    E: SourceExt<
        Path = std::path::Path,
        PathBuf = std::path::PathBuf,
        FsFileType = std::fs::FileType,
    >,
{
    #[inline(always)]
    fn path(&self) -> E::PathBuf {
        self.path()
    }

    #[inline(always)]
    fn file_type(&self) -> std::io::Result<E::FsFileType> {
        std::fs::DirEntry::file_type(self)
    }
}

impl<E> SourceFsReadDir<E> for std::fs::ReadDir where
    E: SourceExt<
        Path = std::path::Path,
        PathBuf = std::path::PathBuf,
        FsDirEntry = std::fs::DirEntry,
        FsFileType = std::fs::FileType,
    >
{
}

impl SourceFsFileType for std::fs::FileType {
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

impl<E> SourceFsMetadata<E> for std::fs::Metadata
where
    E: SourceExt<FsFileType = std::fs::FileType>,
{
    #[inline(always)]
    fn file_type(&self) -> E::FsFileType {
        std::fs::Metadata::file_type(self)
    }
}
