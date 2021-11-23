use std::ops::{Range, RangeFrom, RangeFull};

use nom::{
    branch::alt,
    bytes::streaming::{escaped, tag, take_while},
    character::complete::{alphanumeric1 as alphanumeric, char, none_of, one_of},
    combinator::{cut, map, opt, recognize, value},
    error::{
        context, convert_error, dbg_dmp, ContextError, Error, ErrorKind, ParseError, VerboseError,
    },
    multi::{many1, separated_list0},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    AsChar, Err, IResult, InputIter, InputLength, Needed, Parser, Slice,
};
use xml_sax::StartElement;

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

// [40] STag ::= '<' Name (S Attribute)* S? '>'

fn STag(input: &[u8]) -> IResult<&[u8], xml_sax::StartElement> {
    match tuple((char('<'), name))(input) {
        Ok((i, o)) => Ok((
            i,
            StartElement {
                name: unsafe { std::str::from_utf8_unchecked(o.1) },
            },
        )),

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

//TODO remove c:char , this is not generic
pub fn is_xml_char<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, char, Error>
where
    I: Slice<RangeFrom<usize>> + InputIter,
    <I as InputIter>::Item: AsChar,
{
    move |i: I| match (i).iter_elements().next().map(|t| {
        let chr = t.as_char();
        let b = (chr >= '\u{A}' && chr <= '\u{D}')
            || (chr >= '\u{20}' && chr <= '\u{D7FF}')
            || (chr >= '\u{E000}' && chr <= '\u{FFFD}')
            || (chr >= '\u{10000}' && chr <= '\u{10FFFF}');
        (&c, b)
    }) {
        Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
        _ => Err(Err::Error(Error::from_char(i, c))),
    }
}

pub fn char2<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, I, Error>
where
    I: Slice<RangeFrom<usize>> + Slice<Range<usize>> + InputIter + InputLength,
    <I as InputIter>::Item: AsChar,
{
    move |i: I| match (i).iter_elements().next().map(|t| {
        let b = t.as_char() == c;
        (&c, b)
    }) {
        None => Err(Err::Incomplete(Needed::new(c.len() - i.input_len()))),
        Some((_, false)) => Err(Err::Error(Error::from_char(i, c))),
        Some((c, true)) => Ok((i.slice(c.len()..), i.slice(0..c.len()))),
    }
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

#[test]
fn test_xml4() {
    let data = "lelementnameexample".as_bytes();

    fn parser(s: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
        tuple((alt((char2('e'), char2('l'))), alt((char2('e'), char2('l')))))(s)
    }

    let res = parser(&data);
    println!("{:?}", res);
}
