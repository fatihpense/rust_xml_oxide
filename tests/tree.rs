extern crate xml_oxide;
extern crate xml_sax;

use xml_oxide::tree::*;

#[test]
fn tree() {
    let mut arena = Arena { nodes: Vec::new() };

    let id = new_node(&mut arena, "fp".to_owned());

    {
        let mut node = get_node(&mut arena, id);
        println!("{}", node.data);
    }

    {
        let mut node = get_node(&mut arena, id);
        println!("{}", node.data);
    }

}
