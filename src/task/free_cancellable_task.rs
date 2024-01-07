use std::marker::PhantomData;
use crate::task::cancellable_task::CancellableTask;

pub struct FreeCancellableTask<'a, T, TPayload>
where TPayload: FnOnce() -> T + 'a {
    payload: TPayload,
    lifetime_phantom: PhantomData<&'a ()>
}

impl<'a, T, TPayload> CancellableTask<T, ()> for FreeCancellableTask<'a, T, TPayload>
    where TPayload: FnOnce() -> T + 'a {
    fn request_cancellation(self) -> Result<(), ()> {
        Err(())
    }

    fn join(self) -> T {
        (self.payload)()
    }
}

impl<'a, T, TPayload> FreeCancellableTask<'a, T, TPayload>
    where TPayload: FnOnce() -> T + 'a {
    pub fn new(payload: TPayload) -> Self {
        Self { payload, lifetime_phantom: PhantomData }
    }
}
