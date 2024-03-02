use std::collections::HashSet;
use std::hash::Hash;

pub trait CollectVec {
    type Item;
    fn collect_vec(self) -> Vec<Self::Item>;
}

impl<I, T> CollectVec for I
where
    I: Iterator<Item = T>,
{
    type Item = T;

    fn collect_vec(self) -> Vec<Self::Item> {
        self.collect()
    }
}

pub trait CollectHashSet {
    type Item: Hash + Eq;

    #[allow(unused)]
    fn collect_hashset(self) -> HashSet<Self::Item>;
}

impl<I, T> CollectHashSet for I
where
    T: Hash + Eq,
    I: Iterator<Item = T>,
{
    type Item = T;

    fn collect_hashset(self) -> HashSet<Self::Item> {
        self.collect()
    }
}
