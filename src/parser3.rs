use std::{
    io::{BufRead, BufReader, Read},
    ops::{Range, RangeFrom, RangeFull},
    vec,
};

use nom::{
    branch::alt,
    bytes::streaming::{escaped, is_not, tag, take_while, take_while1},
    character::{
        complete::{alphanumeric1 as alphanumeric, char, multispace1, none_of, one_of},
        is_digit, is_hex_digit,
        streaming::{alpha1, alphanumeric1, digit1, multispace0},
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

pub fn many0_custom_chardata<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
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
                Err(e) => return Ok((i, ())),
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
pub fn many0_custom_trycomplete<I, O, E, F>(mut f: F) -> impl FnMut(I) -> IResult<I, (), E>
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
    recognize(pair(namestart_char, many0_custom_trycomplete(namechar)))(input)
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
fn Reference(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((EntityRef, CharRef))(input)
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Reference<'a> {
    initial: &'a str,
    // resolved: &'a str,
}

// [10] AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"

fn AttValue(input: &[u8]) -> IResult<&[u8], &[u8]> {
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

// let mut Attribute = ParsingRule::new("Attribute".to_owned(), RuleType::Sequence);
// Attribute.children_names.push("Name".to_owned());
// Attribute.children_names.push("Eq".to_owned());
// Attribute.children_names.push("AttValue".to_owned());
// rule_nameRegistry.insert(Attribute.rule_name.clone(), Attribute);

// [40] STag ::= '<' Name (S Attribute)* S? '>'

#[derive(Clone, Debug, Eq, PartialEq)]
struct SAXAttribute<'a> {
    pub value: &'a str,
    pub qualified_name: &'a str,
    // fn get_value(&self) -> &str;
    // fn get_local_name(&self) -> &str;
    // fn get_qualified_name(&self) -> &str;
    // fn get_uri(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SAXAttribute2 {
    pub value: std::ops::Range<usize>,
    pub qualified_name: std::ops::Range<usize>,
    // fn get_value(&self) -> &str;
    // fn get_local_name(&self) -> &str;
    // fn get_qualified_name(&self) -> &str;
    // fn get_uri(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StartElement<'a> {
    pub name: &'a str,
    pub attributes: Vec<SAXAttribute<'a>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndElement<'a> {
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

enum ContentRelaxed<'a> {
    CharData(&'a [u8]),
    StartElement(StartElement<'a>),
    EmptyElemTag(StartElement<'a>),
    EndElement(EndElement<'a>),
    Reference(Reference<'a>),
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

//todo add endelement as next step or inform it is an emptyelem tag via event api?
fn content_relaxed_EmptyElemTag(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    match EmptyElemTag(input) {
        Ok(succ) => Ok((succ.0, ContentRelaxed::StartElement(succ.1))),
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

// [custom] relaxed ::= CharData | STag | EmptyElemTag | ETag | ... todo: add
fn content_relaxed(input: &[u8]) -> IResult<&[u8], ContentRelaxed> {
    alt((
        content_relaxed_CharData,
        content_relaxed_STag,
        content_relaxed_EmptyElemTag,
        content_relaxed_ETag,
        content_relaxed_Reference,
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
//  [24]   	VersionInfo	   ::=   	S 'version' Eq ("'" VersionNum "'" | '"' VersionNum '"')

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

// [81]   	EncName	   ::=   	[A-Za-z] ([A-Za-z0-9._] | '-')*
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

// [80]   	EncodingDecl	   ::=   	S 'encoding' Eq ('"' EncName '"' | "'" EncName "'" )
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

// [27]   	Misc	   ::=   	Comment | PI | S
//todo: comment | PI, we may need to separate
fn Misc(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((multispace1,)))(input)
}

fn docstart_custom(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((XMLDecl, multispace0)))(input)
}

#[test]
fn test_XMLDecl() {
    let data = r#"<?xml version="1.0"  encoding="UTF-8" standalone='yes'?>"#.as_bytes();
    let res = XMLDecl(&data);
    println!("{:?}", res);
}

// [1] document ::= prolog element Misc*
// [22]   	prolog	   ::=   	XMLDecl? Misc* (doctypedecl Misc*)?

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParserState {
    Content,
    DocStart,
    DocEnd,
    //inside cdata ?
    //inside comment ?
}

pub struct OxideParser<R: Read> {
    state: ParserState,
    bufreader: BufReader<R>,
    buffer2: Vec<u8>,
    strbuffer: String,
    offset: usize,
}

fn convert_start_element<'a>(
    strbuffer: &'a mut String,
    event1: StartElement,
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
        });
    }

    xml_sax::StartElement {
        name: &strbuffer[start..(start + size)],
        attributes: attributes,
        is_empty: false,
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

impl<R: Read> OxideParser<R> {
    // This method "consumes" the resources of the caller object
    // `self` desugars to `self: Self`

    pub fn start(reader: R) -> OxideParser<R> {
        OxideParser {
            state: ParserState::DocStart,
            bufreader: BufReader::with_capacity(8192, reader),
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
    pub fn read_event<'a, 'b, 'c>(&'a mut self) -> xml_sax::Event<'a> {
        // self.bufreader.consume(self.offset);
        self.buffer2.drain(0..self.offset);
        self.offset = 0;
        self.strbuffer.clear();

        if self.bufreader.capacity() > self.buffer2.len() {
            self.read_data();
        }

        // let mut event: StartElement = StartElement {
        //     name: "",
        //     attributes: vec![],
        // };
        // let mut event1: StartElement; //<'b>; //&'a
        let mut event2: xml_sax::Event;

        if self.state == ParserState::DocStart {
            let res = docstart_custom(&self.buffer2);
            match res {
                Ok(parseresult) => {
                    self.offset = self.buffer2.offset(parseresult.0);
                    self.state = ParserState::Content;
                    return xml_sax::Event::StartDocument;
                }
                Err(err) => panic!(),
            }
        }

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

                        event2 = xml_sax::Event::Characters(&self.strbuffer[start..(start + size)])
                    }
                    ContentRelaxed::StartElement(event1) => {
                        //todo decode
                        event2 = xml_sax::Event::StartElement(convert_start_element(
                            &mut self.strbuffer,
                            event1,
                        ));
                    }
                    ContentRelaxed::EmptyElemTag(event1) => {
                        //todo decode
                        let mut start_elem = convert_start_element(&mut self.strbuffer, event1);
                        start_elem.is_empty = true;
                        event2 = xml_sax::Event::StartElement(start_elem);
                        //todo add endelement after this?
                    }
                    ContentRelaxed::EndElement(event1) => {
                        let start = self.strbuffer.len();
                        let size = event1.name.len();
                        self.strbuffer.push_str(event1.name);

                        event2 = xml_sax::Event::EndElement(xml_sax::EndElement {
                            name: &self.strbuffer[start..(start + size)],
                        })
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

                        let range_resolved = match event1.initial {
                            "&amp;" => push_str_get_range(&mut self.strbuffer, "&"),
                            "&lt" => push_str_get_range(&mut self.strbuffer, "<"),
                            "&gt;" => push_str_get_range(&mut self.strbuffer, ">"),
                            "&quot;" => push_str_get_range(&mut self.strbuffer, "\""),
                            "&apos;" => push_str_get_range(&mut self.strbuffer, "'"),
                            _ => push_str_get_range(&mut self.strbuffer, event1.initial),
                        };

                        //todo resolve char refs
                        //we are ignoring DTD entity refs
                        event2 = xml_sax::Event::Reference(xml_sax::Reference {
                            raw: &self.strbuffer[range],
                            resolved: &self.strbuffer[range_resolved],
                        })
                    }
                }
            }
            Err(Err::Incomplete(e)) => {
                // panic!()
                // self.read_data();
                // if read bytes are 0 then return eof, otherwise return dummy event
                if self.buffer2.len() == 0 {
                    return xml_sax::Event::EndDocument;
                }
                println!("try to read bytes: {:?}", unsafe { &self.buffer2 });
                println!("try to read: {:?}", unsafe {
                    std::str::from_utf8_unchecked(&self.buffer2)
                });
                println!("err: {:?}", e);
                panic!()
            }
            Err(e) => {
                println!("try to read bytes: {:?}", unsafe { &self.buffer2 });
                println!("try to read: {:?}", unsafe {
                    std::str::from_utf8_unchecked(&self.buffer2)
                });
                println!("err: {:?}", e);

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
    let data = r#"<root><A a='x'>
    <B b="val" a:12='val2' ><C></root>"#
        .as_bytes();

    // let mut buf = vec![];
    let mut p = OxideParser::start(data);
    loop {
        let res = p.read_event();
        println!("{:?}", res);
        match res {
            xml_sax::Event::StartDocument => todo!(),
            xml_sax::Event::EndDocument => todo!(),
            xml_sax::Event::StartElement(el) => {
                if el.name == "C" {
                    break;
                }
            }
            xml_sax::Event::EndElement(_) => todo!(),
            xml_sax::Event::Characters(c) => {}
            xml_sax::Event::Reference(c) => {}
        }
    }

    // let res = p.read_event();
    // println!("{:?}", res);

    // let res = p.read_event();
    // println!("{:?}", res);

    // let res = p.read_event();
    // println!("{:?}", res);
}
