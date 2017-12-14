
//https://rust-leipzig.github.io/architecture/2016/12/20/idiomatic-trees-in-rust/

#[derive(Debug)]
pub struct NodeId {
    index: usize,
}

#[derive(Debug)]
pub struct Arena {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct Node {
    pub parent: Option<usize>,
    pub children: Vec<usize>,

    pub data: String,
}

fn children(node: &Node) {
    match node.parent {
        None => println!("no parent"),
        Some(parent) => println!(" parent => {}", parent),
    }
}

pub fn name(arg: String) -> () {
    let mut node = Node {
        parent: None,
        children: Vec::new(),
        data: "root".to_owned(),
    };

    node.children.push(1);

    println!("{}", arg);
}

pub fn new_node(arena: &mut Arena, data:String) -> usize {
    let mut node = Node {
        parent: None,
        children: Vec::new(),
        data: data,
    };

    arena.nodes.push(node);
    return 0;
}

pub fn get_node(arena: &mut Arena, id:usize) -> &mut Node {

    return &mut arena.nodes[id];
    
}
