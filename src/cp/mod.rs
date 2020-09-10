use std::iter::FromIterator;

use crate::fs;
use crate::wd::{Depth, IntoSome};

/// Convertor from RawDirEntry into final entry type (e.g. DirEntry)
pub trait ContentProcessor<E: fs::FsDirEntry>: Default + std::fmt::Debug {
    /// Final entry type
    type Item;
    /// Collection of items
    type Collection: FromIterator<Self::Item>;

    /// Convert RawDirEntry into final entry type (e.g. DirEntry)
    fn process_direntry_from_path(
        &self,
        path: &E::Path,
        is_dir: bool,
        follow_link: bool,
        depth: Depth,
    ) -> Option<Self::Item>;

    /// Convert RawDirEntry into final entry type (e.g. DirEntry)
    fn process_direntry(
        &self,
        fsdent: &E,
        is_dir: bool,
        follow_link: bool,
        depth: Depth,
    ) -> Option<Self::Item>;

    /// Check if final entry is dir
    fn is_dir(item: &Self::Item) -> bool;

    /// Collects iterator over items into collection
    fn collect(&self, iter: impl Iterator<Item = Self::Item>) -> Self::Collection;
    /// Empty items collection
    fn empty_collection() -> Self::Collection;
}

