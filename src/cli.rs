use clap::{Parser, command};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use clio::Input;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    #[arg(long)]
    pub height: Option<u8>,

    #[clap(short, long, value_parser)]
    pub secrets: Option<Input>,
}
