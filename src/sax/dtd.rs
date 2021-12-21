#![allow(non_snake_case)]

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

// [12] PubidLiteral ::= '"' PubidChar* '"' | "'" (PubidChar - "'")* "'"
fn PubidLiteral_12(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        delimited(char('"'), PubidChar_13_many, char('"')),
        delimited(
            char('\''),
            recognize(PubidChar_13_many_except_single_quote),
            char('\''),
        ),
    ))(input)
}
#[test]
fn test_PubidLiteral_12() {
    assert_eq!(
        recognize(PubidLiteral_12)(r#""a not very interesting file""#.as_bytes()),
        Ok((&b""[..], &br#""a not very interesting file""#[..]))
    );

    assert_eq!(
        recognize(PubidLiteral_12)(r#"'whatever'"#.as_bytes()),
        Ok((&b""[..], &br#"'whatever'"#[..]))
    );
}

//no tab
// [13] PubidChar ::= #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
fn PubidChar_13_many(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom_trycomplete(alt((
        //take_while1 because many0 should always take one char or give error.
        nom::bytes::complete::take_while1(nom::character::is_alphanumeric),
        nom::bytes::complete::is_a("-'()+,./:=?;!*#@$_% \r\n"),
    ))))(input)
}
#[test]
fn test_PubidChar_13_many() {
    assert_eq!(
        PubidChar_13_many(r#"a not very interesting file"#.as_bytes()),
        Ok((&b""[..], &br#"a not very interesting file"#[..]))
    );
}

fn PubidChar_13_many_except_single_quote(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom_trycomplete(alt((
        //take_while1 because many0 should always take one char or give error.
        nom::bytes::complete::take_while1(nom::character::is_alphanumeric),
        nom::bytes::complete::is_a("-()+,./:=?;!*#@$_% \r\n"),
    ))))(input)
}
#[test]
fn test_PubidChar_13_many_except_single_quote() {
    assert_eq!(
        PubidChar_13_many_except_single_quote(r#"a not very interesting' file"#.as_bytes()),
        Ok((&br#"' file"#[..], &br#"a not very interesting"#[..]))
    );
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

#[test]
fn test_SystemLiteral_11() {
    assert_eq!(
        recognize(SystemLiteral_11)(r#""011.ent""#.as_bytes()),
        Ok((&b""[..], &br#""011.ent""#[..]))
    );
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
#[test]
fn test_ExternalID_75() {
    assert_eq!(
        ExternalID_75(r#"PUBLIC "a not very interesting file" "011.ent""#.as_bytes()),
        Ok((
            &b""[..],
            &br#"PUBLIC "a not very interesting file" "011.ent""#[..]
        ))
    );

    assert_eq!(
        ExternalID_75(r#"PUBLIC 'whatever' "e.dtd""#.as_bytes()),
        Ok((&b""[..], &br#"PUBLIC 'whatever' "e.dtd""#[..]))
    );
}

// [69] PEReference ::= '%' Name ';'
fn PEReference_69(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag("%"), name, tag(";"))))(input)
}

// [83] PublicID ::= 'PUBLIC' S PubidLiteral
fn PublicID_83(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((tag("PUBLIC"), multispace1, PubidLiteral_12)))(input)
}

#[test]
fn test_PublicID_83() {
    let data2 = r#"PUBLIC "whatever""#.as_bytes();
    assert_eq!(PublicID_83(data2), Ok((&b""[..], &data2[..])));
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
        tag(">"),
    )))(input)
}

#[test]
fn test_NotationDecl() {
    let data2 = r#"<!NOTATION n PUBLIC "whatever">]"#.as_bytes();
    assert_eq!(
        NotationDecl(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );
}

// [70] EntityDecl ::= GEDecl | PEDecl
fn EntityDecl_70(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((GEDecl_71, PEDecl_72))(input)
}

#[test]
fn test_EntityDecl_70() {
    let data2 = r#"<!ENTITY % e PUBLIC 'whatever' "e.dtd">]"#.as_bytes();
    assert_eq!(
        EntityDecl_70(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );
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

#[test]
fn test_GEDecl_71() {
    let data2 = r#"<!ENTITY e "&#60;foo></foo>">"#.as_bytes();
    assert_eq!(GEDecl_71(data2), Ok((&b""[..], data2)));
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

#[test]
fn test_PEDecl_72() {
    let data2 = r#"<!ENTITY % e PUBLIC 'whatever' "e.dtd">]"#.as_bytes();
    assert_eq!(
        PEDecl_72(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );
}

// [73] EntityDef ::= EntityValue | (ExternalID NDataDecl?)
fn EntityDef_73(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        recognize(tuple((ExternalID_75, opt(NDataDecl_76)))),
        EntityValue,
    ))(input)
}

#[test]
fn test_EntityDef_73() {
    assert_eq!(
        EntityDef_73(r#"PUBLIC "a not very interesting file" "011.ent""#.as_bytes()),
        Ok((
            &b""[..],
            &br#"PUBLIC "a not very interesting file" "011.ent""#[..]
        ))
    );

    let data2 = r#""&#60;foo></foo>""#.as_bytes();
    assert_eq!(
        EntityDef_73(data2),
        Ok((&b""[..], &br#"&#60;foo></foo>"#[..]))
    );
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
                is_not(r#"%&""#),
                super::internal::Reference,
                PEReference_69,
            )))),
            char('"'),
        ),
        delimited(
            char('\''),
            recognize(many0_custom_trycomplete(alt((
                is_not(r#"%&'"#),
                super::internal::Reference,
                PEReference_69,
            )))),
            char('\''),
        ),
    ))(input)
}

#[test]
fn test_EntityValue() {
    let data2 = r#""&#60;foo></foo>""#.as_bytes();

    assert_eq!(
        EntityValue(data2),
        Ok((&b""[..], &data2[1..data2.len() - 1]))
    );
}

// [52] AttlistDecl ::= '<!ATTLIST' S Name AttDef* S? '>'
fn AttlistDecl_52(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(tuple((
        tag("<!ATTLIST"),
        multispace1,
        name,
        // multispace1,
        recognize(many0_custom_trycomplete(AttDef_53)),
        multispace0,
        tag(">"),
    )))(input)
}

#[test]
fn test_AttlistDecl_52() {
    let data2 = r#"<!ATTLIST e
          a1 CDATA "a1 default"
          a2 NMTOKENS "a2 default"
        >"#
    .as_bytes();
    assert_eq!(AttlistDecl_52(data2), Ok((&b""[..], data2)));
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

#[test]
fn test_AttDef_53() {
    let data2 = r#"
    a2 NMTOKENS "a2 default""#
        .as_bytes();
    assert_eq!(AttDef_53(data2), Ok((&b""[..], data2)));

    let data2 = r#"
    a1 CDATA "a1 default""#
        .as_bytes();
    assert_eq!(AttDef_53(data2), Ok((&b""[..], data2)));

    let data2 = r#" 
    a1 CDATA "a1 default"
    a2 NMTOKENS "a2 default"
   >"#
    .as_bytes();
    assert_eq!(
        recognize(many0_custom_trycomplete(AttDef_53))(data2),
        Ok((
            &br#"
   >"#[..],
            &br#" 
    a1 CDATA "a1 default"
    a2 NMTOKENS "a2 default""#[..]
        ))
    );
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
        many0_custom_trycomplete(super::internal::namechar),
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
        tag("IDREFS"),
        tag("IDREF"),
        tag("ID"),
        tag("ENTITY"),
        tag("ENTITIES"),
        tag("NMTOKENS"), //it should be checked before NMTOKEN
        tag("NMTOKEN"),
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

#[test]
fn test_elementdecl() {
    let _data = "]]".as_bytes();

    assert_eq!(
        elementdecl_45("<!ELEMENT br EMPTY>a".as_bytes()),
        Ok((&b"a"[..], &b"<!ELEMENT br EMPTY>"[..]))
    );
    assert_eq!(
        elementdecl_45("<!ELEMENT p (#PCDATA|emph)* >a".as_bytes()),
        Ok((&b"a"[..], &b"<!ELEMENT p (#PCDATA|emph)* >"[..]))
    );

    let data2 = r#"<!ELEMENT doc (e)>"#.as_bytes();
    assert_eq!(elementdecl_45(data2), Ok((&b""[..], data2)));

    // assert_eq!(
    //     elementdecl_45("<!ELEMENT %name.para; %content.para; >".as_bytes()),
    //     Err(Err::Error(error_position!(
    //         "]]>".as_bytes(),
    //         ErrorKind::Char
    //     )))
    // );
    assert_eq!(
        elementdecl_45("<!ELEMENT container ANY>a".as_bytes()),
        Ok((&b"a"[..], &b"<!ELEMENT container ANY>"[..]))
    );
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
#[test]
fn test_markupdecl_29() {
    let data2 = r#"<!ENTITY x SYSTEM "013.ent">"#.as_bytes();
    assert_eq!(markupdecl_29(data2), Ok((&b""[..], data2)));

    let data2 = r#"<!ELEMENT doc (e)>"#.as_bytes();
    assert_eq!(markupdecl_29(data2), Ok((&b""[..], data2)));

    let data2 = r#"<!ENTITY e "&#60;foo></foo>">"#.as_bytes();
    assert_eq!(markupdecl_29(data2), Ok((&b""[..], data2)));
}

// [28a] DeclSep ::= PEReference | S
fn DeclSep_28a(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(alt((PEReference_69, multispace1)))(input)
}
// [28b] intSubset ::= (markupdecl | DeclSep)*
fn intSubset_28b(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(many0_custom_trycomplete(alt((markupdecl_29, DeclSep_28a))))(input)
}
#[test]
fn test_intSubset_28b() {
    let data2 = r#"<!ELEMENT doc (e)>
        <!ELEMENT e (#PCDATA)>
        <!ATTLIST e
          a1 CDATA "a1 default"
          a2 NMTOKENS "a2 default"
        >
        <!ENTITY x SYSTEM "013.ent">
        ]"#
    .as_bytes();
    assert_eq!(
        intSubset_28b(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );

    let data2 = r#"<!ELEMENT doc (foo)>
<!ELEMENT foo (#PCDATA)>
<!ENTITY e "&#60;foo></foo>">
    ]"#
    .as_bytes();
    assert_eq!(
        intSubset_28b(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );

    let data2 = r#"<!ELEMENT doc (#PCDATA)>
    <!NOTATION n PUBLIC "whatever">
    ]"#
    .as_bytes();
    assert_eq!(
        intSubset_28b(data2),
        Ok((&b"]"[..], &data2[0..data2.len() - 1]))
    );
}

//  can contain nested < and > for attlist and internal comments

// [28] doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
pub(crate) fn doctypedecl(input: &[u8]) -> IResult<&[u8], &[u8]> {
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

    let data = r#"<!DOCTYPE doc [
        <!ELEMENT doc (#PCDATA)>
        <!ENTITY e PUBLIC "a not very interesting file" "011.ent">
        ]>"#
    .as_bytes();
    assert_eq!(doctypedecl(data), Ok((&b""[..], data)));

    let data2 = r#"<!DOCTYPE doc [
<!ELEMENT doc (e)>
<!ELEMENT e (#PCDATA)>
<!ATTLIST e
  a1 CDATA "a1 default"
  a2 NMTOKENS "a2 default"
>
<!ENTITY x SYSTEM "013.ent">
]>"#
    .as_bytes();
    assert_eq!(doctypedecl(data2), Ok((&b""[..], data2)));

    let data2 = r#"<!DOCTYPE doc [
<!ENTITY % e PUBLIC 'whatever' "e.dtd">
<!ELEMENT doc (#PCDATA)>
]>
"#
    .as_bytes();
    assert_eq!(
        doctypedecl(data2),
        Ok((&b"\n"[..], &data2[0..data2.len() - 1]))
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
