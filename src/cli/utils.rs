use crate::cli::cli_args::CliArgs;
use crate::collections::collect_collection::CollectVec;
use crate::ui::line::{mkline, Line};
use crate::ui::segment::{Attributes, Color, Segment};
use ibig::IBig;

pub fn command_line_unhighlighted(cmd: &Vec<String>) -> Line {
    let mut v = cmd
        .into_iter()
        .flat_map(|s| [Segment::from(s), Segment::from(" ")])
        .collect_vec();

    v.pop();

    return Line::from_iter(v);
}

pub fn command_line(cmd: &Vec<String>, substitution_string: &str) -> Line {
    if substitution_string.is_empty() {
        return command_line_unhighlighted(cmd);
    }

    let insertion_segment = Segment::new(
        substitution_string.to_string(),
        Color::Blue,
        Attributes::Bold,
    );

    let mut segs = cmd
        .iter()
        .flat_map(|s| {
            let mut v = s
                .split(substitution_string)
                .flat_map(|s2| [Segment::from(s2), insertion_segment.clone()])
                .collect_vec();

            v.pop();
            v.push(Segment::from(" "));
            v
        })
        .collect_vec();

    segs.pop();

    Line::from_iter(segs)
}

pub fn parasect_result_to_lines(args: &CliArgs, last_bad: &IBig) -> Vec<Line> {
    vec![
        Line::join([
            mkline!(("Successfully parasected", Color::Green), " "),
            command_line(&args.command, &args.substitution_string),
        ]),
        mkline!(
            "First bad index: ",
            (last_bad, Color::Blue, Attributes::Bold)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::cli::cli_args::CliArgs;
    use crate::cli::utils::{command_line, parasect_result_to_lines};
    use crate::test_util::test_util::test_util::ib;
    use crate::ui::line::{mkline, Line};
    use crate::ui::segment::{Attributes, Color};
    use clap::Parser;

    #[test]
    fn test_command_line_basic() {
        let cmd = vec![
            "thing".to_string(),
            "--flag=$X".to_string(),
            "--other-flag=z".to_string(),
        ];

        assert_eq!(
            command_line(&cmd, "$X"),
            mkline!(
                "thing --flag=",
                ("$X", Color::Blue, Attributes::Bold),
                " --other-flag=z"
            )
        )
    }

    #[test]
    fn test_command_line_empty() {
        let cmd = Vec::new();

        assert_eq!(command_line(&cmd, "$X"), Line::empty())
    }

    #[test]
    fn test_command_line_empty_ss() {
        let cmd = vec![
            "thing".to_string(),
            "--flag=$X".to_string(),
            "--other-flag=z".to_string(),
        ];

        assert_eq!(
            command_line(&cmd, ""),
            mkline!("thing --flag=$X --other-flag=z")
        )
    }

    #[test]
    fn test_parasect_result_to_lines_success() {
        let args =
            CliArgs::parse_from(["parasect", "--low=5", "--high=10", "--", "foo", "--num=$X"]);

        assert_eq!(
            parasect_result_to_lines(&args, &ib(7)),
            vec![
                mkline!(
                    ("Successfully parasected", Color::Green),
                    " foo --num=",
                    ("$X", Color::Blue, Attributes::Bold)
                ),
                mkline!("First bad index: ", (7, Color::Blue, Attributes::Bold))
            ]
        );
    }

    #[test]
    fn test_parasect_result_to_lines_failure() {
        let args =
            CliArgs::parse_from(["parasect", "--low=5", "--high=10", "--", "foo", "--num=$X"]);

        assert_eq!(
            parasect_result_to_lines(&args, &ib(7)),
            vec![
                mkline!(
                    ("Successfully parasected", Color::Green),
                    " foo --num=",
                    ("$X", Color::Blue, Attributes::Bold)
                ),
                mkline!("First bad index: ", (7, Color::Blue, Attributes::Bold))
            ]
        );
    }
}
