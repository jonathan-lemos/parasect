use std::thread;
use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;

pub struct FreeCancellableTask<T, TPayload> {
    msg: CancellableMessage<T>
}

impl<T, TPayload> CancellableTask<T> for FreeCancellableTask<T, TPayload>
    where TPayload: FnOnce() -> T {
    fn request_cancellation(&self) {
        self.msg.cancel()
    }

    fn join(&self) -> Option<&T> {
        self.msg.recv()
    }

    fn join_into(self) -> Option<T> {
        self.msg.recv_into()
    }
}

impl<T, TPayload> FreeCancellableTask<T, TPayload> {
    pub fn new(payload: TPayload) -> Self
        where TPayload: FnOnce() -> T {

        let ret = Self {
            msg: CancellableMessage::new()
        };

        {
            let msg_ref = &ret.msg;
            thread::spawn(move || {
                msg_ref.send(payload())
            })
        }

        ret
    }
}
