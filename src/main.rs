use std::{io::Write, path::PathBuf};

use clap::Parser;
use log::{error, info};

use dapol::{
    cli::{AccumulatorType, BuildKindCommand, Cli, Command},
    ndm_smt,
    read_write_utils::parse_tree_serialization_path,
    utils::{activate_logging, Consume, IfNoneThen, LogOnErr},
    Accumulator, AccumulatorConfig, EntityIdsParser,
};
use patharg::InputArg;

// STENT TODO fix the unwraps
fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    match args.command {
        Command::BuildTree {
            build_kind,
            gen_proofs,
            serialize,
        } => {
            let serialization_path =
                // Do not try serialize if the command is Deserialize because
                // this means there already is a serialized file.
                if !build_kind_is_deserialize(&build_kind) {
                    // Do path checks before building so that the build does not have to be
                    // repeated for problems with file names etc.
                    match serialize {
                        Some(patharg) => {
                            let path = patharg.into_path().unwrap();
                            parse_tree_serialization_path(path).log_on_err().ok()
                        }
                        None => None,
                    }
                } else {
                    None
                };

            let accumulator: Accumulator = match build_kind {
                BuildKindCommand::New {
                    accumulator,
                    height,
                    secrets_file,
                    entity_source,
                } => match accumulator {
                    AccumulatorType::NdmSmt => {
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
                },
                BuildKindCommand::Deserialize { path } => {
                    Accumulator::deserialize(path.into_path().unwrap()).unwrap()
                }
                BuildKindCommand::ConfigFile { file_path } => {
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

fn build_kind_is_deserialize(build_kind: &BuildKindCommand) -> bool {
    let dummy = BuildKindCommand::Deserialize {
        path: InputArg::default(),
    };
    std::mem::discriminant(build_kind) == std::mem::discriminant(&dummy)
}
