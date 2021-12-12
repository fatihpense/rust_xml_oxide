# xml_oxide

[![crates.io](https://img.shields.io/crates/v/xml_oxide.svg)](https://crates.io/crates/xml_oxide) [![github](https://img.shields.io/badge/github-fatihpense%2Frust__xml__oxide-FFF8C2)](https://github.com/fatihpense/rust_xml_oxide) [![Released API docs](https://img.shields.io/badge/docs.rs-xml__oxide-CFF3CA)](https://docs.rs/xml_oxide)

Rust XML parser implementation that parses any well-formed XML defined in the [W3C Spec](https://www.w3.org/TR/xml/) in a streaming way.

## Features

- It uses constant-like memory for large XML files
- Fast enough for most use cases. It can parse a 1GB XML file(in memory) around 19 seconds. Note that it parses attributes and validates them before returning an event. Even if you don't use an event, this parser aims to ensure well-formedness of input.
- Supports [Namespaces in XML 1.0](https://www.w3.org/TR/xml-names/)
  - Because the namespace spec brings constraints around the usage of ":" in names. ParserBuilder has `namespace-aware=false` option to parse otherwise valid XML 1.0 documents.
- It only supports UTF-8 encoding
- It is a non-validating processor, it does important well-formedness checks
- Currently, it ignores well-formedness inside Processing Instructions, DTD/DOCTYPE and parses them as raw strings. It checks the general well-formedness including these entities. (It even parses comments inside DOCTYPE to achieve this)
- It can parse not-well-formed documents (please report as a bug)
- Entities that can be large are parsed as chunks to keep memory usage low: Character Data, CDATA Section, Comment, Whitespace
- Reading chunk size is currently default 8KB, not configurable. Internal ring buffer is 16KB. If you have an element tag or DOCTYPE declaration that is bigger than the buffer, it can backtrack and allocate more memory for the parsing operation. 1 byte chunk size is used for testing this behavior & UTF-8 handling.

### Unsafe usage

- `unsafe` is used for function `std::str::from_utf8_unchecked`. It is used on a slice of bytes that is already checked to be a valid UTF8 string with `std::str::from_utf8` before.

## To Do

- Provide option & make default getting Reference entities (e.g. `&amp;`) in `Characters` event. Also handling these in attribute values. Or remove Reference from event interface?
- In general, providing more configuration through builder pattern.
- More tests
- Parsing every entity including DTD to be able to utilize conformance test suite.
- Fuzzing

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

## History & Credits

I tried to specify a push parser interface like the Java SAX library and implement it in 2017. The idea was to provide an interface that can have multiple implementations in the community. It was working(albeit slowly) but the main problem was that a push parser is not ergonomic in Rust. After thinking for a long time and learning more about Rust I decided to implement a pull parser. Currently, the interface is just an `enum` and its behavior(like the possibility of splitting characters for each call).

I know the term `StAX` is generally used for pull parsers. But `SAX` just means `Simple API for XML` and it doesn't have a spec unlike DOM. I think it is safe to use the term `SAX` for this library.

If you want to use `xml_sax` interface to implement another parser we can discuss improving the interface. Currently, it is integrated into this crate. Only `crate::sax` module without submodules like `crate::sax::parser` should be regarded as the `xml_sax` interface.

[Why a pull parser?](https://github.com/raphlinus/pulldown-cmark/blob/eb60cb976a12fb99972ddfc9b60cc1c6b20e096c/README.md#why-a-pull-parser) section in `pulldown-cmark` is a great explanation.

The current interface is inspired by `quick-xml`, `xml-rs`, and Java libraries.

`nom` is a great library. It is just a crystallized & better version of what you would do naively at first try(I know). It also shows the power of composability in Rust.
