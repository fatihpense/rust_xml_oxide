use std::fs::File;

use xml_oxide::{parser::OxideParser, sax::Event};

extern crate xml_oxide;

#[test]
fn newer_parser() {
    let f: File = match File::open("tests/xml_files/books.xml") {
        Ok(file) => file,
        Err(e) => {
            println!("{}", e);
            panic!("file error");
        }
    };
    let mut p = OxideParser::start(f);
    let mut result: String = String::new();
    loop {
        let res = p.read_event();
        // println!("{:?}", res);
        result.push_str(&format!("{:?}\n", res));
        match res {
            Event::EndDocument => {
                break;
            }
            _ => {}
        }
    }

    let expected = r#"StartDocument
StartElement(StartElement { name: "fp:books", attributes: [Attribute { value: "http://github.com/fatihpense", name: "xmlns:fp" }], is_empty: false })
Characters("\r\n    ")
StartElement(StartElement { name: "fp:book", attributes: [Attribute { value: "true", name: "fp:archive" }, Attribute { value: "true", name: "fp:read" }, Attribute { value: "false", name: "fp:gifted" }], is_empty: false })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:title_english", attributes: [], is_empty: false })
Characters("Madonna in a Fur Coat")
EndElement(EndElement { name: "fp:title_english" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:title_original", attributes: [], is_empty: false })
Characters("Kürk Mantolu Madonna")
EndElement(EndElement { name: "fp:title_original" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:author", attributes: [], is_empty: false })
Characters("Sabahattin Ali")
EndElement(EndElement { name: "fp:author" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:quote_english", attributes: [], is_empty: false })
Characters("It is, perhaps, easier to dismiss a man whose face gives no indication of an inner life. And what a pity that is: a dash of curiosity is all it takes to stumble upon treasures we never expected.")
EndElement(EndElement { name: "fp:quote_english" })
Characters("\r\n        ")
StartElement(StartElement { name: "fp:original_language", attributes: [], is_empty: false })
Characters("tr")
EndElement(EndElement { name: "fp:original_language" })
Characters("\r\n    ")
EndElement(EndElement { name: "fp:book" })
Characters("\r\n")
EndElement(EndElement { name: "fp:books" })
Characters("\r\n")
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
    let mut p = OxideParser::start(f);
    let mut comments: String = String::new();
    let mut cdatas: String = String::new();

    let mut inside_comment = false;
    let mut inside_cdata = false;

    loop {
        let res = p.read_event();

        match res {
            Event::EndDocument => {
                break;
            }
            Event::StartComment => {
                inside_comment = true;
            }
            Event::EndComment => {
                inside_comment = false;
            }
            Event::Characters(c) => {
                if inside_comment {
                    comments.push_str(c);
                    comments.push_str(",");
                }
                if inside_cdata {
                    cdatas.push_str(c);
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
        }
    }

    let comments_expected = r#" This is a comment ,"#;
    let cdatas_expected = r#"abc,<&>,"#;
    // println!("{}", result);
    assert_eq!(comments, comments_expected);
    assert_eq!(cdatas, cdatas_expected);
}
