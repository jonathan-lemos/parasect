use crate::parasect::types::ParasectError;
use crate::ui::line::{mkline, Line};
use crate::ui::segment::{Attributes, Color};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parasect_error_to_cli_error_payload_error() {
        let err = ParasectError::PayloadError("nope".into());

        assert_eq!(
            parasect_error_to_cli_error(err),
            vec![mkline!(
                ("Subprocess error", Color::Red, Attributes::Bold),
                ": ",
                ("nope", Color::Red)
            )]
        )
    }

    #[test]
    fn test_parasect_error_to_cli_error_inconsistency_error() {
        let err = ParasectError::InconsistencyError("nope".into());

        assert_eq!(
            parasect_error_to_cli_error(err),
            vec![mkline!(
                (
                    "Inconsistent results from subprocess",
                    Color::Red,
                    Attributes::Bold
                ),
                ": ",
                ("nope", Color::Red)
            )]
        )
    }
}
