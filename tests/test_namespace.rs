extern crate xml_oxide;

use std::fs::File;

use xml_oxide::{sax::parser::Parser, sax::Event};

#[test]
fn test_namespaces() {
    let f: File = match File::open("tests/xml_files/namespaces.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut p = Parser::from_reader(f);
    let mut element_namespace_data: String = String::new();
    let mut attribute_namespace_data: String = String::new();

    loop {
        let res = p.read_event();

        match res {
            Ok(event) => {
                match event {
                    Event::StartElement(el) => {
                        element_namespace_data.push_str(el.name);
                        element_namespace_data.push_str("->");
                        element_namespace_data.push_str(el.namespace);
                        element_namespace_data.push_str(",");
                        //        print!("{}->{},", qualified_name, uri);

                        for attr in el.attributes.iter() {
                            attribute_namespace_data.push_str(attr.name);
                            attribute_namespace_data.push_str("->");
                            attribute_namespace_data.push_str(attr.namespace);
                            attribute_namespace_data.push_str(",");
                        }
                    }
                    Event::EndDocument => {
                        break;
                    }

                    _ => {}
                }
            }

            Err(_err) => {
                break;
            }
        }
    }

    let expected_namespace_string = "root->urn:rootns,ns1:a->http://ns1,ns2:b->urn:ns2,\
         c->urn:rootns,ns1:d->http://ns1,e->urn:e,ns2:b2->urn:b2--2,f->urn:d,g->urn:rootns,";
    assert_eq!(element_namespace_data, expected_namespace_string);

    let expected_attribute_namespace_data =
        "xmlns->,xmlns:ns1->,noprefattr->,xmlns:ns2->,ns1:prefattrns1->http://ns1,\
         xmlns->,xmlns->,xmlns:ns2->,ns2:prefattrb22->urn:b2--2,attrf->,";
    assert_eq!(attribute_namespace_data, expected_attribute_namespace_data);
}
