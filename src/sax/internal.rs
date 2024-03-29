#![allow(non_snake_case)]

use std::ops::Range;

use nom::{
    branch::alt,
    bytes::streaming::{is_not, tag, take_while1},
    character::{
        complete::{char, multispace1},
        is_digit, is_hex_digit,
        streaming::{alpha1, alphanumeric1, digit1, multispace0},
    },
    combinator::{map, opt, recognize},
    error::{Error, ErrorKind, ParseError},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Err, IResult, InputLength, Needed, Offset, Parser,
};

#[allow(unused_imports)]
use nom::error_position;

use super::dtd;

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
pub(crate) fn utf8_char_width(b: u8) -> usize {
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
pub(crate) fn is_xml_char_t(chr: char) -> bool {
    chr == '\u{9}'
        || (chr >= '\u{A}' && chr <= '\u{D}')
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
fn is_namestart_char_t(chr: char) -> bool {
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

fn namestart_char(input: &[u8]) -> IResult<&[u8], &[u8]> {
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
    // let c = unsafe { std::str::from_utf8_unchecked(&input[..width]) }.chars().next().unwrap();

    if is_namestart_char_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

// [4a] NameChar ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
#[inline]
fn is_namechar_t(chr: char) -> bool {
    is_namestart_char_t(chr)
        || (chr >= '0' && chr <= '9')
        || (chr >= '\u{0300}' && chr <= '\u{036F}')
        || (chr >= '\u{203F}' && chr <= '\u{2040}')
        || chr == '-'
        || chr == '.'
        || chr == '\u{B7}'
}

pub(crate) fn namechar(input: &[u8]) -> IResult<&[u8], &[u8]> {
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
    // let c = unsafe { std::str::from_utf8_unchecked(&input[..width]) }.chars().next().unwrap();

    // println!("checking char?: {:?}", &c);
    if is_namechar_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        // println!("No: {:?}", &input[0..width]);
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

fn many0_custom_chardata<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
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
                // Err(e) => return Err(e),
                // ref#streamcut
                Err(_e) => return Ok((i, ())),
                Ok((i1, _o)) => {
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

//means streaming in nom's terminology
pub(crate) fn many0_custom_trycomplete<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
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
                Err(e) => return Err(e), //returns incomplete here
                // Err(e) => return Ok((i, ())),
                Ok((i1, _o)) => {
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

pub(crate) fn many1_custom<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
where
    I: Clone + InputLength,
    F: Parser<I, O, E>,
    E: ParseError<I>,
{
    move |mut i: I| match f.parse(i.clone()) {
        Err(Err::Error(err)) => Err(Err::Error(E::append(i, ErrorKind::Many1, err))),
        Err(e) => Err(e),
        Ok((i1, _o)) => {
            i = i1;

            loop {
                let len = i.input_len();
                match f.parse(i.clone()) {
                    Err(Err::Error(_)) => return Ok((i, ())),
                    Err(e) => return Err(e),
                    Ok((i1, _o)) => {
                        // infinite loop check: the parser must always consume
                        if i1.input_len() == len {
                            return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many1)));
                        }

                        i = i1;
                    }
                }
            }
        }
    }
}

pub(crate) fn name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(pair(namestart_char, many0_custom_trycomplete(namechar)))(input)
}

#[test]
fn test_name001() {
    let data2 = r#"A.-:̀·>"#.as_bytes();
    // U+0300	̀	204 128	COMBINING GRAVE ACCENT
    assert_eq!(name(&data2), Ok((&b">"[..], &data2[0..data2.len() - 1])));
}

// [66] CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'

fn CharRef(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        recognize(tuple((tag("&#"), take_while1(is_digit), char(';')))),
        recognize(tuple((tag("&#x"), take_while1(is_hex_digit), char(';')))),
    ))(input)
}

// [68] EntityRef ::= '&' Name ';'

fn EntityRef(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag("&"), name, char(';'))))(input)
}

// [67] Reference ::= EntityRef | CharRef
pub(crate) fn Reference(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((EntityRef, CharRef))(input)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference<'a> {
    pub initial: &'a str,
    // resolved: &'a str,
}

// [10] AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"

pub(crate) fn AttValue(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(
            char('"'),
            recognize(many0_custom_trycomplete(alt((is_not(r#"<&""#), Reference)))),
            char('"'),
        ),
        delimited(
            char('\''),
            recognize(many0_custom_trycomplete(alt((is_not(r#"<&'"#), Reference)))),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeRange {
    pub value: Range<usize>,
    pub name: Range<usize>,
    // namespace aware
    pub local_name: Range<usize>,
    pub prefix: Range<usize>,
    pub namespace: Range<usize>,
}

pub(crate) fn Attribute2(input: &[u8]) -> IResult<&[u8], AttributeRange> {
    //move preceeded here
    match preceded(multispace0, tuple((name, Eq, AttValue)))(input) {
        Ok((i, o)) => {
            let name_start = input.offset(o.0);
            let name_end = name_start + o.0.len();

            let val_start = input.offset(o.2);
            let val_end = val_start + o.2.len();

            return Ok((
                i,
                AttributeRange{
                    name:  std::ops::Range { start:name_start , end: name_end } ,
                    value: (val_start..val_end),
                    local_name: (0..0),
                    prefix: (0..0),
                    namespace: (0..0) }
                // SAXAttribute {
                //     value: unsafe { std::str::from_utf8_unchecked(o.2) },
                //     qualified_name: unsafe { std::str::from_utf8_unchecked(o.0) },
                // },
            ));
        }
        Err(e) => Err(e),
    }
}
#[test]
fn test_attribute2() {
    let data = r#" a:b12='val2'"#.as_bytes();
    let res = Attribute2(&data);
    println!("{:?}", res);
    let range = res.unwrap().1;
    assert_eq!("a:b12".as_bytes(), &data[range.name.clone()]);
    assert_eq!("val2".as_bytes(), &data[range.value.clone()]);
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
struct SAXAttributeNsAware {
    pub value: std::ops::Range<usize>,
    pub qualified_name: std::ops::Range<usize>,
    pub prefix: std::ops::Range<usize>,
    pub local_name: std::ops::Range<usize>,
    pub namespace: std::ops::Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartElement<'a> {
    pub name: &'a str,
    // pub attributes: Vec<SAXAttribute<'a>>,
    pub attributes_chunk: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndElement<'a> {
    pub name: &'a str,
}

fn STag<'a>(input: &'a [u8]) -> IResult<&[u8], StartElement<'a>> {
    match tuple((
        char('<'),
        name,
        recognize(many0_custom_trycomplete(preceded(multispace0, Attribute))),
        // many0_custom_attributes,
        multispace0,
        char('>'),
    ))(input)
    {
        Ok((i, o)) => {
            return Ok((
                i,
                StartElement {
                    name: unsafe { std::str::from_utf8_unchecked(o.1) },
                    attributes_chunk: o.2,
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
        recognize(many0_custom_trycomplete(preceded(multispace0, Attribute))),
        // many0(preceded(multispace0, Attribute)),
        multispace0,
        tag("/>"),
    ))(input)
    {
        Ok((i, o)) => Ok((
            i,
            StartElement {
                name: unsafe { std::str::from_utf8_unchecked(o.1) },
                attributes_chunk: o.2,
            },
        )),

        Err(e) => Err(e),
    }
}

#[test]
fn test_EmptyElemTag() {
    let data = r#"<A.-:̀·/>"#.as_bytes();
    let res = EmptyElemTag(&data);
    println!("{:?}", res);
    assert_eq!(res.is_ok(), true);
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
fn test_etag() {
    let data = r#"</A>"#.as_bytes();
    let res = ETag(&data);
    println!("{:?}", res);
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
// The spec is not clear but we also apply Char restrictions
#[inline]
fn is_CharData_single_pure_t(chr: char) -> bool {
    chr != '<' && chr != '&' && is_xml_char_t(chr)
}

fn CharData_single_pure(input: &[u8]) -> IResult<&[u8], &[u8]> {
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
    // let c = unsafe { std::str::from_utf8_unchecked(&input[..width]) }.chars().next().unwrap();

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
        Ok(_r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
        Err(Err::Incomplete(_n)) => return Err(Err::Incomplete(Needed::Unknown)),
        _ => (),
    };
    CharData_single_pure(input)
}

#[test]
fn test_chardata_single() {
    let _data = "]]".as_bytes();

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
//our implementation requires at least one char
fn CharData(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        CharData_single,
        many0_custom_chardata(CharData_single),
    )))(input)
}

#[test]
fn test_chardata() {
    assert_eq!(CharData("abc]".as_bytes()), Ok((&b"]"[..], &b"abc"[..])));
    assert_eq!(
        CharData("]]".as_bytes()),
        Err(Err::Incomplete(Needed::Unknown))
    );
    //since we want chardata to parse at least 1 char now:
    // assert_eq!(CharData("]]>".as_bytes()), Ok((&b"]]>"[..], &b""[..])));
    assert_eq!(
        CharData("]]>".as_bytes()),
        Err(Err::Error(error_position!(
            "]]>".as_bytes(),
            ErrorKind::Char
        )))
    );
    assert_eq!(CharData("]]<".as_bytes()), Ok((&b"<"[..], &b"]]"[..])));

    //since we want chardata to parse at least 1 char now:
    // assert_eq!(CharData("&".as_bytes()), Ok((&b"&"[..], &b""[..])));

    assert_eq!(CharData("a&".as_bytes()), Ok((&b"&"[..], &b"a"[..])));
    assert_eq!(CharData("a<".as_bytes()), Ok((&b"<"[..], &b"a"[..])));

    //this was returning incomplete since the next char can be the start of "]]>", but we plan to cut it off for streaming!
    //see ref#streamcut
    assert_eq!(CharData("abc".as_bytes()), Ok((&b""[..], &b"abc"[..])));

    let data: Vec<u8> = [
        65, 108, 99, 104, 101, 109, 121, 32, 40, 102, 114, 111, 109, 32, 65, 114, 97, 98, 105, 99,
        58, 32, 97, 108, 45, 107, 196, 171, 109, 105, 121, 196,
    ]
    .to_vec();
    let remainder: Vec<u8> = [196].to_vec();

    println!("try to read: {:?}", unsafe {
        std::str::from_utf8_unchecked(&data[0..31])
    });
    assert_eq!(
        CharData(&data),
        Ok((
            &remainder[0..1],
            &"Alchemy (from Arabic: al-kīmiy".as_bytes()[..]
        ))
    );
}

// [43] content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
//we will use state machine instead of this rule to make it streamable

pub enum ContentRelaxed<'a> {
    CharData(&'a [u8]),
    StartElement(StartElement<'a>),
    EmptyElemTag(StartElement<'a>),
    EndElement(EndElement<'a>),
    Reference(Reference<'a>),
    PI(&'a [u8]),
    CdataStart,
    CommentStart,
}

fn content_relaxed_CharData(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match CharData(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::CharData(succ.1))),
        Err(err) => return Err(err),
    }
}
fn content_relaxed_STag(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match STag(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::StartElement(succ.1))),
        Err(err) => return Err(err),
    }
}

fn content_relaxed_ETag(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match ETag(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::EndElement(succ.1))),
        Err(err) => return Err(err),
    }
}

//add endelement as next step or inform it is an emptyelem tag via event api? - no.
fn content_relaxed_EmptyElemTag(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match EmptyElemTag(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::EmptyElemTag(succ.1))),
        Err(err) => return Err(err),
    }
}

fn content_relaxed_Reference(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match Reference(input) {
        Ok(succ) => Ok((
            succ.0,
            ContentRelaxed::Reference(Reference {
                initial: unsafe { std::str::from_utf8_unchecked(succ.1) },
            }),
        )),
        Err(err) => return Err(err),
    }
}

fn content_relaxed_CdataStart(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match CDATASection_start(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::CdataStart)),
        Err(err) => return Err(err),
    }
}

fn content_relaxed_CommentStart(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match Comment_start(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::CommentStart)),
        Err(err) => return Err(err),
    }
}

// [43] content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
// [custom] relaxed ::= CharData | STag | EmptyElemTag | ETag | Reference | CDATA | Comment | PI
pub fn content_relaxed(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    alt((
        content_relaxed_CharData,
        content_relaxed_STag,
        content_relaxed_EmptyElemTag,
        content_relaxed_ETag,
        content_relaxed_Reference,
        content_relaxed_CdataStart,
        content_relaxed_CommentStart,
        map(PI, |a| ContentRelaxed::PI(a)),
    ))(input)
}

#[test]
fn test_xml3() {
    let data = "<root><A/><B/><C/></root>".as_bytes();

    fn parser(s: &[u8]) -> IResult<&[u8], &[u8]> {
        tag("<root>")(s)
    }

    let res = parser(&data);
    println!("{:?}", res);
}

// Parser Rules organized by W3C Spec

// [26] VersionNum ::= '1.' [0-9]+
fn VersionNum(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag("1."), digit1)))(input)
}

#[test]
fn test_VersionNum() {
    let data = r#"1.123 "#.as_bytes();
    let res = VersionNum(&data);
    println!("{:?}", res);
}
//  [24] VersionInfo ::= S 'version' Eq ("'" VersionNum "'" | '"' VersionNum '"')

fn VersionInfo(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace1,
        tag("version"),
        Eq,
        alt((
            delimited(char('"'), VersionNum, char('"')),
            delimited(char('\''), VersionNum, char('\'')),
        )),
    )))(input)
}
#[test]
fn test_VersionInfo() {
    let data = r#"  version="1.0" "#.as_bytes();
    let res = VersionInfo(&data);
    println!("{:?}", res);
}

// [81] EncName ::= [A-Za-z] ([A-Za-z0-9._] | '-')*
fn EncName(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        alpha1,
        many0_custom_trycomplete(alt((alphanumeric1, tag("-"), tag("."), tag("_")))),
    )))(input)
}
#[test]
fn test_EncName() {
    let data = r#"UTF-8 "#.as_bytes();
    let res = EncName(&data);
    println!("{:?}", res);
}

// [80] EncodingDecl ::= S 'encoding' Eq ('"' EncName '"' | "'" EncName "'" )
fn EncodingDecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace1,
        tag("encoding"),
        Eq,
        alt((
            delimited(char('"'), EncName, char('"')),
            delimited(char('\''), EncName, char('\'')),
        )),
    )))(input)
}
#[test]
fn test_EncodingDecl() {
    let data = r#" encoding='EUC-JP' "#.as_bytes();
    let res = EncodingDecl(&data);
    println!("{:?}", res);
}

// [32] SDDecl ::= S 'standalone' Eq (("'" ('yes' | 'no') "'") | ('"' ('yes' | 'no') '"'))
fn yes_mi_no_mu(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("yes"), tag("no")))(input)
}
fn SDDecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace1,
        tag("standalone"),
        Eq,
        alt((
            delimited(char('"'), yes_mi_no_mu, char('"')),
            delimited(char('\''), yes_mi_no_mu, char('\'')),
        )),
    )))(input)
}
#[test]
fn test_SDDecl() {
    let data = r#"  standalone='yes' "#.as_bytes();
    let res = SDDecl(&data);
    println!("{:?}", res);
}

// [23] XMLDecl ::= '<?xml' VersionInfo EncodingDecl? SDDecl? S? '?>'
fn XMLDecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<?xml"),
        VersionInfo,
        opt(EncodingDecl),
        opt(SDDecl),
        multispace0,
        tag("?>"),
    )))(input)
}

#[test]
fn test_XMLDecl() {
    let data = r#"<?xml version="1.0"  encoding="UTF-8" standalone='yes'?>"#.as_bytes();
    let res = XMLDecl(&data);
    println!("{:?}", res);
}

// [1] document ::= prolog element Misc*
// [22] prolog ::= XMLDecl? Misc* (doctypedecl Misc*)?

// [15] Comment ::= '<!--' ((Char - '-') | ('-' (Char - '-')))* '-->'
//spec seems to not allow empty comments? There are parsers that allow it.
fn Comment_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("<!--")(input)
}

fn Comment_end(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("-->")(input)
}

// We don't need to exclude "-" we handle that in inside_Comment_single
// #[inline]
//  fn is_CharData_single_pure_t(chr: char) -> bool {
//     chr != '<' && chr != '&' && is_xml_char_t(chr)
// }

fn inside_Comment_or_CDATA_single_pure(input: &[u8]) -> IResult<&[u8], &[u8]> {
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
    // let c = unsafe { std::str::from_utf8_unchecked(&input[..width]) }.chars().next().unwrap();

    if is_xml_char_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

fn inside_Comment_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    //if input = 0 , don't send incomplete
    // ref#streamcut
    if input.len() == 0 {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }

    // '--' should not appear in the comment, if we can't be sure because input is eof, we should request more data.
    match tag::<&str, &[u8], Error<&[u8]>>("--")(input) {
        Ok(_r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
        Err(Err::Incomplete(_n)) => return Err(Err::Incomplete(Needed::new(1))),
        _ => (),
    };
    inside_Comment_or_CDATA_single_pure(input)
}

pub(crate) fn Comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        Comment_start,
        many0_custom_chardata(inside_Comment_single),
        Comment_end,
    )))(input)
}

#[test]
fn test_comment() {
    assert_eq!(
        Comment("<!-- comment -->a".as_bytes()),
        Ok((&b"a"[..], &b"<!-- comment -->"[..]))
    );

    assert_eq!(
        Comment("<!---->cc".as_bytes()),
        Ok((&b"cc"[..], &b"<!---->"[..]))
    );

    assert_eq!(
        Comment("<!-- comment --->a".as_bytes()),
        Err(Err::Error(error_position!(
            "--->a".as_bytes(),
            ErrorKind::Tag
        )))
    );

    assert_eq!(
        Comment("<!-- com--ment -->a".as_bytes()),
        Err(Err::Error(error_position!(
            "--ment -->a".as_bytes(),
            ErrorKind::Tag
        )))
    );

    assert_eq!(
        Comment("<!--ok-".as_bytes()),
        Err(Err::Incomplete(Needed::new(2)))
    );
    assert_eq!(
        Comment("<!--ok--".as_bytes()),
        Err(Err::Incomplete(Needed::new(1)))
    );
}

pub enum InsideComment<'a> {
    Characters(&'a [u8]),
    CommentEnd,
}

fn insidecomment_characters(input: &[u8]) -> IResult<&[u8], InsideComment> {
    match recognize(tuple((
        inside_Comment_single,
        many0_custom_chardata(inside_Comment_single),
    )))(input)
    {
        Ok(succ) => Ok((succ.0, InsideComment::Characters(succ.1))),
        Err(err) => return Err(err),
    }
}

fn insidecomment_comment_end(input: &[u8]) -> IResult<&[u8], InsideComment> {
    match Comment_end(input) {
        Ok(succ) => Ok((succ.0, InsideComment::CommentEnd)),
        Err(err) => return Err(err),
    }
}

// [custom]
pub fn insidecomment(input: &[u8]) -> IResult<&[u8], InsideComment> {
    alt((insidecomment_characters, insidecomment_comment_end))(input)
}

// [18] CDSect ::= CDStart CData CDEnd

// [19] CDStart ::= '<![CDATA['
fn CDATASection_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("<![CDATA[")(input)
}
// [21] CDEnd ::= ']]>'
fn CDATASection_end(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("]]>")(input)
}

// [20] CData ::= (Char* - (Char* ']]>' Char*))

fn inside_CDATASection_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    //if input = 0 , don't send incomplete
    // ref#streamcut
    if input.len() == 0 {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }

    // ']]>' should not appear in the cdata section, if we can't be sure because input is eof, we should request more data.
    match tag::<&str, &[u8], Error<&[u8]>>("]]>")(input) {
        Ok(_r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
        Err(Err::Incomplete(_n)) => return Err(Err::Incomplete(Needed::Unknown)),
        _ => (),
    };
    inside_Comment_or_CDATA_single_pure(input)
}

#[allow(dead_code)]
fn CDATASection(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        CDATASection_start,
        many0_custom_chardata(inside_CDATASection_single),
        CDATASection_end,
    )))(input)
}

#[test]
fn test_cdata() {
    assert_eq!(
        CDATASection("<![CDATA[abc]]>a".as_bytes()),
        Ok((&b"a"[..], &b"<![CDATA[abc]]>"[..]))
    );

    assert_eq!(
        CDATASection("<![CDATA[]]>".as_bytes()),
        Ok((&b""[..], &b"<![CDATA[]]>"[..]))
    );

    assert_eq!(
        CDATASection("<![CDATA[ ]]".as_bytes()),
        Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
        CDATASection("<![CDATA[ ]".as_bytes()),
        Err(Err::Incomplete(Needed::new(2)))
    );
}

//only parsed without checking well-formedness inside
// [16] PI ::= '<?' PITarget (S (Char* - (Char* '?>' Char*)))? '?>'
// [17] PITarget ::= Name - (('X' | 'x') ('M' | 'm') ('L' | 'l'))

fn PI_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("<?")(input)
}

fn PI_end(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("?>")(input)
}

fn inside_PI_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    //if input = 0 , don't send incomplete
    // ref#streamcut
    if input.len() == 0 {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }

    // ']]>' should not appear in the cdata section, if we can't be sure because input is eof, we should request more data.
    match tag::<&str, &[u8], Error<&[u8]>>("?>")(input) {
        Ok(_r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
        Err(Err::Incomplete(_n)) => return Err(Err::Incomplete(Needed::Unknown)),
        _ => (),
    };
    inside_Comment_or_CDATA_single_pure(input)
}

pub(crate) fn PI(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        PI_start,
        many0_custom_chardata(inside_PI_single),
        PI_end,
    )))(input)
}

#[test]
fn test_pi() {
    assert_eq!(PI("<??>a".as_bytes()), Ok((&b"a"[..], &b"<??>"[..])));

    assert_eq!(
        PI("<?dummmy?>".as_bytes()),
        Ok((&b""[..], &b"<?dummmy?>"[..]))
    );
}

pub enum InsideCdata<'a> {
    Characters(&'a [u8]),
    CdataEnd,
}

fn insidecdata_characters(input: &[u8]) -> IResult<&[u8], InsideCdata> {
    match recognize(tuple((
        inside_CDATASection_single,
        many0_custom_chardata(inside_CDATASection_single),
    )))(input)
    {
        Ok(succ) => Ok((succ.0, InsideCdata::Characters(succ.1))),
        Err(err) => return Err(err),
    }
}

fn insidecdata_cdata_end(input: &[u8]) -> IResult<&[u8], InsideCdata> {
    match CDATASection_end(input) {
        Ok(succ) => Ok((succ.0, InsideCdata::CdataEnd)),
        Err(err) => return Err(err),
    }
}

// [custom]
pub fn insidecdata(input: &[u8]) -> IResult<&[u8], InsideCdata> {
    alt((insidecdata_characters, insidecdata_cdata_end))(input)
}

pub enum MiscBeforeXmlDecl<'a> {
    PI(&'a [u8]),
    Whitespace(&'a [u8]),
    CommentStart,
    DocType(&'a [u8]),
    XmlDecl(&'a [u8]),
}
pub enum MiscBeforeDoctype<'a> {
    PI(&'a [u8]),
    Whitespace(&'a [u8]),
    CommentStart,
    DocType(&'a [u8]),
}

pub enum Misc<'a> {
    PI(&'a [u8]),
    Whitespace(&'a [u8]),
    CommentStart,
}
// using map combinator...
// fn misc_pi(input: &[u8]) -> IResult<&[u8], Misc> {
//     map(PI, |a| Misc::PI(a))(input)
//     // match recognize(tuple((
//     //     inside_CDATASection_single,
//     //     many0_custom_chardata(inside_CDATASection_single),
//     // )))(input)
//     // {
//     //     Ok(succ) => Ok((succ.0, InsideCdata::Characters(succ.1))),
//     //     Err(err) => return Err(err),
//     // }
// }

// [27] Misc ::= Comment | PI | S
// [custom]
pub fn misc(input: &[u8]) -> IResult<&[u8], Misc> {
    alt((
        map(PI, |a| Misc::PI(a)),
        map(multispace1, |a| Misc::Whitespace(a)),
        map(Comment_start, |_a| Misc::CommentStart),
    ))(input)
}
pub fn misc_before_doctype(input: &[u8]) -> IResult<&[u8], MiscBeforeDoctype> {
    alt((
        map(PI, |a| MiscBeforeDoctype::PI(a)),
        map(multispace1, |a| MiscBeforeDoctype::Whitespace(a)),
        map(Comment_start, |_a| MiscBeforeDoctype::CommentStart),
        map(dtd::doctypedecl, |a| MiscBeforeDoctype::DocType(a)),
    ))(input)
}
pub fn misc_before_xmldecl(input: &[u8]) -> IResult<&[u8], MiscBeforeXmlDecl> {
    alt((
        map(XMLDecl, |a| MiscBeforeXmlDecl::XmlDecl(a)), // currently PI can also match XMLDecl so this is first choice
        map(PI, |a| MiscBeforeXmlDecl::PI(a)),
        map(multispace1, |a| MiscBeforeXmlDecl::Whitespace(a)),
        map(Comment_start, |_a| MiscBeforeXmlDecl::CommentStart),
        map(dtd::doctypedecl, |a| MiscBeforeXmlDecl::DocType(a)),
    ))(input)
}

// Namespaces in XML 1.0 https://www.w3.org/TR/xml-names/

// [1] NSAttName ::= PrefixedAttName | DefaultAttName
// [2] PrefixedAttName ::= 'xmlns:'
// [3] DefaultAttName ::= 'xmlns'
// [4] NCName ::= Name - (Char* ':' Char*)	/* An XML Name, minus the ":" */
// [7] QName ::= PrefixedName | UnprefixedName
// [8] PrefixedName ::= Prefix ':' LocalPart
// [9] UnprefixedName ::= LocalPart
// [10] Prefix ::= NCName
// [11] LocalPart ::= NCName

//non normative:
// [5] NCNameChar ::= NameChar - ':' /* An XML NameChar, minus the ":" */
// [6] NCNameStartChar ::= NCName - ( Char Char Char* ) /* The first letter of an NCName */
#[inline]
fn is_nc_namestart_char_t(chr: char) -> bool {
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
        // || chr == ':'
        || chr == '_'
}

fn nc_namestart_char(input: &[u8]) -> IResult<&[u8], &[u8]> {
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

    if is_nc_namestart_char_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

// [4a] NameChar ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
#[inline]
fn is_nc_namechar_t(chr: char) -> bool {
    is_nc_namestart_char_t(chr)
        || (chr >= '0' && chr <= '9')
        || (chr >= '\u{0300}' && chr <= 'z')
        || (chr >= '\u{203F}' && chr <= '\u{2040}')
        || chr == '-'
        || chr == '.'
        || chr == '\u{B7}'
}

fn nc_namechar(input: &[u8]) -> IResult<&[u8], &[u8]> {
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

    if is_nc_namechar_t(c) {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

fn nc_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(pair(nc_namestart_char, many0_custom_chardata(nc_namechar)))(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QName<'a> {
    pub prefix: &'a str,
    pub local_name: &'a str,
    pub prefix_range: Range<usize>,
    pub local_name_range: Range<usize>,
}
pub fn QName(input: &[u8]) -> IResult<&[u8], QName> {
    alt((
        //first try harder alternative
        (map(
            terminated(
                separated_pair(nc_name, char(':'), nc_name),
                nom::combinator::eof,
            ),
            |(pre, loc)| {
                let pre_start = input.offset(pre);
                let local_start = input.offset(loc);
                QName {
                    prefix: unsafe { std::str::from_utf8_unchecked(pre) },
                    local_name: unsafe { std::str::from_utf8_unchecked(loc) },
                    prefix_range: Range {
                        start: pre_start,
                        end: pre_start + pre.len(),
                    },
                    local_name_range: Range {
                        start: local_start,
                        end: local_start + loc.len(),
                    },
                }
            },
        )),
        map(terminated(nc_name, nom::combinator::eof), |loc| {
            let local_start = input.offset(loc);
            QName {
                prefix: "",
                local_name: unsafe { std::str::from_utf8_unchecked(loc) },
                prefix_range: 0..0,
                local_name_range: Range {
                    start: local_start,
                    end: local_start + loc.len(),
                },
            }
        }),
    ))(input)
}

#[test]
fn test_qname() {
    assert_eq!(
        QName(":no".as_bytes()),
        Err(Err::Error(error_position!(
            ":no".as_bytes(),
            ErrorKind::Char
        )))
    );

    //this should fail
    assert_eq!(
        QName("a:b:".as_bytes()),
        Err(Err::Error(error_position!(
            ":b:".as_bytes(),
            ErrorKind::Eof
        )))
    );

    assert_eq!(
        QName("a:b".as_bytes()),
        Ok((
            &b""[..],
            QName {
                prefix: &"a",
                local_name: &"b",
                prefix_range: 0..1,
                local_name_range: 2..3
            }
        ))
    );

    assert_eq!(
        QName("a:123".as_bytes()),
        Err(Err::Error(error_position!(
            ":123".as_bytes(),
            ErrorKind::Eof
        )))
    );
}
