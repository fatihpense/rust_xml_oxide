# xml_oxide [![crates.io](https://img.shields.io/crates/v/xml_oxide.svg)](https://crates.io/crates/xml_oxide)

Rust XML parser implementation that parses any well-formed XML defined in the [W3C Spec](https://www.w3.org/TR/xml/) in a streaming way.

## Features

- It uses constant-like memory for large XML files
- It only supports UTF-8 encoding
- It is a non-validating parser
- It ignores well-formedness in Processing Instructions(DTD), DOCTYPE and parses them as raw strings
- It can parse not-well-formed documents (please report as a bug)
- Entities that can be large are parsed as chunks to keep memory usage low: Character Data, CDATA Section, Comment, Whitespace
- If you have an element tag or DOCTYPE declaration that is bigger than buffer size(currently default 8KB), it can fail

## Example Usage

In this example `StartElement` and `EndElement` events are counted. `StartElement` also include empty tags. Note that you can find more examples under `tests` directory.

```rust
use std::fs::File;
use xml_oxide::{parser::OxideParser, sax::Event};

fn main() {
    println!("Hello, world!");

    let mut counter: usize = 0;
    let mut end_counter: usize = 0;
    // let char_counter: usize = 0;

    use std::time::Instant;
    let now = Instant::now();

    let f = File::open(
        "C:/dev/wiki-export.xml",
    )
    .unwrap();

    let mut p = OxideParser::start(f);

    loop {
        let res = p.read_event();
        // println!("{:?}", res);
        match res {
            Ok(event) => match event {
                Event::StartDocument => {}
                Event::EndDocument => {}
                Event::StartElement(el) => {
                    counter = counter + 1;
                    // print every 10000th element name
                    if counter % 10000 == 0 {
                        println!("%10000 start {}", el.name);
                    }
                }
                Event::EndElement(el) => {
                    end_counter += 1;
                    if el.name == "feed" {
                        break;
                    }
                }
                Event::Characters(_) => {}
                Event::Reference(_) => {}
                _ => {}
            },
            Err(err) => {
                println!("{}", err);
                break;
            }
        }
    }

    println!("START EVENT COUNT:{}", counter);
    println!("END EVENT COUNT:{}", end_counter);

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}


```
