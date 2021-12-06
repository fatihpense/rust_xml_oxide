# xml_oxide

[![crates.io](https://img.shields.io/crates/v/xml_oxide.svg)](https://crates.io/crates/xml_oxide) [![github](https://img.shields.io/badge/github-fatihpense%2Frust__xml__oxide-FFF8C2)](https://github.com/fatihpense/rust_xml_oxide) [![Released API docs](https://img.shields.io/badge/docs.rs-xml__oxide-CFF3CA)](https://docs.rs/xml_oxide)

Rust XML parser implementation that parses any well-formed XML defined in the [W3C Spec](https://www.w3.org/TR/xml/) in a streaming way.

If you want to use `xml_sax` interface to implement another parser we can discuss to improve the interface. Currently it is integrated to this crate.

## To Do

- Because the [namespace spec](https://www.w3.org/TR/xml-names/) brings constraints around the usage of ":" in names. Provide `namespace-aware=false` option to parse otherwise valid XML 1.0 documents .

## Features

- It uses constant-like memory for large XML files
- Supports [Namespaces in XML 1.0](https://www.w3.org/TR/xml-names/)
- It only supports UTF-8 encoding
- It is a non-validating parser, it does important well-formedness checks
- It ignores well-formedness in Processing Instructions, DTD, DOCTYPE and parses them as raw strings
- It can parse not-well-formed documents (please report as a bug)
- Entities that can be large are parsed as chunks to keep memory usage low: Character Data, CDATA Section, Comment, Whitespace
- If you have an element tag or DOCTYPE declaration that is bigger than buffer size(currently default 8KB), it can fail

## Example Usage

In this example [StartElement](sax::StartElement) and [EndElement](sax::EndElement) events are counted. Note that you can find more examples under `tests` directory.

- `StartElement` also include empty tags. Checked by `is_empty`.
- [Reference](sax::Reference) entities like `&amp;` or `&#60;` comes in its own event(Not in `Characters`).
- Character/numerical and predefined entity references are resolved. Custom entity definitions are passed as raw.
- Check [sax::Event](sax::Event) to see all available event types

```rust
use std::fs::File;
use xml_oxide::{sax::parser::Parser, sax::Event};


fn main() {
    println!("Starting...");

    let mut counter: usize = 0;
    let mut end_counter: usize = 0;

    let now = std::time::Instant::now();

    let f = File::open("./tests/xml_files/books.xml").unwrap();

    let mut p = Parser::from_reader(f);

    loop {
        let res = p.read_event();

        match res {
            Ok(event) => match event {
                Event::StartDocument => {}
                Event::EndDocument => {
                    break;
                }
                Event::StartElement(el) => {
                    //You can differantiate between Starting Tag and Empty Element Tag
                    if !el.is_empty {
                        counter = counter + 1;
                        // print every 10000th element name
                        if counter % 10000 == 0 {
                            println!("%10000 start {}", el.name);
                        }
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

    println!("Start event count:{}", counter);
    println!("End event count:{}", end_counter);

    let elapsed = now.elapsed();
    println!("Time elapsed: {:.2?}", elapsed);
}


```
