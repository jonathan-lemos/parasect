use crate::cli::error_handling::CliResult;
use crate::cli::utils::command_line;
use crate::collections::collect_collection::CollectVec;
use crate::ui::line::{mkline, Line};
use crate::ui::segment::{Attributes, Color};
use ibig::IBig;

#[derive(Debug)]
pub struct CommandGen {
    args: Vec<String>,
    substitution_string: String,
}

impl CommandGen {
    pub fn new<I: IntoIterator<Item = String>>(
        args: I,
        substitution_string: String,
    ) -> CliResult<Self> {
        if substitution_string.is_empty() {
            return Err(vec![mkline!("The substitution string cannot be empty.")]);
        }

        let args = args.into_iter().collect_vec();

        if args.is_empty() {
            return Err(vec![mkline!("The command cannot be empty.")]);
        }

        if !args.iter().any(|a| a.contains(&substitution_string)) {
            return Err(vec![
                mkline!(
                    "The given command does not contain the substitution string ",
                    (&substitution_string, Color::Blue, Attributes::Bold)
                ),
                Line::join([
                    mkline!("Command: "),
                    command_line(&args, substitution_string.as_str()),
                ]),
            ]);
        }

        Ok(Self {
            args,
            substitution_string,
        })
    }

    pub fn command_for_number(&self, num: &IBig) -> Vec<String> {
        let num_string = num.to_string();
        self.args
            .iter()
            .map(|x| x.clone().replace(&self.substitution_string, &num_string))
            .collect_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_util::test_util::ib;

    #[test]
    fn test_cmdgen_basic() {
        let cmdgen = CommandGen::new(
            vec!["thing", "--flag=x", "--other-flag=$X", "--third-flag=z"]
                .into_iter()
                .map(|x| x.to_string())
                .collect_vec(),
            "$X".to_string(),
        )
        .unwrap();

        assert_eq!(
            cmdgen.command_for_number(&ib(69)),
            vec!["thing", "--flag=x", "--other-flag=69", "--third-flag=z"]
        )
    }

    #[test]
    fn test_cmdgen_multiple() {
        let cmdgen = CommandGen::new(
            vec!["thing", "--flag=$X", "--other-flag=$X", "--third-flag=z"]
                .into_iter()
                .map(|x| x.to_string())
                .collect_vec(),
            "$X".to_string(),
        )
        .unwrap();

        assert_eq!(
            cmdgen.command_for_number(&ib(69)),
            vec!["thing", "--flag=69", "--other-flag=69", "--third-flag=z"]
        )
    }

    #[test]
    fn test_cmdgen_positional() {
        let cmdgen = CommandGen::new(
            vec!["thing", "$X", "--other-flag=$X", "--third-flag=z"]
                .into_iter()
                .map(|x| x.to_string())
                .collect_vec(),
            "$X".to_string(),
        )
        .unwrap();

        assert_eq!(
            cmdgen.command_for_number(&ib(69)),
            vec!["thing", "69", "--other-flag=69", "--third-flag=z"]
        )
    }

    #[test]
    fn test_cmdgen_fails_with_no_ss_matches() {
        assert_eq!(
            CommandGen::new(
                vec!["thing", "--third-flag=z"]
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect_vec(),
                "$X".to_string(),
            )
            .unwrap_err(),
            vec![
                mkline!(
                    "The given command does not contain the substitution string ",
                    ("$X", Color::Blue, Attributes::Bold)
                ),
                mkline!("Command: thing --third-flag=z"),
            ]
        )
    }

    #[test]
    fn test_cmdgen_fails_with_blank_ss() {
        assert_eq!(
            CommandGen::new(
                vec!["thing", "--third-flag=$X"]
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect_vec(),
                "".to_string(),
            )
            .unwrap_err(),
            vec![mkline!("The substitution string cannot be empty.")]
        )
    }

    #[test]
    fn test_cmdgen_fails_with_blank_command() {
        assert_eq!(
            CommandGen::new(Vec::new(), "$X".to_string(),).unwrap_err(),
            vec![mkline!("The command cannot be empty.")]
        )
    }
}
