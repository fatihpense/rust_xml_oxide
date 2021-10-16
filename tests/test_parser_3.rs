#![allow(warnings, unused)]
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
    pub attributes_string: String,
}

impl xml_sax::ContentHandler for MySaxHandler {
    fn start_document(&mut self) {}
    fn end_document(&mut self) {}
    fn start_element(
        &mut self,
        uri: &str,
        local_name: &str,
        qualified_name: &str,
        attributes: &xml_sax::SAXAttributes,
    ) {
        for attr in attributes.iter() {
            println!("{}->{}", attr.get_qualified_name(), attr.get_value());
            self.attributes_string.push_str(attr.get_qualified_name());
            self.attributes_string.push_str("->");
            self.attributes_string.push_str(attr.get_value());
            self.attributes_string.push_str(",");
        }
        // println!("{}", name);
    }
    fn end_element(&mut self, uri: &str, local_name: &str, qualified_name: &str) {

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
fn tidy_parser() {
    xml_oxide::parsertidy::get();

    assert_eq!(true, true);
}

#[test]
fn new_parser_3() {
    let mut f: File = match File::open("tests/xml_files/books.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut reader = BufReader::new(f);

    let mut my_sax_handler = MySaxHandler {
        attributes_string: String::new(),
    };
    {
        let mut sax_parser = SaxParser::new();
        let handler = Rc::new(RefCell::new(my_sax_handler));
        sax_parser.set_content_handler(handler);
        sax_parser.parse(&mut reader);
    }

    assert_eq!(true, true);
}
