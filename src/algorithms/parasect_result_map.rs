use dashmap::DashMap;
use ibig::IBig;
use crate::algorithms::parasect::{ParasectError, ParasectPayloadResult};
use crate::algorithms::parasect::ParasectError::{InconsistencyError, PayloadError};
use crate::algorithms::parasect::ParasectPayloadResult::*;
use crate::algorithms::parasect::ParasectPayloadAnswer::*;
use crate::algorithms::parasect_result_map::Criticality::*;
use crate::collections::pred_succ::PredSucc;

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Criticality {
    NoCriticalSeen,
    CriticalSeen,
}

pub struct ParasectResultMap<'a> {
    indices: &'a [IBig],
    predsucc: PredSucc<'a, IBig>,
    results: DashMap<IBig, ParasectPayloadResult>
}

impl<'a> ParasectResultMap<'a> {
    pub fn new(indices: &'a [IBig]) -> Self {
        Self {
            indices,
            predsucc: PredSucc::new(indices),
            results: DashMap::new()
        }
    }

    pub fn add(&self, index: IBig, result: ParasectPayloadResult) -> Criticality {
        let before = self.predsucc.predecessor(&index)
            .and_then(|x| self.results.get(x));

        let after = self.predsucc.successor(&index)
            .and_then(|x| self.results.get(x));

        let criticality = match (before.as_deref(), &result, after.as_deref()) {
            (Some(Stop(_)), _, _) => NoCriticalSeen,
            (_, Stop(_), _) => NoCriticalSeen,
            (_, _, Some(Stop(_))) => NoCriticalSeen,
            (Some(Continue(Good)), Continue(Bad), _) => CriticalSeen,
            (_, Continue(Good), Some(Continue(Bad))) => CriticalSeen,
            _ => NoCriticalSeen
        };

        self.results.insert(index, result);

        criticality
    }

    pub fn critical_point(self) -> Result<IBig, ParasectError> {
        let mut crit_pt: Option<IBig> = None;

        for index in self.indices {
            let (index_owned, result) = match self.results.remove(index) {
                Some(v) => {
                    v
                },
                None => {
                    continue
                }
            };

            match (index_owned, result) {
                (_, Stop(e)) => return Err(PayloadError(e)),
                (n, Continue(Good)) => {
                    if let Some(existing_crit_pt) = crit_pt {
                        return Err(InconsistencyError(format!("Found two critical points at {} and {}", existing_crit_pt, n)));
                    }
                }
                (n, Continue(Bad)) => {
                    crit_pt = crit_pt.or(Some(n));
                }
            };
        }

        match crit_pt {
            Some(n) => Ok(n),
            None => Err(InconsistencyError("All points were good.".into()))
        }
    }
}
