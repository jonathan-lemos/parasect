use std::thread;
use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;

pub struct MapValueCancellableTask<TNew: Send + Sync> {
    msg: CancellableMessage<TNew>,
}

impl<TNew: Send + Sync> MapValueCancellableTask<TNew> {
    pub fn new<TOld, InnerTask, Mapper>(inner: InnerTask, mapper: Mapper) -> Self
        where TOld: Send + Sync,
              Mapper: FnOnce(TOld) -> TNew + Send,
              InnerTask: CancellableTask<TOld> {
        let ret = Self {
            msg: CancellableMessage::new(),
        };

        {
            let msg_ref = &ret.msg;
            thread::spawn(move || {
                match inner.join_into() {
                    Some(v) => msg_ref.send(mapper(v)),
                    None => msg_ref.cancel()
                };
            });
        }

        ret
    }
}

impl<TNew: Send + Sync> CancellableTask<TNew> for MapValueCancellableTask<TNew> {
    fn request_cancellation(&self) -> () {
        self.msg.cancel()
    }

    fn join(&self) -> Option<&TNew> {
        self.msg.recv()
    }

    fn join_into(self) -> Option<TNew> {
        self.msg.recv_into()
    }
}
