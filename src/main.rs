use std::io::Write;

use clap::Parser;
use log::error;

use dapol::{
    activate_logging,
    cli::{AccumulatorTypeCommand, Cli, Command, TreeBuildCommand},
    ndm_smt, AccumulatorParser, EntityIdsParser,
};

// STENT TODO fix the unwraps
fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    match args.command {
        Command::BuildTree {
            accumulator_type,
            gen_proofs,
            keep_alive,
        } => {
            // TODO the type here will need to change once other accumulators
            // are supported.
            let accumulator: ndm_smt::NdmSmt = match accumulator_type {
                AccumulatorTypeCommand::NdmSmt { tree_build_type } => match tree_build_type {
                    TreeBuildCommand::New {
                        height,
                        secrets_file,
                        entity_source,
                        serialize,
                    } => ndm_smt::NdmSmtConfigBuilder::default()
                        .height(height)
                        .secrets_file_path_opt(secrets_file.and_then(|arg| arg.into_path()))
                        .serialization_path_opt(serialize.and_then(|arg| arg.into_path()))
                        .entities_path_opt(
                            entity_source.entities_file.and_then(|arg| arg.into_path()),
                        )
                        .num_entities_opt(entity_source.random_entities)
                        .build()
                        .unwrap()
                        .parse(),
                    TreeBuildCommand::Deserialize { path } => {
                        ndm_smt::deserialize(path.into_path().unwrap()).unwrap()
                    }
                },
                AccumulatorTypeCommand::FromConfig { file_path } => {
                    AccumulatorParser::from_config_fil_path_opt(file_path.into_path())
                        .parse()
                        .unwrap()
                }
            };

            if let Some(patharg) = gen_proofs {
                let entity_ids = EntityIdsParser::from_path(patharg.into_path())
                    .parse()
                    .unwrap();

                let proof = accumulator
                    .generate_inclusion_proof(entity_ids.first().unwrap())
                    .unwrap();

                let a = serde_json::to_string(&proof).unwrap();

                let path = "test_proof.json";
                let mut file = std::fs::File::create(path).unwrap();
                file.write_all(a.as_bytes());
            }
        }
        Command::GenProofs {} => {
            error!("TODO implement");
        }
    }
}
