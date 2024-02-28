#![feature(btree_cursors)] // needed for numeric_range_set

extern crate core;

use crate::cli::cli_args::CliArgs;
use crate::cli::error_handling::{parasect_error_to_cli_error, CliResult};
use crate::cli::utils::{command_line, parasect_result_to_lines};
use crate::collections::collect_collection::CollectVec;
use crate::parasect::parasect::{parasect, ParasectSettings};
use crate::parasect::types::ParasectPayloadAnswer::{Bad, Good};
use crate::parasect::types::ParasectPayloadResult::{Continue, Stop};
use crate::task::cancellable_subprocess::CancellableSubprocess;
use crate::task::cancellable_task::CancellableTask;
use crate::task::result_cancellable_task::ResultCancellableTask;
use crate::ui::line::print_lines;
use crate::ui::ui::Ui;
use clap::Parser;
use crossbeam_channel::unbounded;
use do_notation::m;
use ibig::IBig;
use std::process::ExitCode;

mod cli;
mod collections;
mod command_gen;
mod messaging;
mod parasect;
mod range;
mod task;
#[cfg(test)]
mod test_util;
mod threading;
mod ui;
mod util;

fn run_parasect(args: &CliArgs) -> CliResult<IBig> {
    let (event_sender, event_receiver) = unbounded();
    let title = command_line(&args.command, &args.substitution_string);

    m! {
        cgen <- args.command_gen();
        range <- args.range();
        max_parallelism <- args.max_parallelism();

        let _ui = Ui::start(range.clone(), title, event_receiver, args.no_tty);

        let settings = ParasectSettings::new(range, move |num| {
            let cmd = cgen.command_for_number(&num);

            let cmd_ref = cmd.iter().map(|x| x.as_str()).collect_vec();

            ResultCancellableTask::new(CancellableSubprocess::new(cmd_ref.as_slice())).map(move |r| {
                match r {
                    Ok(Ok(v)) => Continue(if v.status.success() { Good } else { Bad }),
                    Ok(Err(e)) => Stop(format!("Failed to execute {:?}: {}", cmd, e)),
                    Err(e) => Stop(format!("Failed to execute {:?}: {}", cmd, e)),
                }
            })
        }).with_max_parallelism(max_parallelism).with_event_sender(event_sender);

        parasect(settings).map_err(parasect_error_to_cli_error)
    }
}

fn execute(args: &CliArgs) -> ExitCode {
    let result = run_parasect(&args);

    let (lines, ret) = match result {
        Ok(idx) => (parasect_result_to_lines(&args, &idx), ExitCode::SUCCESS),
        Err(e) => (e, ExitCode::FAILURE),
    };

    print_lines(lines.iter());

    ret
}

fn main() -> ExitCode {
    let args = CliArgs::parse();
    execute(&args)
}
