use nom::Offset;

use crate::{
    internal::{
        content_relaxed, insidecdata, insidecomment, misc, misc_before_doctype,
        misc_before_xmldecl, ContentRelaxed, InsideCdata, InsideComment, Misc, MiscBeforeDoctype,
        MiscBeforeXmlDecl, QName, SAXAttribute2,
    },
    sax as xml_sax,
};

use std::{
    io::{BufRead, BufReader, Read},
    ops::Range,
    vec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
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
    buffer2: Vec<u8>,
    strbuffer: String,
    offset: usize,

    // document_complete: bool, //if element_level reaches 0 again , we control this via state
    element_level: usize,
    element_strbuffer: String,
    element_list: Vec<Range<usize>>,

    is_namespace_aware: bool,
    namespace_strbuffer: String,
    namespace_list: Vec<Namespace>,
}

fn convert_start_element<'a>(
    strbuffer: &'a mut String,
    event1: crate::internal::StartElement,
) -> xml_sax::StartElement<'a> {
    let start = strbuffer.len();
    let size = event1.name.len();
    strbuffer.push_str(event1.name);

    let mut attributes2: Vec<SAXAttribute2> = vec![];
    for att in event1.attributes {
        let start = strbuffer.len();
        let size = att.qualified_name.len();
        strbuffer.push_str(att.qualified_name);
        let qualified_name_range = Range {
            start: start,
            end: start + size,
        };

        let start = strbuffer.len();
        let size = att.value.len();
        strbuffer.push_str(att.value);
        let value_range = Range {
            start: start,
            end: start + size,
        };

        // let qualified_name = &self.strbuffer[start..(start + size)];
        // let value = &self.strbuffer[start..(start + size)];

        attributes2.push(SAXAttribute2 {
            value: value_range,
            qualified_name: qualified_name_range,
        });
    }

    let mut attributes: Vec<xml_sax::Attribute> = vec![];
    for att in attributes2 {
        // let qualified_name = &self.strbuffer[start..(start + size)];
        // let value = &self.strbuffer[start..(start + size)];

        attributes.push(xml_sax::Attribute {
            value: &strbuffer[att.value],
            name: &strbuffer[att.qualified_name],
            local_name: "",
            namespace: "",
            prefix: "",
        });
    }

    xml_sax::StartElement {
        name: &strbuffer[start..(start + size)],
        attributes: attributes,
        is_empty: false,

        local_name: "",
        namespace: "",
        prefix: "",
    }
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
    }
}

impl<R: Read> Parser<R> {
    pub fn start(reader: R) -> Parser<R> {
        Parser {
            state: ParserState::Initial,
            bufreader: BufReader::with_capacity(8192, reader),
            offset: 0,
            buffer2: vec![],
            strbuffer: String::new(),

            element_level: 0, // should be same as self.element_list.len()
            element_list: vec![],
            element_strbuffer: String::new(),

            is_namespace_aware: true,
            namespace_list: vec![],
            namespace_strbuffer: String::new(),
        }
    }

    fn read_data(&mut self) -> Result<(), std::io::Error> {
        match self.bufreader.fill_buf() {
            Ok(_ok) => {}
            Err(err) => return Err(err),
        }

        let amt: usize;
        {
            let data2 = self.bufreader.buffer();

            self.buffer2.extend_from_slice(data2);
            amt = data2.len();
        }
        self.bufreader.consume(amt);
        Ok(())
    }

    // , buf: &'b [u8]
    pub fn read_event<'a>(&'a mut self) -> SaxResult<xml_sax::Event<'a>> {
        // self.bufreader.consume(self.offset);
        self.buffer2.drain(0..self.offset);
        self.offset = 0;
        self.strbuffer.clear();

        if self.bufreader.capacity() > self.buffer2.len() {
            self.read_data()?
        }

        // let mut event: StartElement = StartElement {
        //     name: "",
        //     attributes: vec![],
        // };
        // let mut event1: StartElement; //<'b>; //&'a
        let event2: xml_sax::Event;
        match self.state {
            ParserState::Initial => {
                self.state = ParserState::DocStartBeforeXmlDecl;
                return Ok(xml_sax::Event::StartDocument);
            }
            ParserState::DocStartBeforeXmlDecl => {
                let res = misc_before_xmldecl(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        self.state = ParserState::DocStartBeforeDocType;
                        match parseresult.1 {
                            MiscBeforeXmlDecl::XmlDecl(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 = xml_sax::Event::XmlDeclaration(&self.strbuffer[range])
                            }
                            MiscBeforeXmlDecl::PI(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::ProcessingInstruction(&self.strbuffer[range])
                            }
                            MiscBeforeXmlDecl::Whitespace(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 = xml_sax::Event::Whitespace(&self.strbuffer[range])
                            }
                            MiscBeforeXmlDecl::CommentStart => {
                                self.state = ParserState::DocStartBeforeDocTypeInsideComment;

                                event2 = xml_sax::Event::StartComment;
                            }
                            MiscBeforeXmlDecl::DocType(a) => {
                                self.state = ParserState::DocStart;

                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::DocumentTypeDeclaration(&self.strbuffer[range]);
                            }
                        }
                    }
                    Err(_err) => {
                        //try content!
                        self.state = ParserState::Content;
                        return self.read_event();
                    }
                }
            }
            ParserState::DocStartBeforeDocType => {
                let res = misc_before_doctype(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        // self.state = ParserState::DocStartBeforeDocType;
                        match parseresult.1 {
                            MiscBeforeDoctype::PI(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::ProcessingInstruction(&self.strbuffer[range])
                            }
                            MiscBeforeDoctype::Whitespace(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 = xml_sax::Event::Whitespace(&self.strbuffer[range])
                            }
                            MiscBeforeDoctype::CommentStart => {
                                self.state = ParserState::DocStartBeforeDocTypeInsideComment;

                                event2 = xml_sax::Event::StartComment;
                            }
                            MiscBeforeDoctype::DocType(a) => {
                                self.state = ParserState::DocStart;

                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::DocumentTypeDeclaration(&self.strbuffer[range]);
                            }
                        }
                    }
                    Err(_err) => {
                        //try content!
                        self.state = ParserState::Content;
                        return self.read_event();
                    }
                }
            }
            ParserState::DocStartBeforeDocTypeInsideComment => {
                //expect comment or comment-end
                let res = insidecomment(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            InsideComment::Characters(characters) => {
                                let start = self.strbuffer.len();
                                let size = characters.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                                event2 =
                                    xml_sax::Event::Comment(&self.strbuffer[start..(start + size)])
                            }
                            InsideComment::CommentEnd => {
                                self.state = ParserState::DocStartBeforeDocType;
                                event2 = xml_sax::Event::EndComment;
                            }
                        }
                    }
                    Err(_err) => {
                        return Err(error::Error::Parsing(
                            "Expected Comment content or Comment end".to_owned(),
                        ))
                    }
                }
            }
            ParserState::DocStart => {
                let res = misc(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        // self.state = ParserState::DocStartBeforeDocType;
                        match parseresult.1 {
                            Misc::PI(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::ProcessingInstruction(&self.strbuffer[range])
                            }
                            Misc::Whitespace(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 = xml_sax::Event::Whitespace(&self.strbuffer[range])
                            }
                            Misc::CommentStart => {
                                self.state = ParserState::DocStartInsideComment;

                                event2 = xml_sax::Event::StartComment;
                            }
                        }
                    }
                    Err(_err) => {
                        //try content!
                        self.state = ParserState::Content;
                        return self.read_event();
                    }
                }
            }
            ParserState::DocStartInsideComment => {
                //expect comment or comment-end
                let res = insidecomment(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            InsideComment::Characters(characters) => {
                                let start = self.strbuffer.len();
                                let size = characters.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                                event2 =
                                    xml_sax::Event::Comment(&self.strbuffer[start..(start + size)])
                            }
                            InsideComment::CommentEnd => {
                                self.state = ParserState::DocStart;
                                event2 = xml_sax::Event::EndComment;
                            }
                        }
                    }
                    Err(_err) => {
                        return Err(error::Error::Parsing(format!(
                            "Expecting comment content or comment closing tag "
                        )))
                    }
                }
            }
            ParserState::Content => {
                let res = content_relaxed(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);

                        match parseresult.1 {
                            ContentRelaxed::CharData(event1) => {
                                let start = self.strbuffer.len();
                                let size = event1.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(event1) });

                                event2 = xml_sax::Event::Characters(
                                    &self.strbuffer[start..(start + size)],
                                )
                            }
                            ContentRelaxed::StartElement(event1) => {
                                //todo decode

                                if self.is_namespace_aware {
                                    // clear up namespaces
                                    match self
                                        .namespace_list
                                        .iter()
                                        .rposition(|ns| ns.level <= self.element_level)
                                    {
                                        Some(pos) => {
                                            if let Some(starting_pos) = self
                                                .namespace_list
                                                .get(pos + 1)
                                                .map(|ns| ns.prefix.start)
                                            {
                                                self.namespace_list.truncate(pos + 1);
                                                self.namespace_strbuffer.truncate(starting_pos);
                                            }
                                        }
                                        None => {
                                            // nothing to remove
                                        }
                                    }
                                }

                                let mut start_element =
                                    convert_start_element(&mut self.strbuffer, event1);
                                self.element_level += 1;

                                //add element to list for expected tags check

                                let range = push_str_get_range(
                                    &mut self.element_strbuffer,
                                    start_element.name,
                                );
                                self.element_list.push(range);
                                // add namespaces
                                if self.is_namespace_aware {
                                    //first process namespace definitions
                                    for attr in start_element.attributes.iter_mut() {
                                        match QName(attr.name.as_bytes()) {
                                            Ok(qres) => {
                                                let qname = qres.1;

                                                if qname.prefix == "" && qname.local_name == "xmlns"
                                                {
                                                    //set default namespace
                                                    let ns = push_ns_values_get_ns(
                                                        &mut self.namespace_strbuffer,
                                                        "",
                                                        attr.value,
                                                        self.element_level,
                                                    );
                                                    self.namespace_list.push(ns);
                                                }

                                                if qname.prefix == "xmlns" {
                                                    //set prefixed namespace
                                                    let prefix = qname.local_name;
                                                    let ns = push_ns_values_get_ns(
                                                        &mut self.namespace_strbuffer,
                                                        prefix,
                                                        attr.value,
                                                        self.element_level,
                                                    );
                                                    self.namespace_list.push(ns);
                                                }
                                                attr.local_name = qname.local_name;
                                                attr.prefix = qname.prefix;
                                                // let range_local_name = push_str_get_range(
                                                //     &mut self.strbuffer,
                                                //     qname.local_name,
                                                // );
                                                // attr.local_name = &self.strbuffer[range_local_name];
                                            }
                                            Err(_e) => {
                                                return Err(error::Error::Parsing(format!(
                                                    "Attribute does not conform to QName spec: {}",
                                                    attr.name
                                                )))
                                            }
                                        }
                                    }

                                    for attr in start_element.attributes.iter_mut() {
                                        //Default namespace doesn't apply to attributes
                                        if attr.prefix == "" || attr.prefix == "xmlns" {
                                            continue;
                                        }
                                        match self.namespace_list.iter().rfind(|ns| {
                                            &self.namespace_strbuffer[ns.prefix.clone()]
                                                == attr.prefix
                                        }) {
                                                    Some(ns) => {
                                                attr.namespace =
                                                    &self.namespace_strbuffer[ns.value.clone()]
                                            }
                                            None => {
                                                return Err(error::Error::Parsing(format!(
                                                    "Namespace not found for prefix: {} , attribute: {} , element: {}", attr.prefix, attr.name, start_element.name
                                                )))
                                            }
                                        }
                                    }

                                    //resolve namespaces for element and attributes.

                                    match QName(start_element.name.as_bytes()) {
                                        Ok(qres) => {
                                            let qname = qres.1;
                                            start_element.local_name = qname.local_name;
                                            start_element.prefix = qname.prefix;

                                            match self.namespace_list.iter().rfind(|ns| {
                                                &self.namespace_strbuffer[ns.prefix.clone()]
                                                    == start_element.prefix
                                            }) {
                                                Some(ns) => {
                                                    start_element.namespace =
                                                        &self.namespace_strbuffer[ns.value.clone()]
                                                }

                                                None => {
                                                    if start_element.prefix == "" {
                                                        //it is fine
                                                    } else {
                                                        return Err(error::Error::Parsing(format!("Namespace prefix not found for element: {}",start_element.name)));
                                                    }
                                                }
                                            }
                                        }
                                        Err(_e) => {
                                            return Err(error::Error::Parsing(format!(
                                                "Element name does not conform to QName spec: {}",
                                                start_element.name
                                            )))
                                        }
                                    }
                                }

                                event2 = xml_sax::Event::StartElement(start_element);
                            }
                            ContentRelaxed::EmptyElemTag(event1) => {
                                if self.is_namespace_aware {
                                    // clear up namespaces
                                    match self
                                        .namespace_list
                                        .iter()
                                        .rposition(|ns| ns.level <= self.element_level)
                                    {
                                        Some(pos) => {
                                            if let Some(starting_pos) = self
                                                .namespace_list
                                                .get(pos + 1)
                                                .map(|ns| ns.prefix.start)
                                            {
                                                self.namespace_list.truncate(pos + 1);
                                                self.namespace_strbuffer.truncate(starting_pos);
                                            }
                                        }
                                        None => {
                                            // nothing to remove
                                        }
                                    }
                                }

                                let mut start_element =
                                    convert_start_element(&mut self.strbuffer, event1);
                                start_element.is_empty = true;
                                self.element_level += 1;
                                //todo decode

                                if self.is_namespace_aware {
                                    //first process namespace definitions
                                    for attr in start_element.attributes.iter_mut() {
                                        match QName(attr.name.as_bytes()) {
                                            Ok(qres) => {
                                                let qname = qres.1;

                                                if qname.prefix == "" && qname.local_name == "xmlns"
                                                {
                                                    //set default namespace
                                                    let ns = push_ns_values_get_ns(
                                                        &mut self.namespace_strbuffer,
                                                        "",
                                                        attr.value,
                                                        self.element_level,
                                                    );
                                                    self.namespace_list.push(ns);
                                                }

                                                if qname.prefix == "xmlns" {
                                                    //set prefixed namespace
                                                    let prefix = qname.local_name;
                                                    let ns = push_ns_values_get_ns(
                                                        &mut self.namespace_strbuffer,
                                                        prefix,
                                                        attr.value,
                                                        self.element_level,
                                                    );
                                                    self.namespace_list.push(ns);
                                                }
                                                attr.local_name = qname.local_name;
                                                attr.prefix = qname.prefix;
                                                // let range_local_name = push_str_get_range(
                                                //     &mut self.strbuffer,
                                                //     qname.local_name,
                                                // );
                                                // attr.local_name = &self.strbuffer[range_local_name];
                                            }
                                            Err(_e) => {
                                                return Err(error::Error::Parsing(format!(
                                                    "Attribute does not conform to QName spec: {}",
                                                    attr.name
                                                )))
                                            }
                                        }
                                    }

                                    for attr in start_element.attributes.iter_mut() {
                                        //Default namespace doesn't apply to attributes
                                        if attr.prefix == "" || attr.prefix == "xmlns" {
                                            continue;
                                        }
                                        match self.namespace_list.iter().rfind(|ns| {
                                            &self.namespace_strbuffer[ns.prefix.clone()]
                                                == attr.prefix
                                        }) {
                                            Some(ns) => {
                                                attr.namespace =
                                                    &self.namespace_strbuffer[ns.value.clone()]
                                            }
                                            None => {
                                                return Err(error::Error::Parsing(format!(
                                                    "Namespace not found for prefix: {} , attribute: {} , element: {}",
                                                    attr.prefix, attr.name,start_element.name
                                                )));
                                            }
                                        }
                                    }

                                    //resolve namespaces for element and attributes.

                                    match QName(start_element.name.as_bytes()) {
                                        Ok(qres) => {
                                            let qname = qres.1;
                                            start_element.local_name = qname.local_name;
                                            start_element.prefix = qname.prefix;

                                            match self.namespace_list.iter().rfind(|ns| {
                                                &self.namespace_strbuffer[ns.prefix.clone()]
                                                    == start_element.prefix
                                            }) {
                                                Some(ns) => {
                                                    start_element.namespace =
                                                        &self.namespace_strbuffer[ns.value.clone()]
                                                }

                                                None => {
                                                    if start_element.prefix == "" {
                                                        //it is fine
                                                    } else {
                                                        return Err(error::Error::Parsing(format!(
                                                            "Namespace not found for Element prefix. element: {}",
                                                            start_element.name
                                                        )));
                                                    }
                                                }
                                            }
                                        }
                                        Err(_e) => {
                                            return Err(error::Error::Parsing(format!(
                                                "Element name does not conform to QName spec: {}",
                                                start_element.name
                                            )));
                                        }
                                    }
                                }

                                event2 = xml_sax::Event::StartElement(start_element);

                                self.element_level -= 1;
                                if self.element_level == 0 {
                                    //could be a root only document.
                                    self.state = ParserState::DocEnd;
                                }

                                //add endelement after this? no..?
                            }
                            ContentRelaxed::EndElement(event1) => {
                                //todo: check if it is the expected tag

                                match self.element_list.pop() {
                                    Some(r) => {
                                        if &self.element_strbuffer[r.clone()] == event1.name {
                                            self.element_strbuffer.truncate(r.start);
                                        } else {
                                            return Err(error::Error::Parsing(format!(
                                                "Expected closing tag: {} ,found: {}",
                                                &self.element_strbuffer[r.clone()],
                                                event1.name
                                            )));

                                            // TODO Expected closing tag: ... &self.element_strbuffer[r.clone()] found event1.name
                                        }
                                    }
                                    None => {
                                        return Err(error::Error::Parsing(format!(
                                            "No starting tag for: {}",
                                            event1.name
                                        )))
                                    }
                                }

                                if self.is_namespace_aware {
                                    // clear up namespaces
                                    match self
                                        .namespace_list
                                        .iter()
                                        .rposition(|ns| ns.level <= self.element_level)
                                    {
                                        Some(pos) => {
                                            if let Some(starting_pos) = self
                                                .namespace_list
                                                .get(pos + 1)
                                                .map(|ns| ns.prefix.start)
                                            {
                                                self.namespace_list.truncate(pos + 1);
                                                self.namespace_strbuffer.truncate(starting_pos);
                                            }
                                        }
                                        None => {
                                            // nothing to remove
                                        }
                                    }
                                }

                                // let range = push_str_get_range(
                                //     &mut self.element_strbuffer,
                                //     start_element.name,
                                // );
                                // self.element_list.push(range);

                                let start = self.strbuffer.len();
                                let size = event1.name.len();
                                self.strbuffer.push_str(event1.name);
                                let mut end_element = xml_sax::EndElement {
                                    name: &self.strbuffer[start..(start + size)],
                                    local_name: "",
                                    prefix: "",
                                    namespace: "",
                                };

                                self.element_level -= 1;
                                if self.element_level == 0 {
                                    self.state = ParserState::DocEnd;
                                }

                                if self.is_namespace_aware {
                                    match QName(end_element.name.as_bytes()) {
                                        Ok(qres) => {
                                            let qname = qres.1;
                                            end_element.local_name = qname.local_name;
                                            end_element.prefix = qname.prefix;

                                            match self.namespace_list.iter().rfind(|ns| {
                                                &self.namespace_strbuffer[ns.prefix.clone()]
                                                    == end_element.prefix
                                            }) {
                                                Some(ns) => {
                                                    end_element.namespace =
                                                        &self.namespace_strbuffer[ns.value.clone()]
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

                                event2 = xml_sax::Event::EndElement(end_element);
                            }
                            ContentRelaxed::Reference(event1) => {
                                // let start = self.strbuffer.len();
                                // let size = event1.initial.len();
                                // let range_initial = Range {
                                //     start: start,
                                //     end: start + size,
                                // };
                                // self.strbuffer.push_str(event1.initial);

                                let range: Range<usize> =
                                    push_str_get_range(&mut self.strbuffer, event1.initial);

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
                                        Some(push_str_get_range(&mut self.strbuffer, addition))
                                    }
                                    None => None,
                                    // &* -> https://github.com/rust-lang/rust/issues/28606
                                    // "&amp;" => Some(push_str_get_range(&mut self.strbuffer, "&")),
                                    // "&lt;" => Some(push_str_get_range(&mut self.strbuffer, "<")),
                                    // "&gt;" => Some(push_str_get_range(&mut self.strbuffer, ">")),
                                    // "&quot;" => Some(push_str_get_range(&mut self.strbuffer, "\"")),
                                    // "&apos;" => Some(push_str_get_range(&mut self.strbuffer, "'")),
                                    // _ => None,
                                };

                                //we are ignoring DTD entity refs

                                let reference_event = xml_sax::Reference {
                                    raw: &self.strbuffer[range],
                                    resolved: match range_resolved {
                                        Some(range) => Some(&self.strbuffer[range]),
                                        None => None,
                                    },
                                };

                                event2 = xml_sax::Event::Reference(reference_event)
                            }
                            ContentRelaxed::CdataStart => {
                                event2 = xml_sax::Event::StartCdataSection;
                                self.state = ParserState::InsideCdata;
                            }
                            ContentRelaxed::CommentStart => {
                                event2 = xml_sax::Event::StartComment;
                                self.state = ParserState::InsideComment;
                            }
                        }
                    }
                    Err(nom::Err::Incomplete(_e)) => {
                        let ending = String::from_utf8_lossy(&self.buffer2);

                        return Err(error::Error::Parsing(format!(
                            "Incomplete file / Premature end-of-file: {}",
                            ending
                        )));
                    }
                    Err(_e) => {
                        let ending = String::from_utf8_lossy(&self.buffer2);
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
                let res = insidecdata(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            InsideCdata::Characters(characters) => {
                                let start = self.strbuffer.len();
                                let size = characters.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                                event2 =
                                    xml_sax::Event::Cdata(&self.strbuffer[start..(start + size)])
                            }
                            InsideCdata::CdataEnd => {
                                self.state = ParserState::Content;
                                event2 = xml_sax::Event::EndCdataSection;
                            }
                        }
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
                let res = insidecomment(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            InsideComment::Characters(characters) => {
                                let start = self.strbuffer.len();
                                let size = characters.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                                event2 =
                                    xml_sax::Event::Comment(&self.strbuffer[start..(start + size)])
                            }
                            InsideComment::CommentEnd => {
                                self.state = ParserState::Content;
                                event2 = xml_sax::Event::EndComment;
                            }
                        }
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
                if self.buffer2.len() == 0 {
                    return Ok(xml_sax::Event::EndDocument);
                }

                let res = misc(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            Misc::PI(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 =
                                    xml_sax::Event::ProcessingInstruction(&self.strbuffer[range])
                            }
                            Misc::Whitespace(a) => {
                                let str = unsafe { std::str::from_utf8_unchecked(a) };
                                let range = push_str_get_range(&mut self.strbuffer, &str);
                                event2 = xml_sax::Event::Whitespace(&self.strbuffer[range])
                            }
                            Misc::CommentStart => {
                                self.state = ParserState::DocEndInsideComment;

                                event2 = xml_sax::Event::StartComment;
                            }
                        }
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
                let res = insidecomment(&self.buffer2);
                match res {
                    Ok(parseresult) => {
                        self.offset = self.buffer2.offset(parseresult.0);
                        match parseresult.1 {
                            InsideComment::Characters(characters) => {
                                let start = self.strbuffer.len();
                                let size = characters.len();
                                self.strbuffer
                                    .push_str(unsafe { std::str::from_utf8_unchecked(characters) });

                                event2 =
                                    xml_sax::Event::Comment(&self.strbuffer[start..(start + size)])
                            }
                            InsideComment::CommentEnd => {
                                self.state = ParserState::DocEnd;
                                event2 = xml_sax::Event::EndComment;
                            }
                        }
                    }
                    Err(_err) => {
                        return Err(error::Error::Parsing(format!(
                            "Expecting comment content or comment closing tag "
                        )))
                    }
                }
            }
        }

        Ok(event2)
    }
}

#[test]
fn test_parser1() {
    let data = r#"<root><A a='x'>
    <B b="val" a:b12='val2' ><C/></B> </A> </root>"#
        .as_bytes();

    // let mut buf = vec![];
    let mut p = Parser::start(data);
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
