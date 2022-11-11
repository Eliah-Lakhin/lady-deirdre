#![doc = include_str!("readme.md")]

mod id;
mod reference;
mod repository;
mod sequence;

pub(crate) use crate::arena::repository::RepositoryIterator;
pub use crate::arena::{
    id::{Id, Identifiable},
    reference::{Ref, RefIndex, RefVersion},
    repository::Repository,
    sequence::Sequence,
};
