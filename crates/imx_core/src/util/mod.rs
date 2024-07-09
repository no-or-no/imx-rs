use ahash::RandomState;
use dashmap::DashMap;
pub(crate) use factorize::*;

mod factorize;
mod sha;

pub type HashMap<K, V> = DashMap<K, V, RandomState>;