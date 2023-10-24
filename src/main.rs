use clap::Parser;
use log::{error, info};
use logging_timer::{executing, stimer, Level};

use chrono;

use dapol::{
    activate_logging,
    cli::{AccumulatorType, Cli, Commands, SERIALIZED_TREE_EXTENSION},
    generate_random_entities, EntitiesParser, NdmSmt, Secrets, SecretsParser,
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
                    // Path is a:
                    // - non-existing directory
                    // - existing directory
                    // - file with some dirs created
                    // - file with all dirs created

                    let mut path = path_arg.into_path().unwrap();

                    if let Some(ext) = path.extension() {
                        if ext != SERIALIZED_TREE_EXTENSION {
                            error!(
                                "Unknown file extension {:?}, expected {}",
                                ext, SERIALIZED_TREE_EXTENSION
                            );
                            panic!(
                                "Unknown file extension {:?}, expected {}",
                                ext, SERIALIZED_TREE_EXTENSION
                            );
                        }
                        if let Some(parent) = path.parent() {
                            if !parent.is_dir() {
                                std::fs::create_dir_all(parent).unwrap();
                            }
                        }
                        Some(path)
                    } else {
                        if !path.is_dir() {
                            std::fs::create_dir_all(path.clone()).unwrap();
                        }
                        let mut file_name: String = "ndm_smt_".to_owned();
                        let now = chrono::offset::Local::now();
                        file_name.push_str(&now.timestamp().to_string());
                        file_name.push_str(SERIALIZED_TREE_EXTENSION);
                        path.push(file_name);
                        Some(path)
                    }
                }
                None => None,
            };

            let ndmsmt_res = NdmSmt::new(secrets, height, entities);

            match ndmsmt_res {
                Ok(ndmsmt) => {
                    if serialize.is_some() {
                        use std::io::Write;

                        let tmr = stimer!(Level::Debug; "Serialization");
                        let encoded: Vec<u8> = bincode::serialize(&ndmsmt).unwrap();
                        executing!(tmr, "Done encoding");
                        let mut file = std::fs::File::create(
                            serialization_path
                                .expect("Bug in CLI parser: serialization path not set"),
                        )
                        .unwrap();
                        file.write_all(&encoded).unwrap();
                        logging_timer::finish!(tmr, "Done writing file");
                    }
                }
                Err(err) => {
                    error!("{:?} {}", err, err);
                }
            }

            // let proof =
            // ndsmt.generate_inclusion_proof(&EntityId::from_str("entity1
            // ID").unwrap()).unwrap(); println!("{:?}", proof);
        }
        (Commands::FromFile { path }, AccumulatorType::NdmSmt) => {
            use std::io::BufReader;

            let file = std::fs::File::open(path.to_string()).unwrap();
            let mut buf_reader = BufReader::new(file);
            let tmr = stimer!(Level::Debug; "Deserialization");
            let decoded: NdmSmt = bincode::deserialize_from(buf_reader).unwrap();
            logging_timer::finish!(tmr, "Done decoding");
        }
        _ => {
            error!("Command is not supported for the given accumulator");
        }
    }
}
