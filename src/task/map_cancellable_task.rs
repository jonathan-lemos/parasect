use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Mutex;
use crate::task::cancellable_task::CancellableTask;
use crate::task::map_cancellable_task::InnerValue::*;

enum InnerValue<T> {
    NotFinishedYet,
    Finished(T),
    Spent
}

pub struct MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld> {
    inner_task: InnerTask,
    inner_value: Mutex<Cell<InnerValue<TNew>>>,
    mapper: Mutex<Cell<Option<Mapper>>>,
    old: PhantomData<TOld>,
    new: PhantomData<TNew>,
}

impl<TOld, TNew, Mapper, InnerTask> MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld> {
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            inner_task: inner,
            inner_value: Mutex::new(Cell::new(NotFinishedYet)),
            mapper: Mutex::new(Cell::new(mapper)),
            old: PhantomData,
            new: PhantomData,
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> CancellableTask<TNew> for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld> {
    fn request_cancellation(&self) -> () {
        self.inner_task.request_cancellation()
    }

    fn join(&self) -> Option<&TNew> {
        let cell_inner = self.inner_value.lock().unwrap().get_mut();
        match cell_inner {
            NotFinishedYet => {
                let mapper = self.mapper.lock().unwrap().take().unwrap();
                *cell_inner = Finished(self.inner_task.join().map(mapper));
                if let Finished(r) = cell_inner {
                    Some(r)
                } else {
                    panic!("should never happen")
                }
            },
            Finished(v) => v,
            Spent => panic!("should never happen")
        }
    }

    fn join_into(self) -> Option<TNew> {
        let cell_inner = self.inner_value.lock().unwrap().replace(Spent);
        match cell_inner {
            NotFinishedYet => {
                let mapper = self.mapper.lock().unwrap().take().unwrap();
                self.inner_task.join_into().map(mapper)
            }
            Finished(v) => v,
            Spent => panic!("should never happen")
        }
    }
}
