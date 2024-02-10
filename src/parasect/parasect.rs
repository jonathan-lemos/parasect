use crate::collections::collect_collection::CollectVec;
use crate::parasect::background_loop::BackgroundLoop;
use crate::parasect::background_loop::BackgroundLoopBehavior::{Cancel, DontCancel};
use crate::parasect::event_handler::Event;
use crate::parasect::event_handler::Event::{
    ParasectCancelled, RangeInvalidated, WorkerMessageSent,
};
use crate::parasect::types::ParasectError::PayloadError;
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use crate::parasect::types::{ParasectError, ParasectPayloadResult};
use crate::parasect::worker::PointCompletionMessageType::{Cancelled, Completed};
use crate::parasect::worker::Worker;
use crate::range::bisecting_range_queue::BisectingRangeQueue;
use crate::range::numeric_range::NumericRange;
use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::{unbounded, Sender};
use dashmap::DashMap;
use ibig::IBig;
use num_cpus;
use std::cmp::max;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Clone, Debug)]
pub struct ParasectSettings<TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    range: NumericRange,
    payload: FPayload,
    event_sender: Option<Sender<Event>>,
    max_parallelism: usize,
}

impl<TTask, FPayload> ParasectSettings<TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    pub fn new(range: NumericRange, payload: FPayload) -> Self {
        return ParasectSettings {
            range,
            payload,
            event_sender: None,
            max_parallelism: num_cpus::get(),
        };
    }
}

#[allow(unused)]
impl<TTask, FPayload> ParasectSettings<TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    pub fn with_event_sender(mut self, sender: Sender<Event>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn with_max_parallelism(mut self, parallelism: usize) -> Self {
        self.max_parallelism = parallelism;
        self
    }
}

fn process_result_map(
    results: DashMap<IBig, ParasectPayloadResult>,
) -> Result<IBig, ParasectError> {
    let mut good = Vec::new();
    let mut bad = Vec::new();

    for (k, v) in results.into_iter() {
        match v {
            Continue(Good) => good.push(k),
            Continue(Bad) => bad.push(k),
            Stop(err) => return Err(PayloadError(err)),
        }
    }

    good.sort();
    bad.sort();

    if good.is_empty() {
        Err(PayloadError("All points were bad.".into()))
    } else if bad.is_empty() {
        Err(PayloadError("All points were good.".into()))
    } else if good.last().unwrap() < bad.first().unwrap() {
        Ok(bad.first().unwrap().clone())
    } else {
        Err(PayloadError(format!(
            "Found good point {} after bad point {}.",
            good.first().unwrap(),
            bad.first().unwrap()
        )))
    }
}

pub fn parasect<TTask, FPayload>(
    settings: ParasectSettings<TTask, FPayload>,
) -> Result<IBig, ParasectError>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    let (invalidation_sender, invalidation_receiver) = unbounded();
    let (msg_sender, msg_receiver) = unbounded();

    let queue = Arc::new(BisectingRangeQueue::new(
        settings.range.clone(),
        Some(invalidation_sender),
    ));

    let results = DashMap::<IBig, ParasectPayloadResult>::new();

    let workers = (0..settings.max_parallelism)
        .map(|i| Worker::new(i, queue.clone(), msg_sender.clone(), &settings.payload))
        .collect_vec();

    {
        let workers_ref = &workers;
        let ivr = &invalidation_receiver;
        let msgr = &msg_receiver;
        let global_cancel = CancellableMessage::<String>::new();
        let global_cancel_ref = &global_cancel;
        let queue_ref = &queue;
        let settings_ref = &settings;
        let latest_good = RwLock::<IBig>::new(&settings.range.first().unwrap() - 1);
        let latest_bad = RwLock::<IBig>::new(&settings.range.first().unwrap() - 1);

        thread::scope(|scope| {
            let invalidation_thread = BackgroundLoop::spawn(scope, ivr.clone(), |range| {
                if let Some(sender) = &settings_ref.event_sender {
                    sender
                        .send(RangeInvalidated(range.clone()))
                        .expect("Event sender was unexpectedly closed.");
                }
                for worker in workers_ref.iter() {
                    worker.skip_if_in_range(&range);
                }
                DontCancel
            });

            let msg_thread = BackgroundLoop::spawn(scope, msgr.clone(), |msg| {
                if let Some(sender) = &settings_ref.event_sender {
                    sender
                        .send(WorkerMessageSent(msg.clone()))
                        .expect("Event sender was unexpectedly closed.");
                }
                match msg.msg_type {
                    Completed(result) => {
                        match &result {
                            Continue(Good) => {
                                if latest_good.read().unwrap().deref() < &msg.point {
                                    let mut guard = latest_good.write().unwrap();
                                    *guard = max(guard.deref().clone(), msg.point.clone())
                                }
                                queue.invalidate(&msg.left);
                            }
                            Continue(Bad) => {
                                if latest_bad.read().unwrap().deref() < &msg.point {
                                    let mut guard = latest_bad.write().unwrap();
                                    *guard = max(guard.deref().clone(), msg.point.clone())
                                }
                                queue.invalidate(&msg.right);
                            }
                            Stop(reason) => {
                                global_cancel_ref.send(reason.clone());
                                return Cancel;
                            }
                        }
                        results.insert(msg.point, result);
                    }
                    _ => {}
                };
                DontCancel
            });

            scope.spawn(move || match global_cancel_ref.join() {
                None => {
                    msg_thread.cancel();
                    invalidation_thread.cancel();
                }
                Some(reason) => {
                    queue_ref.invalidate(&settings_ref.range);
                    if let Some(sender) = &settings_ref.event_sender {
                        sender
                            .send(ParasectCancelled(reason.as_ref().clone()))
                            .expect("event_sender should not be closed");
                    }
                }
            });

            let worker_threads = workers_ref
                .iter()
                .map(|w| scope.spawn(|| w.process_while_remaining()))
                .collect_vec();

            for t in worker_threads {
                t.join().unwrap();
            }

            global_cancel_ref.cancel();
        });
    }

    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::test_util::test_util::test_util::{ib, r};
    use ibig::ibig;
    use proptest::prelude::*;
    use std::sync::Mutex;

    #[test]
    fn test_parasect() {
        let result = parasect(ParasectSettings::new(r(1, 500), |x| {
            FreeCancellableTask::new(if x < ib(320) {
                Continue(Good)
            } else {
                Continue(Bad)
            })
        }));

        match result {
            Ok(v) => assert_eq!(v, ib(320)),
            x => panic!("expected 320, got {:?}", x),
        }
    }

    #[test]
    fn test_parasect_stop() {
        let result = parasect(ParasectSettings::new(r(1, 500), |x| {
            FreeCancellableTask::new(if x < ib(15) {
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
