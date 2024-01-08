use std::{io, thread};
use std::cell::Cell;
use std::io::{Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, RwLock};
use crossbeam_channel::{bounded, Receiver, Sender};
use crate::task::cancellable_subprocess::InnerValue::*;
use crate::task::cancellable_subprocess::SubprocessError::*;
use crate::task::cancellable_task::CancellableTask;

enum InnerValue {
    NotFinished,
    Finished(Option<Result<SubprocessOutput, SubprocessError>>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SubprocessOutput {
    status: ExitStatus,
    output: String,
}

#[derive(Debug)]
pub enum SubprocessError {
    ProcessSpawnError(io::Error),
    StdoutReadError(io::Error),
    ProcessKillError(io::Error),
    ProcessWaitError(io::Error),
}

pub struct CancellableSubprocess {
    child: Mutex<Child>,
    stdout_sender: Sender<Option<Result<SubprocessOutput, SubprocessError>>>,
    stdout_receiver: Receiver<Option<Result<SubprocessOutput, SubprocessError>>>,
    stdout: RwLock<Cell<InnerValue>>,
}

impl CancellableSubprocess {
    fn new(args: &[&str]) -> Result<CancellableSubprocess, SubprocessError> {
        let mut child = Command::new(args[0])
            .args(&args[1..])
            .stdout(Stdio::piped())
            .stderr(io::stdout())
            .spawn()
            .map_err(ProcessSpawnError)?;

        let (sender, receiver) =
            bounded::<Option<Result<SubprocessOutput, SubprocessError>>>(1);

        let mut ret = CancellableSubprocess { child: Mutex::new(child), read_stdout_thread: None };
        ret.read_stdout_thread = Some(
            thread::spawn(|| {
                let mut s = String::new();
                let mut stdout = {
                    ret.child.lock().unwrap().stdout.unwrap()
                };
                stdout.read_to_string(&mut s)?;
                Ok(s)
            })
        );

        Ok(ret)
    }
}

impl CancellableTask<Result<SubprocessOutput, SubprocessError>> for CancellableSubprocess {
    fn request_cancellation(&self) -> () {
        let _ = self.child.lock().unwrap().kill();
        let _ = self.stdout_sender.try_send(None);
    }

    fn join(&self) -> Option<&Result<SubprocessOutput, SubprocessError>> {
        {
            let stdout_ptr = self.stdout.read().unwrap().get_mut();
            if let Some(Finished(v)) = stdout_ptr {
                return v.map(|x| &x);
            }
        }
        {
            let stdout_ptr = self.stdout.write().unwrap().get_mut();
            if let Some(Finished(v)) = stdout_ptr {
                return v.map(|x| &x);
            }
            let result = self.stdout_receiver.recv().unwrap();
            *stdout_ptr = Finished(result);
            stdout_ptr
        }


        let stdout = self.read_stdout_thread.unwrap().join().unwrap().map_err(StdoutReadError)?;
        let status = self.child.wait().map_err(ProcessWaitError)?;

        Some(Ok(SubprocessOutput { output: stdout, status }))
    }

    fn join_into(self) -> Option<Result<SubprocessOutput, SubprocessError>> {
        todo!()
    }
}