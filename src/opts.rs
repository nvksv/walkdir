use std::fmt;
use std::result;
use std::cmp;

use crate::wd;
use crate::source;
use crate::wd::{Position, ContentFilter, ContentOrder, FnCmp};
use crate::dent::DirEntry;
use crate::source::SourcePath;
use crate::walk::IntoIter;
use crate::dir::FlatDirEntry;
use crate::iter::{WalkDirIter, ClassicIter};

/////////////////////////////////////////////////////////////////////////
//// WalkDirOptions

pub struct WalkDirOptionsImmut<E: source::SourceExt> {
    pub same_file_system: bool,
    pub follow_links: bool,
    pub yield_loop_links: bool,
    pub max_open: usize,
    pub min_depth: usize,
    pub max_depth: usize,
    pub contents_first: bool,
    pub content_filter: ContentFilter,
    pub content_order: ContentOrder,
    /// Extension part
    #[allow(dead_code)]
    ext: E::OptionsExt,
}

pub struct WalkDirOptions<E: source::SourceExt> {
    pub immut: WalkDirOptionsImmut<E>,
    pub sorter: Option<FnCmp<E>>,
}

impl<E: source::SourceExt> Default for WalkDirOptions<E> { 
    fn default() -> Self {
        Self {
            immut: WalkDirOptionsImmut {
                same_file_system: false,
                follow_links: false,
                yield_loop_links: false,
                max_open: 10,
                min_depth: 0,
                max_depth: ::std::usize::MAX,
                contents_first: false,
                content_filter: ContentFilter::None,
                content_order: ContentOrder::None,
                ext: E::OptionsExt::default(),
            },
            sorter: None,
        }
    }
}

impl<E: source::SourceExt> fmt::Debug for WalkDirOptions<E> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> result::Result<(), fmt::Error> {
        let sorter_str = if self.sorter.is_some() {
            // FnMut isn't `Debug`
            "Some(...)"
        } else {
            "None"
        };
        f.debug_struct("WalkDirOptions")
            .field("same_file_system", &self.immut.same_file_system)
            .field("follow_links", &self.immut.follow_links)
            .field("yield_loop_links", &self.immut.yield_loop_links)
            .field("max_open", &self.immut.max_open)
            .field("min_depth", &self.immut.min_depth)
            .field("max_depth", &self.immut.max_depth)
            .field("contents_first", &self.immut.contents_first)
            .field("content_filter", &self.immut.content_filter)
            .field("content_order", &self.immut.content_order)
            .field("sorter", &sorter_str)
            .field("ext", &self.immut.ext)
            .finish()
    }
}




/////////////////////////////////////////////////////////////////////////
//// WalkDir

/// A builder to create an iterator for recursively walking a directory.
///
/// Results are returned in depth first fashion, with directories yielded
/// before their contents. If [`contents_first`] is true, contents are yielded
/// before their directories. The order is unspecified but if [`sort_by`] is
/// given, directory entries are sorted according to this function. Directory
/// entries `.` and `..` are always omitted.
///
/// If an error occurs at any point during iteration, then it is returned in
/// place of its corresponding directory entry and iteration continues as
/// normal. If an error occurs while opening a directory for reading, then it
/// is not descended into (but the error is still yielded by the iterator).
/// Iteration may be stopped at any time. When the iterator is destroyed, all
/// resources associated with it are freed.
///
/// [`contents_first`]: struct.WalkDir.html#method.contents_first
/// [`sort_by`]: struct.WalkDir.html#method.sort_by
///
/// # Usage
///
/// This type implements [`IntoIterator`] so that it may be used as the subject
/// of a `for` loop. You may need to call [`into_iter`] explicitly if you want
/// to use iterator adapters such as [`filter_entry`].
///
/// Idiomatic use of this type should use method chaining to set desired
/// options. For example, this only shows entries with a depth of `1`, `2` or
/// `3` (relative to `foo`):
///
/// ```no_run
/// use walkdir::WalkDir;
/// # use walkdir::Error;
///
/// # fn try_main() -> Result<(), Error> {
/// for entry in <WalkDir>::new("foo").min_depth(1).max_depth(3).into_classic() {
///     println!("{}", entry?.path().display());
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`IntoIterator`]: https://doc.rust-lang.org/stable/std/iter/trait.IntoIterator.html
/// [`into_iter`]: https://doc.rust-lang.org/nightly/core/iter/trait.IntoIterator.html#tymethod.into_iter
/// [`filter_entry`]: struct.IntoIter.html#method.filter_entry
///
/// Note that the iterator by default includes the top-most directory. Since
/// this is the only directory yielded with depth `0`, it is easy to ignore it
/// with the [`min_depth`] setting:
///
/// ```no_run
/// use walkdir::WalkDir;
/// # use walkdir::Error;
///
/// # fn try_main() -> Result<(), Error> {
/// for entry in <WalkDir>::new("foo").min_depth(1).into_classic() {
///     println!("{}", entry?.path().display());
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`min_depth`]: struct.WalkDir.html#method.min_depth
///
/// This will only return descendents of the `foo` directory and not `foo`
/// itself.
///
/// # Loops
///
/// This iterator (like most/all recursive directory iterators) assumes that
/// no loops can be made with *hard* links on your file system. In particular,
/// this would require creating a hard link to a directory such that it creates
/// a loop. On most platforms, this operation is illegal.
///
/// Note that when following symbolic/soft links, loops are detected and an
/// error is reported.
#[derive(Debug)]
pub struct WalkDir<E: source::SourceExt = source::DefaultSourceExt> {
    opts: WalkDirOptions<E>,
    root: E::PathBuf,
    /// Extension part
    ext: E,
}

impl<E: source::SourceExt> WalkDir<E> {
    /// Create a builder for a recursive directory iterator starting at the
    /// file path `root`. If `root` is a directory, then it is the first item
    /// yielded by the iterator. If `root` is a file, then it is the first
    /// and only item yielded by the iterator. If `root` is a symlink, then it
    /// is always followed for the purposes of directory traversal. (A root
    /// `DirEntry` still obeys its documentation with respect to symlinks and
    /// the `follow_links` setting.)
    pub fn new<P: AsRef<E::Path>>(root: P) -> Self {
        WalkDir {
            opts: WalkDirOptions::default(),
            root: root.as_ref().to_path_buf(),
            ext: E::walkdir_new(root),
        }
    }

    /// Into classic iterator
    pub fn into_classic(self) -> ClassicIter<E, IntoIter<E>> {
        self.into_iter().into_classic()
    }

    /// Do not cross file system boundaries.
    ///
    /// When this option is enabled, directory traversal will not descend into
    /// directories that are on a different file system from the root path.
    ///
    /// Currently, this option is only supported on Unix and Windows. If this
    /// option is used on an unsupported platform, then directory traversal
    /// will immediately return an error and will not yield any entries.
    pub fn same_file_system(mut self, yes: bool) -> Self {
        self.opts.immut.same_file_system = yes;
        self
    }

    /// Follow symbolic links. By default, this is disabled.
    ///
    /// When `yes` is `true`, symbolic links are followed as if they were
    /// normal directories and files. If a symbolic link is broken or is
    /// involved in a loop, an error is yielded.
    ///
    /// When enabled, the yielded [`DirEntry`] values represent the target of
    /// the link while the path corresponds to the link. See the [`DirEntry`]
    /// type for more details.
    ///
    /// [`DirEntry`]: struct.DirEntry.html
    pub fn follow_links(mut self, yes: bool) -> Self {
        self.opts.immut.follow_links = yes;
        self
    }

    /// Yield links leading to loop. By default, this is disabled.
    ///
    /// When `yes` is `true`, symbolic links are followed as if they were
    /// normal directories and files. If a symbolic link is broken or is
    /// involved in a loop, an error is yielded.
    ///
    /// When enabled, the yielded [`DirEntry`] values represent the target of
    /// the link while the path corresponds to the link. See the [`DirEntry`]
    /// type for more details.
    ///
    /// [`DirEntry`]: struct.DirEntry.html
    pub fn yield_loop_links(mut self, yes: bool) -> Self {
        self.opts.immut.yield_loop_links = yes;
        self
    }

    /// Set the minimum depth of entries yielded by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    pub fn min_depth(mut self, depth: usize) -> Self {
        self.opts.immut.min_depth = depth;
        if self.opts.immut.min_depth > self.opts.immut.max_depth {
            self.opts.immut.min_depth = self.opts.immut.max_depth;
        }
        self
    }

    /// Set the maximum depth of entries yield by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    ///
    /// Note that this will not simply filter the entries of the iterator, but
    /// it will actually avoid descending into directories when the depth is
    /// exceeded.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.opts.immut.max_depth = depth;
        if self.opts.immut.max_depth < self.opts.immut.min_depth {
            self.opts.immut.max_depth = self.opts.immut.min_depth;
        }
        self
    }

    /// Set the maximum number of simultaneously open file descriptors used
    /// by the iterator.
    ///
    /// `n` must be greater than or equal to `1`. If `n` is `0`, then it is set
    /// to `1` automatically. If this is not set, then it defaults to some
    /// reasonably low number.
    ///
    /// This setting has no impact on the results yielded by the iterator
    /// (even when `n` is `1`). Instead, this setting represents a trade off
    /// between scarce resources (file descriptors) and memory. Namely, when
    /// the maximum number of file descriptors is reached and a new directory
    /// needs to be opened to continue iteration, then a previous directory
    /// handle is closed and has its unyielded entries stored in memory. In
    /// practice, this is a satisfying trade off because it scales with respect
    /// to the *depth* of your file tree. Therefore, low values (even `1`) are
    /// acceptable.
    ///
    /// Note that this value does not impact the number of system calls made by
    /// an exhausted iterator.
    ///
    /// # Platform behavior
    ///
    /// On Windows, if `follow_links` is enabled, then this limit is not
    /// respected. In particular, the maximum number of file descriptors opened
    /// is proportional to the depth of the directory tree traversed.
    pub fn max_open(mut self, mut n: usize) -> Self {
        if n == 0 {
            n = 1;
        }
        self.opts.immut.max_open = n;
        self
    }

    /// Set a function for sorting directory entries.
    ///
    /// If a compare function is set, the resulting iterator will return all
    /// paths in sorted order. The compare function will be called to compare
    /// entries from the same directory.
    ///
    /// ```rust,no_run
    /// use std::cmp;
    /// use std::ffi::OsString;
    /// use walkdir::WalkDir;
    ///
    /// <WalkDir>::new("foo").sort_by(|a,b| a.raw.file_name().cmp(b.raw.file_name())).into_classic();
    /// ```
    pub fn sort_by<F>(mut self, cmp: F) -> Self
    where
        F: FnMut(&FlatDirEntry<E>, &FlatDirEntry<E>) -> cmp::Ordering
            + Send
            + Sync
            + 'static,
    {
        self.opts.sorter = Some(Box::new(cmp));
        self
    }

    /// Yield a directory's contents before the directory itself. By default,
    /// this is disabled.
    ///
    /// When `yes` is `false` (as is the default), the directory is yielded
    /// before its contents are read. This is useful when, e.g. you want to
    /// skip processing of some directories.
    ///
    /// When `yes` is `true`, the iterator yields the contents of a directory
    /// before yielding the directory itself. This is useful when, e.g. you
    /// want to recursively delete a directory.
    ///
    /// # Example
    ///
    /// Assume the following directory tree:
    ///
    /// ```text
    /// foo/
    ///   abc/
    ///     qrs
    ///     tuv
    ///   def/
    /// ```
    ///
    /// With contents_first disabled (the default), the following code visits
    /// the directory tree in depth-first order:
    ///
    /// ```no_run
    /// use walkdir::WalkDir;
    ///
    /// for entry in <WalkDir>::new("foo").into_classic() {
    ///     let entry = entry.unwrap();
    ///     println!("{}", entry.path().display());
    /// }
    ///
    /// // foo
    /// // foo/abc
    /// // foo/abc/qrs
    /// // foo/abc/tuv
    /// // foo/def
    /// ```
    ///
    /// With contents_first enabled:
    ///
    /// ```no_run
    /// use walkdir::WalkDir;
    ///
    /// for entry in <WalkDir>::new("foo").contents_first(true).into_classic() {
    ///     let entry = entry.unwrap();
    ///     println!("{}", entry.path().display());
    /// }
    ///
    /// // foo/abc/qrs
    /// // foo/abc/tuv
    /// // foo/abc
    /// // foo/def
    /// // foo
    /// ```
    pub fn contents_first(mut self, yes: bool) -> Self {
        self.opts.immut.contents_first = yes;
        self
    }

    /// A variants for filtering content
    pub fn content_filter(mut self, filter: ContentFilter) -> Self {
        self.opts.immut.content_filter = filter;
        self
    }

    /// A variants for filtering content
    pub fn content_order(mut self, order: ContentOrder) -> Self {
        self.opts.immut.content_order = order;
        self
    }

}





/////////////////////////////////////////////////////////////////////////
//// IntoIterator

impl<E: source::SourceExt> IntoIterator for WalkDir<E> {
    type Item = Position<DirEntry<E>, DirEntry<E>, wd::Error<E>>;
    type IntoIter = IntoIter<E>;

    fn into_iter(self) -> IntoIter<E> {
        IntoIter::new( self.opts, self.root, self.ext )
    }
}
