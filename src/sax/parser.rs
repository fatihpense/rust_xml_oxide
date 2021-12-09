use nom::Offset;

use crate::{
    sax as xml_sax,
    sax::internal::{
        content_relaxed, insidecdata, insidecomment, misc, misc_before_doctype,
        misc_before_xmldecl, Attribute2, AttributeRange, ContentRelaxed, InsideCdata,
        InsideComment, Misc, MiscBeforeDoctype, MiscBeforeXmlDecl, QName, SAXAttribute2,
    },
};

enum InternalSuccess<'a> {
    StartDocument,
    EndDocument,

    ContentRelaxed(ContentRelaxed<'a>),
    InsideCdata(InsideCdata<'a>),
    InsideComment(InsideComment<'a>),
    Misc(Misc<'a>),
    MiscBeforeDoctype(MiscBeforeDoctype<'a>),
    MiscBeforeXmlDecl(MiscBeforeXmlDecl<'a>),
}

use std::{
    borrow::BorrowMut,
    cell::RefCell,
    io::{BufRead, BufReader, Read, Write},
    ops::Range,
    vec,
};

use super::{circular, Attribute};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParserState {
    Initial,
    DocStartBeforeXmlDecl, // when xmldecl parsed move to DocStartBeforeDocType, if something else parsed(including whitespace) the same!
    // DocStartBeforeXmlDeclInsideComment, // not possible - this means that doc doesn't have xmldecl, move to DocStartBeforeDocType
    DocStartBeforeDocType,              //when doctype parsed move to docstart
    DocStartBeforeDocTypeInsideComment, // this doesn't mean that doc doesn't have doctype, move to DocStartBeforeDocType

    DocStart,
    DocStartInsideComment,

    Content,
    InsideCdata,
    InsideComment, //can be at the start or end of the document? specified all

    DocEnd, //misc
    DocEndInsideComment,
}

struct Namespace {
    level: usize,
    prefix: Range<usize>,
    value: Range<usize>,
}
pub struct Parser<R: Read> {
    state: ParserState,
    bufreader: BufReader<R>,
    buffer3: circular::Buffer,

    strbuffer: String,
    offset: usize,

    // document_complete: bool, //if element_level reaches 0 again , we control this via state
    element_level: usize,
    element_strbuffer: String,
    element_list: Vec<Range<usize>>,

    is_namespace_aware: bool,
    namespace_strbuffer: String,
    namespace_list: Vec<Namespace>,

    attribute_list: Vec<AttributeRange>,
}

pub(crate) fn convert_attribute_range<'a>(
    strbuffer: &'a str,
    namespace_strbuffer: &'a str,
    range: AttributeRange,
) -> Attribute<'a> {
    Attribute {
        value: &strbuffer[range.value],
        name: &strbuffer[range.name],
        local_name: &strbuffer[range.local_name],
        prefix: &strbuffer[range.prefix],
        namespace: &namespace_strbuffer[range.namespace],
    }
}

fn convert_start_element_name_and_add_attributes<'a>(
    strbuffer: &'a mut String,
    namespace_strbuffer: &'a mut String,

    event1: crate::sax::internal::StartElement,
    buffer3: &circular::Buffer,
    attribute_list: &'a mut Vec<AttributeRange>,
) -> SaxResult<Range<usize>> {
    attribute_list.clear();

    let start = strbuffer.len();
    let size = event1.name.len();
    let element_name_range = start..start + size;
    strbuffer.push_str(event1.name);

    // let mut attributes2: Vec<SAXAttribute2> = vec![];

    let start = strbuffer.len();
    let size = event1.attributes_chunk.len();
    let attributes_chunk = unsafe { std::str::from_utf8_unchecked(event1.attributes_chunk) };
    strbuffer.push_str(attributes_chunk);

    let mut inp = strbuffer[start..start + size].as_bytes();
    let mut offset1: usize = start;
    //parse key,value and how many attributes.
    loop {
        if inp.len() == 0 {
            break;
        }

        let res = Attribute2(inp);

        match res {
            Ok((remainder, mut attr_range)) => {
                attr_range.name =
                    (attr_range.name.start + offset1)..(attr_range.name.end + offset1);
                attr_range.value =
                    (attr_range.value.start + offset1)..(attr_range.value.end + offset1);

                offset1 += inp.offset(remainder);
                inp = remainder;

                attribute_list.push(attr_range)
            }
            Err(_e) => {
                return Err(error::Error::Parsing(format!(
                    "Error while parsing attributes.",
                )))
            }
        }
    }

    Ok(element_name_range)
}

struct ElementRange {
    prefix_range: Range<usize>,
    local_name_range: Range<usize>,
    namespace_range: Range<usize>,
}

fn parse_start_element(
    start_element_name_range: Range<usize>,
    is_namespace_aware: bool,
    element_level: usize,

    strbuffer: &mut String,
    attribute_list: &mut Vec<AttributeRange>,
    namespace_strbuffer: &mut String,
    namespace_list: &mut Vec<Namespace>,
) -> SaxResult<ElementRange> {
    let start_element_name = &strbuffer[start_element_name_range];

    // let mut element_local_name = "";
    // let mut element_namespace = "";
    // let mut element_prefix = "";

    let mut prefix_range = 0..0;
    let mut local_name_range = 0..0;
    let mut namespace_range = 0..0;

    // add namespaces
    if is_namespace_aware {
        //first process namespace definitions & parse prefix:local_name
        for attr in attribute_list.iter_mut() {
            let inp = strbuffer[attr.name.clone()].as_bytes();

            match QName(inp) {
                Ok(qres) => {
                    let qname = qres.1;

                    if qname.prefix == "" && qname.local_name == "xmlns" {
                        //set default namespace
                        let ns = push_ns_values_get_ns(
                            namespace_strbuffer,
                            "",
                            &strbuffer[attr.value.clone()],
                            element_level,
                        );
                        namespace_list.push(ns);
                    }

                    if qname.prefix == "xmlns" {
                        //set prefixed namespace
                        let prefix = qname.local_name;
                        let ns = push_ns_values_get_ns(
                            namespace_strbuffer,
                            prefix,
                            &strbuffer[attr.value.clone()],
                            element_level,
                        );
                        namespace_list.push(ns);
                    }
                    attr.local_name = Range {
                        start: qname.local_name_range.start + attr.name.start.clone(),
                        end: qname.local_name_range.end + attr.name.start.clone(),
                    };
                    // println!("TEST: {:?}", &strbuffer[attr.local_name.clone()]);
                    attr.prefix = Range {
                        start: qname.prefix_range.start + attr.name.start.clone(),
                        end: qname.prefix_range.end + attr.name.start.clone(),
                    };
                    // let range_local_name = push_str_get_range(
                    //     &mut strbuffer,
                    //     qname.local_name,
                    // );
                    // attr.local_name = &strbuffer[range_local_name];
                }
                Err(_e) => {
                    return Err(error::Error::Parsing(format!(
                        "Attribute does not conform to QName spec: {}",
                        &strbuffer[attr.name.clone()]
                    )))
                }
            }
        }

        //resolve namespaces for element and attributes.

        for attr in attribute_list.iter_mut() {
            //Default namespace doesn't apply to attributes
            if &strbuffer[attr.prefix.clone()] == "" || &strbuffer[attr.prefix.clone()] == "xmlns" {
                continue;
            }
            match namespace_list.iter().rfind(|ns| {
                &namespace_strbuffer[ns.prefix.clone()] == &strbuffer[attr.prefix.clone()]
            }) {
                Some(ns) => attr.namespace = ns.value.clone(),
                None => {
                    return Err(error::Error::Parsing(format!(
                        "Namespace not found for prefix: {} , attribute: {} , element: {}",
                        &strbuffer[attr.prefix.clone()],
                        &strbuffer[attr.name.clone()],
                        start_element_name
                    )))
                }
            }
        }

        match QName(start_element_name.as_bytes()) {
            Ok(qres) => {
                let qname = qres.1;
                // element_local_name = qname.local_name;
                // element_prefix = qname.prefix;
                local_name_range = qname.local_name_range;
                prefix_range = qname.prefix_range;

                match namespace_list.iter().rfind(|ns| {
                    &namespace_strbuffer[ns.prefix.clone()] == &strbuffer[prefix_range.clone()]
                }) {
                    Some(ns) => namespace_range = ns.value.clone(),

                    None => {
                        if &strbuffer[prefix_range.clone()] == "" {
                            //it is fine
                        } else {
                            return Err(error::Error::Parsing(format!(
                                "Namespace prefix not found for element: {}",
                                start_element_name
                            )));
                        }
                    }
                }
            }
            Err(_e) => {
                return Err(error::Error::Parsing(format!(
                    "Element name does not conform to QName spec: {}",
                    start_element_name
                )))
            }
        }
    }

    Ok(ElementRange {
        prefix_range: prefix_range,
        local_name_range: local_name_range,
        namespace_range: namespace_range,
    })
}

fn push_str_get_range(strbuffer: &mut String, addition: &str) -> Range<usize> {
    let start = strbuffer.len();
    let size = addition.len();
    let range = Range {
        start: start,
        end: start + size,
    };
    strbuffer.push_str(addition);
    range
}

fn push_ns_values_get_ns(
    namespace_strbuffer: &mut String,
    prefix: &str,
    value: &str,
    element_level: usize,
) -> Namespace {
    let range_prefix = push_str_get_range(namespace_strbuffer, prefix);
    let range_value = push_str_get_range(namespace_strbuffer, value);
    Namespace {
        level: element_level,
        prefix: range_prefix,
        value: range_value,
    }
}

pub type SaxResult<T> = Result<T, error::Error>;

mod error {
    use thiserror::Error;
    #[derive(Debug, Error)]
    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),

        // Generic
        #[error("SAX Parsing Err: {0}")]
        Parsing(String),

        #[error("SAX Parsing Err: Unexpected EOF")]
        UnexpectedEof,
    }
}

// https://doc.rust-lang.org/nomicon/borrow-splitting.html
fn read_data_splitted<R: Read>(
    bufreader: &mut BufReader<R>,
    buffer2: &mut Vec<u8>,
) -> Result<(), std::io::Error> {
    match bufreader.fill_buf() {
        Ok(_ok) => {}
        Err(err) => return Err(err),
    }

    let amt: usize;
    {
        let data2 = bufreader.buffer();

        buffer2.extend_from_slice(data2);
        amt = data2.len();
    }
    bufreader.consume(amt);
    Ok(())
}
fn read_data_splitted_refcell<R: Read>(
    bufreader: &mut BufReader<R>,
    buffer2: &RefCell<Vec<u8>>,
) -> Result<(), std::io::Error> {
    match bufreader.fill_buf() {
        Ok(_ok) => {}
        Err(err) => return Err(err),
    }

    let amt: usize;
    {
        let data2 = bufreader.buffer();

        buffer2.borrow_mut().extend_from_slice(data2);
        amt = data2.len();
    }
    bufreader.consume(amt);
    Ok(())
}

//todo move all states to read_event_splitted
//todo? simplify the enum here to remove duplicates,then we move complexity to read_event method
fn event_converter<'a, 'b>(
    mut state: ParserState,
    internal_event: InternalSuccess<'b>,
    buffer3: &'b circular::Buffer,

    element_list: &mut Vec<Range<usize>>,
    mut strbuffer: &'a mut String,
    mut namespace_strbuffer: &'a mut String,
    namespace_list: &mut Vec<Namespace>,

    is_namespace_aware: bool,
    mut element_level: usize,
    mut element_strbuffer: &mut String,

    attribute_list: &'a mut Vec<AttributeRange>,
) -> SaxResult<(xml_sax::Event<'a>, ParserState, usize)> {
    let event = match internal_event {
        InternalSuccess::StartDocument => xml_sax::Event::StartDocument,
        InternalSuccess::EndDocument => xml_sax::Event::EndDocument,
        InternalSuccess::ContentRelaxed(cr) => match cr {
            ContentRelaxed::CharData(event1) => {
                let start = strbuffer.len();
                let size = event1.len();
                strbuffer.push_str(unsafe { std::str::from_utf8_unchecked(event1) });
                xml_sax::Event::Characters(&strbuffer[start..(start + size)])
            }
            ContentRelaxed::StartElement(event1) => {
                //todo decode

                if is_namespace_aware {
                    // clear up namespaces
                    match namespace_list
                        .iter()
                        .rposition(|ns| ns.level <= element_level)
                    {
                        Some(pos) => {
                            if let Some(starting_pos) =
                                namespace_list.get(pos + 1).map(|ns| ns.prefix.start)
                            {
                                namespace_list.truncate(pos + 1);
                                namespace_strbuffer.truncate(starting_pos);
                            }
                        }
                        None => {
                            // nothing to remove
                        }
                    }
                }

                let start_element_name_range = convert_start_element_name_and_add_attributes(
                    strbuffer,
                    namespace_strbuffer,
                    event1,
                    buffer3,
                    attribute_list,
                )?;

                element_level += 1;

                //add element to list for expected tags check

                let element_list_range = push_str_get_range(
                    &mut element_strbuffer,
                    &strbuffer[start_element_name_range.clone()],
                );
                element_list.push(element_list_range.clone());

                // let mut element_local_name = "";
                // let mut element_namespace = "";
                // let mut element_prefix = "";
                let element_ranges = parse_start_element(
                    start_element_name_range.clone(),
                    is_namespace_aware,
                    element_level,
                    strbuffer,
                    attribute_list,
                    namespace_strbuffer,
                    namespace_list,
                )?;

                let start_element = xml_sax::StartElement {
                    name: &strbuffer[start_element_name_range],
                    // attributes: attributes,
                    is_empty: false,

                    local_name: &strbuffer[element_ranges.local_name_range],
                    namespace: &namespace_strbuffer[element_ranges.namespace_range],
                    prefix: &strbuffer[element_ranges.prefix_range],

                    range_list: attribute_list,
                    strbuffer: strbuffer,
                    namespace_strbuffer: namespace_strbuffer,
                };

                xml_sax::Event::StartElement(start_element)
            }
            ContentRelaxed::EmptyElemTag(event1) => {
                if is_namespace_aware {
                    // clear up namespaces
                    match namespace_list
                        .iter()
                        .rposition(|ns| ns.level <= element_level)
                    {
                        Some(pos) => {
                            if let Some(starting_pos) =
                                namespace_list.get(pos + 1).map(|ns| ns.prefix.start)
                            {
                                namespace_list.truncate(pos + 1);
                                namespace_strbuffer.truncate(starting_pos);
                            }
                        }
                        None => {
                            // nothing to remove
                        }
                    }
                }

                let start_element_name_range = convert_start_element_name_and_add_attributes(
                    strbuffer,
                    namespace_strbuffer,
                    event1,
                    buffer3,
                    attribute_list,
                )?;

                element_level += 1; // this is important before namespace handling

                // element_list_range is not important for empty element tag

                let element_ranges = parse_start_element(
                    start_element_name_range.clone(),
                    is_namespace_aware,
                    element_level,
                    strbuffer,
                    attribute_list,
                    namespace_strbuffer,
                    namespace_list,
                )?;

                let start_element = xml_sax::StartElement {
                    name: &strbuffer[start_element_name_range],
                    // attributes: attributes,
                    is_empty: false,

                    local_name: &strbuffer[element_ranges.local_name_range],
                    namespace: &namespace_strbuffer[element_ranges.namespace_range],
                    prefix: &strbuffer[element_ranges.prefix_range],

                    range_list: attribute_list,
                    strbuffer: strbuffer,
                    namespace_strbuffer: namespace_strbuffer,
                };

                // element_level -= 1;
                // if element_level == 0 {
                //     //could be a root only document.
                //     state = ParserState::DocEnd;
                // }

                element_level -= 1;
                if element_level == 0 {
                    state = ParserState::DocEnd;
                }

                xml_sax::Event::StartElement(start_element)
            }
            ContentRelaxed::EndElement(event1) => {
                //todo: check if it is the expected tag

                match element_list.pop() {
                    Some(r) => {
                        if &element_strbuffer[r.clone()] == event1.name {
                            element_strbuffer.truncate(r.start);
                        } else {
                            return Err(error::Error::Parsing(format!(
                                "Expected closing tag: {} ,found: {}",
                                &element_strbuffer[r.clone()],
                                event1.name
                            )));

                            // TODO Expected closing tag: ... &element_strbuffer[r.clone()] found event1.name
                        }
                    }
                    None => {
                        return Err(error::Error::Parsing(format!(
                            "No starting tag for: {}",
                            event1.name
                        )))
                    }
                }

                if is_namespace_aware {
                    // clear up namespaces
                    match namespace_list
                        .iter()
                        .rposition(|ns| ns.level <= element_level)
                    {
                        Some(pos) => {
                            if let Some(starting_pos) =
                                namespace_list.get(pos + 1).map(|ns| ns.prefix.start)
                            {
                                namespace_list.truncate(pos + 1);
                                namespace_strbuffer.truncate(starting_pos);
                            }
                        }
                        None => {
                            // nothing to remove
                        }
                    }
                }

                // let range = push_str_get_range(
                //     &mut element_strbuffer,
                //     start_element.name,
                // );
                // element_list.push(range);

                let start = strbuffer.len();
                let size = event1.name.len();
                strbuffer.push_str(event1.name);
                let mut end_element = xml_sax::EndElement {
                    name: &strbuffer[start..(start + size)],
                    local_name: "",
                    prefix: "",
                    namespace: "",
                };

                element_level -= 1;
                if element_level == 0 {
                    state = ParserState::DocEnd;
                }

                if is_namespace_aware {
                    match QName(end_element.name.as_bytes()) {
                        Ok(qres) => {
                            let qname = qres.1;
                            end_element.local_name = qname.local_name;
                            end_element.prefix = qname.prefix;

                            match namespace_list.iter().rfind(|ns| {
                                &namespace_strbuffer[ns.prefix.clone()] == end_element.prefix
                            }) {
                                Some(ns) => {
                                    end_element.namespace = &namespace_strbuffer[ns.value.clone()]
                                }
                                None => {
                                    if end_element.prefix == "" {
                                        //it is fine
                                    } else {
                                        return Err(error::Error::Parsing(format!(
                                            "Namespace prefix not found for element: {}",
                                            end_element.name
                                        )));
                                    }
                                }
                            }
                        }
                        Err(_e) => {
                            return Err(error::Error::Parsing(format!(
                                "Element name does not conform to QName spec: {}",
                                end_element.name
                            )))
                        }
                    }
                }
                xml_sax::Event::EndElement(end_element)
            }
            ContentRelaxed::Reference(event1) => {
                // let start = strbuffer.len();
                // let size = event1.initial.len();
                // let range_initial = Range {
                //     start: start,
                //     end: start + size,
                // };
                // strbuffer.push_str(event1.initial);

                let range: Range<usize> = push_str_get_range(&mut strbuffer, event1.initial);

                //we handle the case when it is a character, not a string reference
                let raw = event1.initial;
                let resolved_char: Option<char>;
                if raw.starts_with("&#x") {
                    let hex_val = &raw[3..raw.len() - 1];

                    resolved_char = match u32::from_str_radix(&hex_val, 16) {
                        Ok(a) => match char::from_u32(a) {
                            Some(c) => Some(c),
                            None => None,
                        },
                        Err(_) => None,
                    }
                } else if raw.starts_with("&#") {
                    let hex_val = &raw[2..raw.len() - 1];

                    resolved_char = match u32::from_str_radix(&hex_val, 10) {
                        Ok(a) => match char::from_u32(a) {
                            Some(c) => Some(c),
                            None => None,
                        },
                        Err(_) => None,
                    }
                } else {
                    resolved_char = match event1.initial {
                        // we don't need .as_ref() or &* as it is not String -> https://github.com/rust-lang/rust/issues/28606
                        "&amp;" => Some('&'),
                        "&lt;" => Some('<'),
                        "&gt;" => Some('>'),
                        "&quot;" => Some('"'),
                        "&apos;" => Some('\''),
                        _ => None,
                    }
                }

                let range_resolved: Option<Range<usize>> = match resolved_char {
                    Some(ch) => {
                        let mut tmp = [0u8; 4];
                        let addition = ch.encode_utf8(&mut tmp);
                        Some(push_str_get_range(&mut strbuffer, addition))
                    }
                    None => None,
                    // &* -> https://github.com/rust-lang/rust/issues/28606
                    // "&amp;" => Some(push_str_get_range(&mut strbuffer, "&")),
                    // "&lt;" => Some(push_str_get_range(&mut strbuffer, "<")),
                    // "&gt;" => Some(push_str_get_range(&mut strbuffer, ">")),
                    // "&quot;" => Some(push_str_get_range(&mut strbuffer, "\"")),
                    // "&apos;" => Some(push_str_get_range(&mut strbuffer, "'")),
                    // _ => None,
                };

                //we are ignoring DTD entity refs

                let reference_event = xml_sax::Reference {
                    raw: &strbuffer[range],
                    resolved: match range_resolved {
                        Some(range) => Some(&strbuffer[range]),
                        None => None,
                    },
                };

                xml_sax::Event::Reference(reference_event)
            }
            ContentRelaxed::CdataStart => xml_sax::Event::StartCdataSection,
            ContentRelaxed::CommentStart => xml_sax::Event::StartComment,
        },
        InternalSuccess::InsideCdata(ic) => match ic {
            InsideCdata::Characters(characters) => {
                let start = strbuffer.len();
                let size = characters.len();
                strbuffer.push_str(unsafe { std::str::from_utf8_unchecked(characters) });
                xml_sax::Event::Cdata(&strbuffer[start..(start + size)])
            }
            InsideCdata::CdataEnd => xml_sax::Event::EndCdataSection,
        },
        InternalSuccess::InsideComment(ic) => match ic {
            InsideComment::Characters(characters) => {
                let start = strbuffer.len();
                let size = characters.len();
                strbuffer.push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                xml_sax::Event::Comment(&strbuffer[start..(start + size)])
            }
            InsideComment::CommentEnd => xml_sax::Event::EndComment,
        },
        InternalSuccess::Misc(misc) => match misc {
            Misc::PI(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::ProcessingInstruction(&strbuffer[range])
            }
            Misc::Whitespace(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::Whitespace(&strbuffer[range])
            }
            Misc::CommentStart => xml_sax::Event::StartComment,
        },
        InternalSuccess::MiscBeforeDoctype(misc) => match misc {
            MiscBeforeDoctype::PI(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::ProcessingInstruction(&strbuffer[range])
            }
            MiscBeforeDoctype::Whitespace(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::Whitespace(&strbuffer[range])
            }
            MiscBeforeDoctype::CommentStart => xml_sax::Event::StartComment,
            MiscBeforeDoctype::DocType(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::DocumentTypeDeclaration(&strbuffer[range])
            }
        },
        InternalSuccess::MiscBeforeXmlDecl(misc) => match misc {
            MiscBeforeXmlDecl::XmlDecl(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::XmlDeclaration(&strbuffer[range])
            }
            MiscBeforeXmlDecl::PI(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::ProcessingInstruction(&strbuffer[range])
            }
            MiscBeforeXmlDecl::Whitespace(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::Whitespace(&strbuffer[range])
            }
            MiscBeforeXmlDecl::CommentStart => xml_sax::Event::StartComment,
            MiscBeforeXmlDecl::DocType(a) => {
                let str = unsafe { std::str::from_utf8_unchecked(a) };
                let range = push_str_get_range(&mut strbuffer, &str);
                xml_sax::Event::DocumentTypeDeclaration(&strbuffer[range])
            }
        },
    };
    Ok((event, state, element_level))
}

fn read_event_splitted<'a, 'b, R: Read>(
    mut state: ParserState,

    bufreader: &BufReader<R>,

    buffer3: &'b circular::Buffer,

    mut offset: usize,
    // document_complete: bool, //if element_level reaches 0 again , we control this via state
) -> SaxResult<(InternalSuccess<'b>, ParserState, usize)> {
    let event2: InternalSuccess;
    match state {
        ParserState::Initial => {
            state = ParserState::DocStartBeforeXmlDecl;
            return Ok((InternalSuccess::StartDocument, state, offset));
        }
        ParserState::DocStartBeforeXmlDecl => {
            let res = misc_before_xmldecl(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);
                    state = ParserState::DocStartBeforeDocType;

                    match parseresult.1 {
                        MiscBeforeXmlDecl::XmlDecl(_a) => {}
                        MiscBeforeXmlDecl::PI(_a) => {}
                        MiscBeforeXmlDecl::Whitespace(_a) => {}
                        MiscBeforeXmlDecl::CommentStart => {
                            state = ParserState::DocStartBeforeDocTypeInsideComment;
                        }
                        MiscBeforeXmlDecl::DocType(_a) => {
                            state = ParserState::DocStart;
                        }
                    }
                    event2 = InternalSuccess::MiscBeforeXmlDecl(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    //try content!
                    state = ParserState::Content;
                    return read_event_splitted(state, bufreader, buffer3, offset);
                }
            }
        }
        ParserState::DocStartBeforeDocType => {
            let res = misc_before_doctype(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        MiscBeforeDoctype::PI(_a) => {}
                        MiscBeforeDoctype::Whitespace(_a) => {}
                        MiscBeforeDoctype::CommentStart => {
                            state = ParserState::DocStartBeforeDocTypeInsideComment;
                        }
                        MiscBeforeDoctype::DocType(_a) => {
                            state = ParserState::DocStart;
                        }
                    }
                    event2 = InternalSuccess::MiscBeforeDoctype(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    //try content!
                    state = ParserState::Content;
                    return read_event_splitted(state, bufreader, buffer3, offset);
                }
            }
        }
        ParserState::DocStartBeforeDocTypeInsideComment => {
            //expect comment or comment-end
            let res = insidecomment(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        InsideComment::Characters(_characters) => {}
                        InsideComment::CommentEnd => {
                            state = ParserState::DocStartBeforeDocType;
                        }
                    }
                    event2 = InternalSuccess::InsideComment(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(
                        "Expected Comment content or Comment end".to_owned(),
                    ))
                }
            }
        }
        ParserState::DocStart => {
            let res = misc(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);
                    // state = ParserState::DocStartBeforeDocType;

                    match parseresult.1 {
                        Misc::PI(_a) => {}
                        Misc::Whitespace(_a) => {}
                        Misc::CommentStart => {
                            state = ParserState::DocStartInsideComment;
                        }
                    }
                    event2 = InternalSuccess::Misc(parseresult.1);
                }

                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    //try content!
                    state = ParserState::Content;
                    return read_event_splitted(state, bufreader, buffer3, offset);
                }
            }
        }
        ParserState::DocStartInsideComment => {
            //expect comment or comment-end
            let res = insidecomment(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        InsideComment::Characters(_characters) => {}
                        InsideComment::CommentEnd => {
                            state = ParserState::DocStart;
                        }
                    }
                    event2 = InternalSuccess::InsideComment(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(format!(
                        "Expecting comment content or comment closing tag "
                    )))
                }
            }
        }
        ParserState::Content => {
            let res = content_relaxed(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match &parseresult.1 {
                        ContentRelaxed::CharData(_event1) => {}
                        ContentRelaxed::StartElement(_event1) => {}
                        ContentRelaxed::EmptyElemTag(_event1) => {}
                        ContentRelaxed::EndElement(_event1) => {}
                        ContentRelaxed::Reference(_event1) => {}
                        ContentRelaxed::CdataStart => {
                            state = ParserState::InsideCdata;
                        }
                        ContentRelaxed::CommentStart => {
                            state = ParserState::InsideComment;
                        }
                    }
                    event2 = InternalSuccess::ContentRelaxed(parseresult.1);
                }
                // let ending = String::from_utf8_lossy(&buffer2);
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_e) => {
                    let ending = String::from_utf8_lossy(&buffer3.data());
                    let ending_truncated = match ending.char_indices().nth(50) {
                        None => &ending,
                        Some((idx, _)) => &ending[..idx],
                    };

                    return Err(error::Error::Parsing(format!(
                        "Expected one of (CharData | element | Reference | CDSect | PI | Comment), found: {}",
                        ending_truncated
                    )));
                }
            }
        }

        ParserState::InsideCdata => {
            //expect cdata or cdata-end
            let res = insidecdata(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        InsideCdata::Characters(_characters) => {}
                        InsideCdata::CdataEnd => {
                            state = ParserState::Content;
                        }
                    }
                    event2 = InternalSuccess::InsideCdata(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(format!(
                        "Expecting CDATA content or CDATA closing tag "
                    )))
                }
            }
        }
        ParserState::InsideComment => {
            //expect comment or comment-end
            let res = insidecomment(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        InsideComment::Characters(_characters) => {}
                        InsideComment::CommentEnd => {
                            state = ParserState::Content;
                        }
                    }
                    event2 = InternalSuccess::InsideComment(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(format!(
                        "Expecting comment content or comment closing tag "
                    )))
                }
            }
        }
        ParserState::DocEnd => {
            // EOF
            if buffer3.data().len() == 0 {
                // event2 = xml_sax::Event::EndDocument;
                return Ok((InternalSuccess::EndDocument, state, offset));
            }

            let res = misc(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        Misc::PI(_a) => {}
                        Misc::Whitespace(_a) => {}
                        Misc::CommentStart => {
                            state = ParserState::DocEndInsideComment;
                        }
                    }
                    event2 = InternalSuccess::Misc(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(format!(
                        "Unexpected entity/content at the end of the document."
                    )))
                }
            }
        }
        ParserState::DocEndInsideComment => {
            //expect comment or comment-end
            let res = insidecomment(&buffer3.data());
            match res {
                Ok(parseresult) => {
                    offset = buffer3.data().offset(parseresult.0);

                    match parseresult.1 {
                        InsideComment::Characters(_characters) => {}
                        InsideComment::CommentEnd => {
                            state = ParserState::DocEnd;
                        }
                    }
                    event2 = InternalSuccess::InsideComment(parseresult.1);
                }
                Err(nom::Err::Incomplete(_e)) => {
                    return Err(error::Error::UnexpectedEof);
                }
                Err(_err) => {
                    return Err(error::Error::Parsing(format!(
                        "Expecting comment content or comment closing tag "
                    )))
                }
            }
        }
    }

    Ok((event2, state, offset))
}

impl<R: Read> Parser<R> {
    pub fn from_reader(reader: R) -> Parser<R> {
        Parser {
            state: ParserState::Initial,
            bufreader: BufReader::with_capacity(8 * 1024, reader),
            offset: 0,

            buffer3: circular::Buffer::with_capacity(16 * 1024),
            strbuffer: String::new(),

            element_level: 0, // should be same as self.element_list.len()
            element_list: Vec::with_capacity(10),
            element_strbuffer: String::new(),

            is_namespace_aware: true,
            namespace_list: Vec::with_capacity(10),
            namespace_strbuffer: String::new(),

            attribute_list: Vec::with_capacity(5),
        }
    }

    fn read_data(&mut self) -> Result<usize, std::io::Error> {
        let newread: usize;
        match self.bufreader.fill_buf() {
            Ok(ok) => {
                newread = ok.len();
            }
            Err(err) => return Err(err),
        }

        let amt: usize;
        {
            let data2 = self.bufreader.buffer();
            let data_len = data2.len();
            //is it bigger than available space?

            self.buffer3.shift();
            if data_len > self.buffer3.available_space() {
                let new_size = std::cmp::max(
                    self.buffer3.position() + data_len,
                    self.buffer3.capacity() * 2,
                );

                self.buffer3.grow(new_size);
            }

            // self.buffer2.borrow_mut().extend_from_slice(data2);
            // println!("buffer: {:?} , datalen: {:?}",self.buffer3.available_space(),data2.len());
            self.buffer3.write_all(data2).unwrap();
            // self.buffer3.spa
            amt = data2.len();
        }
        self.bufreader.consume(amt);

        Ok(newread)
    }

    // rust is not yet smart about loops, nll, structs, conditional lifetimes

    pub fn read_event<'a>(&'a mut self) -> SaxResult<xml_sax::Event<'a>> {
        self.buffer3.consume(self.offset);
        // self.buffer2.borrow_mut().drain(0..self.offset);
        self.offset = 0;
        // {
        //     let vec1;
        //     {
        //         vec1 = self.buffer2.borrow_mut().split_off(self.offset)
        //     }
        //     // let mut buf = self.buffer2.borrow_mut();
        //     *self.buffer2.borrow_mut() = vec1;
        // }

        self.strbuffer.clear();
        // read_data_splitted(&mut self.bufreader, &mut self.buffer2)?;
        // let event1;

        let mut bytes_read: usize = 1; //magic number

        // if self.bufreader.capacity() > self.buffer2.borrow().len() {
        if self.buffer3.available_space() > self.bufreader.capacity() {
            bytes_read = self.read_data()?;
        }

        let mut read_more_data = false;
        loop {
            if read_more_data {
                // read_data_splitted(&mut self.bufreader, &mut self.buffer2.borrow_mut())?;
                bytes_read = self.read_data()?;
                read_more_data = false;
            } else {
                let res =
                    read_event_splitted(self.state, &self.bufreader, &self.buffer3, self.offset);
                match res {
                    Ok(o) => {
                        self.state = o.1;
                        self.offset = o.2;

                        // event1 = o.0;

                        let event = event_converter(
                            self.state,
                            o.0,
                            &self.buffer3,
                            &mut self.element_list,
                            &mut self.strbuffer,
                            &mut self.namespace_strbuffer,
                            &mut self.namespace_list,
                            self.is_namespace_aware,
                            self.element_level,
                            &mut self.element_strbuffer,
                            &mut self.attribute_list,
                        );
                        match event {
                            Ok(tpl) => {
                                self.state = tpl.1;
                                self.element_level = tpl.2;

                                return Ok(tpl.0);
                            }
                            Err(err) => return Err(err),
                        };
                    }
                    Err(error::Error::UnexpectedEof) => {
                        //try reading again
                        // read_data_splitted_refcell(&mut self.bufreader, &self.buffer2)?;
                        if bytes_read == 0 {
                            return Err(error::Error::UnexpectedEof);
                        } else {
                            read_more_data = true;
                        }
                    }
                    Err(err) => {
                        //todo check eof increase internal buffer.
                        return Err(err);
                    }
                }
            }
        }
    }
}

#[test]
fn test_parser1() {
    let data = r#"<root><A a='x'>
    <B b="val" a:b12='val2' ><C/></B> </A> </root>"#
        .as_bytes();

    // let mut buf = vec![];
    let mut p = Parser::from_reader(data);
    loop {
        let res = p.read_event();
        println!("{:?}", res);
        match res {
            Ok(event) => match event {
                xml_sax::Event::StartDocument => {}
                xml_sax::Event::EndDocument => {
                    break;
                }
                xml_sax::Event::StartElement(_el) => {}
                xml_sax::Event::EndElement(_) => {}
                xml_sax::Event::Characters(_c) => {}
                xml_sax::Event::Reference(_c) => {}
                _ => {}
            },

            Err(_err) => {
                break;
            }
        }
    }
}
