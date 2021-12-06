use std::fs::File;
use xml_oxide::{sax::parser::Parser, sax::Event};

#[test]
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
