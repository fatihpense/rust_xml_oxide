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
    pub book_element_attributes: String,
}

impl xml_sax::ContentHandler for MySaxHandler {
    fn start_document(&mut self) {}
    fn end_document(&mut self) {}
    fn start_element(
        &mut self,
        uri: &str,
        local_name: &str,
        qualified_name: &str,
        attributes: &dyn xml_sax::SAXAttributes,
    ) {
        for attr in attributes.iter() {
            //println!("{}->{}", attr.get_qualified_name(), attr.get_value());
            self.attributes_string.push_str(attr.get_qualified_name());
            self.attributes_string.push_str("->");
            self.attributes_string.push_str(attr.get_value());
            self.attributes_string.push_str(",");
        }

        if qualified_name == "fp:book" {
            for attr in attributes.iter() {
                //println!("{}->{}", attr.get_qualified_name(), attr.get_value());
                self.book_element_attributes.push_str("qname");
                self.book_element_attributes.push_str("->");
                self.book_element_attributes
                    .push_str(attr.get_qualified_name());
                self.book_element_attributes.push_str(", ");

                self.book_element_attributes.push_str("uri");
                self.book_element_attributes.push_str("->");
                self.book_element_attributes.push_str(attr.get_uri());
                self.book_element_attributes.push_str(", ");

                self.book_element_attributes.push_str("lname");
                self.book_element_attributes.push_str("->");
                self.book_element_attributes.push_str(attr.get_local_name());
                self.book_element_attributes.push_str(", ");

                self.book_element_attributes.push_str("value");
                self.book_element_attributes.push_str("->");
                self.book_element_attributes.push_str(attr.get_value());
                self.book_element_attributes.push_str(". ");
            }
            println!("{}", self.book_element_attributes);
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
fn books_attributes() {
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
        book_element_attributes: String::new(),
    };

    let mut sax_parser = SaxParser::new();
    let handler = Rc::new(RefCell::new(my_sax_handler));
    sax_parser.set_content_handler(handler.clone());
    sax_parser.parse(&mut reader);

    let expected_attributes_string = "xmlns:fp->http://github.com/fatihpense,fp:archive->true,fp:\
                                      read->true,fp:gifted->false,";
    assert_eq!(
        handler.borrow().attributes_string,
        expected_attributes_string
    );

    let expected_book_element_attributes =
        "qname->fp:archive, uri->http://github.com/fatihpense, lname->archive, value->true. \
         qname->fp:read, uri->http://github.com/fatihpense, lname->read, value->true. \
         qname->fp:gifted, uri->http://github.com/fatihpense, lname->gifted, value->false. ";

    assert_eq!(
        handler.borrow().book_element_attributes,
        expected_book_element_attributes
    );
}
