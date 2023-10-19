use clap::Parser;
use log::error;

use dapol::{
    activate_logging, Cli, EntitiesParser, NdmSmt, Secrets, SecretsParser, generate_random_entities,
};

fn main() {
    // let num_leaves: usize = 2usize.pow(27); // 134M
    // let num_leaves: usize = 2usize.pow(23); // 8.4M
    // let num_leaves: usize = 2usize.pow(10);

    let args = Cli::parse();

    activate_logging(args.verbose.log_level_filter());

    let secrets = if let Some(path_arg) = args.secrets_file {
        let path = path_arg.into_path().unwrap();
        SecretsParser::from_path(path).parse().unwrap()
    } else {
        Secrets::generate_random()
    };

    let height = args.height;

    let entities = if let Some(path_arg) = args.entity_source.entities_file {
        let path = path_arg.into_path().unwrap();
        EntitiesParser::from_path(path).parse().unwrap()
    } else if let Some(num_leaves) = args.entity_source.random_entities {
        generate_random_entities(num_leaves)
    } else {
        panic!("This code should not be reachable because the cli arguments are required");
    };

    let ndsmt_res = NdmSmt::new(secrets, height, entities);

    match ndsmt_res {
        Ok(_ndmsmt) => {}
        Err(err) => {
            error!("{:?} {}", err, err);
        }
    }

    // let proof = ndsmt.generate_inclusion_proof(&EntityId::from_str("entity1 ID").unwrap()).unwrap(); println!("{:?}", proof);
}
