use std::fmt::Debug;

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::sync::Mutex;

use super::{Mergeable, LeftSibling, RightSibling, Coordinate, Node, MatchedPair, Sibling};

// TODO unused
static thread_count: Mutex<u32> = Mutex::new(0);

fn find_split_index<C: Clone>(leaves: &Vec<Node<C>>, x_coord_mid: u64) -> usize {
    let mut index = 0;
    while leaves
        .get(index)
    // TODO this default false is not good if it gets hit because that means there is a bug
        .map_or(false, |leaf| leaf.coord.x <= x_coord_mid)
    {
        index += 1;
    }
    index
}

impl<C: Clone> LeftSibling<C> {
    fn new_sibling_padding_node_2<F>(&self, new_padding_node_content: Arc<F>) -> RightSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        RightSibling(node)
    }
}

impl<C: Clone> RightSibling<C> {
    fn new_sibling_padding_node_2<F>(&self, new_padding_node_content: Arc<F>) -> LeftSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        LeftSibling(node)
    }
}

pub fn build_node<C: Clone + Mergeable + Send + 'static + Debug, F>(
    x_coord_min: u64,
    x_coord_max: u64,
    y: u8,
    height: u8,
    mut leaves: Vec<Node<C>>,
    new_padding_node_content: Arc<F>,
) -> Node<C>
where
    F: Fn(&Coordinate) -> C + Send + 'static + Sync,
{
    assert!(leaves.len() <= 2usize.pow(y as u32));

    // println!("\nfunction call, num leaves {:?}", leaves.len());
    // println!("x_coord_min {:?}", x_coord_min);
    // println!("x_coord_max {:?}", x_coord_max);

    // base case: reached layer above leaves
    if y == 1 {
        // len should never reach 0
        // println!("base case reached");
        let pair = if leaves.len() == 2 {
            let left = LeftSibling::from_node(leaves.remove(0));
            let right = RightSibling::from_node(leaves.remove(0));
            MatchedPair { left, right }
        } else {
            let node = Sibling::from_node(leaves.remove(0));
            match node {
                Sibling::Left(left) => MatchedPair {
                    right: left.new_sibling_padding_node_2(new_padding_node_content),
                    left,
                },
                Sibling::Right(right) => MatchedPair {
                    left: right.new_sibling_padding_node_2(new_padding_node_content),
                    right,
                },
            }
        };

        return pair.merge();
    }

    let x_coord_mid = (x_coord_min + x_coord_max) / 2;
    // println!("x_coord_mid {}", x_coord_mid);
    // count the number of nodes that belong under the left child node
    let left_count = find_split_index(&leaves, x_coord_mid);
    // println!("left_count {}", left_count);

    // if count > 0 for 1st & 2nd half then spawn a new thread to go down the right
    // node
    let pair = if 0 < left_count && left_count < leaves.len() {
        // println!("2 children");
        let right_leaves = leaves.split_off(left_count);
        let left_leaves = leaves;

        // let str = format!("x_coord_mid {} x_coord_max {}", x_coord_mid, x_coord_max);
        // let count = {
        //     let mut value = thread_count.lock().unwrap();
        //     *value += 1;
        //     // println!("STENT thread count {}", value);
        //     value.clone()
        // };

        let f = new_padding_node_content.clone();

        // for right child
        if y > height - 4 {
            let (tx, rx) = mpsc::channel();
            let builder = thread::Builder::new(); //.name(count.to_string());

            builder.spawn(move || {
                println!("thread spawned");
                let node = build_node(x_coord_mid + 1, x_coord_max, y - 1, height, right_leaves, f);
                // println!("thread about to send, node {:?}", node);
                tx.send(RightSibling::from_node(node))
                    .map_err(|err| {
                        println!("ERROR STENT SEND {:?}", err);
                        err
                    })
                    .unwrap();
            });
            let left = LeftSibling::from_node(build_node(
                x_coord_min,
                x_coord_mid,
                y - 1,
                height,
                left_leaves,
                new_padding_node_content,
            ));

            let right = rx
                .recv()
                .map_err(|err| {
                    println!("ERROR STENT REC {:?}", err);
                    err
                })
                .unwrap();

            MatchedPair { left, right }
        } else {
            let right = RightSibling::from_node(build_node(
                x_coord_mid + 1,
                x_coord_max,
                y - 1,
                height,
                right_leaves,
                f,
            ));

            let left = LeftSibling::from_node(build_node(
                x_coord_min,
                x_coord_mid,
                y - 1,
                height,
                left_leaves,
                new_padding_node_content,
            ));

            MatchedPair { left, right }
        }
    } else if left_count > 0 {
        // println!("left child");
        // go down left child
        let left = LeftSibling::from_node(build_node(
            x_coord_min,
            x_coord_mid,
            y - 1,
            height,
            leaves,
            new_padding_node_content.clone(),
        ));
        let right = left.new_sibling_padding_node_2(new_padding_node_content);
        MatchedPair { left, right }
    } else {
        // println!("right child");
        // go down right child
        let right = RightSibling::from_node(build_node(
            x_coord_mid + 1,
            x_coord_max,
            y - 1,
            height,
            leaves,
            new_padding_node_content.clone(),
        ));
        let left = right.new_sibling_padding_node_2(new_padding_node_content);
        MatchedPair { left, right }
    };

    pair.merge()
}
