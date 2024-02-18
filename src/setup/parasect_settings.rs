use crate::collections::collect_collection::CollectVec;
use crate::parasect::parasect::ParasectSettings;
use crate::parasect::types::ParasectPayloadResult::Stop;
use crate::range::numeric_range::NumericRange;
use crate::setup::command_gen::CommandGen;
use crate::task::cancellable_subprocess::CancellableSubprocess;
use crate::task::cancellable_task::CancellableTask;
use crate::task::result_cancellable_task::ResultCancellableTask;
use crate::ui::line::Line;

pub fn run_parasect(range: NumericRange, args: Vec<String>, ss: String) -> Result<(), Line> {
    let cgen = CommandGen::new(args, ss)?;

    /*
    let settings = ParasectSettings::new(range, move |num| {
        let cmd = cgen.command_for_number(&num);

        let cmd_ref = cmd.iter().map(|x| x.as_str()).collect_vec();

        ResultCancellableTask::new(CancellableSubprocess::new(cmd_ref.as_slice())).map(
            |r| match r {
                Ok(Ok(v)) => v,
                Err(e) => Stop(format!("Failed to execute {:?}: {}", cmd_ref, e)),
            },
        )
    });
    */

    todo!();
}
