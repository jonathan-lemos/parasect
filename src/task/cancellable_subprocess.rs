use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_subprocess::SubprocessError::*;
use crate::task::cancellable_task::CancellableTask;
use shared_child::SharedChild;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SubprocessOutput {
    status: ExitStatus,
    output: String,
}

#[derive(Debug)]
#[allow(unused)]
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
    msg: Arc<CancellableMessage<Result<SubprocessOutput, SubprocessError>>>,
    thread: JoinHandle<()>,
}

#[allow(unused)]
impl CancellableSubprocess {
    pub fn new(args: &[&str]) -> Result<CancellableSubprocess, SubprocessError> {
        let child = SharedChild::spawn(
            &mut Command::new(args[0])
                .args(&args[1..])
                .stdout(Stdio::piped())
                .stderr(io::stdout()),
        )
        .map_err(ProcessSpawnError)?;

        let child = Arc::new(child);
        let msg = Arc::new(CancellableMessage::new());

        let thread = {
            let child_clone = child.clone();
            let msg_clone = msg.clone();
            thread::spawn(move || {
                let mut output = String::new();

                if let Err(e) = child_clone
                    .take_stdout()
                    .unwrap()
                    .read_to_string(&mut output)
                {
                    msg_clone.send(Err(StdoutReadError(e)));
                    let _ = child_clone.kill();
                    return;
                }

                let status = match child_clone.wait() {
                    Err(e) => {
                        msg_clone.send(Err(ProcessWaitError(e)));
                        let _ = child_clone.kill();
                        return;
                    }
                    Ok(v) => v,
                };

                msg_clone.send(Ok(SubprocessOutput { output, status }));
            })
        };

        let ret = Self { child, msg, thread };

        Ok(ret)
    }
}

impl CancellableTask<Result<SubprocessOutput, SubprocessError>> for CancellableSubprocess {
    fn join(&self) -> Option<&Result<SubprocessOutput, SubprocessError>> {
        self.msg.recv()
    }

    fn join_into(self) -> Option<Result<SubprocessOutput, SubprocessError>> {
        self.thread.join().unwrap();
        Arc::into_inner(self.msg).unwrap().recv_into()
    }

    fn request_cancellation(&self) -> () {
        self.msg.cancel();
        let _ = self.child.kill();
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_echo() {
        let sp = CancellableSubprocess::new(&["echo", "foo"]).unwrap();

        let result_arc = sp.join().unwrap();
        let output = match result_arc.as_ref() {
            Ok(thing) => thing,
            Err(e) => panic!("{:?}", e),
        };

        assert_eq!(output.output, "foo\n");
    }

    #[test]
    fn test_cancel() {
        let start = Instant::now();

        let sp = CancellableSubprocess::new(&["sleep", "5"]).unwrap();

        let result_option = thread::scope(|scope| {
            let t = scope.spawn(|| sp.join());
            sp.request_cancellation();
            t.join().unwrap()
        });

        let end = Instant::now();

        assert!(result_option.is_none());
        assert!(end - start < Duration::from_secs(2));
    }
}
