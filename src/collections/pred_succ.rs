use std::collections::HashMap;
use std::hash::Hash;

pub struct PredSucc<'a, V>
where V: Hash {
    predecessors: HashMap<&'a V, &'a V>,
    successors: HashMap<&'a V, &'a V>
}

impl<'a, V> PredSucc<'a, V>
where V: Hash + Eq {
    pub fn new(slice: &'a [V]) -> Self {
        let mut predecessors = HashMap::new();
        let mut successors = HashMap::new();

        for (a, b) in slice.into_iter().zip(slice.into_iter().skip(1)) {
            predecessors.insert(b, a);
            successors.insert(a, b);
        }

        Self {
            predecessors,
            successors
        }
    }

    pub fn predecessor(&self, v: &'a V) -> Option<&'a V> {
        self.predecessors.get(v).map(|x| *x)
    }

    pub fn successor(&self, v: &'a V) -> Option<&'a V> {
        self.successors.get(v).map(|x| *x)
    }
}

