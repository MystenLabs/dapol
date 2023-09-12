//////////////////////////////////////////////////////////////////////////////////////////////////////
// use std::str::FromStr;                                                                           //
//                                                                                                  //
// use dapol::{NdmSmt, User, D256, UserId};                                                         //
//                                                                                                  //
// fn main() {                                                                                      //
//     let user1 = User{                                                                            //
//         liability: 10u64,                                                                        //
//         id: UserId::from_str("user1 ID").unwrap(),                                               //
//     };                                                                                           //
//                                                                                                  //
//     let master_secret: D256 = D256::from(3u64);                                                  //
//     let salt_b: D256 = D256::from(5u64);                                                         //
//     let salt_s: D256 = D256::from(7u64);                                                         //
//     let height: u8 = 5u8;                                                                        //
//     let users: Vec<User> = vec![user1];                                                          //
//     let ndsmt = NdmSmt::new(master_secret, salt_b, salt_s, height, users).unwrap();              //
//                                                                                                  //
//     let proof = ndsmt.generate_inclusion_proof(&UserId::from_str("user1 ID").unwrap()).unwrap(); //
//     println!("{:?}", proof);                                                                     //
// }                                                                                                //
//////////////////////////////////////////////////////////////////////////////////////////////////////
use std::str::FromStr;

use dapol::{NdmSmt, User, UserId, D256};

use core::fmt::Debug;
use dapol::{
    utils::get_secret, Dapol, DapolNode, RangeProofPadding, RangeProofSplitting, RangeProvable,
    RangeVerifiable,
};
use digest::Digest;
use rand::{distributions::Uniform, thread_rng, Rng};
use smtree::{
    index::TreeIndex,
    traits::{ProofExtractable, Rand, Serializable, TypeName},
};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let start = SystemTime::now();
    println!("start {:?}", start);
    new();
    let end = SystemTime::now();
    let dur = end.duration_since(start);
    println!("end {:?}", end);
    println!("duration {:?}", dur);
}

fn new() {
    println!("new");

    let tree_height = 32;
    // let num_leaves: usize = 2usize.pow(27); // 134M
    let num_leaves: usize = 2usize.pow(23); // 8.4M
    // let num_leaves: usize = 2usize.pow(14);

    let users = build_item_list_new(num_leaves, tree_height);

    let master_secret: D256 = D256::from(3u64);
    let salt_b: D256 = D256::from(5u64);
    let salt_s: D256 = D256::from(7u64);

    let ndsmt = NdmSmt::new(master_secret, salt_b, salt_s, tree_height as u8, users).unwrap();

    // let proof = ndsmt.generate_inclusion_proof(&UserId::from_str("user1 ID").unwrap()).unwrap();
    // println!("{:?}", proof);
}

fn old() {
    println!("old");
    // let num_leaves: usize = 2usize.pow(27); // 134M
    let num_leaves: usize = 2usize.pow(23); // 8.4M

    // bench tree height = 32
    let tree_height = 32;
    let items = build_item_list(num_leaves, tree_height);
    // we bench range proof padding only because building a tree does not depend on
    // the type of range proof we do
    build_dapol_tree::<blake3::Hasher, RangeProofPadding>(&items, tree_height);
}

fn build_dapol_tree<D, R>(items: &[(TreeIndex, DapolNode<D>)], tree_height: usize) -> Dapol<D, R>
where
    D: Digest + Default + Clone + TypeName + Debug,
    R: Clone + Serializable + RangeProvable + RangeVerifiable + TypeName,
{
    let secret = get_secret();
    let mut dapol = Dapol::<D, R>::new_blank(tree_height, tree_height);
    dapol.build(&items, &secret);
    dapol
}

fn build_item_list_new(num_leaves: usize, tree_height: usize) -> Vec<User> {
    let start = SystemTime::now();
    println!("build_item_list_new {:?}", start);

    let mut result = Vec::with_capacity(num_leaves);
    for i in 0..num_leaves {
        result.push(User {
            liability: i as u64,
            id: UserId::from_str(i.to_string().as_str()).unwrap(),
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

fn build_item_list(
    num_leaves: usize,
    tree_height: usize,
) -> Vec<(TreeIndex, DapolNode<blake3::Hasher>)> {
    let start = SystemTime::now();
    println!("build_item_list {:?}", start);

    let mut result = Vec::with_capacity(num_leaves);
    let mut value = DapolNode::<blake3::Hasher>::default();
    let stride = 2usize.pow(tree_height as u32) / num_leaves;
    for i in 0..num_leaves {
        let idx = TreeIndex::from_u64(tree_height, (i * stride) as u64);
        value.randomize();
        result.push((idx, value.clone()));
    }

    let after_loop = SystemTime::now();
    let dur = after_loop.duration_since(start);
    println!(
        "built item list (next is sorting), time now {:?}, duration {:?}",
        after_loop, dur
    );

    result.sort_by_key(|(index, _)| *index);

    let end = SystemTime::now();
    let dur = end.duration_since(start);
    println!(
        "done building item list, time now {:?}, duration {:?}",
        end, dur
    );

    result
}
