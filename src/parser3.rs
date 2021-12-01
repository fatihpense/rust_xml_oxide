use std::{
    io::{BufRead, BufReader, Read},
    ops::{Range, RangeFrom, RangeFull},
    vec,
};

use nom::{
    branch::alt,
    bytes::streaming::{escaped, is_not, tag, take_while, take_while1},
    character::{
        complete::{alphanumeric1 as alphanumeric, char, none_of, one_of},
        is_digit, is_hex_digit,
        streaming::multispace0,
    },
    combinator::{cut, map, opt, recognize, value},
    error::{
        context, convert_error, dbg_dmp, ContextError, Error, ErrorKind, ParseError, VerboseError,
    },
    error_position,
    multi::{many0, many1, separated_list0},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    AsChar, Err, IResult, InputIter, InputLength, Needed, Offset, Parser, Slice,
};

// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, /* 0x1F */
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, /* 0x3F */
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, /* 0x5F */
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, /* 0x7F */
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, /* 0x9F */
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, /* 0xBF */
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, /* 0xDF */
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, /* 0xEF */
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, /* 0xFF */
];

/// Given a first byte, determine how many bytes are in this UTF-8 character
#[inline]
fn utf8_char_width(b: u8) -> usize {
    return UTF8_CHAR_WIDTH[b as usize] as usize;
}

// enum CharResult {
//     Char(char, usize),
//     Err,
//     Eof,
// }

// #[inline]
// fn parse_char(input: &[u8]) -> CharResult {
//     if input.len() == 0 {
//         return CharResult::Eof;
//     }
//     let width = utf8_char_width(input[0]);
//     if input.len() < width {
//         return CharResult::Eof;
//     }

//     match std::str::from_utf8(&input[..width]).ok() {
//         Some(s) => CharResult::Char(s.chars().next().unwrap(), width),
//         None => CharResult::Err,
//     }
// }

//   [^<&]
// none_of("<&")

// CdataEnd
// tag("]]>")

// [2] Char ::= #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]
#[inline]
pub fn is_xml_char_t(chr: char) -> bool {
    (chr >= '\u{A}' && chr <= '\u{D}')
        || (chr >= '\u{20}' && chr <= '\u{D7FF}')
        || (chr >= '\u{E000}' && chr <= '\u{FFFD}')
        || (chr >= '\u{10000}' && chr <= '\u{10FFFF}')
}

// [4] NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |
// [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] |
// [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]

// ('A', 'Z'), /* ('A', 'Z'), veya ('\u{0041}', '\u{005A}'), */
// ('a', 'z'), // ('a', 'z') veya ('\u{61}', '\u{7A}'),
// ('\u{C0}', '\u{D6}'),
// ('\u{D8}', '\u{F6}'),
// ('\u{F8}', '\u{2FF}'),
// ('\u{370}', '\u{37D}'),
// ('\u{37F}', '\u{1FFF}'),
// ('\u{200C}', '\u{200D}'),
// ('\u{2070}', '\u{218F}'),
// ('\u{2C00}', '\u{2FEF}'),
// ('\u{3001}', '\u{D7FF}'),
// ('\u{F900}', '\u{FDCF}'),
// ('\u{FDF0}', '\u{FFFD}'),
// ('\u{10000}', '\u{EFFFF}'),

// NameStartChar.expected_chars.push(':');
// NameStartChar.expected_chars.push('_');
#[inline]
pub fn is_namestart_char_t(chr: char) -> bool {
    (chr >= 'A' && chr <= 'Z')
        || (chr >= 'a' && chr <= 'z')
        || (chr >= '\u{C0}' && chr <= '\u{D6}')
        || (chr >= '\u{D8}' && chr <= '\u{F6}')
        || (chr >= '\u{F8}' && chr <= '\u{2FF}')
        || (chr >= '\u{370}' && chr <= '\u{37D}')
        || (chr >= '\u{37F}' && chr <= '\u{1FFF}')
        || (chr >= '\u{200C}' && chr <= '\u{200D}')
        || (chr >= '\u{2070}' && chr <= '\u{218F}')
        || (chr >= '\u{2C00}' && chr <= '\u{2FEF}')
        || (chr >= '\u{3001}' && chr <= '\u{D7FF}')
        || (chr >= '\u{F900}' && chr <= '\u{FDCF}')
        || (chr >= '\u{FDF0}' && chr <= '\u{FFFD}')
        || (chr >= '\u{10000}' && chr <= '\u{EFFFF}')
        || chr == ':'
        || chr == '_'
}

pub fn namestart_char(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() == 0 {
        return Err(Err::Incomplete(Needed::new(1)));
    }
    let width = utf8_char_width(input[0]);

    if input.len() < width {
        return Err(Err::Incomplete(Needed::new(width - input.len())));
    }

    let c = match std::str::from_utf8(&input[..width]).ok() {
        Some(s) => s.chars().next().unwrap(),
        None => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
    };

    if is_namestart_char_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

// [4a] NameChar ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
#[inline]
pub fn is_namechar_t(chr: char) -> bool {
    is_namestart_char_t(chr)
        || (chr >= '0' && chr <= '9')
        || (chr >= '\u{0300}' && chr <= 'z')
        || (chr >= '\u{203F}' && chr <= '\u{2040}')
        || chr == '-'
        || chr == '.'
        || chr == '\u{B7}'
}

pub fn namechar(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() == 0 {
        return Err(Err::Incomplete(Needed::new(1)));
    }
    let width = utf8_char_width(input[0]);

    if input.len() < width {
        return Err(Err::Incomplete(Needed::new(width - input.len())));
    }

    let c = match std::str::from_utf8(&input[..width]).ok() {
        Some(s) => s.chars().next().unwrap(),
        None => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
    };

    if is_namechar_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

pub fn many0_custom<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    E: ParseError<I>,
{
    move |mut i: I| {
        // let mut acc = crate::lib::std::vec::Vec::with_capacity(4);
        loop {
            let len = i.input_len();
            match f.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, ())),
                Err(e) => return Err(e),
                Ok((i1, o)) => {
                    // infinite loop check: the parser must always consume
                    if i1.input_len() == len {
                        return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many0)));
                    }

                    i = i1;
                    //   acc.push(o);
                }
            }
        }
    }
}

fn name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(pair(namestart_char, many0_custom(namechar)))(input)
}

// [66] CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'

fn CharRef(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(tag("&#"), take_while1(is_digit), char(';')),
        delimited(tag("&#x"), take_while1(is_hex_digit), char(';')),
    ))(input)
}

// [68] EntityRef ::= '&' Name ';'

fn EntityRef(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag("&"), name, char(';'))(input)
}

// [67] Reference ::= EntityRef | CharRef
fn Reference(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((EntityRef, CharRef))(input)
}

// [10] AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"

fn AttValue(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(
            char('"'),
            recognize(many0_custom(alt((is_not(r#"<&""#), Reference)))),
            char('"'),
        ),
        delimited(
            char('\''),
            recognize(many0_custom(alt((is_not(r#"<&'"#), Reference)))),
            char('\''),
        ),
    ))(input)
}

// [25] Eq ::= S? '=' S?
fn Eq(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((multispace0, char('='), multispace0)))(input)
}

// [41] Attribute ::= Name Eq AttValue
fn Attribute(input: &[u8]) -> IResult<&[u8], SAXAttribute> {
    match tuple((name, Eq, AttValue))(input) {
        Ok((i, o)) => {
            return Ok((
                i,
                SAXAttribute {
                    value: unsafe { std::str::from_utf8_unchecked(o.2) },
                    qualified_name: unsafe { std::str::from_utf8_unchecked(o.0) },
                },
            ));
        }
        Err(e) => Err(e),
    }
}

// let mut Attribute = ParsingRule::new("Attribute".to_owned(), RuleType::Sequence);
// Attribute.children_names.push("Name".to_owned());
// Attribute.children_names.push("Eq".to_owned());
// Attribute.children_names.push("AttValue".to_owned());
// rule_nameRegistry.insert(Attribute.rule_name.clone(), Attribute);

// [40] STag ::= '<' Name (S Attribute)* S? '>'

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SAXAttribute<'a> {
    pub value: &'a str,
    pub qualified_name: &'a str,
    // fn get_value(&self) -> &str;
    // fn get_local_name(&self) -> &str;
    // fn get_qualified_name(&self) -> &str;
    // fn get_uri(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SAXAttribute2 {
    pub value: std::ops::Range<usize>,
    pub qualified_name: std::ops::Range<usize>,
    // fn get_value(&self) -> &str;
    // fn get_local_name(&self) -> &str;
    // fn get_qualified_name(&self) -> &str;
    // fn get_uri(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartElement<'a> {
    pub name: &'a str,
    pub attributes: Vec<SAXAttribute<'a>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndElement<'a> {
    pub name: &'a str,
}

fn STag<'a>(input: &'a [u8]) -> IResult<&[u8], StartElement<'a>> {
    match tuple((
        char('<'),
        name,
        many0(preceded(multispace0, Attribute)),
        multispace0,
        char('>'),
    ))(input)
    {
        Ok((i, o)) => {
            return Ok((
                i,
                StartElement {
                    name: unsafe { std::str::from_utf8_unchecked(o.1) },
                    attributes: o.2,
                },
            ));
        }

        Err(e) => Err(e),
    }
}

// [44] EmptyElemTag ::= '<' Name (S Attribute)* S? '/>'
fn EmptyElemTag(input: &[u8]) -> IResult<&[u8], StartElement> {
    match tuple((
        char('<'),
        name,
        many0(preceded(multispace0, Attribute)),
        multispace0,
        tag("/>"),
    ))(input)
    {
        Ok((i, o)) => Ok((
            i,
            StartElement {
                name: unsafe { std::str::from_utf8_unchecked(o.1) },
                attributes: o.2,
            },
        )),

        Err(e) => Err(e),
    }
}

// [3] S ::= (#x20 | #x9 | #xD | #xA)+
// multispace0 fits

// [42] ETag ::= '</' Name S? '>'
fn ETag(input: &[u8]) -> IResult<&[u8], EndElement> {
    match tuple((tag("</"), name, multispace0, char('>')))(input) {
        Ok((i, o)) => {
            // println!("{:?}", o);
            return Ok((
                i,
                EndElement {
                    name: unsafe { std::str::from_utf8_unchecked(o.1) },
                },
            ));
        }

        Err(e) => Err(e),
    }
}

#[test]
fn test_namestart_char_t() {
    let data = "<a.abc-ab1çroot><A/><B/><C/></root>".as_bytes();

    // fn parser(s: &[u8]) -> IResult<&[u8], &[u8]> {
    //     namestart_char_t(s)
    // }

    let res = STag(&data);
    println!("{:?}", res);
}

#[test]
fn test_stag() {
    let data = r#"<A a="b"  c = "d"></A>"#.as_bytes();
    let res = STag(&data);
    println!("{:?}", res);

    let data = r#"<A a='x'>"#.as_bytes();
    let res = STag(&data);
    println!("{:?}", res);

    let data = r#"<B b="val" >"#.as_bytes();
    let res = STag(&data);
    println!("{:?}", res);
}

// [14] CharData ::= [^<&]* - ([^<&]* ']]>' [^<&]*)
// no '>' except ']]>'

#[inline]
pub fn is_CharData_single_pure_t(chr: char) -> bool {
    chr != '<' && chr != '&'
}

pub fn CharData_single_pure(input: &[u8]) -> IResult<&[u8], &[u8]> {
    if input.len() == 0 {
        return Err(Err::Incomplete(Needed::new(1)));
    }
    let width = utf8_char_width(input[0]);

    if input.len() < width {
        return Err(Err::Incomplete(Needed::new(width - input.len())));
    }

    let c = match std::str::from_utf8(&input[..width]).ok() {
        Some(s) => s.chars().next().unwrap(),
        None => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
    };

    if is_CharData_single_pure_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

fn CharData_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    //if input = 0 , don't send incomplete
    // ref#streamcut
    if input.len() == 0 {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }

    // ']]>' should not appear in the chardata, if we can't be sure because input is eof, we should request more data.
    match tag::<&str, &[u8], Error<&[u8]>>("]]>")(input) {
        Ok(r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
        Err(Err::Incomplete(n)) => return Err(Err::Incomplete(Needed::Unknown)),
        _ => (),
    };
    CharData_single_pure(input)
}

#[test]
fn test_chardata_single() {
    let data = "]]".as_bytes();

    assert_eq!(
        CharData_single("]".as_bytes()),
        Err(Err::Incomplete(Needed::Unknown))
    );
    assert_eq!(
        CharData_single("]]".as_bytes()),
        Err(Err::Incomplete(Needed::Unknown))
    );
    assert_eq!(
        CharData_single("]]>".as_bytes()),
        Err(Err::Error(error_position!(
            "]]>".as_bytes(),
            ErrorKind::Char
        )))
    );
    assert_eq!(
        CharData_single("]]<".as_bytes()),
        Ok((&b"]<"[..], &b"]"[..]))
    );
    assert_eq!(
        CharData_single("&".as_bytes()),
        Err(Err::Error(error_position!("&".as_bytes(), ErrorKind::Char)))
    );
    assert_eq!(
        CharData_single("<".as_bytes()),
        Err(Err::Error(error_position!("<".as_bytes(), ErrorKind::Char)))
    );
    assert_eq!(
        CharData_single("abc".as_bytes()),
        Ok((&b"bc"[..], &b"a"[..]))
    );
}

// [14] CharData ::= [^<&]* - ([^<&]* ']]>' [^<&]*)
fn CharData(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom(CharData_single))(input)
}

#[test]
fn test_chardata() {
    assert_eq!(
        CharData("abc]".as_bytes()),
        Err(Err::Incomplete(Needed::Unknown))
    );
    assert_eq!(
        CharData("]]".as_bytes()),
        Err(Err::Incomplete(Needed::Unknown))
    );
    assert_eq!(CharData("]]>".as_bytes()), Ok((&b"]]>"[..], &b""[..])));
    assert_eq!(CharData("]]<".as_bytes()), Ok((&b"<"[..], &b"]]"[..])));
    assert_eq!(CharData("&".as_bytes()), Ok((&b"&"[..], &b""[..])));
    assert_eq!(CharData("<".as_bytes()), Ok((&b"<"[..], &b""[..])));

    //this was returning incomplete since the next char can be the start of "]]>", but we plan to cut it off for streaming!
    //see ref#streamcut
    assert_eq!(CharData("abc".as_bytes()), Ok((&b""[..], &b"abc"[..])));
}

// [43] content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
//we will use state machine instead of this rule to make it streamable

#[test]
fn test_xml3() {
    let data = "<root><A/><B/><C/></root>".as_bytes();

    fn parser(s: &[u8]) -> IResult<&[u8], &[u8]> {
        tag("<root>")(s)
    }

    let res = parser(&data);
    println!("{:?}", res);
}

enum ParserState {
    Content,
    TagStart,
}

struct OxideParser<R: Read> {
    state: ParserState,
    bufreader: BufReader<R>,
    buffer2: Vec<u8>,
    strbuffer: String,
    offset: usize,
}

impl<R: Read> OxideParser<R> {
    // This method "consumes" the resources of the caller object
    // `self` desugars to `self: Self`

    fn start(reader: R) -> OxideParser<R> {
        OxideParser {
            state: ParserState::Content,
            bufreader: BufReader::with_capacity(30, reader),
            offset: 0,
            buffer2: vec![],
            strbuffer: String::new(),
        }
    }

    fn read_data(&mut self) {
        self.bufreader.fill_buf().unwrap();
        let data2 = self.bufreader.buffer();

        self.buffer2.extend_from_slice(data2);

        self.bufreader.consume(data2.len());
    }

    // , buf: &'b [u8]
    fn read_event<'a, 'b, 'c>(&'a mut self) -> StartElement<'a> {
        // self.bufreader.consume(self.offset);
        self.buffer2.drain(0..self.offset);
        self.offset = 0;
        let mut done = false;

        if self.bufreader.capacity() > self.buffer2.len() {
            self.read_data();
        }

        // let mut event: StartElement = StartElement {
        //     name: "",
        //     attributes: vec![],
        // };
        let mut event1: StartElement; //<'b>; //&'a
        let mut event2: StartElement;

        println!("try to read: {:?}", unsafe {
            std::str::from_utf8_unchecked(&self.buffer2)
        });

        let res = STag(&self.buffer2);
        match res {
            Ok(parseresult) => {
                self.offset = self.buffer2.offset(parseresult.0);
                event1 = parseresult.1;
                // return parseresult.1;
                done = true;

                //todo decode
                let start = self.strbuffer.len();
                let size = event1.name.len();
                self.strbuffer.push_str(event1.name);

                let mut attributes2: Vec<SAXAttribute2> = vec![];
                for att in event1.attributes {
                    let start = self.strbuffer.len();
                    let size = att.qualified_name.len();
                    self.strbuffer.push_str(att.qualified_name);
                    let qualified_name_range = Range {
                        start: start,
                        end: start + size,
                    };

                    let start = self.strbuffer.len();
                    let size = att.value.len();
                    self.strbuffer.push_str(att.value);
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

                let mut attributes: Vec<SAXAttribute> = vec![];
                for att in attributes2 {
                    // let qualified_name = &self.strbuffer[start..(start + size)];
                    // let value = &self.strbuffer[start..(start + size)];

                    attributes.push(SAXAttribute {
                        value: &self.strbuffer[att.value],
                        qualified_name: &self.strbuffer[att.qualified_name],
                    });
                }

                event2 = StartElement {
                    name: &self.strbuffer[start..(start + size)],
                    attributes: attributes,
                }
            }
            Err(Err::Incomplete(err)) => {
                // panic!()
                // self.read_data();
                // if read bytes are 0 then return eof, otherwise return dummy event
                panic!()
            }
            Err(e) => {
                println!("err: {:?}", e);
                done = true;
                panic!()
            }
        }

        event2
        // let res = STag(&self.buffer2);
        // match res {
        //     Ok(parseresult) => {
        //         self.offset = self.buffer2.offset(parseresult.0);
        //         event = parseresult.1;
        //         done = true
        //     }
        //     Err(Err::Incomplete(err)) => {
        //         panic!();
        //     }
        //     Err(_) => {
        //         done = true;
        //         panic!();
        //     }
        // }
    }
}

#[test]
fn test_parser1() {
    let data = r#"<root><A a='x'><B b="val" a:12='val2' ><C></root>"#.as_bytes();

    // let mut buf = vec![];
    let mut p = OxideParser::start(data);
    loop {
        let res = p.read_event();
        println!("{:?}", res);
        if res.name == "C" {
            break;
        }
    }

    // let res = p.read_event();
    // println!("{:?}", res);

    // let res = p.read_event();
    // println!("{:?}", res);

    // let res = p.read_event();
    // println!("{:?}", res);
}
