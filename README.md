# xml_oxide [![crates.io](https://meritbadge.herokuapp.com/actix-web)](https://crates.io/crates/actix-web)
Rust XML parser implementation for SAX interface: xml_sax

### Features
* It uses constant-like memory for large xml files.
* It supports common XML structures and namespaces. No DTD etc. yet.
* It leverages an interface that is familiar if you have dealt with SAX before.
* There can be more than one implementation 
* I'm open to advice and contribution!


### Example Usage
In this example "start element" and "end element" events are counted. You can see there is a handler that stores variables. We add `xml_sax::ContentHandler` interface to this handler and run parser. The beauty of having an simple interface is that more parser libraries can implement it. So there is an opportunity of code reuse across the community.

Note that you can find more examples under `tests` directory.


```rust
extern crate xml_oxide;
extern crate xml_sax;

use std::io::BufReader;
use std::fs::File;
use std::time::Instant;

use xml_oxide::sax::*;

use std::rc::Rc;
use std::cell::RefCell;

struct MySaxHandler2 {
    pub counter: usize,
    pub end_counter: usize,
    pub char_counter: usize,
}

impl xml_sax::ContentHandler for MySaxHandler2 {
    fn start_document(&mut self) {}

    fn end_document(&mut self) {}

    fn start_element(
        &mut self,
        uri: &str,
        local_name: &str,
        qualified_name: &str,
        attributes: &xml_sax::SAXAttributes,
    ) {
        self.counter = self.counter + 1;
        // print every 10000th element name
        if self.counter % 10000 == 0 {
            println!("%10000 start {}", qualified_name);
        }
        //println!("qname: {}", qualified_name);

        for attr in attributes.iter() {
            //println!("iter attr: {}", attr.get_value());
        }
    }
    fn end_element(&mut self, uri: &str, local_name: &str, qualified_name: &str) {
        self.end_counter += 1;
    }
    fn characters(&mut self, characters: &str) {}
}

fn main() {
    let now = Instant::now();

    let f: File = match File::open("test/simple.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            return ();
        }
    };
    let mut reader = BufReader::new(f);

    let my_sax_handler2 = MySaxHandler2 {
        counter: 0,
        end_counter: 0,
        char_counter: 0,
    };

    let mut sax_parser = SaxParser::new();
    let handler = Rc::new(RefCell::new(my_sax_handler2));
    sax_parser.set_content_handler(handler.clone());
    sax_parser.parse(&mut reader);

    println!("START EVENT COUNT:{}", handler.borrow().counter);
    println!("END EVENT COUNT:{}", handler.borrow().end_counter);
    println!("TOTAL CHARS:{}", handler.borrow().char_counter);

    println!(
        "secs: {} nanos: {}",
        now.elapsed().as_secs(),
        now.elapsed().subsec_nanos()
    );
}

```
