pub trait CollectVec {
    type Item;
    fn collect_vec(self) -> Vec<Self::Item>;
}

impl<I, T> CollectVec for I
where I: Iterator<Item = T> {
    type Item = T;

    fn collect_vec(self) -> Vec<Self::Item> {
        self.collect()
    }
}