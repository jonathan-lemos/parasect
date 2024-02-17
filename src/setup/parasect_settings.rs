use crate::parasect::parasect::ParasectSettings;
use crate::range::numeric_range::NumericRange;
use crate::setup::cli_args::CliArgs;
use crate::setup::command_gen::CommandGen;
use crate::task::cancellable_subprocess::CancellableSubprocess;
use crate::ui::line::Line;

/*
pub fn run_parasect(range: NumericRange, args: Vec<String>, ss: String) -> Result<(), Line> {
    let cgen = CommandGen::new(args, ss)?;

    let settings = ParasectSettings::new(range, move |num| {
        let cmd = cgen.command_for_number(&num);

        CancellableSubprocess::new(cmd.as_slice())
    });

    todo!();
}
*/
