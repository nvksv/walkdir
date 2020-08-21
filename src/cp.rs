use std::iter::FromIterator;
use std::vec::Vec;

use crate::source;
use crate::wd::{Depth, IntoSome};
use crate::dir::FlatDirEntry;
use crate::dent::DirEntry;

pub trait ContentProcessor<E: source::SourceExt>: Default + std::fmt::Debug {
    type Item;
    type Collection: FromIterator<Self::Item>;

    fn process_direntry(&mut self, flat: &FlatDirEntry<E>, depth: Depth, ctx: &mut E::IteratorExt) -> Option<Self::Item>;
    fn is_dir(item: &Self::Item) -> bool;

    fn collect(&mut self, iter: impl Iterator<Item=Self::Item>) -> Self::Collection;
    fn empty_collection() -> Self::Collection;
}

#[derive(Debug, Default)]
pub struct DirEntryContentProcessor {}

impl<E: source::SourceExt> ContentProcessor<E> for DirEntryContentProcessor {
    type Item = DirEntry<E>;
    type Collection = Vec<DirEntry<E>>;

    #[inline(always)]
    fn process_direntry(&mut self, flat: &FlatDirEntry<E>, depth: Depth, ctx: &mut E::IteratorExt) -> Option<Self::Item> {
        Self::Item::from_flat( flat, depth, ctx ).into_some()
    }

    #[inline(always)]
    fn is_dir(item: &Self::Item) -> bool {
        item.is_dir()
    }

    #[inline(always)]
    fn collect(&mut self, iter: impl Iterator<Item=Self::Item>) -> Self::Collection {
        iter.collect()
    }

    #[inline(always)]
    fn empty_collection() -> Self::Collection {
        vec![]
    }
}

