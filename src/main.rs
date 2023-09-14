use std::str::FromStr;

use dapol::{NdmSmt, User, UserId, D256};

fn main() {
    let user1 = User {
        liability: 10u64,
        id: UserId::from_str("user1 ID").unwrap(),
    };

    let user2 = User {
        liability: 20u64,
        id: UserId::from_str("user2 ID").unwrap(),
    };

    let user4 = User {
        liability: 40u64,
        id: UserId::from_str("user4 ID").unwrap(),
    };

    let master_secret: D256 = D256::from(3u64);
    let salt_b: D256 = D256::from(5u64);
    let salt_s: D256 = D256::from(7u64);
    let height: u8 = 3u8;
    let users: Vec<User> = vec![user1, user2, user4];

    let ndsmt = NdmSmt::new(master_secret, salt_b, salt_s, height, users).unwrap();
    ndsmt.print_tree();

    //let proof = ndsmt.generate_inclusion_proof(&UserId::from_str("user1 ID").unwrap()).unwrap();
    //println!("{:?}", proof);
}
