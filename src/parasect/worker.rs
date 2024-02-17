use crate::parasect::types::ParasectPayloadResult;
use crate::parasect::worker::PointCompletionMessageType::*;
use crate::range::bisecting_range_queue::BisectingRangeQueue;
use crate::range::numeric_range::NumericRange;
use crate::task::cancellable_task::CancellableTask;
use crate::threading::background_loop::BackgroundLoopBehavior::{Cancel, DontCancel};
use crate::threading::background_loop::ScopedBackgroundLoop;
use crossbeam_channel::{bounded, Receiver, Sender};
use ibig::IBig;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
pub enum PointCompletionMessageType {
    Started,
    Completed(ParasectPayloadResult),
    Cancelled,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
pub struct WorkerMessage {
    pub thread_id: usize,
    pub point: IBig,
    pub left: NumericRange,
    pub right: NumericRange,
    pub msg_type: PointCompletionMessageType,
}

pub struct Worker<TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: Fn(IBig) -> TTask + Sync,
{
    id: usize,
    queue: Arc<BisectingRangeQueue>,
    cancel_sender: Sender<NumericRange>,
    cancel_receiver: Receiver<NumericRange>,
    worker_message_sender: Sender<WorkerMessage>,
    payload: FPayload,
}

impl<TTask, FPayload> Worker<TTask, FPayload>
where
    TTask: CancellableTask<ParasectPayloadResult> + Send,
    FPayload: Fn(IBig) -> TTask + Sync,
{
    pub fn new(
        id: usize,
        queue: Arc<BisectingRangeQueue>,
        worker_message_sender: Sender<WorkerMessage>,
        payload: FPayload,
    ) -> Self {
        let (cancel_sender, cancel_receiver) = bounded(1);

        Self {
            id,
            queue,
            cancel_sender,
            cancel_receiver,
            worker_message_sender,
            payload,
        }
    }

    fn result_to_msg(
        &self,
        midpoint: IBig,
        left: NumericRange,
        right: NumericRange,
        result: Option<&ParasectPayloadResult>,
    ) -> WorkerMessage {
        let msg_type = match result {
            None => Cancelled,
            Some(a) => Completed(a.clone()),
        };

        WorkerMessage {
            thread_id: self.id,
            point: midpoint,
            left,
            right,
            msg_type,
        }
    }

    pub fn process_while_remaining(&self) {
        while let Some((midpoint, left, right)) = self.queue.dequeue() {
            self.worker_message_sender
                .send(WorkerMessage {
                    thread_id: self.id,
                    point: midpoint.clone(),
                    left: left.clone(),
                    right: right.clone(),
                    msg_type: Started,
                })
                .expect("worker_message_sender closed unexpectedly.");

            let task = (self.payload)(midpoint.clone());

            let v = thread::scope(|scope| {
                let cancel_receiver_loop =
                    ScopedBackgroundLoop::spawn(scope, self.cancel_receiver.clone(), |range| {
                        if range.contains(midpoint.clone()) {
                            task.request_cancellation();
                            Cancel
                        } else {
                            DontCancel
                        }
                    });

                let ret = task.join();
                cancel_receiver_loop.cancel();

                ret
            });

            self.worker_message_sender
                .send(self.result_to_msg(midpoint, left, right, v))
                .expect("worker_message_sender should not be disconnected");
        }
    }

    pub fn skip_if_in_range(&self, range: &NumericRange) {
        let _ = self.cancel_sender.try_send(range.clone());
    }
}
