use clap::Parser;
use log::error;

use dapol::{
    activate_logging,
    cli::{AccumulatorTypeCommand, Cli, Command, TreeBuildCommand},
    ndm_smt, read_write_utils,
    read_write_utils::LogOnErr,
    NdmSmtConfigBuilder,
};

// STENT TODO fix the unwraps
// STENT TODO move all this stuff outside of main.rs, and do unit tests
fn main() {
    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    match args.command {
        Command::BuildTree {
            accumulator_type,
            gen_proofs,
            keep_alive,
        } => {
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
                        let ndmsmt =
                            read_write_utils::deserialize_from_bin_file::<ndm_smt::NdmSmt>(
                                path.into_path().unwrap(),
                            )
                            .log_on_err()
                            .unwrap();

                        ndmsmt
                    }
                },
                AccumulatorTypeCommand::FromConfig { file_path } => {
                    error!("TODO implement");
                    panic!("");
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
