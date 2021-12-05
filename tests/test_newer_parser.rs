use std::fs::File;

use xml_oxide::{parser::Parser, sax::Event};

extern crate xml_oxide;

#[test]
fn newer_parser_1() {
    let f: File = match File::open("tests/xml_files/books.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut p = Parser::start(f);
    let mut result: String = String::new();
    loop {
        let res = p.read_event();

        match res {
            Ok(event) => match event {
                Event::EndDocument => {
                    println!("{:?}", event);
                    result.push_str(&format!("{:?}\n", event));
                    break;
                }
                _ => {
                    println!("{:?}", event);
                    result.push_str(&format!("{:?}\n", event));
                }
            },
            Err(_err) => {
                break;
            }
        }
    }

    let expected = r#"StartDocument
XmlDeclaration("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
Whitespace("\r\n")
StartElement(StartElement { name: "fp:books", attributes: [Attribute { value: "http://github.com/fatihpense", name: "xmlns:fp", local_name: "fp", prefix: "xmlns", namespace: "" }], is_empty: false, local_name: "books", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n    ")
StartElement(StartElement { name: "fp:book", attributes: [Attribute { value: "true", name: "fp:archive", local_name: "archive", prefix: "fp", namespace: "http://github.com/fatihpense" }, Attribute { value: "true", name: "fp:read", local_name: "read", prefix: "fp", namespace: "http://github.com/fatihpense" }, Attribute { value: "false", name: "fp:gifted", local_name: "gifted", prefix: "fp", namespace: "http://github.com/fatihpense" }], is_empty: false, local_name: "book", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:title_english", attributes: [], is_empty: false, local_name: "title_english", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("Madonna in a Fur Coat")
EndElement(EndElement { name: "fp:title_english", local_name: "title_english", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:title_original", attributes: [], is_empty: false, local_name: "title_original", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("Kürk Mantolu Madonna")
EndElement(EndElement { name: "fp:title_original", local_name: "title_original", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:author", attributes: [], is_empty: false, local_name: "author", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("Sabahattin Ali")
EndElement(EndElement { name: "fp:author", local_name: "author", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:quote_english", attributes: [], is_empty: false, local_name: "quote_english", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("It is, perhaps, easier to dismiss a man whose face gives no indication of an inner life. And what a pity that is: a dash of curiosity is all it takes to stumble upon treasures we never expected.")
EndElement(EndElement { name: "fp:quote_english", local_name: "quote_english", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:original_language", attributes: [], is_empty: false, local_name: "original_language", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("tr")
EndElement(EndElement { name: "fp:original_language", local_name: "original_language", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n    ")
EndElement(EndElement { name: "fp:book", local_name: "book", prefix: "fp", namespace: "http://github.com/fatihpense" })
Characters("\r\n")
EndElement(EndElement { name: "fp:books", local_name: "books", prefix: "fp", namespace: "http://github.com/fatihpense" })
Whitespace("\r\n")
EndDocument
"#;
    // println!("{}", result);
    assert_eq!(result, expected);
}

#[test]
fn newer_parser_commentcdata() {
    let f: File = match File::open("tests/xml_files/comment-cdata.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut p = Parser::start(f);
    let mut comments: String = String::new();
    let mut cdatas: String = String::new();

    let mut inside_comment = false;
    let mut inside_cdata = false;

    loop {
        let res = p.read_event();
        match res {
            Ok(event) => match event {
                Event::EndDocument => {
                    break;
                }
                Event::StartComment => {
                    inside_comment = true;
                }
                Event::EndComment => {
                    inside_comment = false;
                }
                Event::Characters(c) => {}
                Event::Comment(c) => {
                    if inside_comment {
                        comments.push_str(c);
                        comments.push_str(",");
                    }
                }

                Event::Cdata(d) => {
                    if inside_cdata {
                        cdatas.push_str(d);
                        cdatas.push_str(",");
                    }
                }
                Event::StartCdataSection => {
                    inside_cdata = true;
                }
                Event::EndCdataSection => {
                    inside_cdata = false;
                }

                _ => {}
            },

            Err(_err) => {
                break;
            }
        }
    }

    let comments_expected = r#" This is a comment ,comments can be,here,here,and here,"#;
    let cdatas_expected = r#"abc,<&>, ]] & < >  ,"#;
    // println!("{}", result);
    assert_eq!(comments, comments_expected);
    assert_eq!(cdatas, cdatas_expected);
}
