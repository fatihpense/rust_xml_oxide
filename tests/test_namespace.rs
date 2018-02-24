extern crate xml_oxide;
extern crate xml_sax;

// imports traits.
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufRead;

use std::fs::File;

use xml_oxide::sax::*;

use std::rc::Rc;
use std::cell::RefCell;

struct MySaxHandler {
    pub element_namespace_data: String,
    pub attribute_namespace_data: String,
    pub document_state: usize,
}

impl xml_sax::ContentHandler for MySaxHandler {
    fn start_document(&mut self) {
        assert_eq!(self.document_state, 0);
        self.document_state = 1;
    }

    fn end_document(&mut self) {
        assert_eq!(self.document_state, 1);
        self.document_state = 2;
    }
    fn start_element(
        &mut self,
        uri: &str,
        local_name: &str,
        qualified_name: &str,
        attributes: &xml_sax::SAXAttributes,
    ) {
        assert_eq!(self.document_state, 1);
        self.element_namespace_data.push_str(qualified_name);
        self.element_namespace_data.push_str("->");
        self.element_namespace_data.push_str(uri);
        self.element_namespace_data.push_str(",");
        //        print!("{}->{},", qualified_name, uri);

        for attr in attributes.iter() {
            self.attribute_namespace_data
                .push_str(attr.get_qualified_name());
            self.attribute_namespace_data.push_str("->");
            self.attribute_namespace_data.push_str(attr.get_uri());
            self.attribute_namespace_data.push_str(",");
        }
        println!("X->{}", self.attribute_namespace_data);
    }

    fn end_element(&mut self, uri: &str, local_name: &str, qualified_name: &str) {
        assert_eq!(self.document_state, 1);

        // println!("{}", name);
    }
    fn characters(&mut self, characters: &str) {
        // println!("{}", characters);
    }
}

impl xml_sax::StatsHandler for MySaxHandler {
    fn offset(&mut self, offset: usize) {}
}

#[test]
fn test_namespaces() {
    let mut f: File = match File::open("tests/xml_files/namespaces.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut reader = BufReader::new(f);

    let mut my_sax_handler = MySaxHandler {
        element_namespace_data: String::new(),
        attribute_namespace_data: String::new(),
        document_state: 0,
    };

    let mut sax_parser = SaxParser::new();
    let handler = Rc::new(RefCell::new(my_sax_handler));
    sax_parser.set_content_handler(handler.clone());
    sax_parser.parse(&mut reader);

    assert_eq!(handler.borrow().document_state, 2);

    let expected_namespace_string =
        "root->urn:rootns,ns1:a->http://ns1,ns2:b->urn:ns2,\
         c->urn:rootns,ns1:d->http://ns1,e->urn:e,ns2:b2->urn:b2--2,f->urn:d,g->urn:rootns,";
    assert_eq!(
        handler.borrow().element_namespace_data,
        expected_namespace_string
    );

    let expected_attribute_namespace_data =
        "xmlns->,xmlns:ns1->,noprefattr->,xmlns:ns2->,ns1:prefattrns1->http://ns1,\
         xmlns->,xmlns->,xmlns:ns2->,ns2:prefattrb22->urn:b2--2,attrf->,";
    assert_eq!(
        handler.borrow().attribute_namespace_data,
        expected_attribute_namespace_data
    );
}
