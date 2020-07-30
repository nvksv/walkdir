use std::cmp::{min, Ordering};
use std::fmt;
use std::io;
use std::result;
use std::vec;


use crate::{Result, FnCmp, ContentFilter, ContentOrder};
use crate::dent::DirEntry;
#[cfg(unix)]
use crate::dent::DirEntryExt;
use crate::error::Error;
use crate::source;
use crate::source::{SourceFsFileType, SourceFsMetadata, SourcePath};
use crate::dir::{DirState, Position};

/// Like try, but for iterators that return [`Option<Result<_, _>>`].
///
/// [`Option<Result<_, _>>`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
macro_rules! ortry {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Some(Err(From::from(err))),
        }
    };
}

/// Like try, but for iterators that return [`Option<Result<_, _>>`].
///
/// [`Option<Result<_, _>>`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
macro_rules! rtry {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Err(From::from(err)),
        }
    };
}




/////////////////////////////////////////////////////////////////////////
//// WalkDirOptions

pub struct WalkDirOptions<E: source::SourceExt> {
    pub same_file_system: bool,
    pub follow_links: bool,
    pub max_open: usize,
    pub min_depth: usize,
    pub max_depth: usize,
    pub sorter: Option<FnCmp<E>>,
    pub contents_first: bool,
    pub content_filter: ContentFilter,
    pub content_order: ContentOrder,
    /// Extension part
    #[allow(dead_code)]
    ext: E::OptionsExt,
}

impl<E: source::SourceExt> Default for WalkDirOptions<E> { 
    fn default() -> Self {
        Self {
            same_file_system: false,
            follow_links: false,
            max_open: 10,
            min_depth: 0,
            max_depth: ::std::usize::MAX,
            sorter: None,
            contents_first: false,
            content_filter: ContentFilter::None,
            content_order: ContentOrder::None,
            ext: E::OptionsExt::default(),
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
            .field("same_file_system", &self.same_file_system)
            .field("follow_links", &self.follow_links)
            .field("max_open", &self.max_open)
            .field("min_depth", &self.min_depth)
            .field("max_depth", &self.max_depth)
            .field("sorter", &sorter_str)
            .field("contents_first", &self.contents_first)
            .field("content_filter", &self.content_filter)
            .field("content_order", &self.content_order)
            .field("ext", &self.ext)
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
/// for entry in <WalkDir>::new("foo").min_depth(1).max_depth(3) {
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
/// for entry in <WalkDir>::new("foo").min_depth(1) {
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

    /// Do not cross file system boundaries.
    ///
    /// When this option is enabled, directory traversal will not descend into
    /// directories that are on a different file system from the root path.
    ///
    /// Currently, this option is only supported on Unix and Windows. If this
    /// option is used on an unsupported platform, then directory traversal
    /// will immediately return an error and will not yield any entries.
    pub fn same_file_system(mut self, yes: bool) -> Self {
        self.opts.same_file_system = yes;
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
        self.opts.follow_links = yes;
        self
    }

    /// Set the minimum depth of entries yielded by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    pub fn min_depth(mut self, depth: usize) -> Self {
        self.opts.min_depth = depth;
        if self.opts.min_depth > self.opts.max_depth {
            self.opts.min_depth = self.opts.max_depth;
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
        self.opts.max_depth = depth;
        if self.opts.max_depth < self.opts.min_depth {
            self.opts.max_depth = self.opts.min_depth;
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
        self.opts.max_open = n;
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
    /// <WalkDir>::new("foo").sort_by(|a,b| a.file_name().cmp(b.file_name()));
    /// ```
    pub fn sort_by<F>(mut self, cmp: F) -> Self
    where
        F: FnMut(&DirEntry<E>, &DirEntry<E>) -> Ordering
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
    /// for entry in <WalkDir>::new("foo") {
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
    /// for entry in <WalkDir>::new("foo").contents_first(true) {
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
        self.opts.contents_first = yes;
        self
    }

    /// A variants for filtering content
    pub fn content_filter(mut self, filter: ContentFilter) -> Self {
        self.opts.content_filter = filter;
        self
    }

    /// A variants for filtering content
    pub fn content_order(mut self, order: ContentOrder) -> Self {
        self.opts.content_order = order;
        self
    }

}

impl<E: source::SourceExt> IntoIterator for WalkDir<E> {
    type Item = Position<DirEntry<E>, Error<E>>;
    type IntoIter = IntoIter<E>;

    fn into_iter(self) -> IntoIter<E> {
        IntoIter::new( self.opts, self.root, self.ext )
    }
}




/////////////////////////////////////////////////////////////////////////
//// Ancestor

/// An ancestor is an item in the directory tree traversed by walkdir, and is
/// used to check for loops in the tree when traversing symlinks.
#[derive(Debug)]
struct Ancestor<E: source::SourceExt> {
    /// The path of this ancestor.
    path: E::PathBuf,
    /// Extension part
    ext: E::AncestorExt,
}

impl<E: source::SourceExt> Ancestor<E> {
    /// Create a new ancestor from the given directory path.
    fn new(dent: &DirEntry<E>) -> io::Result<Self> {
        Ok(Self {
            path: dent.path().to_path_buf(),
            ext: E::ancestor_new(dent)?,
        })
    }

    /// Returns true if and only if the given open file handle corresponds to
    /// the same directory as this ancestor.
    fn is_same(&self, child: &E::SameFileHandle) -> io::Result<bool> {
        E::is_same(&self.path, &self.ext, child)
    }
}








/////////////////////////////////////////////////////////////////////////
//// IntoIter

#[derive(Debug, PartialEq, Eq)]
enum TransitionState {
    None,
    BeforePushDown,
    BeforePopUp,
    AfterPopUp,
}

/// An iterator for recursively descending into a directory.
///
/// A value with this type must be constructed with the [`WalkDir`] type, which
/// uses a builder pattern to set options such as min/max depth, max open file
/// descriptors and whether the iterator should follow symbolic links. After
/// constructing a `WalkDir`, call [`.into_iter()`] at the end of the chain.
///
/// The order of elements yielded by this iterator is unspecified.
///
/// [`WalkDir`]: struct.WalkDir.html
/// [`.into_iter()`]: struct.WalkDir.html#into_iter.v
#[derive(Debug)]
pub struct IntoIter<E: source::SourceExt = source::DefaultSourceExt> {
    /// Options specified in the builder. Depths, max fds, etc.
    opts: WalkDirOptions<E>,
    /// The start path.
    ///
    /// This is only `Some(...)` at the beginning. After the first iteration,
    /// this is always `None`.
    start: Option<E::PathBuf>,
    /// A stack of open (up to max fd) or closed handles to directories.
    /// An open handle is a plain [`fs::ReadDir`] while a closed handle is
    /// a `Vec<fs::DirEntry>` corresponding to the as-of-yet consumed entries.
    ///
    /// [`fs::ReadDir`]: https://doc.rust-lang.org/stable/std/fs/struct.ReadDir.html
    states: Vec<DirState<E>>,
    /// before push down / after pop up
    transition_state: TransitionState,
    /// A stack of file paths.
    ///
    /// This is *only* used when [`follow_links`] is enabled. In all other
    /// cases this stack is empty.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    states_path: Vec<Ancestor<E>>,
    /// An index into `states` that points to the oldest open directory
    /// handle. If the maximum fd limit is reached and a new directory needs to
    /// be read, the handle at this index is closed before the new directory is
    /// opened.
    oldest_opened: usize,
    /// The current depth of iteration (the length of the stack at the
    /// beginning of each iteration).
    depth: usize,
    /// The device of the root file path when the first call to `next` was
    /// made.
    ///
    /// If the `same_file_system` option isn't enabled, then this is always
    /// `None`. Conversely, if it is enabled, this is always `Some(...)` after
    /// handling the root path.
    root_device: Option<u64>,
    /// Extension part.
    ext: E::IntoIterExt,
}

impl<E: source::SourceExt> IntoIter<E> {
    pub fn new( opts: WalkDirOptions<E>, root: E::PathBuf, ext: E ) -> Self {
        IntoIter {
            opts: opts,
            start: Some(root),
            states: vec![],
            transition_state: TransitionState::None,
            states_path: vec![],
            oldest_opened: 0,
            depth: 0,
            root_device: None,
            ext: E::intoiter_new(ext),
        }
    }

    // Follow symlinks and check same_file_system. Also determine is_dir flag.
    // - Some(Ok((dent, is_dir))) -- normal entry to yielding
    // - Some(Err(_)) -- some error occured
    // - None -- entry must be ignored
    fn process_dent(&self, dent: DirEntry<E>) -> Option<Result<(DirEntry<E>, bool), E>> {
        
        if self.opts.follow_links && dent.file_type().is_symlink() {
            dent = ortry!(self.follow(dent));
        }

        let is_normal_dir = !dent.file_type().is_symlink() && dent.is_dir();

        if is_normal_dir {
            if self.opts.same_file_system && dent.depth() > 0 {
                if ! ortry!(self.is_same_file_system(&dent)) {
                    return None;
                };
            };
        } else if self.depth == 0 && dent.file_type().is_symlink() {
            // As a special case, if we are processing a root entry, then we
            // always follow it even if it's a symlink and follow_links is
            // false. We are careful to not let this change the semantics of
            // the DirEntry however. Namely, the DirEntry should still respect
            // the follow_links setting. When it's disabled, it should report
            // itself as a symlink. When it's enabled, it should always report
            // itself as the target.
            let md = ortry!(E::metadata(dent.path()).map_err(|err| {
                Error::from_path(dent.path().to_path_buf(), err).set_depth(self.depth)
            }));
            if ! md.file_type().is_dir() {
                return None;
            };
        };

        Some(Ok((dent, is_normal_dir)))
    }

    fn init(&mut self, start: E::PathBuf) -> Result<(), E> {
        if self.opts.same_file_system {
            let result = E::device_num(&start)
                .map_err(|e| Error::<E>::from_path(start.clone(), e).set_depth(0));
            self.root_device = Some(rtry!(result));
        }
        let dent = rtry!(DirEntry::<E>::from_path(start, false)).set_depth(0);

        self.push_dir(dent, 0, true)?;

        Ok(())
    }

    // fn handle_entry(
    //     &mut self,
    //     mut dent: DirEntry<E>,
    // ) -> Option<Result<DirEntry<E>, E>> {
    //     if self.opts.follow_links && dent.file_type().is_symlink() {
    //         dent = itry!(self.follow(dent));
    //     }
    //     let is_normal_dir = !dent.file_type().is_symlink() && dent.is_dir();
    //     if is_normal_dir {
    //         if self.opts.same_file_system && dent.depth() > 0 {
    //             if itry!(self.is_same_file_system(&dent)) {
    //                 itry!(self.push(&dent));
    //             }
    //         } else {
    //             itry!(self.push(&dent));
    //         }
    //     } else if dent.depth() == 0 && dent.file_type().is_symlink() {
    //         // As a special case, if we are processing a root entry, then we
    //         // always follow it even if it's a symlink and follow_links is
    //         // false. We are careful to not let this change the semantics of
    //         // the DirEntry however. Namely, the DirEntry should still respect
    //         // the follow_links setting. When it's disabled, it should report
    //         // itself as a symlink. When it's enabled, it should always report
    //         // itself as the target.
    //         let md = itry!(E::metadata(dent.path()).map_err(|err| {
    //             Error::from_path(dent.depth(), dent.path().to_path_buf(), err)
    //         }));
    //         if md.file_type().is_dir() {
    //             itry!(self.push(&dent));
    //         }
    //     }
    //     if is_normal_dir && self.opts.contents_first {
    //         self.deferred_dirs.push(dent);
    //         None
    //     } else if self.skippable() {
    //         None
    //     } else {
    //         Some(Ok(dent))
    //     }
    // }

    fn push_dir(&mut self, dent: DirEntry<E>, depth: usize, is_root: bool) -> Result<(), E> {
        // Make room for another open file descriptor if we've hit the max.
        let free = self.states.len().checked_sub(self.oldest_opened).unwrap();
        if free == self.opts.max_open {
            self.states[self.oldest_opened].load_all(&self.opts, &mut |dent| self.process_dent(dent));
        }

        let mut state = if is_root { 
            DirState::<E>::new_once( dent.clone(), depth, &mut self.opts, &mut |dent| self.process_dent(dent) ) 
        } else {
            // Open a handle to reading the directory's entries.
            let rd = E::read_dir(&dent, dent.path()).map_err(|err| Error::<E>::from_path(dent.path().to_path_buf(), err).set_depth(depth));
            DirState::<E>::new( rd, depth, &mut self.opts, &mut |dent| self.process_dent(dent) )
        };

        if self.opts.follow_links {
            let ancestor = Ancestor::new(&dent)
                .map_err(|err| Error::from_io(err).set_depth(depth))?;
            self.states_path.push(ancestor);
        }

        // We push this after states_path since creating the Ancestor can fail.
        // If it fails, then we return the error and won't descend.
        self.states.push(state);
        
        // If we had to close out a previous directory stream, then we need to
        // increment our index the oldest still-open stream. We do this only
        // after adding to our stack, in order to ensure that the oldest_opened
        // index remains valid. The worst that can happen is that an already
        // closed stream will be closed again, which is a no-op.
        //
        // We could move the close of the stream above into this if-body, but
        // then we would have more than the maximum number of file descriptors
        // open at a particular point in time.
        if free == self.opts.max_open {
            // Unwrap is safe here because self.oldest_opened is guaranteed to
            // never be greater than `self.stack_list.len()`, which implies
            // that the subtraction won't underflow and that adding 1 will
            // never overflow.
            self.oldest_opened = self.oldest_opened.checked_add(1).unwrap();
        }
        Ok(())
    }

    fn pop_dir(&mut self) {
        self.states.pop().expect("BUG: cannot pop from empty stack");
        if self.opts.follow_links {
            self.states_path.pop().expect("BUG: list/path stacks out of sync");
        }
        // If everything in the stack is already closed, then there is
        // room for at least one more open descriptor and it will
        // always be at the top of the stack.
        self.oldest_opened = min(self.oldest_opened, self.states.len());

        self.transition_state = TransitionState::AfterPopUp;
    }

    /// Skips the current directory.
    ///
    /// This causes the iterator to stop traversing the contents of the least
    /// recently yielded directory. This means any remaining entries in that
    /// directory will be skipped (including sub-directories).
    ///
    /// Note that the ergonomics of this method are questionable since it
    /// borrows the iterator mutably. Namely, you must write out the looping
    /// condition manually. For example, to skip hidden entries efficiently on
    /// unix systems:
    ///
    /// ```no_run
    /// use walkdir::{DirEntry, WalkDir};
    ///
    /// fn is_hidden(entry: &DirEntry) -> bool {
    ///     entry.file_name()
    ///          .to_str()
    ///          .map(|s| s.starts_with("."))
    ///          .unwrap_or(false)
    /// }
    ///
    /// let mut it = <WalkDir>::new("foo").into_iter();
    /// loop {
    ///     let entry = match it.next() {
    ///         None => break,
    ///         Some(Err(err)) => panic!("ERROR: {}", err),
    ///         Some(Ok(entry)) => entry,
    ///     };
    ///     if is_hidden(&entry) {
    ///         if entry.file_type().is_dir() {
    ///             it.skip_current_dir();
    ///         }
    ///         continue;
    ///     }
    ///     println!("{}", entry.path().display());
    /// }
    /// ```
    ///
    /// You may find it more convenient to use the [`filter_entry`] iterator
    /// adapter. (See its documentation for the same example functionality as
    /// above.)
    ///
    /// [`filter_entry`]: #method.filter_entry
    pub fn skip_current_dir(&mut self) {
        if !self.states.is_empty() {
            self.pop_dir();
        }
    }

    pub fn get_current_dir_content(&mut self, filter: ContentFilter) -> Option<Vec<DirEntry<E>>> {
        let cur_state = match self.states.last_mut() {
            Some(state) => state,
            None => return None,
        };

        let content = cur_state.clone_all_content(filter, &self.opts, &mut |dent| self.process_dent(dent) );
        
        Some(content)
    }
}

impl<E: source::SourceExt> Iterator for IntoIter<E> {
    type Item = Position<DirEntry<E>, Error<E>>;
    /// Advances the iterator and returns the next value.
    ///
    /// # Errors
    ///
    /// If the iterator fails to retrieve the next value, this method returns
    /// an error value. The error will be wrapped in an Option::Some.
    fn next(&mut self) -> Option<Self::Item> {
        // Initial actions
        if let Some(start) = self.start.take() {
            if let Err(e) = self.init(start) {
                return Some(Position::Error(e));
                // Here self.states is empty, so next call will always return None.
            };
        }

        loop {
            let cur_state = match self.states.last_mut() {
                Some(state) => state,
                None => return None,
            };

            match cur_state.get_current_position() {
                Position::BeforeContent => {
                    assert!( self.transition_state == TransitionState::None );

                    cur_state.next_position( &self.opts, &mut |dent| self.process_dent(dent) );
                    return Some(Position::BeforeContent);
                }, 
                Position::Entry((dent, is_dir)) => {
                    if is_dir {
                        match self.transition_state {
                            TransitionState::AfterPopUp => {
                                self.transition_state = TransitionState::None;
                                cur_state.next_position( &self.opts, &mut |dent| self.process_dent(dent) );
                                if self.opts.contents_first {
                                    return Some(Position::Entry(dent));
                                };
                            },
                            TransitionState::BeforePushDown => {
                                self.transition_state = TransitionState::None;
                                self.push_dir(dent.clone(), cur_state.depth()+1, false);
                            },
                            TransitionState::None => {},
                            _ => unreachable!(),
                        };

                        self.transition_state == TransitionState::BeforePushDown;

                        if !self.opts.contents_first {
                            return Some(Position::Entry(dent));
                        };
                    } else {
                        assert!( self.transition_state == TransitionState::None );

                        cur_state.next_position( &self.opts, &mut |dent| self.process_dent(dent) );
                        return Some(Position::Entry(dent));
                    }
                },
                Position::Error(e) => {
                    assert!( self.transition_state == TransitionState::None );

                    cur_state.next_position( &self.opts, &mut |dent| self.process_dent(dent) );
                    return Some(Position::Error(e));
                },
                Position::AfterContent => {
                    match self.transition_state {
                        TransitionState::None => {
                            self.transition_state = TransitionState::BeforePopUp;
                            return Some(Position::AfterContent);
                        },
                        TransitionState::BeforePopUp => {
                            self.pop_dir();
                            self.transition_state = TransitionState::AfterPopUp;
                        },
                        _ => unreachable!()
                    }
                }
            };
        }

        //     if last_position == Position::BeforeContent && self.opts.contents_first
        // }
        // while !self.stack.is_empty() {
        //     self.depth = self.stack.len();
        //     if let Some(dentry) = self.get_deferred_dir() {
        //         return Some(Ok(dentry));
        //     }
        //     if self.depth > self.opts.max_depth {
        //         // If we've exceeded the max depth, pop the current dir
        //         // so that we don't descend.
        //         self.pop();
        //         continue;
        //     }
        //     // Unwrap is safe here because we've verified above that
        //     // `self.stack_list` is not empty
        //     let next = self
        //         .stack
        //         .last_mut()
        //         .expect("BUG: stack should be non-empty")
        //         .next();
        //     match next {
        //         None => self.pop(),
        //         Some(Err(err)) => return Some(Err(err)),
        //         Some(Ok(dent)) => {
        //             if let Some(result) = self.handle_entry(dent) {
        //                 return Some(result);
        //             }
        //         }
        //     }
        // }
        // if self.opts.contents_first {
        //     self.depth = self.stack_list.len();
        //     if let Some(dentry) = self.get_deferred_dir() {
        //         return Some(Ok(dentry));
        //     }
        // }
        // None
    }
}

impl<E: source::SourceExt> IntoIter<E> {


    /// Yields only entries which satisfy the given predicate and skips
    /// descending into directories that do not satisfy the given predicate.
    ///
    /// The predicate is applied to all entries. If the predicate is
    /// true, iteration carries on as normal. If the predicate is false, the
    /// entry is ignored and if it is a directory, it is not descended into.
    ///
    /// This is often more convenient to use than [`skip_current_dir`]. For
    /// example, to skip hidden files and directories efficiently on unix
    /// systems:
    ///
    /// ```no_run
    /// use walkdir::{DirEntry, WalkDir};
    /// # use walkdir::Error;
    ///
    /// fn is_hidden(entry: &DirEntry) -> bool {
    ///     entry.file_name()
    ///          .to_str()
    ///          .map(|s| s.starts_with("."))
    ///          .unwrap_or(false)
    /// }
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// for entry in <WalkDir>::new("foo")
    ///                      .into_iter()
    ///                      .filter_entry(|e| !is_hidden(e)) {
    ///     println!("{}", entry?.path().display());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note that the iterator will still yield errors for reading entries that
    /// may not satisfy the predicate.
    ///
    /// Note that entries skipped with [`min_depth`] and [`max_depth`] are not
    /// passed to this predicate.
    ///
    /// Note that if the iterator has `contents_first` enabled, then this
    /// method is no different than calling the standard `Iterator::filter`
    /// method (because directory entries are yielded after they've been
    /// descended into).
    ///
    /// [`skip_current_dir`]: #method.skip_current_dir
    /// [`min_depth`]: struct.WalkDir.html#method.min_depth
    /// [`max_depth`]: struct.WalkDir.html#method.max_depth
    pub fn filter_entry<P>(self, predicate: P) -> FilterEntry<Self, P>
    where
        P: FnMut(&DirEntry<E>) -> bool,
    {
        FilterEntry { inner: self, predicate: predicate }
    }


    // fn get_deferred_dir(&mut self) -> Option<DirEntry<E>> {
    //     if self.opts.contents_first {
    //         if self.depth < self.deferred_dirs.len() {
    //             // Unwrap is safe here because we've guaranteed that
    //             // `self.deferred_dirs.len()` can never be less than 1
    //             let deferred: DirEntry<E> = self
    //                 .deferred_dirs
    //                 .pop()
    //                 .expect("BUG: deferred_dirs should be non-empty");
    //             if !self.skippable() {
    //                 return Some(deferred);
    //             }
    //         }
    //     }
    //     None
    // }


    fn follow(&self, mut dent: DirEntry<E>) -> Result<DirEntry<E>, E> {
        dent = DirEntry::<E>::from_path(
            dent.path().to_path_buf(),
            true,
        )?;
        // The only way a symlink can cause a loop is if it points
        // to a directory. Otherwise, it always points to a leaf
        // and we can omit any loop checks.
        if dent.is_dir() {
            self.check_loop(dent.path())?;
        }
        Ok(dent)
    }

    fn check_loop<P: AsRef<E::Path>>(&self, child: P) -> Result<(), E> {
        let hchild = E::get_handle(&child)
            .map_err(|err| Error::from_io(err).set_depth(self.depth))?;

        for ancestor in self.states_path.iter().rev() {
            let is_same = ancestor
                .is_same(&hchild)
                .map_err(|err| Error::from_io(err).set_depth(self.depth))?;
            if is_same {
                return Err(Error::<E>::from_loop(
                    &ancestor.path,
                    child.as_ref()
                ).set_depth(self.depth));
            }
        }
        Ok(())
    }

    fn is_same_file_system(&mut self, dent: &DirEntry<E>) -> Result<bool, E> {
        let dent_device = E::device_num(dent.path())
            .map_err(|err| Error::from_entry(dent, err))?;
        Ok(self
            .root_device
            .map(|d| d == dent_device)
            .expect("BUG: called is_same_file_system without root device"))
    }

    fn skippable(&self) -> bool {
        self.depth < self.opts.min_depth || self.depth > self.opts.max_depth
    }
}




/////////////////////////////////////////////////////////////////////////
//// FilterEntry

/// A recursive directory iterator that skips entries.
///
/// Values of this type are created by calling [`.filter_entry()`] on an
/// `IntoIter`, which is formed by calling [`.into_iter()`] on a `WalkDir`.
///
/// Directories that fail the predicate `P` are skipped. Namely, they are
/// never yielded and never descended into.
///
/// Entries that are skipped with the [`min_depth`] and [`max_depth`] options
/// are not passed through this filter.
///
/// If opening a handle to a directory resulted in an error, then it is yielded
/// and no corresponding call to the predicate is made.
///
/// Type parameter `I` refers to the underlying iterator and `P` refers to the
/// predicate, which is usually `FnMut(&DirEntry) -> bool`.
///
/// [`.filter_entry()`]: struct.IntoIter.html#method.filter_entry
/// [`.into_iter()`]: struct.WalkDir.html#into_iter.v
/// [`min_depth`]: struct.WalkDir.html#method.min_depth
/// [`max_depth`]: struct.WalkDir.html#method.max_depth
#[derive(Debug)]
pub struct FilterEntry<I, P> {
    inner: I,
    predicate: P,
}

impl<P, E> Iterator for FilterEntry<IntoIter<E>, P>
where
    P: FnMut(&Position<DirEntry<E>, Error<E>>) -> bool,
    E: source::SourceExt,
{
    type Item = Position<DirEntry<E>, Error<E>>;

    /// Advances the iterator and returns the next value.
    ///
    /// # Errors
    ///
    /// If the iterator fails to retrieve the next value, this method returns
    /// an error value. The error will be wrapped in an `Option::Some`.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = match self.inner.next() {
                Some(item) => item,
                None => return None,
            };

            if !(self.predicate)(&item) {
                if let Position::Entry(dent) = item {
                    if dent.is_dir() {
                        self.inner.skip_current_dir();
                    }
                }
                continue;
            }

            return Some(item);
        }
    }
}

impl<P, E> FilterEntry<IntoIter<E>, P>
where
    P: FnMut(&Position<DirEntry<E>, Error<E>>) -> bool,
    E: source::SourceExt,
{
    /// Yields only entries which satisfy the given predicate and skips
    /// descending into directories that do not satisfy the given predicate.
    ///
    /// The predicate is applied to all entries. If the predicate is
    /// true, iteration carries on as normal. If the predicate is false, the
    /// entry is ignored and if it is a directory, it is not descended into.
    ///
    /// This is often more convenient to use than [`skip_current_dir`]. For
    /// example, to skip hidden files and directories efficiently on unix
    /// systems:
    ///
    /// ```no_run
    /// use walkdir::{DirEntry, WalkDir};
    /// # use walkdir::Error;
    ///
    /// fn is_hidden(entry: &DirEntry) -> bool {
    ///     entry.file_name()
    ///          .to_str()
    ///          .map(|s| s.starts_with("."))
    ///          .unwrap_or(false)
    /// }
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// for entry in <WalkDir>::new("foo")
    ///                      .into_iter()
    ///                      .filter_entry(|e| !is_hidden(e)) {
    ///     println!("{}", entry?.path().display());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note that the iterator will still yield errors for reading entries that
    /// may not satisfy the predicate.
    ///
    /// Note that entries skipped with [`min_depth`] and [`max_depth`] are not
    /// passed to this predicate.
    ///
    /// Note that if the iterator has `contents_first` enabled, then this
    /// method is no different than calling the standard `Iterator::filter`
    /// method (because directory entries are yielded after they've been
    /// descended into).
    ///
    /// [`skip_current_dir`]: #method.skip_current_dir
    /// [`min_depth`]: struct.WalkDir.html#method.min_depth
    /// [`max_depth`]: struct.WalkDir.html#method.max_depth
    pub fn filter_entry(self, predicate: P) -> FilterEntry<Self, P> {
        FilterEntry { inner: self, predicate: predicate }
    }

    /// Skips the current directory.
    ///
    /// This causes the iterator to stop traversing the contents of the least
    /// recently yielded directory. This means any remaining entries in that
    /// directory will be skipped (including sub-directories).
    ///
    /// Note that the ergonomics of this method are questionable since it
    /// borrows the iterator mutably. Namely, you must write out the looping
    /// condition manually. For example, to skip hidden entries efficiently on
    /// unix systems:
    ///
    /// ```no_run
    /// use walkdir::{DirEntry, WalkDir};
    ///
    /// fn is_hidden(entry: &DirEntry) -> bool {
    ///     entry.file_name()
    ///          .to_str()
    ///          .map(|s| s.starts_with("."))
    ///          .unwrap_or(false)
    /// }
    ///
    /// let mut it = <WalkDir>::new("foo").into_iter();
    /// loop {
    ///     let entry = match it.next() {
    ///         None => break,
    ///         Some(Err(err)) => panic!("ERROR: {}", err),
    ///         Some(Ok(entry)) => entry,
    ///     };
    ///     if is_hidden(&entry) {
    ///         if entry.file_type().is_dir() {
    ///             it.skip_current_dir();
    ///         }
    ///         continue;
    ///     }
    ///     println!("{}", entry.path().display());
    /// }
    /// ```
    ///
    /// You may find it more convenient to use the [`filter_entry`] iterator
    /// adapter. (See its documentation for the same example functionality as
    /// above.)
    ///
    /// [`filter_entry`]: #method.filter_entry
    pub fn skip_current_dir(&mut self) {
        self.inner.skip_current_dir();
    }
}
