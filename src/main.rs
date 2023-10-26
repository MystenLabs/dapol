use clap::Parser;
use log::error;

use dapol::{
    activate_logging,
    cli::{AccumulatorTypeCommand, Cli, Command, TreeBuildCommand},
    ndm_smt, AccumulatorParser, NdmSmtConfigBuilder,
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
                    } => {
                        let config = NdmSmtConfigBuilder::default()
                            .height(Some(height))
                            .secrets_file_path(secrets_file.and_then(|arg| arg.into_path()))
                            .serialization_path(serialize.and_then(|arg| arg.into_path()))
                            .entities_path(
                                entity_source.entities_file.and_then(|arg| arg.into_path()),
                            )
                            .num_entities(entity_source.random_entities)
                            .build()
                            .unwrap();

                        config.parse()
                    }
                    TreeBuildCommand::Deserialize { path } => {
                        ndm_smt::deserialize(path.into_path().unwrap()).unwrap()
                    }
                },
                AccumulatorTypeCommand::FromConfig { file_path } => {
                    AccumulatorParser::from_config_fil_path(file_path.into_path())
                        .parse()
                        .unwrap()
                }
            };

            // gen_proofs.and_then(|patharg| {
            //     let proof = accumulator
            //         .generate_inclusion_proof(
            //             &EntityId::from_str(
            //                 "entity1 ID",
            //             )
            //             .unwrap(),
            //         )
            //         .unwrap();
            //     println!("{:?}", proof);
            // })
        }
        Command::GenProofs {} => {
            error!("TODO implement");
        }
    }
}
