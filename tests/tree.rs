extern crate indextree;
extern crate xml_oxide;
extern crate xml_sax;


use indextree::Arena;
use std::ops::Index;


#[test]
fn tree2() {
    // Create a new arena
    let arena = &mut Arena::new();

    // Add some new nodes to the arena
    let a = arena.new_node("Char".to_owned());
    let b = arena.new_node("Char".to_owned());

    // Append a to b
    a.append(b, arena);
    assert_eq!(b.ancestors(arena).into_iter().count(), 2);
    println!("{:?}", arena.index(a).data);
}
