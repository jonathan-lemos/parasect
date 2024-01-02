use std::{io, thread};
use std::collections::HashMap;
use num_cpus;
use ibig::{ibig, IBig};
use crate::algorithms::parasect::ParasectOrientation::*;
use crate::algorithms::parasect::ParasectError::*;
use crate::algorithms::parasect::ParasectPayloadAnswer::*;
use crate::algorithms::parasect::ParasectPayloadResult::*;

#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Clone)]
pub enum ParasectPayloadAnswer {
    Good,
    Bad,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ParasectPayloadResult {
    Continue(ParasectPayloadAnswer),
    Stop(String),
}

pub enum ParasectError {
    ThreadError(io::Error),
    PayloadError(String),
    InconsistencyError(String),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum ParasectOrientation {
    GoodFirst,
    BadFirst,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ParasectSettings<TPayload>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Clone + Send + Sync + 'static {
    low: IBig,
    high: IBig,
    payload: TPayload,
    before_level_callback: Option<fn(&[IBig]) -> ()>,
    after_level_callback: Option<fn(&HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>) -> ()>,
    max_parallelism: usize,
    orientation: Option<ParasectOrientation>,
}

impl<TPayload> ParasectSettings<TPayload>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Clone + Send + Sync {
    pub fn new(low: IBig, high: IBig, payload: TPayload) -> Self {
        return ParasectSettings {
            low,
            high,
            payload,
            before_level_callback: None,
            after_level_callback: None,
            max_parallelism: num_cpus::get(),
            orientation: None,
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

    pub fn with_orientation(mut self, orientation: ParasectOrientation) -> Self {
        self.orientation = Some(orientation);
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
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Send + Sync + Clone + 'static {

    let threads = indices.into_iter()
        .map(|x| {
            let xclone = x.clone();
            let payload_clone = payload.clone();
            (x.clone(), thread::spawn(move || payload_clone(xclone)))
        });

    threads.map(|(n, t)| {
        let t_result = t.join().expect("spawned parasect thread panicked");
        (n, payload_result_to_result(t_result))
    }).collect()
}

pub(crate) fn find_bounds_from_bool_list(
    sorted_bool_results: &[(IBig, bool)], original_low: IBig, original_high: IBig, orientation: ParasectOrientation)
    -> Result<(IBig, IBig), ParasectError> {
    let transition_marker = match orientation {
        GoodFirst => false,
        BadFirst => true
    };

    let mut last_before: Option<IBig> = None;
    let mut first_transition: Option<IBig> = None;
    for (n, b) in sorted_bool_results {
        if *b == transition_marker {
            first_transition = first_transition.or_else(|| Some(n.clone()));
        } else {
            if let Some(idx) = first_transition {
                let (sequence, outlier) = match orientation {
                    GoodFirst => ("bad", "good"),
                    BadFirst => ("good", "bad")
                };
                return Err(InconsistencyError(format!("Found a sequence of {} starting at {}, then another {} at {}.", sequence, idx, outlier, n)));
            }
            last_before = Some(n.clone());
        }
    }

    match (last_before, first_transition) {
        (Some(b), Some(a)) => Ok((b, a - 1)),
        (None, Some(a)) => Ok((original_low, a - 1)),
        (Some(b), None) => Ok((b, original_high)),
        _ => panic!("Cannot find bounds of an empty result sequence.")
    }
}

pub(crate) fn find_new_bounds(
    level_map: HashMap<IBig, Result<ParasectPayloadAnswer, ParasectError>>,
    original_low: IBig,
    original_high: IBig,
    orientation: ParasectOrientation) -> Result<(IBig, IBig), ParasectError> {
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

    find_bounds_from_bool_list(&bool_results, original_low, original_high, orientation)
}

pub(crate) fn determine_orientation<TPayload>(low: IBig, high: IBig, payload: &TPayload) -> Result<(IBig, IBig, ParasectOrientation), ParasectError>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Clone + Send + Sync + 'static {
    let mut bounds_result_map = parallel_execute_payload(&[low.clone(), high.clone()], payload);

    let low_result = bounds_result_map.remove(&low).unwrap();
    let high_result = bounds_result_map.remove(&high).unwrap();

    match (low_result, high_result) {
        (Ok(Good), Ok(Bad)) => Ok((low, high - 1, GoodFirst)),
        (Ok(Bad), Ok(Good)) => Ok((low, high - 1, BadFirst)),
        (Ok(Good), Ok(Good)) => Err(InconsistencyError(format!("Both the low point {} and high point {} are good.", low, high))),
        (Ok(Bad), Ok(Bad)) => Err(InconsistencyError(format!("Both the low point {} and high point {} are bad.", low, high))),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e)
    }
}

pub fn parasect<TPayload>(settings: ParasectSettings<TPayload>) -> Result<IBig, ParasectError>
    where TPayload: (Fn(IBig) -> ParasectPayloadResult) + Clone + Send + Sync {

    let mut low = settings.low;
    let mut high = settings.high;

    let orientation = if let Some(o) = settings.orientation {
        o
    } else {
        let (new_low, new_high, orientation) =
            determine_orientation(low, high, &settings.payload)?;
        low = new_low;
        high = new_high;
        orientation
    };

    while low < high {
        let indices = compute_parasect_indices(&low, &high, settings.max_parallelism);

        if let Some(before_cb) = settings.before_level_callback {
            before_cb(&indices);
        }

        let level_results = parallel_execute_payload(&indices, &settings.payload);

        if let Some(after_cb) = settings.after_level_callback {
            after_cb(&level_results);
        }

        (low, high) = find_new_bounds(level_results, low, high, orientation)?;
    }

    Ok(low)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::iter::zip;
    use ibig::ops::Abs;
    use quickcheck::*;
    use super::*;

    fn ibig_vec(v: &[isize]) -> Vec<IBig> {
        v.into_iter().map(|x| IBig::from(*x)).collect()
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

        assert_eq!(result.len(), (hi as isize - lo as isize) as usize + 1);
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
}
