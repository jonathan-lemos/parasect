use crate::parasect::types::ParasectError;
use crate::ui::line::{mkline, Line};
use crate::ui::segment::{Attributes, Color};
use std::process::ExitCode;

pub type CliResult<T> = Result<T, Vec<Line>>;

pub fn parasect_error_to_cli_error(e: ParasectError) -> Vec<Line> {
    vec![match e {
        ParasectError::PayloadError(e) => mkline!(
            ("Subprocess error", Color::Red, Attributes::Bold),
            ": ",
            (e, Color::Red)
        ),
        ParasectError::InconsistencyError(e) => mkline!(
            (
                "Inconsistent results from subprocess",
                Color::Red,
                Attributes::Bold
            ),
            ": ",
            (e, Color::Red)
        ),
    }]
}
