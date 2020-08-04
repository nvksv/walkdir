use crate::source;
use crate::error;
use crate::dent::DirEntry;

pub use error::{Error, ErrorInner};

/// A result type for walkdir operations.
///
/// Note that this result type embeds the error type in this crate. This
/// is only useful if you care about the additional information provided by
/// the error (such as the path associated with the error or whether a loop
/// was dectected). If you want things to Just Work, then you can use
/// [`io::Result`] instead since the error type in this package will
/// automatically convert to an [`io::Result`] when using the [`try!`] macro.
///
/// [`io::Result`]: https://doc.rust-lang.org/stable/std/io/type.Result.html
/// [`try!`]: https://doc.rust-lang.org/stable/std/macro.try.html
pub type Result<T, E = source::DefaultSourceExt> = ::std::result::Result<T, self::Error<E>>;

pub type ResultInner<T, E = source::DefaultSourceExt> = ::std::result::Result<T, self::ErrorInner<E>>;

/// A DirEntry sorter function.
pub type FnCmp<E> = Box<
    dyn FnMut(&DirEntry<E>, &DirEntry<E>) -> std::cmp::Ordering
        + Send
        + Sync
        + 'static,
>;

pub type DeviceNum = u64;

// Convert FsReadDir.next() to some Option<T>.
// - Some(T) -- add T to collected vec,
// - None -- entry must be ignored
//pub trait FnProcessFsDirEntry<E: source::SourceExt, T>: FnMut(self::Result<DirEntry<E>, E>) -> Option<T> {}

/// Follow symlinks and check same_file_system. Also determine is_dir flag.
/// - Some(Ok((dent, is_dir))) -- normal entry to yielding
/// - Some(Err(_)) -- some error occured
/// - None -- entry must be ignored
//pub type ProcessDirEntry<E: source::SourceExt> = self::Result<(DirEntry<E>, bool), E>

/// A variants for filtering content
#[derive(Debug, PartialEq, Eq)]
pub enum ContentFilter {
    /// No filter, all content will be yielded (default)
    None,
    /// Yield files only
    FilesOnly,
    /// Yield dirs only
    DirsOnly
}

/// A variants for ordering content
#[derive(Debug, PartialEq, Eq)]
pub enum ContentOrder {
    /// No arrange (default)
    None,
    /// Yield files first, then dirs
    FilesFirst,
    /// Yield dirs (with theirs content) first, then files
    DirsFirst
}

/// A position in dirs tree
#[derive(Debug, PartialEq, Eq)]
pub enum Position<BC, EN, ER> {
    /// Before content of current dir
    BeforeContent(BC),
    /// An entry
    Entry(EN),
    /// An error
    Error(ER),
    /// After content of current dir
    AfterContent,
}

