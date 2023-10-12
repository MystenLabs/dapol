use clap::{Parser, command};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use clio::Input;

use std::str::FromStr;

use crate::binary_tree::Height;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Height to use for the binary tree.
    #[arg(long, value_parser = Height::from_str)]
    pub height: Option<Height>,

    /// TOML file containing secrets (see secrets_example.toml).
    #[clap(short, long, value_parser)]
    pub secrets: Option<Input>,
}
