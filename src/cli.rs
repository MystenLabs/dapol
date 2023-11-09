//! Command Line Interface.
//!
//! See [LONG_ABOUT] for more information.

use clap::{command, Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use patharg::{InputArg, OutputArg};
use primitive_types::H256;

use std::str::FromStr;

use crate::{
    binary_tree::Height,
    inclusion_proof::DEFAULT_UPPER_BOUND_BIT_LENGTH,
    percentage::{Percentage, ONE_HUNDRED_PERCENT},
};

// -------------------------------------------------------------------------------------------------
// Main structs.

// TODO we want a keep-running flag after new or from-file, for doing
// proofs

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = MAIN_LONG_ABOUT)]
pub struct Cli {
    /// Initial command for the program.
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Construct a tree from the given parameters.
    ///
    /// There are 3 different ways to build a tree:
    /// - new, using CLI options for configuration
    /// - new, using a file for configuration
    /// - existing, deserializing from a .dapoltree file
    ///
    /// Inclusion proofs can be generated, but configuration is not supported.
    /// If you want more config options then use the `gen-proofs` command.
    BuildTree {
        /// Choose the accumulator type for the tree.
        #[command(subcommand)]
        build_kind: BuildKindCommand,

        #[arg(short, long, value_name = "ENTITY_IDS_FILE_PATH", global = true, long_help = GEN_PROOFS_HELP)]
        gen_proofs: Option<InputArg>,

        #[arg(short = 'S', long, value_name = "FILE_PATH", global = true, long_help = SERIALIZE_HELP)]
        serialize: Option<OutputArg>,
    },

    /// Generate inclusion proofs for entities.
    ///
    /// The entity IDs file is expected to be a list of entity IDs, each on a
    /// new line. All file formats are accepted. It is also possible to use
    /// the same entity IDs & liabilities file that is accepted by the
    /// `entity-source` option in the `build-tree new` command.
    ///
    /// A tree is required to generate proofs. The only option supported in
    /// in terms of tree input/construction is deserialization of an
    /// already-built tree. More options for building trees can be found in
    /// the `build-tree` command.
    GenProofs {
        /// List of entity IDs to generate proofs for, can be a file path or
        /// simply a comma separated list read from stdin (use "-" to
        /// indicate stdin).
        #[arg(short, long)]
        entity_ids: InputArg,

        /// Path to the tree file that will be deserialized.
        #[arg(short, long, value_name = "FILE_PATH")]
        tree_file: InputArg,

        /// Percentage of the range proofs that
        /// are aggregated using the Bulletproofs protocol.
        #[arg(short, long, value_parser = Percentage::from_str, default_value = ONE_HUNDRED_PERCENT, value_name = "PERCENTAGE")]
        range_proof_aggregation: Percentage,

        /// Upper bound for the range proofs is 2^(this_number).
        #[arg(short, long, default_value_t = DEFAULT_UPPER_BOUND_BIT_LENGTH, value_name = "U8_INT")]
        upper_bound_bit_length: u8,
    },

    /// Verify an inclusion proof.
    ///
    /// The root hash of the tree is logged out on tree creation (an info-level log).
    VerifyProof {
        /// File path for the serialized inclusion proof json file.
        #[arg(short, long)]
        file_path: InputArg,

        /// Hash digest/bytes for the root node of the tree.
        #[arg(short, long, value_parser = H256::from_str, value_name = "BYTES")]
        root_hash: H256,
    },
}

#[derive(Debug, Subcommand)]
pub enum BuildKindCommand {
    /// Create a new tree using CLI options.
    ///
    /// The options available are similar to those
    /// supported by the configuration file format which can be found in the
    ///`build-tree config-file` command.";
    New {
        /// Choose an accumulator type for the tree.
        #[arg(short, long, value_enum)]
        accumulator: AccumulatorType,

        /// Height to use for the binary tree.
        #[arg(long, value_parser = Height::from_str, default_value = Height::default(), value_name = "U8_INT")]
        height: Height,

        #[arg(short, long, value_name = "FILE_PATH", long_help = NDM_SMT_SECRETS_HELP)]
        secrets_file: Option<InputArg>,

        #[command(flatten)]
        entity_source: EntitySource,
    },

    #[command(about = COMMAND_CONFIG_FILE_ABOUT, long_about = COMMAND_CONFIG_FILE_LONG_ABOUT)]
    ConfigFile {
        /// Path to the config file (supported file formats: TOML)
        file_path: InputArg,
    },

    /// Deserialize a tree from a .dapoltree file.
    Deserialize { path: InputArg },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum AccumulatorType {
    NdmSmt,
    // TODO other accumulators..
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct EntitySource {
    #[arg(short, long, value_name = "FILE_PATH", long_help = ENTITIES_FILE_HELP)]
    pub entities_file: Option<InputArg>,

    /// Randomly generate a number of entities.
    #[arg(short, long, value_name = "NUM_ENTITIES")]
    pub random_entities: Option<u64>,
}

// -------------------------------------------------------------------------------------------------
// Long help texts.

const MAIN_LONG_ABOUT: &str = "
DAPOL+ Proof of Liabilities protocol in Rust.

**NOTE** This project is currently still a work in progress, but is ready for
use as is. The code has _not_ been audited yet (as of Nov 2023).

DAPOL+ paper: https://eprint.iacr.org/2021/1350

Top-level doc for the project: https://hackmd.io/p0dy3R0RS5qpm3sX-_zreA

Source code: https://github.com/silversixpence-crypto/dapol/";

const GEN_PROOFS_HELP: &str = "
Generate inclusion proofs for the provided entity IDs, after building the tree.
The entity IDs file is expected to be a list of entity IDs, each on a new line.
All file formats are accepted. It is also possible to use the same entity IDs &
liabilities file that is accepted by the `entity-source` option in the
`build-tree new` command.

Custom configuration of the proofs is not supported here. The `gen-proofs`
command offers more options.";

const SERIALIZE_HELP: &str = "
Serialize the tree to a file. If the path given is a directory then a default
file name will be given. If the path given is a file then that file will be
overwritten (if it exists) or created (if it does not exist). The file
extension must be `.dapoltree`. The serialization option is ignored if
`build-tree deserialize` command is used.";

const NDM_SMT_SECRETS_HELP: &str = "
TOML file containing secrets. The file format is as follows:
```
master_secret = \"master_secret\"
salt_b = \"salt_b\"
salt_s = \"salt_s\"
```
All secrets should have at least 128-bit security, but need not be chosen from a
uniform distribution as they are passed through a key derivation function before
being used.";

const ENTITIES_FILE_HELP: &str = "
Path to file containing entity ID & liability entries (supported file
types: CSV).

CSV file format:
entity_id,liability";

const COMMAND_CONFIG_FILE_ABOUT: &str =
    "Read accumulator type and other tree configuration from a file. Supported file formats: TOML.";

const COMMAND_CONFIG_FILE_LONG_ABOUT: &str = "
Read accumulator type and other tree configuration from a file.
Supported file formats: TOML.

Config file format (TOML):
```
# Accumulator type of the tree.
# This value determines what other values are required.
accumulator_type = \"ndm-smt\"

# Height of the tree.
# If the height is not set the default height will be used.
height = 16

# Path to the secrets file.
# If not present the secrets will be generated randomly.
secrets_file_path = \"./resources/secrets_example.toml\"

# Can be a file or directory (default file name given in this case)
# If not present then no serialization is done.
serialization_path = \"./tree.dapoltree\"

# At least one of file_path & generate_random must be present.
# If both are given then file_path is prioritized.
[entities]

# Path to a file containing a list of entity IDs and their liabilities.
file_path = \"./resources/entities_example.csv\"

# Generate the given number of entities, with random IDs & liabilities.
generate_random = 4
```";
