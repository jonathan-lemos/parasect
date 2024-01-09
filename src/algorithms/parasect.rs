use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use num_cpus;
use ibig::IBig;
use crate::algorithms::parasect::ParasectError::*;
use crate::algorithms::parasect::ParasectPayloadAnswer::*;
use crate::algorithms::parasect::ParasectPayloadResult::*;
use crate::task::cancellable_task::CancellableTask;
use crate::task::cancellable_task_util::CancellationType::*;
use crate::task::cancellable_task_util::execute_parallel_with_cancellation;

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
    where TTask: CancellableTask<ParasectPayloadResult> + Send,
          TPayload: (Fn(IBig) -> TTask) + Send + Sync,
          TBeforeCb: Fn(&[IBig]) -> (),
          TAfterCb: Fn(&IBig, &IBig) -> () {
    low: IBig,
    high: IBig,
    payload: TPayload,
    before_level_callback: TBeforeCb,
    after_level_callback: TAfterCb,
    max_parallelism: usize,
}


impl<TTask, TPayload> ParasectSettings<TTask, TPayload, fn(&[IBig]) -> (), fn(&IBig, &IBig) -> ()>
    where TTask: CancellableTask<ParasectPayloadResult> + Send,
          TPayload: (Fn(IBig) -> TTask) + Send + Sync {
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
    where TTask: CancellableTask<ParasectPayloadResult> + Send,
          TPayload: (Fn(IBig) -> TTask) + Send + Sync,
          TBeforeCb: Fn(&[IBig]) -> (),
          TAfterCb: Fn(&IBig, &IBig) -> () {
    pub fn with_before_level_callback<TNewBeforeCb>(self, callback: TNewBeforeCb)
                                                    -> ParasectSettings<TTask, TPayload, TNewBeforeCb, TAfterCb>
        where TNewBeforeCb: Fn(&[IBig]) -> () {
        ParasectSettings {
            low: self.low,
            high: self.high,
            payload: self.payload,
            before_level_callback: callback,
            after_level_callback: self.after_level_callback,
            max_parallelism: self.max_parallelism,
        }
    }

    pub fn with_after_level_callback<TNewAfterCb>(self, callback: TNewAfterCb)
                                                  -> ParasectSettings<TTask, TPayload, TBeforeCb, TNewAfterCb>
        where TNewAfterCb: Fn(&IBig, &IBig) -> () {
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

pub(crate) fn predecessor_map<T: Hash + PartialEq + Eq>(elements: &[T]) -> HashMap<&T, &T> {
    elements.iter().skip(1).zip(elements.iter()).collect()
}

pub(crate) fn successor_map<T: Hash + PartialEq + Eq>(elements: &[T]) -> HashMap<&T, &T> {
    elements.iter().zip(elements.iter().skip(1)).collect()
}

pub(crate) fn first_bad_index_from_results(results: Vec<Option<(IBig, ParasectPayloadResult, bool)>>) -> Result<IBig, ParasectError> {
    let mut crit_pt: Option<IBig> = None;

    for res in results {
        match res {
            Some((_, Stop(e), _)) => return Err(PayloadError(e)),
            Some((n, Continue(_), true)) => {
                if let Some(existing_crit_pt) = crit_pt {
                    return Err(InconsistencyError(format!("Found two critical points at {} and {}", existing_crit_pt, n)));
                }
                crit_pt = Some(n);
            }
            _ => ()
        };
    }

    match crit_pt {
        Some(n) => Ok(n),
        None => Err(InconsistencyError("All points were good.".into()))
    }
}

pub(crate) fn get_first_bad_index<TTask, TPayload>(indices: &[IBig], payload: &TPayload)
                                                   -> Result<IBig, ParasectError>
    where TTask: CancellableTask<ParasectPayloadResult> + Send,
          TPayload: (Fn(IBig) -> TTask) + Send + Sync {
    let result_map = Mutex::new(HashMap::<IBig, ParasectPayloadResult>::new());
    let predecessors = predecessor_map(indices);
    let successors = successor_map(indices);

    let is_critical_point = |x: &IBig, rm: &HashMap<IBig, ParasectPayloadResult>| -> bool {
        match (rm.get(x), predecessors.get(x), successors.get(x)) {
            (Some(Continue(Good)), _, Some(succ)) =>
                rm.get(succ) == Some(&Continue(Bad)),
            (Some(Continue(Bad)), Some(pred), _) =>
                rm.get(pred) == Some(&Continue(Good)),
            (Some(Continue(Bad)), None, _) => true,
            _ => false
        }
    };

    let ctask_results = execute_parallel_with_cancellation(indices.into_iter().map(|x| {
        payload(x.clone()).map(|result| {
            let crit_pt = {
                let mut guard = result_map.lock().unwrap();
                guard.insert(x.clone(), result.clone());

                is_critical_point(x, &guard)
            };

            ((x.clone(), result, crit_pt), if crit_pt { CancelOthers } else { ContinueOthers })
        })
    }));

    first_bad_index_from_results(ctask_results)
}

pub(crate) fn get_new_bounds(low: IBig, indices: &[IBig], first_bad_index: IBig) -> (IBig, IBig) {
    let predecessors = predecessor_map(&indices);

    (predecessors.get(&first_bad_index).map(|x| (*x).clone()).unwrap_or(low), first_bad_index)
}

pub fn parasect<TTask, TPayload, TBeforeCb, TAfterCb>(settings: ParasectSettings<TTask, TPayload, TBeforeCb, TAfterCb>)
                                                      -> Result<IBig, ParasectError>
    where TTask: CancellableTask<ParasectPayloadResult> + Send,
          TPayload: (Fn(IBig) -> TTask) + Send + Sync,
          TBeforeCb: Fn(&[IBig]) -> (),
          TAfterCb: Fn(&IBig, &IBig) -> () {
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
    use std::collections::HashSet;
    use std::iter::zip;
    use std::sync::Mutex;
    use ibig::ops::Abs;
    use quickcheck::*;
    use ibig::ibig;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use super::*;

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

    #[quickcheck]
    fn qc_ibig_range(lo: i16, hi: i16) -> TestResult {
        if lo > hi {
            return TestResult::discard();
        }

        let lo_ibig = IBig::from(lo);
        let hi_ibig = IBig::from(hi);

        let result = ibig_range(&lo_ibig, &hi_ibig);

        assert_eq!(result.len(), (hi as i64 - lo as i64) as usize + 1);
        assert_eq!(result.first().unwrap(), &lo_ibig);
        assert_eq!(result.last().unwrap(), &hi_ibig);

        for (a, b) in zip(result.iter(), result.iter().skip(1)) {
            assert_eq!(&(a + 1), b)
        }

        TestResult::passed()
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

    #[quickcheck]
    fn qc_compute_parasect_indices_gaps_even(start: i16, end: i16, count: u8) -> TestResult {
        let start = IBig::from(start);
        let end = IBig::from(end);

        let delta = &end - &start;
        if delta <= IBig::from(count) {
            return TestResult::discard();
        }

        let mut gap_sizes = HashSet::<IBig>::new();

        let mut vec = vec!(start.clone());
        vec.append(&mut compute_parasect_indices(&start, &end, count as usize));
        vec.push(end.clone());

        for (a, b) in zip(vec.iter(), vec.iter().skip(1)) {
            gap_sizes.insert(b - a);
        }

        let gap_sizes = gap_sizes.into_iter().collect::<Vec<IBig>>();

        match gap_sizes.len() {
            1 => TestResult::passed(),
            2 => {
                assert_eq!((gap_sizes.first().unwrap() - gap_sizes.last().unwrap()).abs(), IBig::from(1));
                TestResult::passed()
            }
            _ => TestResult::failed()
        }
    }

    #[test]
    fn test_get_first_bad_index() {
        let indices = ibig_vec(&[-100, -69, 0, 1, 42]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Good)),
            (ibig(-69), Continue(Good)),
            (ibig(42), Continue(Bad)),
            (ibig(420), Continue(Bad)),
            (ibig(69420), Continue(Bad))]));

        let actual = {
            let statuses_ref = &statuses;

            get_first_bad_index(&indices, &|idx| FreeCancellableTask::new({
                let mut ss = statuses_ref.lock().unwrap();
                Some(ss.remove(&idx).unwrap())
            }))
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
            (ibig(69420), Stop("amogus".into()))]));

        let actual = get_first_bad_index(&indices, &|idx| FreeCancellableTask::new({
            let mut ss = statuses.lock().unwrap();
            Some(ss.remove(&idx).unwrap())
        }));

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
            (ibig(69420), Continue(Bad))]));

        let actual = get_first_bad_index(&indices, &|idx| FreeCancellableTask::new({
            let mut ss = statuses.lock().unwrap();
            Some(ss.remove(&idx).unwrap())
        }));

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
            (ibig(69420), Continue(Good))]));

        let actual = get_first_bad_index(&indices, &|idx| FreeCancellableTask::new({
            let mut ss = statuses.lock().unwrap();
            Some(ss.remove(&idx).unwrap())
        }));

        assert_eq!(Err(InconsistencyError("All points were good.".into())), actual);
    }

    #[test]
    fn test_parasect() {
        let result = parasect(
            ParasectSettings::new(ibig(1), ibig(500), |x|
                FreeCancellableTask::new(
                    if x < ibig(320) { Some(Continue(Good)) } else { Some(Continue(Bad)) }))
        );

        match result {
            Ok(v) => assert_eq!(v, ibig(320)),
            x => panic!("expected 320, got {:?}", x)
        }
    }

    #[test]
    fn test_parasect_stop() {
        let result = parasect(
            ParasectSettings::new(ibig(1), ibig(500), |x|
                FreeCancellableTask::new(
                    || if x < ibig(15) { Stop("error".into()) } else { Continue(Bad) }))
        );

        match result {
            Err(PayloadError(s)) => assert_eq!(s, "error"),
            x => panic!("expected PayloadError(\"error\"), got {:?}", x)
        }
    }

    #[test]
    fn test_parasect_all_good() {
        let result = parasect(
            ParasectSettings::new(ibig(1), ibig(500), |_| FreeCancellableTask::new(|| Continue(Good)))
        );

        assert_eq!(result, Err(InconsistencyError("All values are good.".into())));
    }

    #[quickcheck]
    fn qc_parasect(a: i16, b: i16, c: i16) -> TestResult {
        let mut nums = [a, b, c];
        nums.sort();
        let [lo, lt, hi] = nums;

        let result =
            parasect(
                ParasectSettings::new(lo, hi, |x|
                    FreeCancellableTask::new(|| if x < IBig::from(lt) { Continue(Good) } else { Continue(Bad) })));

        match result {
            Ok(v) => TestResult::from_bool(v == IBig::from(lt)),
            _ => TestResult::failed()
        }
    }
}
