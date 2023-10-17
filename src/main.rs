use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use env_logger;
use log::error;

use dapol::{
    activate_logging, Cli, Entity, EntityId, EntitiesParser, NdmSmt, Secrets, SecretsParser,
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
        build_item_list_new(num_leaves as usize, height.as_usize())
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

// TODO move this to the entities file
fn build_item_list_new(num_leaves: usize, tree_height: usize) -> Vec<Entity> {
    let start = SystemTime::now();
    println!("build_item_list_new {:?}", start);

    let mut result = Vec::with_capacity(num_leaves);
    for i in 0..num_leaves {
        result.push(Entity {
            liability: i as u64,
            id: EntityId::from_str(i.to_string().as_str()).unwrap(),
        })
    }

    let end = SystemTime::now();
    let dur = end.duration_since(start);
    println!(
        "done building item list new, time now {:?}, duration {:?}",
        end, dur
    );

    result
}
