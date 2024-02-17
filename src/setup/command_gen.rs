use crate::collections::collect_collection::CollectVec;
use crate::ui::line::{mkline, Line};
use crate::ui::segment::{Attributes, Color};
use ibig::IBig;

pub struct CommandGen {
    args: Vec<String>,
    substitution_string: String,
}

impl CommandGen {
    pub fn new<I: IntoIterator<Item = String>>(
        args: I,
        substitution_string: String,
    ) -> Result<Self, Line> {
        let args = args.into_iter().collect_vec();

        if !args.iter().any(|a| a.contains(&substitution_string)) {
            return Err(mkline!(
                "The given command does not contain the substitution string ",
                (substitution_string, Color::Green, Attributes::Bold)
            ));
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
