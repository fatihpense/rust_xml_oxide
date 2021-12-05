extern crate xml_oxide;

use std::fs::File;

use xml_oxide::{parser::Parser, sax::Event};

#[test]
fn books_attributes() {
    let f: File = match File::open("tests/xml_files/books.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut p = Parser::start(f);
    let mut attributes_string: String = String::new();
    let mut book_element_attributes: String = String::new();

    loop {
        let res = p.read_event();

        match res {
            Ok(event) => {
                match event {
                    Event::StartElement(el) => {
                        for attr in el.attributes.iter() {
                            //println!("{}->{}", attr.get_qualified_name(), attr.get_value());
                            attributes_string.push_str(attr.name);
                            attributes_string.push_str("->");
                            attributes_string.push_str(attr.value);
                            attributes_string.push_str(",");

                            // if el.name == "fp:book" {
                            if el.local_name == "book"
                                && el.namespace == "http://github.com/fatihpense"
                            {
                                book_element_attributes.push_str("qname");
                                book_element_attributes.push_str("->");
                                book_element_attributes.push_str(attr.name);
                                book_element_attributes.push_str(", ");

                                book_element_attributes.push_str("uri");
                                book_element_attributes.push_str("->");
                                book_element_attributes.push_str(attr.namespace);
                                book_element_attributes.push_str(", ");

                                book_element_attributes.push_str("lname");
                                book_element_attributes.push_str("->");
                                book_element_attributes.push_str(attr.local_name);
                                book_element_attributes.push_str(", ");

                                book_element_attributes.push_str("value");
                                book_element_attributes.push_str("->");
                                book_element_attributes.push_str(attr.value);
                                book_element_attributes.push_str(". ");
                            }
                        }
                    }
                    Event::EndDocument => {
                        break;
                    }

                    _ => {}
                }
            }
            Err(err) => {
                println!("{}", err);
                break;
            }
        }
    }

    let expected_attributes_string = "xmlns:fp->http://github.com/fatihpense,fp:archive->true,fp:\
                                      read->true,fp:gifted->false,";
    assert_eq!(attributes_string, expected_attributes_string);

    let expected_book_element_attributes =
        "qname->fp:archive, uri->http://github.com/fatihpense, lname->archive, value->true. \
         qname->fp:read, uri->http://github.com/fatihpense, lname->read, value->true. \
         qname->fp:gifted, uri->http://github.com/fatihpense, lname->gifted, value->false. ";

    assert_eq!(book_element_attributes, expected_book_element_attributes);
}
