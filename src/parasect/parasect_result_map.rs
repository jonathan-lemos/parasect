use crate::collections::pred_succ::PredSucc;
use crate::parasect::parasect_result_map::Criticality::*;
use crate::parasect::types::ParasectError::{InconsistencyError, PayloadError};
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use crate::parasect::types::{ParasectError, ParasectPayloadResult};
use dashmap::DashMap;
use ibig::IBig;

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Criticality {
    NoCriticalSeen,
    CriticalSeen,
}

pub struct ParasectResultMap<'a> {
    indices: &'a [IBig],
    predsucc: PredSucc<'a, IBig>,
    results: DashMap<IBig, ParasectPayloadResult>,
}

impl<'a> ParasectResultMap<'a> {
    pub fn new(indices: &'a [IBig]) -> Self {
        Self {
            indices,
            predsucc: PredSucc::new(indices),
            results: DashMap::new(),
        }
    }

    /// Saves the given result for the given index.
    ///
    /// Returns CriticalSeen if a critical point was seen as a result of this insertion, meaning a bad point where the previous point is good or there is no previous point, otherwise returns NoCriticalSeen.
    ///
    /// After this function returns CriticalSeen once, future results are not guaranteed.
    ///
    /// Note that the inserted element does not have to be bad to return CriticalSeen, as long as the next element is bad.
    pub fn add(&self, index: IBig, result: ParasectPayloadResult) -> Criticality {
        {
            let before_idx = self.predsucc.predecessor(&index);

            if let None = before_idx {
                if let Continue(Bad) = result {
                    self.results.insert(index, result);
                    return CriticalSeen;
                }
            }
        }

        let criticality = {
            let before = self
                .predsucc
                .predecessor(&index)
                .and_then(|x| self.results.get(x));

            let after = self
                .predsucc
                .successor(&index)
                .and_then(|x| self.results.get(x));

            match (before.as_deref(), &result, after.as_deref()) {
                (Some(Stop(_)), _, _) => NoCriticalSeen,
                (_, Stop(_), _) => NoCriticalSeen,
                (_, _, Some(Stop(_))) => NoCriticalSeen,
                (Some(Continue(Good)), Continue(Bad), _) => CriticalSeen,
                (_, Continue(Good), Some(Continue(Bad))) => CriticalSeen,
                _ => NoCriticalSeen,
            }
        };

        self.results.insert(index, result);

        criticality
    }

    /// Returns the critical point if CriticalSeen was returned by the above function at least once *and* there were no Stop entries.
    /// Otherwise, returns an appropriate error.
    pub fn critical_point(self) -> Result<IBig, ParasectError> {
        let mut crit_pt: Option<IBig> = None;

        for index in self.indices {
            let (index_owned, result) = match self.results.remove(index) {
                Some(v) => v,
                None => continue,
            };

            match (index_owned, result) {
                (_, Stop(e)) => return Err(PayloadError(e)),
                (n, Continue(Good)) => {
                    if let Some(existing_crit_pt) = crit_pt {
                        return Err(InconsistencyError(format!(
                            "Found a critical point at {} followed by a good point at {}.",
                            existing_crit_pt, n
                        )));
                    }
                }
                (n, Continue(Bad)) => {
                    crit_pt = crit_pt.or(Some(n));
                }
            };
        }

        match crit_pt {
            Some(n) => Ok(n),
            None => Err(InconsistencyError("All points were good.".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectVec;
    use crate::test_util::test_util::test_util::ib;
    use proptest::prelude::*;

    #[test]
    fn test_add_critpt_1() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Bad)), CriticalSeen);
    }

    #[test]
    fn test_add_critpt_2() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(2), Continue(Bad)), NoCriticalSeen);
        assert_eq!(map.add(ib(1), Continue(Good)), CriticalSeen);
    }

    #[test]
    fn test_add_critpt_3() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(3), Continue(Bad)), CriticalSeen);
    }

    #[test]
    fn test_add_critpt_4() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(3), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Bad)), CriticalSeen);
    }

    #[test]
    fn test_add_critpt_5() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Bad)), CriticalSeen);
    }

    #[test]
    fn test_add_critpt_6() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(3), Continue(Good)), NoCriticalSeen);
    }

    #[test]
    fn test_critpt_stop_not_critical_1() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(3), Continue(Bad)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Stop("".into())), NoCriticalSeen);
    }

    #[test]
    fn test_critpt_stop_not_critical_2() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(1), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(3), Stop("".into())), NoCriticalSeen);
    }

    #[test]
    fn test_critpt_stop_not_critical_3() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        assert_eq!(map.add(ib(3), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(2), Continue(Good)), NoCriticalSeen);
        assert_eq!(map.add(ib(1), Stop("".into())), NoCriticalSeen);
    }

    #[test]
    fn test_find_critpt_1() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Bad));

        assert_eq!(map.critical_point(), Ok(ib(1)));
    }

    #[test]
    fn test_find_critpt_2() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Good));
        map.add(ib(2), Continue(Bad));

        assert_eq!(map.critical_point(), Ok(ib(2)));
    }

    #[test]
    fn test_find_critpt_3() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Good));
        map.add(ib(2), Continue(Good));
        map.add(ib(3), Continue(Bad));

        assert_eq!(map.critical_point(), Ok(ib(3)));
    }

    #[test]
    fn test_find_critpt_4() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Good));
        map.add(ib(2), Continue(Good));
        map.add(ib(3), Continue(Good));

        assert_eq!(
            map.critical_point(),
            Err(InconsistencyError("All points were good.".into()))
        );
    }

    #[test]
    fn test_find_critpt_5() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Bad));
        map.add(ib(2), Continue(Bad));
        map.add(ib(3), Continue(Good));

        assert_eq!(
            map.critical_point(),
            Err(InconsistencyError(
                "Found a critical point at 1 followed by a good point at 3.".into()
            ))
        );
    }

    #[test]
    fn test_find_critpt_6() {
        let indices = &[ib(1), ib(2), ib(3)];
        let map = ParasectResultMap::new(indices);

        map.add(ib(1), Continue(Good));
        map.add(ib(2), Continue(Bad));
        map.add(ib(3), Stop("it's gone bad".into()));

        assert_eq!(
            map.critical_point(),
            Err(PayloadError("it's gone bad".into()))
        );
    }

    proptest! {
        #[test]
        fn fuzz_find_critpt(a in 1..100, b in 1..100) {
            prop_assume!(a <= b);

            let indices = (1..(b + 1)).map(ib).into_iter().collect_vec();

            let map = ParasectResultMap::new(&indices);

            for i in &indices {
                map.add(i.clone(), if i < &ib(a) { Continue(Good) } else { Continue(Bad) } );
            }

            assert_eq!(map.critical_point(), Ok(ib(a)));
        }
    }
}
