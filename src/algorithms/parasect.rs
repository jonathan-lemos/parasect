use crate::algorithms::parasect::ParasectError::*;
use crate::algorithms::parasect::ParasectPayloadAnswer::*;
use crate::algorithms::parasect::ParasectPayloadResult::*;
use crate::algorithms::parasect_result_map::Criticality::CriticalSeen;
use crate::algorithms::parasect_result_map::ParasectResultMap;
use crate::collections::pred_succ::PredSucc;
use crate::task::cancellable_task::CancellableTask;
use crate::task::cancellable_task_util::execute_parallel_cancellable;
use crate::task::cancellable_task_util::CancellationType::*;
use ibig::IBig;
use num_cpus;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Clone, Debug)]
pub enum ParasectPayloadAnswer {
    Good,
    Bad,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectPayloadResult {
    Continue(ParasectPayloadAnswer),
    Stop(String),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectError {
    PayloadError(String),
    InconsistencyError(String),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ParasectSettings<TTask, TPayload, TBeforeCb, TAfterCb>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    TPayload: (Fn(IBig) -> TTask) + Send + Sync,
    TBeforeCb: Fn(&[IBig]) -> (),
    TAfterCb: Fn(&IBig, &IBig) -> (),
{
    low: IBig,
    high: IBig,
    payload: TPayload,
    before_level_callback: TBeforeCb,
    after_level_callback: TAfterCb,
    max_parallelism: usize,
}

impl<TTask, TPayload> ParasectSettings<TTask, TPayload, fn(&[IBig]) -> (), fn(&IBig, &IBig) -> ()>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    TPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    pub fn new<A: Into<IBig>, B: Into<IBig>>(low: A, high: B, payload: TPayload) -> Self {
        return ParasectSettings {
            low: low.into(),
            high: high.into(),
            payload,
            before_level_callback: |_| {},
            after_level_callback: |_, _| {},
            max_parallelism: num_cpus::get(),
        };
    }
}

impl<TTask, TPayload, TBeforeCb, TAfterCb> ParasectSettings<TTask, TPayload, TBeforeCb, TAfterCb>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    TPayload: (Fn(IBig) -> TTask) + Send + Sync,
    TBeforeCb: Fn(&[IBig]) -> (),
    TAfterCb: Fn(&IBig, &IBig) -> (),
{
    pub fn with_before_level_callback<TNewBeforeCb>(
        self,
        callback: TNewBeforeCb,
    ) -> ParasectSettings<TTask, TPayload, TNewBeforeCb, TAfterCb>
    where
        TNewBeforeCb: Fn(&[IBig]) -> (),
    {
        ParasectSettings {
            low: self.low,
            high: self.high,
            payload: self.payload,
            before_level_callback: callback,
            after_level_callback: self.after_level_callback,
            max_parallelism: self.max_parallelism,
        }
    }

    pub fn with_after_level_callback<TNewAfterCb>(
        self,
        callback: TNewAfterCb,
    ) -> ParasectSettings<TTask, TPayload, TBeforeCb, TNewAfterCb>
    where
        TNewAfterCb: Fn(&IBig, &IBig) -> (),
    {
        ParasectSettings {
            low: self.low,
            high: self.high,
            payload: self.payload,
            before_level_callback: self.before_level_callback,
            after_level_callback: callback,
            max_parallelism: self.max_parallelism,
        }
    }

    pub fn with_max_parallelism(mut self, parallelism: usize) -> Self {
        self.max_parallelism = parallelism;
        self
    }
}

pub(crate) fn ibig_range(low: &IBig, high: &IBig) -> Vec<IBig> {
    let mut ret = Vec::new();
    let mut i = low.clone();

    while &i <= high {
        ret.push(i.clone());
        i += 1;
    }

    ret
}

pub(crate) fn compute_parasect_indices(low: &IBig, high: &IBig, count: usize) -> Vec<IBig> {
    debug_assert!(low <= high);

    let delta = high - low;

    if delta <= IBig::from(count) {
        return ibig_range(low, high);
    }

    let gap = (&delta - count) / (count + 1);
    let mut remainder = usize::try_from((&delta - count) % (count + 1))
        .expect("this value cannot exceed count, and delta must be positive, so the out of bounds condition should not happen");

    let mut ret = Vec::new();
    let mut i = low.clone();

    for _ in 0..count {
        i += &gap;
        if remainder > 0 {
            remainder -= 1;
            i += 1;
        }
        ret.push(i.clone());
        i += 1;
    }
    ret
}

pub(crate) fn get_first_bad_index<TTask, TPayload>(
    indices: &[IBig],
    payload: &TPayload,
) -> Result<IBig, ParasectError>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    TPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    let result_map = ParasectResultMap::new(indices);

    execute_parallel_cancellable(indices.into_iter().map(|x| {
        payload(x.clone()).map(|result| {
            let crit_pt = result_map.add(x.clone(), (*result).clone());

            (
                (),
                if crit_pt == CriticalSeen {
                    CancelOthers
                } else {
                    ContinueOthers
                },
            )
        })
    }));

    result_map.critical_point()
}

pub(crate) fn get_new_bounds(low: IBig, indices: &[IBig], first_bad_index: IBig) -> (IBig, IBig) {
    let predsucc = PredSucc::new(indices);

    let low = predsucc.predecessor(&low).map(|x| x.clone()).unwrap_or(low);
    let high = first_bad_index;

    (low, high)
}

pub fn parasect<TTask, TPayload, TBeforeCb, TAfterCb>(
    settings: ParasectSettings<TTask, TPayload, TBeforeCb, TAfterCb>,
) -> Result<IBig, ParasectError>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    TPayload: (Fn(IBig) -> TTask) + Send + Sync,
    TBeforeCb: Fn(&[IBig]) -> (),
    TAfterCb: Fn(&IBig, &IBig) -> (),
{
    let mut low = settings.low;
    let mut high = settings.high;

    while low < high {
        let indices = compute_parasect_indices(&low, &high, settings.max_parallelism);

        (settings.before_level_callback)(&indices);

        let first_bad_index = get_first_bad_index(&indices, &settings.payload)?;
        (low, high) = get_new_bounds(low, &indices, first_bad_index);

        (settings.after_level_callback)(&low, &high);
    }

    Ok(low)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use ibig::ibig;
    use proptest::prelude::*;
    use std::iter::zip;
    use std::sync::Mutex;

    fn ibig(i: isize) -> IBig {
        IBig::from(i)
    }

    fn ibig_vec(v: &[isize]) -> Vec<IBig> {
        v.into_iter().map(|x| ibig(*x)).collect()
    }

    #[test]
    fn test_ibig_range() {
        assert_eq!(ibig_vec(&[2, 3, 4, 5]), ibig_range(&ibig!(2), &ibig!(5)))
    }

    #[test]
    fn test_ibig_empty_range() {
        assert_eq!(ibig_vec(&[]), ibig_range(&ibig!(2), &ibig!(1)))
    }

    #[test]
    fn test_compute_parasect_indices() {
        let results = compute_parasect_indices(&ibig!(2), &ibig!(22), 5);
        assert_eq!(ibig_vec(&[5, 9, 13, 16, 19]), results)
    }

    #[test]
    fn test_compute_parasect_indices_empty() {
        let results = compute_parasect_indices(&ibig!(2), &ibig!(22), 0);
        assert_eq!(ibig_vec(&[]), results)
    }

    #[test]
    fn test_compute_parasect_indices_single() {
        let results = compute_parasect_indices(&ibig!(2), &ibig!(22), 1);
        assert_eq!(ibig_vec(&[12]), results)
    }

    #[test]
    fn test_get_first_bad_index() {
        let indices = ibig_vec(&[-100, -69, 42, 420, 69420]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Good)),
            (ibig(-69), Continue(Good)),
            (ibig(42), Continue(Bad)),
            (ibig(420), Continue(Bad)),
            (ibig(69420), Continue(Bad)),
        ]));

        let actual = {
            let statuses_ref = &statuses;

            get_first_bad_index(&indices, &|idx| {
                FreeCancellableTask::new({
                    let mut ss = statuses_ref.lock().unwrap();
                    ss.remove(&idx).unwrap()
                })
            })
        };

        assert_eq!(Ok(ibig(42)), actual);
    }

    #[test]
    fn test_get_first_bad_index_stop() {
        let indices = ibig_vec(&[-100, -69, 0, 1, 42]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Good)),
            (ibig(-69), Continue(Good)),
            (ibig(42), Continue(Bad)),
            (ibig(420), Continue(Bad)),
            (ibig(69420), Stop("amogus".into())),
        ]));

        let actual = get_first_bad_index(&indices, &|idx| {
            FreeCancellableTask::new({
                let mut ss = statuses.lock().unwrap();
                ss.remove(&idx).unwrap()
            })
        });

        assert_eq!(Err(PayloadError("amogus".into())), actual);
    }

    #[test]
    fn test_get_first_bad_index_all_bad() {
        let indices = ibig_vec(&[-100, -69, 0, 1, 42]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Bad)),
            (ibig(-69), Continue(Bad)),
            (ibig(42), Continue(Bad)),
            (ibig(420), Continue(Bad)),
            (ibig(69420), Continue(Bad)),
        ]));

        let actual = get_first_bad_index(&indices, &|idx| {
            FreeCancellableTask::new({
                let mut ss = statuses.lock().unwrap();
                ss.remove(&idx).unwrap()
            })
        });

        assert_eq!(Ok(ibig(-100)), actual);
    }

    #[test]
    fn test_get_first_bad_index_all_good() {
        let indices = ibig_vec(&[-100, -69, 0, 1, 42]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Good)),
            (ibig(-69), Continue(Good)),
            (ibig(42), Continue(Good)),
            (ibig(420), Continue(Good)),
            (ibig(69420), Continue(Good)),
        ]));

        let actual = get_first_bad_index(&indices, &|idx| {
            FreeCancellableTask::new({
                let mut ss = statuses.lock().unwrap();
                ss.remove(&idx).unwrap()
            })
        });

        assert_eq!(
            Err(InconsistencyError("All points were good.".into())),
            actual
        );
    }

    #[test]
    fn test_parasect() {
        let result = parasect(ParasectSettings::new(ibig(1), ibig(500), |x| {
            FreeCancellableTask::new(if x < ibig(320) {
                Continue(Good)
            } else {
                Continue(Bad)
            })
        }));

        match result {
            Ok(v) => assert_eq!(v, ibig(320)),
            x => panic!("expected 320, got {:?}", x),
        }
    }

    #[test]
    fn test_parasect_stop() {
        let result = parasect(ParasectSettings::new(ibig(1), ibig(500), |x| {
            FreeCancellableTask::new(if x < ibig(15) {
                Stop("error".into())
            } else {
                Continue(Bad)
            })
        }));

        match result {
            Err(PayloadError(s)) => assert_eq!(s, "error"),
            x => panic!("expected PayloadError(\"error\"), got {:?}", x),
        }
    }

    #[test]
    fn test_parasect_all_good() {
        let result = parasect(ParasectSettings::new(ibig(1), ibig(500), |_| {
            FreeCancellableTask::new(Continue(Good))
        }));

        assert_eq!(
            result,
            Err(InconsistencyError("All values are good.".into()))
        );
    }

    proptest! {
        #[test]
        fn prop_parasect_fuzz(a in 1..1000, b in 1..1000, c in 1..1000) {
            let mut nums = [a, b, c];
            nums.sort();
            let [lo, lt, hi] = nums;

            let result =
                parasect(
                    ParasectSettings::new(lo, hi, |x|
                        FreeCancellableTask::new(if x < IBig::from(lt) { Continue(Good) } else { Continue(Bad) })));

            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap() == IBig::from(lt));
        }
    }
}
