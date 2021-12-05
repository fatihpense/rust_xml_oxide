extern crate xml_oxide;
extern crate xml_sax;

use std::char;

use std::time::{Duration, Instant};

use xml_oxide::parser3::OxideParser;
use xml_sax::EndElement;

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

fn collect_with_parser<R: std::io::Read>(f: R) -> MyCollectorSaxHandler {
    let mut data = MyCollectorSaxHandler {
        start_counter: 0,
        end_counter: 0,
        char_counter: 0,
        start_el_name_vec: Vec::new(),
        end_el_name_vec: Vec::new(),
        characters_collected_vec: Vec::new(),
        characters_buf: String::new(),
    };

    let mut p = OxideParser::start(f);

    loop {
        let res = p.read_event();

        match res {
            xml_sax::Event::StartElement(el) => {
                data.start_counter = data.start_counter + 1;
                data.start_el_name_vec.push(el.name.to_owned());
                for attr in el.attributes.iter() {}
            }
            xml_sax::Event::EndElement(el) => {
                data.end_counter += 1;
                data.end_el_name_vec.push(el.name.to_owned());
            }
            xml_sax::Event::EndDocument => {
                break;
            }
            xml_sax::Event::Characters(chars) => {
                data.characters_buf.push_str(chars);
            }

            _ => {}
        }
    }

    data
}

#[test]
fn test_basic() {
    let mut s = String::from("<rootEl><value>5</value></rootEl>");

    let data = collect_with_parser(s.as_bytes());

    assert_eq!(data.start_counter, 2);
    assert_eq!(data.end_counter, 2);
    assert_eq!(data.start_el_name_vec.get(0).unwrap(), "rootEl");
    println!("{:?}", data.characters_collected_vec);
    assert_eq!(data.end_el_name_vec.get(0).unwrap(), "value");

    assert_eq!(data.characters_collected_vec.get(0).unwrap(), "5");
    // assert_eq!(my_sax_handler.end, );
}

#[test]
fn test_66_EntityRef() {
    let c = char::from_u32(60).unwrap();
    // println!("{}", c);
    assert_eq!(c, '<'); //8898  &#x022C2;    60 &#x0003C;

    let c2 = char::from_u32(u32::from_str_radix("0003C", 16).unwrap()).unwrap();
    // println!("{}", c2);
    assert_eq!(c2, '<'); //8898  &#x022C2;    60 &#x0003C;

    let mut s = String::from("<rootEl><value>1&lt;2&#60;3&#x0003C;4</value></rootEl>");

    let data = collect_with_parser(s.as_bytes());

    assert_eq!(data.characters_collected_vec.get(0).unwrap(), "1<2<3<4");
    // assert_eq!(my_sax_handler.end, );
}

#[test]
fn test_18_CDATA() {
    let mut s = String::from(
        "<rootEl>1&lt;2&#60;3&#x0003C;4<![CDATA[ \
         1&lt;2&#60;3&#x0003C;4]]><![CDATA[]]></rootEl>",
    );
    let data = collect_with_parser(s.as_bytes());

    println!("{}", data.characters_collected_vec.get(0).unwrap());
    assert_eq!(
        data.characters_collected_vec.get(0).unwrap(),
        "1<2<3<4 1&lt;2&#60;3&#x0003C;4"
    );
}

// comments should be ignored in content handler
#[test]
fn test_15_Comment() {
    let mut s = String::from("<rootEl>comments<!--are ignored--><!---->.</rootEl>");
    let data = collect_with_parser(s.as_bytes());

    println!("{}", data.characters_collected_vec.get(0).unwrap());
    assert_eq!(data.characters_collected_vec.get(0).unwrap(), "comments.");
}

#[test]
#[should_panic]
fn test_15_Comment_not_well_formed() {
    let mut s = String::from(
        "<rootEl>comments<!-- are not well formed with 3 \
         hyphen at the end unless it is empty---><!---->.</rootEl>",
    );
    let data = collect_with_parser(s.as_bytes());

    println!("{}", data.characters_collected_vec.get(0).unwrap());
    assert_eq!(data.characters_collected_vec.get(0).unwrap(), "comments.");
}
