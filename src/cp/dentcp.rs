use std::vec::Vec;

use crate::dent::DirEntry;
use crate::dir::FlatDirEntry;

/// Convertor from RawDirEntry into DirEntry
#[derive(Debug, Default)]
pub struct DirEntryContentProcessor {}

impl<E: storage::StorageExt> ContentProcessor<E> for DirEntryContentProcessor {
    type Item = DirEntry<E>;
    type Collection = Vec<DirEntry<E>>;

    #[inline(always)]
    fn process_direntry_from_path(
        &self,
        path: &E::Path,
        is_dir: bool,
        follow_link: bool,
        depth: Depth,
        raw_ext: &mut E::RawDirEntryExt,
        ctx: &mut E::IteratorExt,
    ) -> Option<Self::Item> {
        Self::Item::from_flat(flat, depth, ctx).into_some()
    }

    #[inline(always)]
    fn process_direntry(
        &self,
        flat: &FlatDirEntry<E>,
        depth: Depth,
        ctx: &mut E::IteratorExt,
    ) -> Option<Self::Item> {
        Self::Item::from_flat(flat, depth, ctx).into_some()
    }
    #[inline(always)]
    fn is_dir(item: &Self::Item) -> bool {
        item.is_dir()
    }

    #[inline(always)]
    fn collect(&self, iter: impl Iterator<Item = Self::Item>) -> Self::Collection {
        iter.collect()
    }

    #[inline(always)]
    fn empty_collection() -> Self::Collection {
        vec![]
    }
}
