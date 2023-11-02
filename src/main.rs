use std::{io::Write, path::PathBuf};

use clap::Parser;
use log::debug;

use dapol::{
    cli::{AccumulatorType, BuildKindCommand, Cli, Command},
    ndm_smt,
    read_write_utils::{self, parse_tree_serialization_path},
    utils::{activate_logging, Consume, IfNoneThen, LogOnErr},
    Accumulator, AccumulatorConfig, AggregationFactor, EntityIdsParser, InclusionProof,
};
use patharg::InputArg;

fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    // STENT TODO get rid of the unwraps, we need to print out the full errors
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
                            let path = patharg.into_path().expect("Expected a file path, not stdout");
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
                BuildKindCommand::Deserialize { path } => Accumulator::deserialize(
                    path.into_path().expect("Expected file path, not stdout"),
                )
                .unwrap(),
                BuildKindCommand::ConfigFile { file_path } => AccumulatorConfig::deserialize(
                    file_path
                        .into_path()
                        .expect("Expected file path, not stdin"),
                )
                .unwrap()
                .parse()
                .unwrap(),
            };

            serialization_path
                .if_none_then(|| {
                    debug!("No serialization path set, skipping serialization of the tree");
                })
                .consume(|path| accumulator.serialize(path).unwrap());

            if let Some(patharg) = gen_proofs {
                let entity_ids = EntityIdsParser::from_path(
                    patharg.into_path().expect("Expected file path, not stdin"),
                )
                .parse()
                .unwrap();

                let dir = PathBuf::from("./inclusion_proofs/");
                std::fs::create_dir(dir.as_path()).unwrap();

                for entity_id in entity_ids {
                    let proof = accumulator.generate_inclusion_proof(&entity_id).unwrap();
                    proof.serialize(&entity_id, dir.clone());
                }
            }
        }
        Command::GenProofs {
            entity_ids,
            tree_file,
            range_proof_aggregation,
            upper_bound_bit_length,
        } => {
            let accumulator = Accumulator::deserialize(
                tree_file
                    .into_path()
                    .expect("Expected file path, not stdout"),
            )
            .unwrap();

            // TODO for entity IDs: accept either path or stdin
            let entity_ids = EntityIdsParser::from_path(
                entity_ids
                    .into_path()
                    .expect("Expected file path, not stdin"),
            )
            .parse()
            .unwrap();

            let dir = PathBuf::from("./inclusion_proofs/");
            std::fs::create_dir(dir.as_path()).unwrap();

            let aggregation_factor = AggregationFactor::Percent(range_proof_aggregation);

            for entity_id in entity_ids {
                let proof = accumulator
                    .generate_inclusion_proof_with(
                        &entity_id,
                        aggregation_factor.clone(),
                        upper_bound_bit_length,
                    )
                    .unwrap();
                proof.serialize(&entity_id, dir.clone());
            }
        }
        Command::VerifyProof {
            file_path,
            root_hash,
        } => {
            let proof: InclusionProof = read_write_utils::deserialize_from_bin_file(
                file_path
                    .into_path()
                    .expect("Expected file path, not stdin"),
            )
            .unwrap();
            proof.verify(root_hash);
        }
    }
}

fn build_kind_is_deserialize(build_kind: &BuildKindCommand) -> bool {
    let dummy = BuildKindCommand::Deserialize {
        path: InputArg::default(),
    };
    std::mem::discriminant(build_kind) == std::mem::discriminant(&dummy)
}
