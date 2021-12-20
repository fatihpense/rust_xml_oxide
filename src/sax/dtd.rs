
use std::ops::Mul;

use super::internal::{
    is_xml_char_t, many0_custom_trycomplete, many1_custom, name, utf8_char_width, Comment, PI,
};

use nom::error_position;

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

//only parsed without checking well-formedness inside
// [28] doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
fn doctypedecl_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("<!DOCTYPE")(input)
}

fn doctypedecl_end(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag(">")(input)
}

// fn inside_doctypedecl_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
//     //if input = 0 , don't send incomplete
//     // ref#streamcut
//     if input.len() == 0 {
//         return Err(Err::Error(Error::new(input, ErrorKind::Char)));
//     }

//     // ']]>' should not appear in the cdata section, if we can't be sure because input is eof, we should request more data.
//     match tag::<&str, &[u8], Error<&[u8]>>(">")(input) {
//         Ok(r) => return Err(Err::Error(Error::new(input, ErrorKind::Char))),
//         Err(Err::Incomplete(n)) => return Err(Err::Incomplete(Needed::Unknown)),
//         _ => (),
//     };
//     inside_Comment_or_CDATA_single_pure(input)
// }

//char that is not > or <
fn inside_doctypedecl_single_pure(input: &[u8]) -> IResult<&[u8], &[u8]> {
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

    if is_xml_char_t(c) && c != '<' && c != '>' {
        return Ok((&input[width..], &input[0..width]));
    } else {
        return Err(Err::Error(Error::new(input, ErrorKind::Char)));
    }
}

fn doctypedecl_dummy_internal(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<"),
        many0_custom_trycomplete(alt((
            recognize(many1_custom(inside_doctypedecl_single_pure)),
            Comment,
            doctypedecl_dummy_internal,
        ))),
        tag(">"),
    )))(input)
}

// [12] PubidLiteral ::= '"' PubidChar* '"' | "'" (PubidChar - "'")* "'"
fn PubidLiteral_12(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(char('"'), PubidChar_13_many, char('"')),
        delimited(
            char('\''),
            recognize(many0_custom_trycomplete(alt((
                is_not(r#"'"#),
                PubidChar_13_many,
            )))),
            char('\''),
        ),
    ))(input)
}

//no tab
// [13] PubidChar ::= #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
fn PubidChar_13_many(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom_trycomplete(alt((
        nom::bytes::streaming::take_while(nom::character::is_alphanumeric),
        nom::bytes::streaming::is_a("-'()+,./:=?;!*#@$_% \r\n"),
    ))))(input)
}

// [11] SystemLiteral ::= ('"' [^"]* '"') | ("'" [^']* "'")
fn SystemLiteral_11(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(
            char('"'),
            recognize(many0_custom_trycomplete(is_not(r#"""#))),
            char('"'),
        ),
        delimited(
            char('\''),
            recognize(many0_custom_trycomplete(is_not(r#"'"#))),
            char('\''),
        ),
    ))(input)
}

// [75] ExternalID ::= 'SYSTEM' S SystemLiteral | 'PUBLIC' S PubidLiteral S SystemLiteral
fn ExternalID_75(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        recognize(tuple((tag("SYSTEM"), multispace1, SystemLiteral_11))),
        recognize(tuple((
            tag("PUBLIC"),
            multispace1,
            PubidLiteral_12,
            multispace1,
            SystemLiteral_11,
        ))),
    ))(input)
}

// [69] PEReference ::= '%' Name ';'
fn PEReference_69(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag("%"), name, tag(";"))))(input)
}

// [28a] DeclSep ::= PEReference | S
fn DeclSep_28a(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((PEReference_69, multispace1)))(input)
}

// [83] PublicID ::= 'PUBLIC' S PubidLiteral
fn PublicID_83(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((PEReference_69, multispace1)))(input)
}

// [82] NotationDecl ::= '<!NOTATION' S Name S (ExternalID | PublicID) S? '>'
fn NotationDecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!NOTATION"),
        multispace1,
        name,
        multispace1,
        alt((ExternalID_75, PublicID_83)),
        multispace0,
    )))(input)
}

// [70] EntityDecl ::= GEDecl | PEDecl
fn EntityDecl_70(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((GEDecl_71, PEDecl_72))(input)
}

// [71] GEDecl ::= '<!ENTITY' S Name S EntityDef S? '>'
fn GEDecl_71(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!ENTITY"),
        multispace1,
        name,
        multispace1,
        EntityDef_73,
        multispace0,
        tag(">"),
    )))(input)
}

// [72] PEDecl ::= '<!ENTITY' S '%' S Name S PEDef S? '>'
fn PEDecl_72(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!ENTITY"),
        multispace1,
        tag("%"),
        multispace1,
        name,
        multispace1,
        PEDef_74,
        multispace0,
        tag(">"),
    )))(input)
}

// [73] EntityDef ::= EntityValue | (ExternalID NDataDecl?)
fn EntityDef_73(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        recognize(tuple((ExternalID_75, opt(NDataDecl_76)))),
        EntityValue,
    ))(input)
}

// [76] NDataDecl ::= S 'NDATA' S Name
fn NDataDecl_76(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((multispace1, tag("NDATA"), multispace1, name)))(input)
}

// [74] PEDef ::= EntityValue | ExternalID
fn PEDef_74(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((ExternalID_75, EntityValue))(input)
}

// [9] EntityValue ::= '"' ([^%&"] | PEReference | Reference)* '"'
// |  "'" ([^%&'] | PEReference | Reference)* "'"
fn EntityValue(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(
            char('"'),
            recognize(many0_custom_trycomplete(alt((
                is_not(r#"<%""#),
                super::internal::Reference,
                PEReference_69,
            )))),
            char('"'),
        ),
        delimited(
            char('\''),
            recognize(many0_custom_trycomplete(alt((
                is_not(r#"<%'"#),
                super::internal::Reference,
                PEReference_69,
            )))),
            char('\''),
        ),
    ))(input)
}

// [52] AttlistDecl ::= '<!ATTLIST' S Name AttDef* S? '>'
fn AttlistDecl_52(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!ATTLIST"),
        multispace1,
        name,
        multispace1,
        recognize(many0_custom_trycomplete(AttDef_53)),
        multispace0,
        tag(">"),
    )))(input)
}
// [53] AttDef ::= S Name S AttType S DefaultDecl
fn AttDef_53(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        multispace1,
        name,
        multispace1,
        AttType_54,
        multispace1,
        DefaultDecl_60,
    )))(input)
}
// [60] DefaultDecl ::= '#REQUIRED' | '#IMPLIED' | (('#FIXED' S)? AttValue)
fn DefaultDecl_60(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((
        tag("#REQUIRED"),
        tag("#IMPLIED"),
        recognize(tuple((
            opt(tuple((tag("#FIXED"), multispace1))),
            super::internal::AttValue,
        ))),
    )))(input)
}

// [57] EnumeratedType ::= NotationType | Enumeration
fn EnumeratedType_57(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((NotationType_58, Enumeration_59))(input)
}

// [58] NotationType ::= 'NOTATION' S '(' S? Name (S? '|' S? Name)* S? ')'
fn NotationType_58(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("NOTATION"),
        multispace1,
        tag("("),
        multispace0,
        name,
        recognize(many0_custom_trycomplete(tuple((
            multispace0,
            tag("|"),
            multispace0,
            name,
            multispace0,
        )))),
        tag(")"),
    )))(input)
}

// [59] Enumeration ::= '(' S? Nmtoken (S? '|' S? Nmtoken)* S? ')'
fn Enumeration_59(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("("),
        multispace0,
        Nmtoken_7,
        recognize(many0_custom_trycomplete(tuple((
            multispace0,
            tag("|"),
            multispace0,
            Nmtoken_7,
        )))),
        multispace0,
        tag(")"),
    )))(input)
}

// [54] AttType ::= StringType | TokenizedType | EnumeratedType
fn AttType_54(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((StringType_55, TokenizedType_56, EnumeratedType_57))(input)
}

// [7] Nmtoken ::= (NameChar)+
fn Nmtoken_7(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(pair(
        super::internal::namechar,
        many0_custom_trycomplete( super::internal::namechar),
    ))(input)
}
// [8] Nmtokens ::= Nmtoken (#x20 Nmtoken)*
fn Nmtokens_8(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(pair(
        Nmtoken_7,
        many0_custom_trycomplete(tuple((char(' '), Nmtoken_7))),
    ))(input)
}

// [55] StringType ::= 'CDATA'
fn StringType_55(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("CDATA")(input)
}

// [56] TokenizedType ::= 'ID' | 'IDREF' | 'IDREFS' | 'ENTITY' | 'ENTITIES' | 'NMTOKEN' | 'NMTOKENS'
fn TokenizedType_56(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((
        tag("ID"),
        tag("IDREF"),
        tag("IDREFS"),
        tag("ENTITY"),
        tag("ENTITIES"),
        tag("NMTOKEN"),
        tag("NMTOKENS"),
    )))(input)
}

// [45] elementdecl ::= '<!ELEMENT' S Name S contentspec S? '>'
fn elementdecl_45(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!ELEMENT"),
        multispace1,
        name,
        multispace1,
        contentspec_46,
        multispace0,
        tag(">"),
    )))(input)
}

// [46] contentspec ::= 'EMPTY' | 'ANY' | Mixed | children
fn contentspec_46(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((tag("EMPTY"), tag("ANY"), Mixed_51, children_47)))(input)
}

// [51] Mixed ::= '(' S? '#PCDATA' (S? '|' S? Name)* S? ')*' | '(' S? '#PCDATA' S? ')'
fn Mixed_51(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((
        recognize(tuple((
            tag("("),
            multispace0,
            tag("#PCDATA"),
            recognize(many0_custom_trycomplete(tuple((
                multispace0,
                tag("|"),
                multispace0,
                name,
            )))),
            multispace0,
            tag(")*"),
        ))),
        recognize(tuple((
            tag("("),
            multispace0,
            tag("#PCDATA"),
            multispace0,
            tag(")"),
        ))),
    )))(input)
}

// [47] children ::= (choice | seq) ('?' | '*' | '+')?
fn children_47(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        alt((choice_49, seq_50)),
        opt(alt((char('?'), char('*'), char('+')))),
    )))(input)
}

// [48] cp ::= (Name | choice | seq) ('?' | '*' | '+')?
fn cp_48(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        alt((name, choice_49, seq_50)),
        opt(alt((char('?'), char('*'), char('+')))),
    )))(input)
}

// [49] choice ::= '(' S? cp ( S? '|' S? cp )+ S? ')'
fn choice_49(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("("),
        multispace0,
        cp_48,
        multispace0, // second choice (first for +)
        tag("|"),
        multispace0,
        cp_48, // second choice end
        recognize(many0_custom_trycomplete(tuple((
            multispace0,
            tag("|"),
            multispace0,
            cp_48,
        )))),
        multispace0,
        tag(")"),
    )))(input)
}

// [50] seq ::= '(' S? cp ( S? ',' S? cp )* S? ')'
fn seq_50(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("("),
        multispace0,
        cp_48,
        recognize(many0_custom_trycomplete(tuple((
            multispace0,
            tag(","),
            multispace0,
            cp_48,
        )))),
        multispace0,
        tag(")"),
    )))(input)
}

//TODO
// [29] markupdecl ::= elementdecl | AttlistDecl | EntityDecl | NotationDecl | PI | Comment
fn markupdecl_29(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((
        elementdecl_45,
        AttlistDecl_52,
        EntityDecl_70,
        NotationDecl,
        PI,
        Comment,
    )))(input)
}

// [28b] intSubset ::= (markupdecl | DeclSep)*
fn intSubset_28b(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom_trycomplete(alt((markupdecl_29, DeclSep_28a))))(input)
}

//  can contain nested < and > for attlist and internal comments

// [28] doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
pub fn doctypedecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        doctypedecl_start,
        multispace1,
        name,
        opt(tuple((multispace1, ExternalID_75))),
        multispace0,
        opt(tuple((tag("["), intSubset_28b, tag("]"), multispace0))),
        doctypedecl_end,
    )))(input)
}

#[test]
fn test_doctypedecl() {
    assert_eq!(
        doctypedecl(r#"<!DOCTYPE>a"#.as_bytes()),
        // Ok((&b"a"[..], &br#"<!DOCTYPE>"#[..]))
        Err(Err::Error(error_position!(
            ">a".as_bytes(),
            ErrorKind::MultiSpace
        )))
    );

    assert_eq!(
        doctypedecl(r#"<!DOCTYPE greeting SYSTEM "hello.dtd">a"#.as_bytes()),
        Ok((&b"a"[..], &br#"<!DOCTYPE greeting SYSTEM "hello.dtd">"#[..]))
    );

    assert_eq!(
        doctypedecl(r#"<!DOCTYPE dummy>"#.as_bytes()),
        Ok((&b""[..], &br#"<!DOCTYPE dummy>"#[..]))
    );

    assert_eq!(
        doctypedecl(r#"<!DOCTYPE myhtml  [  <!-- -->  ] >dummy"#.as_bytes()),
        Ok((&b"dummy"[..], &br#"<!DOCTYPE myhtml  [  <!-- -->  ] >"#[..]))
    );

    //also works > inside comment
    assert_eq!(
        doctypedecl(r#"<!DOCTYPE test  [ ]>dummy"#.as_bytes()),
        Ok((&b"dummy"[..], &br#"<!DOCTYPE test  [ ]>"#[..]))
    );
}
