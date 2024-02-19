use crate::cli::error_handling::CliResult;
use crate::command_gen::CommandGen;
use crate::range::numeric_range::NumericRange;
use crate::ui::line::mkline;
use clap::Parser;
use ibig::IBig;

/// Parasect searches the given command in parallel to find a point where it transitions from good (returning 0) to bad (returning != 0).
///
/// Example usage: parasect --low=50 --high=100 -- ./test-script.sh --revision-number='$X'
///
/// Make sure you put your command after `--` and put `$X` in single quotes.
///
/// By default, the magic string "$X" in the given command is replaced with the current number. This can be overriden with --substitution-string.
/// If your range includes negative numbers, take care to make sure that the negative numbers are not parsed as flags.
/// The command must return a non-empty sequence of 0 followed by a non-empty sequence of != 0 within [low, high]. If it doesn't, the search will fail or give erroneous results.
#[derive(Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    /// The command to parasect, along with its arguments.
    ///
    /// The magic string "$X" will be replaced with the number. To change this string, use --substitution-string=NEW_STRING
    ///
    /// This command should return 0 if good, != 0 if bad
    #[arg()]
    pub command: Vec<String>,

    /// The highest number to search, inclusive.
    ///
    /// This value, given to the command, should return != 0. It must also be greater than low.
    #[arg(short = 'y', long)]
    high: IBig,

    /// The lowest number to search, inclusive.
    ///
    /// This value, given to the command, should return 0. It must also be less than high.
    #[arg(short = 'x', long)]
    low: IBig,

    /// The maximum amount of processes to spawn at any time.
    ///
    /// Defaults to the number of logical CPU's on the machine.
    #[arg(short = 'j', long)]
    max_parallelism: Option<usize>,

    /// Pass this flag to disable the fancy TTY interface.
    ///
    /// The TTY interface will also be disabled if stdout is not a TTY.
    #[arg(short = 't', long, default_value_t = false)]
    pub no_tty: bool,

    /// The string that will be replaced with the current number in the given command's execution.
    ///
    /// By default, this is "$X".
    #[arg(short, long, default_value = "$X")]
    pub substitution_string: String,
}

impl CliArgs {
    pub fn command_gen(&self) -> CliResult<CommandGen> {
        CommandGen::new(self.command.clone(), self.substitution_string.clone())
    }

    pub fn range(&self) -> CliResult<NumericRange> {
        if self.low >= self.high {
            return Err(vec![mkline!(
                "Low must be strictly less than high (low was ",
                &self.low,
                ", which is >= the high of ",
                &self.high,
                ")"
            )]);
        }

        Ok(NumericRange::from_endpoints_inclusive(
            self.low.clone(),
            self.high.clone(),
        ))
    }

    pub fn max_parallelism(&self) -> CliResult<usize> {
        let ret = self.max_parallelism.unwrap_or(num_cpus::get());
        if ret == 0 {
            Err(vec![mkline!(
                "The max parallelism cannot be 0. Specify a value >= 1 for --max-parallelism"
            )])
        } else {
            Ok(ret)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::cli_args::CliArgs;
    use crate::test_util::test_util::test_util::ib;
    use clap::Parser;

    #[test]
    fn test_basic_parse() {
        let args =
            CliArgs::parse_from(["parasect", "--low=5", "--high=10", "--", "foo", "--bar=$X"]);

        assert_eq!(args.command, vec!["foo", "--bar=$X"]);
        assert_eq!(args.low, ib(5));
        assert_eq!(args.high, ib(10));
        assert_eq!(args.max_parallelism, None);
        assert_eq!(args.no_tty, false);
        assert_eq!(args.substitution_string, "$X");
    }

    #[test]
    fn test_notty_parse() {
        let args = CliArgs::parse_from([
            "parasect",
            "--low=5",
            "--high=10",
            "--no-tty",
            "--",
            "foo",
            "--bar=$X",
        ]);

        assert_eq!(args.command, vec!["foo", "--bar=$X"]);
        assert_eq!(args.low, ib(5));
        assert_eq!(args.high, ib(10));
        assert_eq!(args.max_parallelism, None);
        assert_eq!(args.no_tty, true);
        assert_eq!(args.substitution_string, "$X");
    }

    #[test]
    fn test_max_parallelism_parse() {
        let args = CliArgs::parse_from([
            "parasect",
            "--low=5",
            "--high=10",
            "--max-parallelism=2",
            "--",
            "foo",
            "--bar=$X",
        ]);

        assert_eq!(args.command, vec!["foo", "--bar=$X"]);
        assert_eq!(args.low, ib(5));
        assert_eq!(args.high, ib(10));
        assert_eq!(args.max_parallelism, Some(2usize));
        assert_eq!(args.no_tty, false);
        assert_eq!(args.substitution_string, "$X");
    }

    #[test]
    fn test_command_gen() {}
}
