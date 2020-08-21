use std::iter::FromIterator;
use std::vec::Vec;

use crate::source;
use crate::wd::{Depth, IntoSome};
use crate::dir::FlatDirEntry;
use crate::dent::DirEntry;

/// Convertor from RawDirEntry into final entry type (e.g. DirEntry)
pub trait ContentProcessor<E: source::SourceExt>: Default + std::fmt::Debug {
    /// Final entry type
    type Item;
    /// Collection of items
    type Collection: FromIterator<Self::Item>;

    /// Convert RawDirEntry into final entry type (e.g. DirEntry)
    fn process_direntry(&self, flat: &FlatDirEntry<E>, depth: Depth, ctx: &mut E::IteratorExt) -> Option<Self::Item>;
    /// Check if final entry is dir
    fn is_dir(item: &Self::Item) -> bool;

    /// Collects iterator over items into collection
    fn collect(&self, iter: impl Iterator<Item=Self::Item>) -> Self::Collection;
    /// Empty items collection
    fn empty_collection() -> Self::Collection;
}

/// Convertor from RawDirEntry into DirEntry
#[derive(Debug, Default)]
pub struct DirEntryContentProcessor {}

impl<E: source::SourceExt> ContentProcessor<E> for DirEntryContentProcessor {
    type Item = DirEntry<E>;
    type Collection = Vec<DirEntry<E>>;

    #[inline(always)]
    fn process_direntry(&self, flat: &FlatDirEntry<E>, depth: Depth, ctx: &mut E::IteratorExt) -> Option<Self::Item> {
        Self::Item::from_flat( flat, depth, ctx ).into_some()
    }

    #[inline(always)]
    fn is_dir(item: &Self::Item) -> bool {
        item.is_dir()
    }

    #[inline(always)]
    fn collect(&self, iter: impl Iterator<Item=Self::Item>) -> Self::Collection {
        iter.collect()
    }

    #[inline(always)]
    fn empty_collection() -> Self::Collection {
        vec![]
    }
}

