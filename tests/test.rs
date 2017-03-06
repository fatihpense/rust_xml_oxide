extern crate xml_oxide;
extern crate xml_sax;

// imports traits.
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufRead;







use std::time::{Duration, Instant};

use xml_oxide::sax::*;






struct MySaxHandler {
    pub counter: usize,
    pub end_counter: usize,
    pub char_counter: usize,
}

impl xml_sax::ContentHandler for MySaxHandler {
    fn start_element(&mut self, name: &str, attributes: &xml_sax::SAXAttributes) {
        self.counter = self.counter + 1;
        println!("{}", name);
    } //need attributes
    fn end_element(&mut self, name: &str) {
        self.end_counter += 1;
        println!("{}", name);
    }
    fn characters(&mut self, characters: &str) {
        println!("{}", characters);
    }
    fn offset(&mut self, offset: usize) {
        self.char_counter = self.char_counter + offset;
        // println!("offset:{}", offset);
    }
}





// let mut my_sax_handler = MySaxHandler {
//     counter: 0,
//     end_counter: 0,
//     char_counter: 0,
// };
// {
//     let mut sax_parser = SaxParser::new();
//     sax_parser.set_content_handler(&mut my_sax_handler);
//     sax_parser.parse(&mut f);
// }
// println!("START EVENT COUNT:{}", my_sax_handler.counter);
// println!("END EVENT COUNT:{}", my_sax_handler.end_counter);
// println!("TOTAL CHARS:{}", my_sax_handler.char_counter);




struct MyCollectorSaxHandler {
    start_counter: usize,
    end_counter: usize,
    char_counter: usize,
    start_el_name_vec: Vec<String>,
    end_el_name_vec: Vec<String>,
    // characters should be collected because SAX parser can send them splitted for various reasons.
    characters_collected_vec: Vec<String>,
    characters_buf: String,
}

impl xml_sax::ContentHandler for MyCollectorSaxHandler {
    fn start_element(&mut self, qualified_name: &str, attributes: &xml_sax::SAXAttributes) {
        self.start_counter = self.start_counter + 1;
        self.start_el_name_vec.push(qualified_name.to_owned());

        for attr in attributes.iter() {
            println!("iter attr: {}", attr.get_value());
        }

        if self.characters_buf.len() > 0 {
            self.characters_collected_vec.push(self.characters_buf.clone());
            self.characters_buf = String::new();
        }
    }
    fn end_element(&mut self, qualified_name: &str) {
        self.end_counter += 1;
        self.end_el_name_vec.push(qualified_name.to_owned());

        if self.characters_buf.len() > 0 {
            self.characters_collected_vec.push(self.characters_buf.clone());
            self.characters_buf = String::new();
        }
    }
    fn characters(&mut self, characters: &str) {
        println!("characters: {}", characters);
        self.characters_buf.push_str(characters);
    }
    fn offset(&mut self, offset: usize) {
        self.char_counter = self.char_counter + offset;
    }
}




#[test]
fn test1() {

    let mut s = String::from("<rootEl><value>5</value></rootEl>");
    let mut reader = BufReader::new(s.as_bytes());
    let mut my_sax_handler = MyCollectorSaxHandler {
        start_counter: 0,
        end_counter: 0,
        char_counter: 0,
        start_el_name_vec: Vec::new(),
        end_el_name_vec: Vec::new(),
        characters_collected_vec: Vec::new(),
        characters_buf: String::new(),
    };
    {
        let mut sax_parser = SaxParser::new();
        sax_parser.set_content_handler(&mut my_sax_handler);
        sax_parser.parse(&mut reader);
    }
    assert_eq!(my_sax_handler.char_counter, 33);
    assert_eq!(my_sax_handler.start_counter, 2);
    assert_eq!(my_sax_handler.end_counter, 2);
    assert_eq!(my_sax_handler.start_el_name_vec.get(0).unwrap(), "rootEl");
    println!("{:?}", my_sax_handler.characters_collected_vec);
    assert_eq!(my_sax_handler.end_el_name_vec.get(0).unwrap(), "value");

    assert_eq!(my_sax_handler.characters_collected_vec.get(0).unwrap(), "5");
    // assert_eq!(my_sax_handler.end, );

}