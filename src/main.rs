use std::io::Write;

use clap::Parser;
use log::{error, info};

use dapol::{
    cli::{AccumulatorTypeCommand, Cli, Command, TreeBuildCommand},
    ndm_smt,
    read_write_utils::parse_tree_serialization_path,
    utils::{activate_logging, Consume, IfNoneThen, LogOnErr},
    Accumulator, AccumulatorConfig, EntityIdsParser,
};

// STENT TODO is this fine? surely we can do better
const TREE_SERIALIZATION_FILE_PREFIX: &str = "accumulator_";

// STENT TODO fix the unwraps
fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    match args.command {
        Command::BuildTree {
            accumulator_type,
            gen_proofs,
            serialize,
        } => {
            // Do path checks before building so that the build does not have to be
            // repeated for problems with file names etc.
            let serialization_path = match serialize {
                Some(patharg) => {
                    let path = patharg.into_path().unwrap();
                    parse_tree_serialization_path(path, TREE_SERIALIZATION_FILE_PREFIX)
                        .log_on_err()
                        .ok()
                }
                None => None,
            };

            // TODO the type here will need to change once other accumulators
            // are supported.
            let accumulator: Accumulator = match accumulator_type {
                AccumulatorTypeCommand::NdmSmt { tree_build_type } => match tree_build_type {
                    TreeBuildCommand::New {
                        height,
                        secrets_file,
                        entity_source,
                    } => {
                        let ndm_smt = ndm_smt::NdmSmtConfigBuilder::default()
                            .height(height)
                            .secrets_file_path_opt(secrets_file.and_then(|arg| arg.into_path()))
                            .entities_path_opt(
                                entity_source.entities_file.and_then(|arg| arg.into_path()),
                            )
                            .num_entities_opt(entity_source.random_entities)
                            .build()
                            .unwrap()
                            .parse()
                            .unwrap();

                        Accumulator::NdmSmt(ndm_smt)
                    }
                    TreeBuildCommand::Deserialize { path } => {
                        Accumulator::deserialize(path.into_path().unwrap()).unwrap()
                    }
                },
                AccumulatorTypeCommand::FromConfig { file_path } => {
                    AccumulatorConfig::deserialize(file_path.into_path().unwrap())
                        .unwrap()
                        .parse()
                        .unwrap()
                }
            };

            serialization_path
                .if_none_then(|| {
                    info!("No serialization path set, skipping serialization of the tree");
                })
                .consume(|path| accumulator.serialize(path).unwrap());

            // if let Some(patharg) = gen_proofs {
            //     let entity_ids = EntityIdsParser::from_path(patharg.into_path())
            //         .parse()
            //         .unwrap();

            //     let proof = accumulator
            //         .generate_inclusion_proof(entity_ids.first().unwrap())
            //         .unwrap();

            //     let a = serde_json::to_string(&proof).unwrap();

            //     let path = "test_proof.json";
            //     let mut file = std::fs::File::create(path).unwrap();
            //     file.write_all(a.as_bytes());
            // }
        }
        Command::GenProofs {} => {
            error!("TODO implement");
        }
    }
}
