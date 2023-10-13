use clap::{command, Args, Parser};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use clio::Input;
use patharg::InputArg;

use std::str::FromStr;

use crate::binary_tree::Height;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub entity_source: EntitySource,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Height to use for the binary tree.
    #[arg(long, value_parser = Height::from_str)]
    pub height: Option<Height>,

    /// TOML file containing secrets (see secrets_example.toml).
    #[clap(short, long, value_parser)]
    pub secrets: Option<Input>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct EntitySource {
    // TODO say where one can find supported file formats
    // TODO also say it supports stdin
    /// Path to file containing entity ID & liability entries (supported file
    /// types: csv).
    #[arg(short, long)]
    pub entity_file: Option<InputArg>,

    /// Randomly generate a number of entities.
    #[arg(short, long)]
    pub random_entities: Option<u64>,
}
