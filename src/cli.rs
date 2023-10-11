use clap::{Parser, command};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use clio::Input;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Height to use for the binary tree.
    #[arg(long)]
    pub height: Option<u8>,

    /// TOML file containing secrets (see secrets_example.toml).
    #[clap(short, long, value_parser)]
    pub secrets: Option<Input>,
}
