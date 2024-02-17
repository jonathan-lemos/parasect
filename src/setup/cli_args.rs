use crate::range::numeric_range::NumericRange;
use crate::ui::line::{mkline, Line};
use clap::Parser;
use ibig::IBig;

/// Parasect searches the given command in parallel to find a point where it transitions from good (returning 0) to bad (returning != 0).
///
/// By default, the magic string "$X" in the given command is replaced with the current number. This can be overriden with --substitution-string.
/// If your range includes negative numbers, take care to make sure that the negative numbers are not parsed as flags.
/// The command must return a non-empty sequence of 0 followed by a non-empty sequence of != 0 within [low, high]. If it doesn't, the search will fail or give erroneous results.
#[derive(Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    /// If given, defines returning 0 as bad, and returning != 0 as good.
    #[arg(short, long)]
    pub bad_is_zero: bool,

    /// The command to parasect, along with its arguments.
    ///
    /// The magic string "$X" will be replaced with the number. To change this string, use --substitution-string=NEW_STRING
    ///
    /// This command should return 0 if good, != 0 if bad. To flip this behavior, pass --bad-is-zero.
    #[arg()]
    pub command: Vec<String>,

    /// The highest number to search, inclusive.
    ///
    /// This value, given to the command, should return != 0, unless --bad-first is given, in which case it should return 0. It must also be greater than low.
    #[arg(short, long)]
    pub high: IBig,

    /// The lowest number to search, inclusive.
    ///
    /// This value, given to the command, should return 0, unless --bad-first is given, in which case it should return != 0. It must also be less than high.
    #[arg(short, long)]
    pub low: IBig,

    /// The maximum amount of processes to spawn at any time.
    ///
    /// Defaults to the number of logical CPU's on the machine.
    #[arg(short = 'j', long)]
    pub max_parallelism: Option<usize>,

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
    pub fn parasect_range(&self) -> Result<NumericRange, Line> {
        if self.low >= self.high {
            return Err(mkline!(
                "Low must be strictly less than high (low was ",
                &self.low,
                ", which is >= the high of ",
                &self.high,
                ")"
            ));
        }

        Ok(NumericRange::from_endpoints_inclusive(
            self.low.clone(),
            self.high.clone(),
        ))
    }
}
