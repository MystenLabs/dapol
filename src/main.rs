use clap::Parser;
use log::error;

use dapol::{
    activate_logging,
    cli::{AccumulatorTypeCommand, Cli, Command, TreeBuildCommand},
    read_write_utils,
    read_write_utils::LogOnErr,
    EntitiesParser, NdmSmt, SecretsParser,
};

// STENT TODO fix the unwraps
// STENT TODO move all this stuff outside of main.rs, and do unit tests
fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    match args.command {
        Command::BuildTree {
            acc,
            gen_proofs,
            keep_alive,
        } => {
            match acc {
                AccumulatorTypeCommand::NdmSmt { tree_build_type } => {
                    match tree_build_type {
                        TreeBuildCommand::New {
                            height,
                            secrets_file,
                            entity_source,
                            serialize,
                        } => {
                            let secrets = SecretsParser::from_patharg(secrets_file)
                                .parse_or_generate_random()
                                .unwrap();

                            let entities =
                                EntitiesParser::from_patharg(entity_source.entities_file)
                                    .with_num_entities(entity_source.random_entities)
                                    .parse_or_generate_random()
                                    .unwrap();

                            // Do path checks before building so that the build does not have to be
                            // repeated for problems with file names etc.
                            let serialization_path = match serialize.clone() {
                                Some(path_arg) => {
                                    // STENT TODO HERE make this thing accept an InputArg
                                    let path = read_write_utils::parse_tree_serialization_path(
                                        path_arg.into_path().unwrap(),
                                    )
                                    .log_on_err()
                                    .unwrap();

                                    Some(path)
                                }
                                None => None,
                            };

                            let ndmsmt =
                                NdmSmt::new(secrets, height, entities).log_on_err().unwrap();

                            if serialize.is_some() {
                                read_write_utils::serialize_to_bin_file(
                                    ndmsmt,
                                    serialization_path.expect(
                                        "Bug in CLI parser: serialization path not set for ndmsmt",
                                    ),
                                )
                                .log_on_err()
                                .unwrap();
                            }

                            // let proof =
                            // ndsmt.generate_inclusion_proof(&
                            // EntityId::from_str("entity1
                            // ID").unwrap()).unwrap(); println!("{:?}", proof);
                        }
                        TreeBuildCommand::Deserialize { path } => {
                            read_write_utils::deserialize_from_bin_file::<NdmSmt>(
                                path.into_path().unwrap(),
                            )
                            .log_on_err()
                            .unwrap();
                        }
                    }
                }
                AccumulatorTypeCommand::FromConfig { file_path } => {
                    error!("TODO implement");
                }
            }
        }
        Command::GenProofs {} => {
            error!("TODO implement");
        }
    }
}
