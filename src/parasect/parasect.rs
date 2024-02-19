use crate::collections::collect_collection::CollectVec;
use crate::parasect::event::Event;
use crate::parasect::event::Event::{ParasectCancelled, RangeInvalidated, WorkerMessageSent};
use crate::parasect::types::ParasectError::{InconsistencyError, PayloadError};
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use crate::parasect::types::{ParasectError, ParasectPayloadAnswer, ParasectPayloadResult};
use crate::parasect::worker::PointCompletionMessageType::Completed;
use crate::parasect::worker::{Worker, WorkerMessage};
use crate::range::bisecting_range_queue::BisectingRangeQueue;
use crate::range::numeric_range::NumericRange;
use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;
use crate::threading::background_loop::BackgroundLoopBehavior::{Cancel, DontCancel};
use crate::threading::background_loop::{BackgroundLoopBehavior, ScopedBackgroundLoop};
use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use ibig::IBig;
use num_cpus;
use std::cmp::{max, min};
use std::ops::Deref;
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

struct ParasectController<'a, TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    settings: &'a ParasectSettings<TTask, FPayload>,
    message_receiver: Receiver<WorkerMessage>,
    queue: Arc<BisectingRangeQueue>,
    workers: Vec<Worker<TTask, &'a FPayload>>,
    latest_good: RwLock<IBig>,
    earliest_bad: RwLock<IBig>,
    results: DashMap<IBig, ParasectPayloadResult>,
    failure_message: CancellableMessage<String>,
}

impl<'a, TTask, FPayload> ParasectController<'a, TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    fn new(settings: &'a ParasectSettings<TTask, FPayload>) -> Self {
        let (message_sender, message_receiver) = unbounded();

        let queue = Arc::new(BisectingRangeQueue::new(settings.range.clone()));

        let workers = (0..settings.max_parallelism)
            .map(|i| Worker::new(i, queue.clone(), message_sender.clone(), &settings.payload))
            .collect_vec();

        Self {
            settings: &settings,
            message_receiver,
            queue,
            workers,
            latest_good: RwLock::new(IBig::from(settings.range.first().unwrap() - 1)),
            earliest_bad: RwLock::new(IBig::from(settings.range.last().unwrap() + 1)),
            results: DashMap::new(),
            failure_message: CancellableMessage::new(),
        }
    }

    fn invalidate_range(
        &self,
        range: &NumericRange,
        answer: ParasectPayloadAnswer,
    ) -> BackgroundLoopBehavior {
        self.queue.invalidate(&range);

        for worker in self.workers.iter() {
            worker.skip_if_in_range(&range);
        }

        if let Some(sender) = &self.settings.event_sender {
            sender
                .send(RangeInvalidated(range.clone(), answer))
                .expect("Event sender was unexpectedly closed.");
        }

        DontCancel
    }

    fn check_good_does_not_exceed_bad(&self) {
        let good_read = self.latest_good.read().unwrap();
        let bad_read = self.earliest_bad.read().unwrap();
        if good_read.deref() > bad_read.deref() {
            self.failure_message.send(format!("A good {} was detected after a bad {}. Parasect requires 1 or more good followed by remaining bad.", good_read.deref(), bad_read.deref()))
        }
    }

    fn adjust_latest_good(&self, point: &IBig) {
        if self.latest_good.read().unwrap().deref() < &point {
            let mut guard = self.latest_good.write().unwrap();
            *guard = max(guard.deref().clone(), point.clone());
        }

        self.check_good_does_not_exceed_bad();
    }

    fn adjust_earliest_bad(&self, point: &IBig) {
        if self.earliest_bad.read().unwrap().deref() > point {
            let mut guard = self.earliest_bad.write().unwrap();
            *guard = min(guard.deref().clone(), point.clone());
        }

        self.check_good_does_not_exceed_bad();
    }

    fn handle_message(&self, message: WorkerMessage) -> BackgroundLoopBehavior {
        if let Some(sender) = &self.settings.event_sender {
            sender
                .send(WorkerMessageSent(message.clone()))
                .expect("Event sender was unexpectedly closed.");
        }

        match message.msg_type {
            Completed(result) => {
                match &result {
                    Continue(Good) => {
                        self.adjust_latest_good(&message.point);
                        self.invalidate_range(&message.left.map_last(|x| x + 1), Good);
                    }
                    Continue(Bad) => {
                        self.adjust_earliest_bad(&message.point);
                        self.invalidate_range(&message.right.map_first(|x| x - 1), Bad);
                    }
                    Stop(reason) => {
                        self.failure_message.send(reason.clone());
                        self.results.insert(message.point, result);
                        return Cancel;
                    }
                }
                self.results.insert(message.point, result);
            }
            _ => {}
        };

        DontCancel
    }

    fn run(self) -> DashMap<IBig, ParasectPayloadResult> {
        let self_ref = &self;

        thread::scope(|scope| {
            let message_loop =
                ScopedBackgroundLoop::spawn(scope, self_ref.message_receiver.clone(), |msg| {
                    self_ref.handle_message(msg)
                });

            scope.spawn(move || {
                let result = self_ref.failure_message.join();

                message_loop.cancel();

                result.inspect(|reason| {
                    self_ref.queue.invalidate(&self_ref.settings.range.clone());
                    if let Some(sender) = &self_ref.settings.event_sender {
                        sender
                            .send(ParasectCancelled((*reason).clone()))
                            .expect("event_sender should not be closed");
                    }
                });
            });

            let worker_threads = self_ref
                .workers
                .iter()
                .map(|w| scope.spawn(|| w.process_while_remaining()))
                .collect_vec();

            for t in worker_threads {
                t.join().unwrap();
            }

            self_ref.failure_message.cancel();
        });

        while let Ok(msg) = self.message_receiver.try_recv() {
            self.handle_message(msg);
        }

        self.results
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
        Err(InconsistencyError("All points were bad.".into()))
    } else if bad.is_empty() {
        Err(InconsistencyError("All points were good.".into()))
    } else if good.last().unwrap() < bad.first().unwrap() {
        Ok(bad.first().unwrap().clone())
    } else {
        Err(InconsistencyError(format!(
            "Found good point {} after bad point {}.",
            good.first().unwrap(),
            bad.first().unwrap()
        )))
    }
}

/// Returns the first bad index in the given search space.
pub fn parasect<TTask, FPayload>(
    settings: ParasectSettings<TTask, FPayload>,
) -> Result<IBig, ParasectError>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: (Fn(IBig) -> TTask) + Send + Sync,
{
    if settings.range.is_empty() {
        return Err(InconsistencyError("Cannot parasect an empty range.".into()));
    }

    let controller = ParasectController::new(&settings);
    let results = controller.run();
    process_result_map(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::function_cancellable_task::FunctionCancellableTask;
    use crate::test_util::test_util::test_util::{ib, r};
    use proptest::prelude::*;
    use rand::random;
    use std::time::Duration;

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
        let result = parasect(ParasectSettings::new(r(1, 500), |_| {
            FreeCancellableTask::new(Continue(Good))
        }));

        assert_eq!(
            result,
            Err(InconsistencyError("All points were good.".into()))
        );
    }

    #[test]
    fn test_parasect_all_bad() {
        let result = parasect(ParasectSettings::new(r(1, 500), |_| {
            FreeCancellableTask::new(Continue(Bad))
        }));

        assert_eq!(
            result,
            Err(InconsistencyError("All points were bad.".into()))
        );
    }

    proptest! {
        #[test]
        fn prop_parasect_fuzz(a in 1..1000, b in 1..1000, c in 1..1000) {
            let mut nums = [a, b, c];
            nums.sort();
            let [lo, lt, hi] = nums;

            prop_assume!(lo < lt && lt < hi);

            let result =
                parasect(
                    ParasectSettings::new(r(lo, hi), |x|
                        FreeCancellableTask::new(if x < IBig::from(lt) { Continue(Good) } else { Continue(Bad) })).with_max_parallelism(3));

            prop_assert_eq!(result, Ok(IBig::from(lt)));
        }

        #[test]
        fn prop_parasect_slow_payload_fuzz(a in 1..1000, b in 1..1000, c in 1..1000) {
            let mut nums = [a, b, c];
            nums.sort();
            let [lo, lt, hi] = nums;

            prop_assume!(lo < lt && lt < hi);

            let result =
                parasect(
                    ParasectSettings::new(r(lo, hi), |x|
                        FunctionCancellableTask::new(move || {
                        thread::sleep(Duration::from_millis(random::<u64>() % 51));
                        if x < IBig::from(lt) { Continue(Good) } else { Continue(Bad) }
                    })).with_max_parallelism(3));

            prop_assert_eq!(result, Ok(IBig::from(lt)));
        }
    }
}
