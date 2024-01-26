use std::{io, thread};
use std::io::{Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use shared_child::SharedChild;
use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_subprocess::SubprocessError::*;
use crate::task::cancellable_task::CancellableTask;


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

/// A subprocess that can be cancelled mid-execution.
///
/// Cancellation sends a SIGKILL
pub struct CancellableSubprocess {
    child: Arc<SharedChild>,
    msg: CancellableMessage<Result<SubprocessOutput, SubprocessError>>,
}

impl CancellableSubprocess {
    fn new(args: &[&str]) -> Result<CancellableSubprocess, SubprocessError> {
        let child = SharedChild::spawn(
            &mut Command::new(args[0])
            .args(&args[1..])
            .stdout(Stdio::piped())
            .stderr(io::stdout()))
            .map_err(ProcessSpawnError)?;

        let ret = Self {
            child: Arc::new(child),
            msg: CancellableMessage::new(),
        };

        {
            let child_clone = ret.child.clone();
            let msg_clone = ret.msg.clone();
            thread::spawn(move || {
                let mut output = String::new();

                if let Err(e) = child_clone.take_stdout().unwrap().read_to_string(&mut output) {
                    msg_clone.send(Err(StdoutReadError(e)));
                    let _ = child_clone.kill();
                    return;
                }

                let status = match child_clone.wait() {
                    Err(e) => {
                        msg_clone.send(Err(ProcessWaitError(e)));
                        let _ = child_clone.kill();
                        return;
                    },
                    Ok(v) => v
                };

                msg_clone.send(Ok(SubprocessOutput { output, status }));
            });
        }

        Ok(ret)
    }
}

impl CancellableTask<Result<SubprocessOutput, SubprocessError>> for CancellableSubprocess {
    fn request_cancellation(&self) -> () {
        self.msg.cancel();
        let _ = self.child.kill();
    }

    fn join(&self) -> Option<Arc<Result<SubprocessOutput, SubprocessError>>> {
        self.msg.recv()
    }
}
