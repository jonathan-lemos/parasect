use std::{io, thread};
use std::io::{Read};
use std::thread::JoinHandle;
use std::process::{Child, Command, ExitStatus, Stdio};
use crate::task::cancellable_subprocess::SubprocessError::*;
use crate::task::cancellable_task::CancellableTask;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SubprocessOutput {
    status: ExitStatus,
    output: String
}

#[derive(Debug)]
pub enum SubprocessError {
    ProcessSpawnError(io::Error),
    StdoutReadError(io::Error),
    ProcessKillError(io::Error),
    ProcessWaitError(io::Error),
}

pub struct CancellableSubprocess {
    child: Child,
    read_stdout_thread: Option<JoinHandle<Result<String, io::Error>>>
}

impl CancellableSubprocess {
    fn new(args: &[&str]) -> Result<CancellableSubprocess, SubprocessError> {
        let mut child = Command::new(args[0])
            .args(&args[1..])
            .stdout(Stdio::piped())
            .stderr(io::stdout())
            .spawn()
            .map_err(ProcessSpawnError)?;

        let mut ret = CancellableSubprocess { child, read_stdout_thread: None };
        ret.read_stdout_thread = Some(
            thread::spawn(|| {
                let mut s = String::new();
                ret.child.stdout.unwrap().read_to_string(&mut s)?;
                Ok(s)
            })
        );

        Ok(ret)
    }
}

impl CancellableTask<Result<SubprocessOutput, SubprocessError>, SubprocessError> for CancellableSubprocess {
    fn request_cancellation(mut self) -> Result<(), SubprocessError> {
        self.child.kill().map(|_| ()).map_err(StdoutReadError)
    }

    fn join(mut self) -> Result<SubprocessOutput, SubprocessError> {
        let stdout = self.read_stdout_thread.unwrap().join().unwrap().map_err(StdoutReadError)?;
        let status = self.child.wait().map_err(ProcessWaitError)?;

        Ok(SubprocessOutput { output: stdout, status })
    }
}