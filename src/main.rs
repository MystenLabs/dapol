use clap::Parser;
use log::{error, info};
use logging_timer::{executing, stimer, Level};

use chrono;

use dapol::{
    activate_logging,
    cli::{AccumulatorType, Cli, Commands},
    generate_random_entities, read_write_utils,
    read_write_utils::LogOnErr,
    EntitiesParser, NdmSmt, Secrets, SecretsParser,
};

fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    // STENT TODO fix the unwraps
    // STENT TODO move all this stuff outside of main.rs, and do unit tests
    match (args.command, args.accumulator) {
        (
            Commands::New {
                height,
                secrets_file,
                entity_source,
                serialize,
            },
            AccumulatorType::NdmSmt,
        ) => {
            let secrets = if let Some(path_arg) = secrets_file {
                let path = path_arg.into_path().unwrap();
                SecretsParser::from_path(path).parse().unwrap()
            } else {
                Secrets::generate_random()
            };

            let entities = if let Some(path_arg) = entity_source.entities_file {
                let path = path_arg.into_path().unwrap();
                EntitiesParser::from_path(path).parse().unwrap()
            } else if let Some(num_leaves) = entity_source.random_entities {
                generate_random_entities(num_leaves)
            } else {
                panic!("This code should not be reachable because the cli arguments are required");
            };

            // Do path checks before building so that the build does not have to be repeated
            // for problems with file names etc.
            let serialization_path = match serialize.clone() {
                Some(path_arg) => {
                    let path = read_write_utils::parse_tree_serialization_path(
                        path_arg.into_path().unwrap(),
                    )
                    .log_on_err()
                    .unwrap();

                    Some(path)
                }
                None => None,
            };

            let ndmsmt = NdmSmt::new(secrets, height, entities).log_on_err().unwrap();

            if serialize.is_some() {
                read_write_utils::serialize_to_bin_file(
                    ndmsmt,
                    serialization_path
                        .expect("Bug in CLI parser: serialization path not set for ndmsmt"),
                )
                .log_on_err()
                .unwrap();
            }

            // let proof =
            // ndsmt.generate_inclusion_proof(&EntityId::from_str("entity1
            // ID").unwrap()).unwrap(); println!("{:?}", proof);
        }
        (Commands::FromFile { path }, AccumulatorType::NdmSmt) => {
            read_write_utils::deserialize_from_bin_file::<NdmSmt>(path.into_path().unwrap())
                .log_on_err()
                .unwrap();
        }
        _ => {
            error!("Command is not supported for the given accumulator");
        }
    }
}
