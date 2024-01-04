use std::{io, thread};
use std::collections::HashMap;
use num_cpus;
use ibig::{ibig, IBig};
use crate::algorithms::parasect::ParasectError::*;
use crate::algorithms::parasect::ParasectPayloadAnswer::*;
use crate::algorithms::parasect::ParasectPayloadResult::*;

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
pub struct ParasectSettings<TPayload>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Send + Sync {
    low: IBig,
    high: IBig,
    payload: TPayload,
    before_level_callback: Option<fn(&[IBig]) -> ()>,
    after_level_callback: Option<fn(&HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>) -> ()>,
    max_parallelism: usize,
}

impl<TPayload> ParasectSettings<TPayload>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Send + Sync {
    pub fn new<A: Into<IBig>, B: Into<IBig>>(low: A, high: B, payload: TPayload) -> Self {
        return ParasectSettings {
            low: low.into(),
            high: high.into(),
            payload,
            before_level_callback: None,
            after_level_callback: None,
            max_parallelism: num_cpus::get(),
        };
    }

    pub fn with_before_level_callback(mut self, callback: fn(&[IBig]) -> ()) -> Self {
        self.before_level_callback = Some(callback);
        self
    }

    pub fn with_after_level_callback(mut self, callback: fn(&HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>) -> ()) -> Self {
        self.after_level_callback = Some(callback);
        self
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

pub(crate) fn payload_result_to_result(payload_result: ParasectPayloadResult)
                                       -> Result<ParasectPayloadAnswer, ParasectError> {
    match payload_result {
        Continue(v) => Ok(v),
        Stop(e) => Err(PayloadError(e))
    }
}

pub(crate) fn parallel_execute_payload<TPayload>(indices: &[IBig], payload: &TPayload)
                                                 -> HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Send + Sync {
    thread::scope(|scope| {
        let threads = indices.into_iter()
            .map(|x| {
                (x.clone(), scope.spawn(|| payload(x.clone())))
            });

        threads.map(|(n, t)| {
            let t_result = t.join().expect("spawned parasect thread panicked");
            (n, payload_result_to_result(t_result))
        }).collect()
    })
}

pub(crate) fn find_bounds_from_bool_list(
    sorted_bool_results: &[(IBig, bool)], original_low: IBig, original_high: IBig)
    -> Result<(IBig, IBig), ParasectError> {
    let mut last_true: Option<IBig> = None;
    let mut first_false: Option<IBig> = None;

    for (n, b) in sorted_bool_results {
        if !*b {
            first_false = first_false.or_else(|| Some(n.clone()));
        } else {
            if let Some(idx) = first_false {
                return Err(InconsistencyError(format!("Found a sequence of bad starting at {}, then another good at {}.", idx, n)));
            }
            last_true = Some(n.clone());
        }
    }

    match (last_true, first_false) {
        (Some(b), Some(a)) => Ok((b + 1, a)),
        (None, Some(a)) => Ok((original_low, a)),
        (Some(b), None) => {
            if &b + 1 > original_high {
                Err(InconsistencyError(format!("All values in [{}, {}] are good.", original_low, original_high)))
            } else {
                Ok((b + 1, original_high))
            }
        },
        _ => panic!("Cannot find bounds of an empty result sequence.")
    }
}

pub(crate) fn find_new_bounds(
    level_map: HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>,
    original_low: IBig,
    original_high: IBig) -> Result<(IBig, IBig), ParasectError> {
    let mut return_error: Option<ParasectError> = None;

    let mut bool_results: Vec<(IBig, bool)> = Vec::new();

    for (n, r) in level_map {
        match r {
            Err(e) => return_error = return_error.or(Some(e)),
            Ok(Good) => bool_results.push((n.clone(), true)),
            Ok(Bad) => bool_results.push((n.clone(), false))
        }
    }

    if let Some(e) = return_error {
        return Err(e);
    }

    bool_results.sort();

    find_bounds_from_bool_list(&bool_results, original_low, original_high)
}

pub fn parasect<TPayload>(settings: ParasectSettings<TPayload>) -> Result<IBig, ParasectError>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Send + Sync {
    let mut low = settings.low;
    let mut high = settings.high;

    while low < high {
        let indices = compute_parasect_indices(&low, &high, settings.max_parallelism);

        if let Some(before_cb) = settings.before_level_callback {
            before_cb(&indices);
        }

        let level_results = parallel_execute_payload(&indices, &settings.payload);

        if let Some(after_cb) = settings.after_level_callback {
            after_cb(&level_results);
        }

        let (new_low, new_high) = find_new_bounds(level_results, low.clone(), high.clone())?;
        assert_ne!((low.clone(), high.clone()), (new_low.clone(), new_high.clone()));
        (low, high) = (new_low, new_high);
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
    fn test_parallel_execute_payload() {
        let indices = ibig_vec(&[-100, -69, 0, 1, 42]);
        let statuses = Mutex::new(HashMap::from([
            (ibig(-100), Continue(Good)),
            (ibig(-69), Continue(Good)),
            (ibig(0), Continue(Bad)),
            (ibig(1), Continue(Bad)),
            (ibig(42), Stop("amogus".into()))]));

        let expected = HashMap::from([
            (ibig(-100), Ok(Good)),
            (ibig(-69), Ok(Good)),
            (ibig(0), Ok(Bad)),
            (ibig(1), Ok(Bad)),
            (ibig(42), Err(PayloadError("amogus".into())))]);

        let actual = parallel_execute_payload(&indices, &|idx| {
            let mut ss = statuses.lock().unwrap();
            ss.remove(&idx).unwrap()
        });

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_find_bounds_from_bool_list_peak() {
        let result = find_bounds_from_bool_list(&[
            (ibig(0), true),
            (ibig(2), true),
            (ibig(4), false),
            (ibig(6), false),
            (ibig(8), false)
        ], ibig(-2), ibig(10));

        match result {
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(3), ibig(4))),
            Err(e) => panic!("expected (3, 4), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_bounds_from_bool_list_all_bad() {
        let result = find_bounds_from_bool_list(&[
            (ibig(0), false),
            (ibig(2), false),
            (ibig(4), false),
            (ibig(6), false),
            (ibig(8), false)
        ], ibig(-2), ibig(10));

        match result {
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(-2), ibig(0))),
            Err(e) => panic!("expected (-2, 0), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_bounds_from_bool_list_all_bad_boundary() {
        let result = find_bounds_from_bool_list(&[
            (ibig(0), false),
            (ibig(1), false),
        ], ibig(0), ibig(10));

        match result {
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(0), ibig(0))),
            Err(e) => panic!("expected (0, 0), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_bounds_from_bool_list_all_good_boundary() {
        let result = find_bounds_from_bool_list(&[
            (ibig(-1), true),
            (ibig(0), true),
        ], ibig(-10), ibig(0));

        match result {
            Err(InconsistencyError(s)) => assert_eq!("All values in [-10, 0] are good.", s),
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(0), ibig(0))),
            Err(e) => panic!("expected InconsistencyError(\"All values in [-10, 0] are good.\"), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_bounds_from_bool_list_all_good() {
        let result = find_bounds_from_bool_list(&[
            (ibig(0), true),
            (ibig(2), true),
            (ibig(4), true),
            (ibig(6), true),
            (ibig(8), true)
        ], ibig(-2), ibig(10));

        match result {
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(9), ibig(10))),
            Err(e) => panic!("expected (9, 10), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_bounds_from_bool_list_inconsistency() {
        let result = find_bounds_from_bool_list(&[
            (ibig(100), true),
            (ibig(102), false),
            (ibig(104), false),
            (ibig(106), true),
            (ibig(108), false)
        ], ibig(98), ibig(110));

        match result {
            Ok(o) => panic!("expected InconsistencyError, but got positive result {:?}", o),
            Err(InconsistencyError(s)) =>
                assert_eq!(s, "Found a sequence of bad starting at 102, then another good at 106."),
            Err(e) => panic!("expected InconsistencyError, but got {:?}", e)
        }
    }

    #[test]
    fn test_find_new_bounds() {
        let result = find_new_bounds(HashMap::from([
            (ibig(100), Ok(Good)),
            (ibig(102), Ok(Good)),
            (ibig(104), Ok(Bad)),
            (ibig(106), Ok(Bad)),
            (ibig(108), Ok(Bad))]), ibig(98), ibig(110));

        match result {
            Ok((lo, hi)) => assert_eq!((lo, hi), (ibig(103), ibig(104))),
            Err(e) => panic!("expected (103, 104), but got error {:?}", e)
        }
    }

    #[test]
    fn test_find_new_bounds_err() {
        let result = find_new_bounds(HashMap::from([
            (ibig(100), Ok(Good)),
            (ibig(102), Ok(Good)),
            (ibig(104), Ok(Bad)),
            (ibig(106), Err(PayloadError("it's broken".into()))),
            (ibig(108), Ok(Bad))]), ibig(98), ibig(110));

        match result {
            Err(PayloadError(e)) => assert_eq!("it's broken", e),
            x => panic!("expected PayloadError(\"it's broken\"), got {:?}", x)
        }
    }

    #[test]
    fn test_parasect() {
        let result = parasect(
            ParasectSettings::new(ibig(1), ibig(500), |x|
                if x < ibig(320) { Continue(Good) } else { Continue(Bad) })
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
                if x < ibig(15) { Stop("error".into()) } else { Continue(Bad) })
        );

        match result {
            Err(PayloadError(s)) => assert_eq!(s, "error"),
            x => panic!("expected PayloadError(\"error\"), got {:?}", x)
        }
    }

    #[quickcheck]
    fn qc_parasect(a: i16, b: i16, c: i16) -> TestResult {
        let mut nums = [a, b, c];
        nums.sort();
        let [lo, lt, hi] = nums;

        println!("testing {} {} {}", lo, lt, hi);

        let result =
            parasect(
                ParasectSettings::new(lo, hi, |x|
                    if x < IBig::from(lt) { Continue(Good) } else { Continue(Bad) }));

        match result {
            Ok(v) => {
                println!("positive result {}", v);
                TestResult::from_bool(v == IBig::from(lt))
            },
            e => {
                println!("error result {:?}", e);
                TestResult::failed()
            }
        }
    }
}
