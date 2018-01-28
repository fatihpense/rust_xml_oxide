
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
#[derive(Debug,Clone)]
pub enum RuleType {
    And, //new parser specific
    Not, //new parser specific
    Sequence,
    Or,
    Chars, // directly chars
    CharSequence, // easy char sequence for <!-- cdata etc. TODO
    CharsNot,
    ZeroOrMore, //new parser delete
    WithException, //new parser delete
    Optional, //new parser delete
}

pub struct Parser {
    pub rule_vec: Vec<ParsingRule>,
    pub rule_registry: HashMap<String, usize>,
}
#[derive(Clone,Debug)]
pub struct ParsingRule {
    pub rule_name: String,
    pub rule_type: RuleType,
    pub children: Vec<usize>,
    pub children_names: Vec<String>,
    pub expected_char_ranges: Vec<(char, char)>,
    pub expected_chars: Vec<char>,
    is_chunkable: bool,
}

impl ParsingRule {
    pub fn new(rule_name: String, rule_type: RuleType) -> ParsingRule {
        ParsingRule {
            rule_name: rule_name,
            rule_type: rule_type,
            children: Vec::new(),
            children_names: Vec::new(),
            expected_char_ranges: Vec::new(),
            expected_chars: Vec::new(),
            is_chunkable: true,
        }
    }
}

// it is easier to follow XML Spec naming
#[allow(non_snake_case)]
pub fn prepare_rules<'a>() -> Parser {
    let mut rule_vec = Vec::new();

    let mut ruleRegistry: HashMap<String, usize> = HashMap::new();
    let mut rule_nameRegistry: HashMap<String, ParsingRule> = HashMap::new();


    // general rule implementations
    let mut AposChar = ParsingRule::new("'".to_owned(), RuleType::Chars);
    AposChar.expected_chars.push('\'');
    rule_nameRegistry.insert(AposChar.rule_name.clone(), AposChar);

    let mut QuoteChar = ParsingRule::new("\"".to_owned(), RuleType::Chars);
    QuoteChar.expected_chars.push('"');
    rule_nameRegistry.insert(QuoteChar.rule_name.clone(), QuoteChar);

    let mut charLT = ParsingRule::new("<".to_owned(), RuleType::Chars);
    charLT.expected_chars.push('<');
    rule_nameRegistry.insert(charLT.rule_name.clone(), charLT);


    let mut charBT = ParsingRule::new(">".to_owned(), RuleType::Chars);
    charBT.expected_chars.push('>');
    rule_nameRegistry.insert(charBT.rule_name.clone(), charBT);


    let mut endTagToken = ParsingRule::new("'</'".to_owned(), RuleType::CharSequence);
    endTagToken.expected_chars.push('<');
    endTagToken.expected_chars.push('/');
    rule_nameRegistry.insert(endTagToken.rule_name.clone(), endTagToken);

    let mut percent_char = ParsingRule::new("'%'".to_owned(), RuleType::Chars);
    percent_char.expected_chars.push('%');
    rule_nameRegistry.insert(percent_char.rule_name.clone(), percent_char);

    let mut ampersand_char = ParsingRule::new("'&'".to_owned(), RuleType::Chars);
    ampersand_char.expected_chars.push('&');
    rule_nameRegistry.insert(ampersand_char.rule_name.clone(), ampersand_char);

    let mut semicolon_char = ParsingRule::new("';'".to_owned(), RuleType::Chars);
    semicolon_char.expected_chars.push(';');
    rule_nameRegistry.insert(semicolon_char.rule_name.clone(), semicolon_char);


    let mut notLTorAmp = ParsingRule::new("[^<&]".to_owned(), RuleType::CharsNot);
    notLTorAmp.expected_chars.push('<');
    notLTorAmp.expected_chars.push('&');
    rule_nameRegistry.insert(notLTorAmp.rule_name.clone(), notLTorAmp);


    let mut notLTorAmpZeroOrMore = ParsingRule::new("[^<&]*".to_owned(), RuleType::ZeroOrMore);
    notLTorAmpZeroOrMore.children_names.push("[^<&]".to_owned());

    rule_nameRegistry.insert(notLTorAmpZeroOrMore.rule_name.clone(), notLTorAmpZeroOrMore);

    let mut not_lt_amp_quote = ParsingRule::new("[^<&\"]".to_owned(), RuleType::CharsNot);
    not_lt_amp_quote.expected_chars.push('<');
    not_lt_amp_quote.expected_chars.push('&');
    not_lt_amp_quote.expected_chars.push('\"');
    rule_nameRegistry.insert(not_lt_amp_quote.rule_name.clone(), not_lt_amp_quote);

    let mut not_lt_amp_quote_zom = ParsingRule::new("[^<&\"]*".to_owned(), RuleType::ZeroOrMore);
    not_lt_amp_quote_zom.children_names.push("[^<&\"]".to_owned());
    rule_nameRegistry.insert(not_lt_amp_quote_zom.rule_name.clone(), not_lt_amp_quote_zom);

    //  ([^<&]* ']]>' [^<&]*)

    let mut CdataEnd = ParsingRule::new("']]>'".to_owned(), RuleType::CharSequence);
    CdataEnd.expected_chars.push(']');
    CdataEnd.expected_chars.push(']');
    CdataEnd.expected_chars.push('>');

    rule_nameRegistry.insert(CdataEnd.rule_name.clone(), CdataEnd);



    // Parser Rules organized by W3C Spec

    // [1] document ::= prolog element Misc*
    // TODO prolog Misc
    let mut document = ParsingRule::new("document".to_owned(), RuleType::Sequence);
    document.children_names.push("XMLDecl?".to_owned());
    // TODO remove S? as it is contained in prolog
    document.children_names.push("S?".to_owned());
    document.children_names.push("element".to_owned());
    rule_nameRegistry.insert(document.rule_name.clone(), document);

    // [2] Char ::= #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]
    let mut Char = ParsingRule::new("Char".to_owned(), RuleType::Chars);
    let char_rule_ranges_arr = [('\u{A}', '\u{D}'),
                                ('\u{20}', '\u{D7FF}'),
                                ('\u{E000}', '\u{FFFD}'),
                                ('\u{10000}', '\u{10FFFF}')];
    Char.expected_char_ranges.extend(char_rule_ranges_arr.iter().cloned());
    rule_nameRegistry.insert(Char.rule_name.clone(), Char);

    let mut Char_optional = ParsingRule::new("Char?".to_owned(), RuleType::Optional);
    Char_optional.children_names.push("Char".to_owned());
    rule_nameRegistry.insert(Char_optional.rule_name.clone(), Char_optional);

    // zero or more
    let mut Char_zom = ParsingRule::new("Char*".to_owned(), RuleType::ZeroOrMore);
    Char_zom.children_names.push("Char".to_owned());
    rule_nameRegistry.insert(Char_zom.rule_name.clone(), Char_zom);



    // [3] S ::= (#x20 | #x9 | #xD | #xA)+
    let mut S_single = ParsingRule::new("S_single".to_owned(), RuleType::Chars);
    S_single.expected_chars = vec!['\x20', '\x09', '\x0D', '\x0A'];
    rule_nameRegistry.insert(S_single.rule_name.clone(), S_single);

    let mut S_ZeroOrMore = ParsingRule::new("S_single*".to_owned(), RuleType::ZeroOrMore);
    S_ZeroOrMore.children_names.push("S_single".to_owned());
    rule_nameRegistry.insert(S_ZeroOrMore.rule_name.clone(), S_ZeroOrMore);

    let mut S = ParsingRule::new("S".to_owned(), RuleType::Sequence);
    S.children_names.push("S_single".to_owned());
    S.children_names.push("S_single*".to_owned());
    rule_nameRegistry.insert(S.rule_name.clone(), S);

    let mut S_optional = ParsingRule::new("S?".to_owned(), RuleType::Optional);
    S_optional.children_names.push("S".to_owned());
    rule_nameRegistry.insert(S_optional.rule_name.clone(), S_optional);

    // [4] NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |
    // [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] |
    // [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]
    let mut NameStartChar = ParsingRule::new("NameStartChar".to_owned(), RuleType::Chars);

    let name_start_char_rule_ranges_arr = [('A', 'Z'), /* ('A', 'Z'), veya ('\u{0041}', '\u{005A}'), */
                                           ('a', 'z'), // ('a', 'z') veya ('\u{61}', '\u{7A}'),
                                           ('\u{C0}', '\u{D6}'),
                                           ('\u{D8}', '\u{F6}'),
                                           ('\u{F8}', '\u{2FF}'),
                                           ('\u{370}', '\u{37D}'),
                                           ('\u{37F}', '\u{1FFF}'),
                                           ('\u{200C}', '\u{200D}'),
                                           ('\u{2070}', '\u{218F}'),
                                           ('\u{2C00}', '\u{2FEF}'),
                                           ('\u{3001}', '\u{D7FF}'),
                                           ('\u{F900}', '\u{FDCF}'),
                                           ('\u{FDF0}', '\u{FFFD}'),
                                           ('\u{10000}', '\u{EFFFF}')];

    NameStartChar.expected_char_ranges.extend(name_start_char_rule_ranges_arr.iter().cloned());
    NameStartChar.expected_chars.push(':');
    NameStartChar.expected_chars.push('_');

    rule_nameRegistry.insert(NameStartChar.rule_name.clone(), NameStartChar);


    // [4a] NameChar ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
    let mut NameCharExtraRule = ParsingRule::new("NameCharExtra".to_owned(), RuleType::Chars);
    let name_char_extra_rule_ranges_arr =
        [('0', '9'), ('a', 'z'), ('\u{0300}', '\u{036F}'), ('\u{203F}', '\u{2040}')];
    NameCharExtraRule.expected_char_ranges.extend(name_char_extra_rule_ranges_arr.iter().cloned());
    NameCharExtraRule.expected_chars.push('-');
    NameCharExtraRule.expected_chars.push('.');
    NameCharExtraRule.expected_chars.push('\u{B7}');
    rule_nameRegistry.insert(NameCharExtraRule.rule_name.clone(), NameCharExtraRule);

    let mut NameChar = ParsingRule::new("NameChar".to_owned(), RuleType::Or);
    NameChar.children_names.push("NameStartChar".to_owned());
    NameChar.children_names.push("NameCharExtra".to_owned());
    rule_nameRegistry.insert(NameChar.rule_name.clone(), NameChar);

    let mut NameCharZeroOrMore = ParsingRule::new("NameChar*".to_owned(), RuleType::ZeroOrMore);
    NameCharZeroOrMore.children_names.push("NameChar".to_owned());
    rule_nameRegistry.insert(NameCharZeroOrMore.rule_name.clone(), NameCharZeroOrMore);

    let mut Name = ParsingRule::new("Name".to_owned(), RuleType::Sequence);
    Name.children_names.push("NameStartChar".to_owned());
    Name.children_names.push("NameChar*".to_owned());
    rule_nameRegistry.insert(Name.rule_name.clone(), Name);


    // [10] AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"
    let mut AttValue_char_or_ref = ParsingRule::new("([^<&\"] | Reference)".to_owned(),
                                                    RuleType::Or);
    AttValue_char_or_ref.children_names.push("[^<&\"]".to_owned());
    AttValue_char_or_ref.children_names.push("ReferenceInAttrVal".to_owned());
    rule_nameRegistry.insert(AttValue_char_or_ref.rule_name.clone(), AttValue_char_or_ref);

    let mut AttValue_char_or_ref_zom = ParsingRule::new("([^<&\"] | Reference)*".to_owned(),
                                                        RuleType::ZeroOrMore);
    AttValue_char_or_ref_zom.children_names.push("([^<&\"] | Reference)".to_owned());
    rule_nameRegistry.insert(AttValue_char_or_ref_zom.rule_name.clone(),
                             AttValue_char_or_ref_zom);

    let mut AttValue_alt_1 = ParsingRule::new("AttValue_alt_1".to_owned(), RuleType::Sequence);
    AttValue_alt_1.children_names.push("\"".to_owned());
    AttValue_alt_1.children_names.push("([^<&\"] | Reference)*".to_owned());
    AttValue_alt_1.children_names.push("\"".to_owned());
    rule_nameRegistry.insert(AttValue_alt_1.rule_name.clone(), AttValue_alt_1);

    let mut AttValue_alt_2 = ParsingRule::new("AttValue_alt_2".to_owned(), RuleType::Sequence);
    AttValue_alt_2.children_names.push("'".to_owned());
    AttValue_alt_2.children_names.push("([^<&\"] | Reference)*".to_owned());
    AttValue_alt_2.children_names.push("'".to_owned());
    rule_nameRegistry.insert(AttValue_alt_2.rule_name.clone(), AttValue_alt_2);

    let mut AttValue = ParsingRule::new("AttValue".to_owned(), RuleType::Or);
    AttValue.children_names.push("AttValue_alt_1".to_owned());
    AttValue.children_names.push("AttValue_alt_2".to_owned());
    rule_nameRegistry.insert(AttValue.rule_name.clone(), AttValue);





    // [14] CharData ::= [^<&]* - ([^<&]* ']]>' [^<&]*)

    // let mut CharDataExceptionSeq = ParsingRule::new("([^<&]* ']]>' [^<&]*)".to_owned() , RuleType::Sequence);
    // CharDataExceptionSeq.children_names.push("[^<&]*".to_owned() );
    // CharDataExceptionSeq.children_names.push("']]>'".to_owned() );
    // CharDataExceptionSeq.children_names.push("[^<&]*".to_owned() );
    // rule_nameRegistry.insert(CharDataExceptionSeq.rule_name.clone(), CharDataExceptionSeq);

    let mut charDataSingleBeforeException = ParsingRule::new("([^<&] - ']]>')".to_owned(),
                                                             RuleType::WithException);
    charDataSingleBeforeException.children_names.push("[^<&]".to_owned());
    charDataSingleBeforeException.children_names.push("']]>'".to_owned());
    rule_nameRegistry.insert(charDataSingleBeforeException.rule_name.clone(),
                             charDataSingleBeforeException);

    let mut charDataBeforeException = ParsingRule::new("([^<&] - ']]>')*".to_owned(),
                                                       RuleType::ZeroOrMore);
    charDataBeforeException.children_names.push("([^<&] - ']]>')".to_owned());
    rule_nameRegistry.insert(charDataBeforeException.rule_name.clone(),
                             charDataBeforeException);


    let mut CharDataException = ParsingRule::new("([^<&] - ']]>')* ']]>' [^<&]*".to_owned(),
                                                 RuleType::Sequence);
    CharDataException.children_names.push("([^<&] - ']]>')*".to_owned());
    CharDataException.children_names.push("']]>'".to_owned());
    CharDataException.children_names.push("[^<&]*".to_owned());
    rule_nameRegistry.insert(CharDataException.rule_name.clone(), CharDataException);


    let mut CharData = ParsingRule::new("CharData".to_owned(), RuleType::WithException);
    CharData.children_names.push("[^<&]*".to_owned());
    CharData.children_names.push("([^<&] - ']]>')* ']]>' [^<&]*".to_owned());
    rule_nameRegistry.insert(CharData.rule_name.clone(), CharData);


    let mut CharDataOptional = ParsingRule::new("CharData?".to_owned(), RuleType::Optional);
    CharDataOptional.children_names.push("CharData".to_owned());
    rule_nameRegistry.insert(CharDataOptional.rule_name.clone(), CharDataOptional);


    // [15] Comment ::= '<!--' ((Char - '-') | ('-' (Char - '-')))* '-->'
    let mut Comment_start = ParsingRule::new("'<!--'".to_owned(), RuleType::CharSequence);
    Comment_start.expected_chars = "<!--".chars().collect();
    rule_nameRegistry.insert(Comment_start.rule_name.clone(), Comment_start);

    let mut Comment_end = ParsingRule::new("'-->'".to_owned(), RuleType::CharSequence);
    Comment_end.expected_chars = "-->".chars().collect();
    rule_nameRegistry.insert(Comment_end.rule_name.clone(), Comment_end);

    let mut Comment_2hyphen = ParsingRule::new("'--'".to_owned(), RuleType::CharSequence);
    Comment_2hyphen.expected_chars = "--".chars().collect();
    rule_nameRegistry.insert(Comment_2hyphen.rule_name.clone(), Comment_2hyphen);

    let mut Comment_inside = ParsingRule::new("(Char - '--')".to_owned(), RuleType::WithException);
    Comment_inside.children_names.push("Char".to_owned());
    Comment_inside.children_names.push("'--'".to_owned());
    rule_nameRegistry.insert(Comment_inside.rule_name.clone(), Comment_inside);

    let mut Comment_inside_zom = ParsingRule::new("(Char - '--')*".to_owned(),
                                                  RuleType::ZeroOrMore);
    Comment_inside_zom.children_names.push("(Char - '--')".to_owned());
    rule_nameRegistry.insert(Comment_inside_zom.rule_name.clone(), Comment_inside_zom);

    let mut Comment = ParsingRule::new("Comment".to_owned(), RuleType::Sequence);
    Comment.children_names.push("'<!--'".to_owned());
    Comment.children_names.push("(Char - '--')*".to_owned());
    Comment.children_names.push("'-->'".to_owned());
    rule_nameRegistry.insert(Comment.rule_name.clone(), Comment);


    // [18] CDSect ::= CDStart CData CDEnd
    let mut CDSect = ParsingRule::new("CDSect".to_owned(), RuleType::Sequence);
    CDSect.children_names.push("CDStart".to_owned());
    CDSect.children_names.push("CData".to_owned());
    CDSect.children_names.push("CDEnd".to_owned());
    rule_nameRegistry.insert(CDSect.rule_name.clone(), CDSect);

    // [19] CDStart ::= '<![CDATA['
    let mut CDStart = ParsingRule::new("CDStart".to_owned(), RuleType::CharSequence);
    CDStart.expected_chars = "<![CDATA[".chars().collect();
    rule_nameRegistry.insert(CDStart.rule_name.clone(), CDStart);


    // [20] CData ::= (Char* - (Char* ']]>' Char*))

    let mut CDataSingleWithException = ParsingRule::new("(Char - ']]>')".to_owned(),
                                                        RuleType::WithException);
    CDataSingleWithException.children_names.push("Char".to_owned());
    CDataSingleWithException.children_names.push("']]>'".to_owned());
    rule_nameRegistry.insert(CDataSingleWithException.rule_name.clone(),
                             CDataSingleWithException);

    let mut CData = ParsingRule::new("CData".to_owned(), RuleType::ZeroOrMore);
    CData.children_names.push("(Char - ']]>')".to_owned());
    rule_nameRegistry.insert(CData.rule_name.clone(), CData);


    // [21] CDEnd ::= ']]>'
    let mut CDEnd = ParsingRule::new("CDEnd".to_owned(), RuleType::CharSequence);
    CDEnd.expected_chars = "]]>".chars().collect();
    rule_nameRegistry.insert(CDEnd.rule_name.clone(), CDEnd);


    // [23] XMLDecl ::= '<?xml' VersionInfo EncodingDecl? SDDecl? S? '?>'
    let mut XMLDecl_start = ParsingRule::new("'<?xml'".to_owned(), RuleType::CharSequence);
    XMLDecl_start.expected_chars = "<?xml".chars().collect();
    rule_nameRegistry.insert(XMLDecl_start.rule_name.clone(), XMLDecl_start);

    let mut XMLDecl_end = ParsingRule::new("'?>'".to_owned(), RuleType::CharSequence);
    XMLDecl_end.expected_chars = "?>".chars().collect();
    rule_nameRegistry.insert(XMLDecl_end.rule_name.clone(), XMLDecl_end);

    let mut XMLDecl = ParsingRule::new("XMLDecl".to_owned(), RuleType::Sequence);
    XMLDecl.children_names.push("'<?xml'".to_owned());
    XMLDecl.children_names.push("VersionInfo".to_owned());
    XMLDecl.children_names.push("EncodingDecl?".to_owned());
    // TODO MISSING SDDecl?
    XMLDecl.children_names.push("S?".to_owned());
    XMLDecl.children_names.push("'?>'".to_owned());
    rule_nameRegistry.insert(XMLDecl.rule_name.clone(), XMLDecl);

    let mut XMLDecl_optional = ParsingRule::new("XMLDecl?".to_owned(), RuleType::Optional);
    XMLDecl_optional.children_names.push("XMLDecl".to_owned());
    rule_nameRegistry.insert(XMLDecl_optional.rule_name.clone(), XMLDecl_optional);



    // [24] VersionInfo ::= S 'version' Eq ("'" VersionNum "'" | '"' VersionNum '"')
    let mut VersionInfo_version = ParsingRule::new("'version'".to_owned(), RuleType::CharSequence);
    VersionInfo_version.expected_chars = "version".chars().collect();
    rule_nameRegistry.insert(VersionInfo_version.rule_name.clone(), VersionInfo_version);


    let mut VersionInfo_VNum1 = ParsingRule::new("\"'\" VersionNum \"'\"".to_owned(),
                                                 RuleType::Sequence);
    VersionInfo_VNum1.children_names.push("'".to_owned());
    VersionInfo_VNum1.children_names.push("VersionNum".to_owned());
    VersionInfo_VNum1.children_names.push("'".to_owned());
    rule_nameRegistry.insert(VersionInfo_VNum1.rule_name.clone(), VersionInfo_VNum1);

    let mut VersionInfo_VNum2 = ParsingRule::new("'\"' VersionNum '\"'".to_owned(),
                                                 RuleType::Sequence);
    VersionInfo_VNum2.children_names.push("\"".to_owned());
    VersionInfo_VNum2.children_names.push("VersionNum".to_owned());
    VersionInfo_VNum2.children_names.push("\"".to_owned());
    rule_nameRegistry.insert(VersionInfo_VNum2.rule_name.clone(), VersionInfo_VNum2);

    let mut VersionInfo_VNum = ParsingRule::new("VersionInfo_VersionNum".to_owned(), RuleType::Or);
    VersionInfo_VNum.children_names.push("\"'\" VersionNum \"'\"".to_owned());
    VersionInfo_VNum.children_names.push("'\"' VersionNum '\"'".to_owned());
    rule_nameRegistry.insert(VersionInfo_VNum.rule_name.clone(), VersionInfo_VNum);

    let mut VersionInfo = ParsingRule::new("VersionInfo".to_owned(), RuleType::Sequence);
    VersionInfo.children_names.push("S".to_owned());
    VersionInfo.children_names.push("'version'".to_owned());
    VersionInfo.children_names.push("Eq".to_owned());
    VersionInfo.children_names.push("VersionInfo_VersionNum".to_owned());
    rule_nameRegistry.insert(VersionInfo.rule_name.clone(), VersionInfo);


    // [25] Eq ::= S? '=' S?
    let mut equalsCharRule = ParsingRule::new("'='".to_owned(), RuleType::Chars);
    equalsCharRule.expected_chars.push('=');
    rule_nameRegistry.insert(equalsCharRule.rule_name.clone(), equalsCharRule);

    let mut _Eq = ParsingRule::new("Eq".to_owned(), RuleType::Sequence);
    _Eq.children_names.push("S?".to_owned());
    _Eq.children_names.push("'='".to_owned());
    _Eq.children_names.push("S?".to_owned());
    rule_nameRegistry.insert(_Eq.rule_name.clone(), _Eq);

    // [26] VersionNum ::= '1.' [0-9]+
    let mut VersionNum_1 = ParsingRule::new("'1.'".to_owned(), RuleType::CharSequence);
    VersionNum_1.expected_chars = "1.".to_owned().chars().collect();
    rule_nameRegistry.insert(VersionNum_1.rule_name.clone(), VersionNum_1);

    let mut VersionNum_09 = ParsingRule::new("[0-9]".to_owned(), RuleType::Chars);
    VersionNum_09.expected_char_ranges.push(('0', '9'));
    rule_nameRegistry.insert(VersionNum_09.rule_name.clone(), VersionNum_09);

    let mut VersionNum_09_zom = ParsingRule::new("[0-9]*".to_owned(), RuleType::ZeroOrMore);
    VersionNum_09_zom.children_names.push("[0-9]".to_owned());
    rule_nameRegistry.insert(VersionNum_09_zom.rule_name.clone(), VersionNum_09_zom);

    let mut VersionNum_09_oom = ParsingRule::new("[0-9]+".to_owned(), RuleType::Sequence);
    VersionNum_09_oom.children_names.push("[0-9]".to_owned());
    VersionNum_09_oom.children_names.push("[0-9]*".to_owned());
    rule_nameRegistry.insert(VersionNum_09_oom.rule_name.clone(), VersionNum_09_oom);

    let mut VersionNum = ParsingRule::new("VersionNum".to_owned(), RuleType::Sequence);
    VersionNum.children_names.push("'1.'".to_owned());
    VersionNum.children_names.push("[0-9]+".to_owned());
    rule_nameRegistry.insert(VersionNum.rule_name.clone(), VersionNum);




    // [39] element ::= EmptyElemTag | STag content ETag
    // TODO spec incomplete
    let mut element_notempty = ParsingRule::new("STag content ETag".to_owned(), RuleType::Sequence);
    element_notempty.children_names.push("STag".to_owned());
    element_notempty.children_names.push("content?".to_owned());
    element_notempty.children_names.push("ETag".to_owned());

    rule_nameRegistry.insert(element_notempty.rule_name.clone(), element_notempty);

    let mut element = ParsingRule::new("element".to_owned(), RuleType::Or);
    element.children_names.push("EmptyElemTag".to_owned());
    element.children_names.push("STag content ETag".to_owned());
    rule_nameRegistry.insert(element.rule_name.clone(), element);



    // [40] STag ::= '<' Name (S Attribute)* S? '>'
    let mut STag = ParsingRule::new("STag".to_owned(), RuleType::Sequence);
    STag.is_chunkable = false;
    STag.children_names.push("<".to_owned());  //'<' Name (S Attribute)* S? '>'
    STag.children_names.push("Name".to_owned());
    STag.children_names.push("(S Attribute)*".to_owned());
    STag.children_names.push("S?".to_owned());
    STag.children_names.push(">".to_owned());
    rule_nameRegistry.insert(STag.rule_name.clone(), STag);


    // [41] Attribute ::= Name Eq AttValue
    let mut Attribute = ParsingRule::new("Attribute".to_owned(), RuleType::Sequence);
    Attribute.children_names.push("Name".to_owned());
    Attribute.children_names.push("Eq".to_owned());
    Attribute.children_names.push("AttValue".to_owned());
    rule_nameRegistry.insert(Attribute.rule_name.clone(), Attribute);

    let mut Attribute_optional = ParsingRule::new("Attribute?".to_owned(), RuleType::Optional);
    Attribute_optional.children_names.push("Attribute".to_owned());
    rule_nameRegistry.insert(Attribute_optional.rule_name.clone(), Attribute_optional);

    // (S Attribute)
    let mut Attribute_after_s = ParsingRule::new("(S Attribute)".to_owned(), RuleType::Sequence);
    Attribute_after_s.children_names.push("S".to_owned());
    Attribute_after_s.children_names.push("Attribute".to_owned());
    rule_nameRegistry.insert(Attribute_after_s.rule_name.clone(), Attribute_after_s);

    // (S Attribute)*
    let mut Attribute_after_s_zom = ParsingRule::new("(S Attribute)*".to_owned(),
                                                     RuleType::ZeroOrMore);
    Attribute_after_s_zom.children_names.push("(S Attribute)".to_owned());
    rule_nameRegistry.insert(Attribute_after_s_zom.rule_name.clone(),
                             Attribute_after_s_zom);


    // [42] ETag ::= '</' Name S? '>'
    let mut ETag = ParsingRule::new("ETag".to_owned(), RuleType::Sequence);
    ETag.is_chunkable = false;
    ETag.children_names.push("'</'".to_owned());
    ETag.children_names.push("Name".to_owned());
    ETag.children_names.push("S?".to_owned());
    ETag.children_names.push(">".to_owned());

    rule_nameRegistry.insert(ETag.rule_name.clone(), ETag);




    // [43] content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
    // TODO unimplemented spec
    // need to seperate circular ref-seperated..
    // put element to last child for reducing backtrack needs
    let mut content_inside = ParsingRule::new("(element | Reference | CDSect | PI | Comment)"
                                                  .to_owned(),
                                              RuleType::Or);
    content_inside.children_names.push("Reference".to_owned());
    content_inside.children_names.push("CDSect".to_owned());
    content_inside.children_names.push("Comment".to_owned());
    content_inside.children_names.push("element".to_owned());
    // TODO add child here
    rule_nameRegistry.insert(content_inside.rule_name.clone(), content_inside);


    let mut content_inside_and_CharData = ParsingRule::new("(content_inside CharData?)".to_owned(),
                                                           RuleType::Sequence);
    // ruleRegistry.insert(elementAndCharData.rule_name.clone(), rule_vec.len());
    content_inside_and_CharData.children_names
        .push("(element | Reference | CDSect | PI | Comment)".to_owned());
    content_inside_and_CharData.children_names.push("CharData?".to_owned());
    rule_nameRegistry.insert(content_inside_and_CharData.rule_name.clone(),
                             content_inside_and_CharData);

    let mut content_inside_and_CharData_zom = ParsingRule::new("(content_inside CharData?)*"
                                                                   .to_owned(),
                                                               RuleType::ZeroOrMore);
    content_inside_and_CharData_zom.children_names.push("(content_inside CharData?)".to_owned());
    rule_nameRegistry.insert(content_inside_and_CharData_zom.rule_name.clone(),
                             content_inside_and_CharData_zom);

    let mut content = ParsingRule::new("content".to_owned(), RuleType::Sequence);
    content.children_names.push("CharData?".to_owned());
    content.children_names.push("(content_inside CharData?)*".to_owned());
    rule_nameRegistry.insert(content.rule_name.clone(), content);

    let mut content2 = ParsingRule::new("content?".to_owned(), RuleType::Optional);
    content2.children_names.push("content".to_owned());
    rule_nameRegistry.insert(content2.rule_name.clone(), content2);

    // [44] EmptyElemTag ::= '<' Name (S Attribute)* S? '/>'

    let mut EmptyElemTag_end = ParsingRule::new("'/>'".to_owned(), RuleType::CharSequence);
    EmptyElemTag_end.expected_chars.push('/');
    EmptyElemTag_end.expected_chars.push('>');
    rule_nameRegistry.insert(EmptyElemTag_end.rule_name.clone(), EmptyElemTag_end);

    let mut EmptyElemTag = ParsingRule::new("EmptyElemTag".to_owned(), RuleType::Sequence);
    EmptyElemTag.is_chunkable = false;
    EmptyElemTag.children_names.push("<".to_owned());
    EmptyElemTag.children_names.push("Name".to_owned());
    EmptyElemTag.children_names.push("(S Attribute)*".to_owned());
    EmptyElemTag.children_names.push("S?".to_owned());
    EmptyElemTag.children_names.push("'/>'".to_owned());

    rule_nameRegistry.insert(EmptyElemTag.rule_name.clone(), EmptyElemTag);


    // [66] CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'
    let mut CharRef_ampdial = ParsingRule::new("'&#'".to_owned(), RuleType::CharSequence);
    CharRef_ampdial.expected_chars.push('&');
    CharRef_ampdial.expected_chars.push('#');
    rule_nameRegistry.insert(CharRef_ampdial.rule_name.clone(), CharRef_ampdial);

    let mut CharRef_ampx = ParsingRule::new("'&#x'".to_owned(), RuleType::CharSequence);
    CharRef_ampx.expected_chars.push('&');
    CharRef_ampx.expected_chars.push('#');
    CharRef_ampx.expected_chars.push('x');
    rule_nameRegistry.insert(CharRef_ampx.rule_name.clone(), CharRef_ampx);

    let mut CharRef_09 = ParsingRule::new("[0-9]".to_owned(), RuleType::Chars);
    CharRef_09.expected_char_ranges.push(('0', '9'));
    rule_nameRegistry.insert(CharRef_09.rule_name.clone(), CharRef_09);

    let mut CharRef_09_zom = ParsingRule::new("[0-9]*".to_owned(), RuleType::ZeroOrMore);
    CharRef_09_zom.children_names.push("[0-9]".to_owned());
    rule_nameRegistry.insert(CharRef_09_zom.rule_name.clone(), CharRef_09_zom);

    let mut CharRef_09_oom = ParsingRule::new("[0-9]+".to_owned(), RuleType::Sequence);
    CharRef_09_oom.children_names.push("[0-9]".to_owned());
    CharRef_09_oom.children_names.push("[0-9]*".to_owned());
    rule_nameRegistry.insert(CharRef_09_oom.rule_name.clone(), CharRef_09_oom);

    let mut CharRef_alt_1 = ParsingRule::new("CharRef_alt_1".to_owned(), RuleType::Sequence);
    CharRef_alt_1.children_names.push("'&#'".to_owned());
    CharRef_alt_1.children_names.push("[0-9]+".to_owned());
    CharRef_alt_1.children_names.push("';'".to_owned());
    rule_nameRegistry.insert(CharRef_alt_1.rule_name.clone(), CharRef_alt_1);

    let mut CharRef_09af = ParsingRule::new("[0-9a-fA-F]".to_owned(), RuleType::Chars);
    CharRef_09af.expected_char_ranges.push(('0', '9'));
    CharRef_09af.expected_char_ranges.push(('a', 'f'));
    CharRef_09af.expected_char_ranges.push(('A', 'F'));
    rule_nameRegistry.insert(CharRef_09af.rule_name.clone(), CharRef_09af);

    let mut CharRef_09af_zom = ParsingRule::new("[0-9a-fA-F]*".to_owned(), RuleType::ZeroOrMore);
    CharRef_09af_zom.children_names.push("[0-9a-fA-F]".to_owned());
    rule_nameRegistry.insert(CharRef_09af_zom.rule_name.clone(), CharRef_09af_zom);

    let mut CharRef_09af_oom = ParsingRule::new("[0-9a-fA-F]+".to_owned(), RuleType::Sequence);
    CharRef_09af_oom.children_names.push("[0-9a-fA-F]".to_owned());
    CharRef_09af_oom.children_names.push("[0-9a-fA-F]*".to_owned());
    rule_nameRegistry.insert(CharRef_09af_oom.rule_name.clone(), CharRef_09af_oom);

    // '&#x' [0-9a-fA-F]+ ';'
    let mut CharRef_alt_2 = ParsingRule::new("CharRef_alt_2".to_owned(), RuleType::Sequence);
    CharRef_alt_2.children_names.push("'&#x'".to_owned());
    CharRef_alt_2.children_names.push("[0-9a-fA-F]+".to_owned());
    CharRef_alt_2.children_names.push("';'".to_owned());
    rule_nameRegistry.insert(CharRef_alt_2.rule_name.clone(), CharRef_alt_2);

    let mut CharRef = ParsingRule::new("CharRef".to_owned(), RuleType::Or);
    CharRef.children_names.push("CharRef_alt_1".to_owned());
    CharRef.children_names.push("CharRef_alt_2".to_owned());
    rule_nameRegistry.insert(CharRef.rule_name.clone(), CharRef);


    // [67] Reference ::= EntityRef | CharRef
    let mut Reference = ParsingRule::new("Reference".to_owned(), RuleType::Or);
    Reference.children_names.push("EntityRef".to_owned());
    Reference.children_names.push("CharRef".to_owned());
    rule_nameRegistry.insert(Reference.rule_name.clone(), Reference);

    // [67.5] Reference in AttVal ::= EntityRef | CharRef
    let mut ReferenceInAttrVal = ParsingRule::new("ReferenceInAttrVal".to_owned(), RuleType::Or);
    ReferenceInAttrVal.children_names.push("EntityRef".to_owned());
    ReferenceInAttrVal.children_names.push("CharRef".to_owned());
    rule_nameRegistry.insert(ReferenceInAttrVal.rule_name.clone(), ReferenceInAttrVal);

    // [68] EntityRef ::= '&' Name ';'
    let mut EntityRef = ParsingRule::new("EntityRef".to_owned(), RuleType::Sequence);
    EntityRef.children_names.push("'&'".to_owned());
    EntityRef.children_names.push("Name".to_owned());
    EntityRef.children_names.push("';'".to_owned());
    rule_nameRegistry.insert(EntityRef.rule_name.clone(), EntityRef);

    // [69] PEReference ::= '%' Name ';'
    let mut PEReference = ParsingRule::new("PEReference".to_owned(), RuleType::Sequence);
    PEReference.children_names.push("'%'".to_owned());
    PEReference.children_names.push("Name".to_owned());
    PEReference.children_names.push("';'".to_owned());
    rule_nameRegistry.insert(PEReference.rule_name.clone(), PEReference);

    // [80] EncodingDecl EncodingDecl	   ::=   	S 'encoding' Eq ('"' EncName '"' | "'" EncName "'" )
    let mut EncodingDecl_encoding = ParsingRule::new("'encoding'".to_owned(),
                                                     RuleType::CharSequence);
    EncodingDecl_encoding.expected_chars = "encoding".to_owned().chars().collect();
    rule_nameRegistry.insert(EncodingDecl_encoding.rule_name.clone(),
                             EncodingDecl_encoding);

    let mut EncodingDecl_encname_1 = ParsingRule::new("EncodingDecl_encname_1".to_owned(),
                                                      RuleType::Sequence);
    EncodingDecl_encname_1.children_names.push("\"".to_owned());
    EncodingDecl_encname_1.children_names.push("EncName".to_owned());
    EncodingDecl_encname_1.children_names.push("\"".to_owned());
    rule_nameRegistry.insert(EncodingDecl_encname_1.rule_name.clone(),
                             EncodingDecl_encname_1);

    let mut EncodingDecl_encname_2 = ParsingRule::new("EncodingDecl_encname_2".to_owned(),
                                                      RuleType::Sequence);
    EncodingDecl_encname_2.children_names.push("'".to_owned());
    EncodingDecl_encname_2.children_names.push("EncName".to_owned());
    EncodingDecl_encname_2.children_names.push("'".to_owned());
    rule_nameRegistry.insert(EncodingDecl_encname_2.rule_name.clone(),
                             EncodingDecl_encname_2);

    let mut EncodingDecl_encname = ParsingRule::new("EncodingDecl_encname".to_owned(),
                                                    RuleType::Or);
    EncodingDecl_encname.children_names.push("EncodingDecl_encname_1".to_owned());
    EncodingDecl_encname.children_names.push("EncodingDecl_encname_2".to_owned());
    rule_nameRegistry.insert(EncodingDecl_encname.rule_name.clone(), EncodingDecl_encname);

    let mut EncodingDecl = ParsingRule::new("EncodingDecl".to_owned(), RuleType::Sequence);
    EncodingDecl.children_names.push("S".to_owned());
    EncodingDecl.children_names.push("'encoding'".to_owned());
    EncodingDecl.children_names.push("Eq".to_owned());
    EncodingDecl.children_names.push("EncodingDecl_encname".to_owned());
    rule_nameRegistry.insert(EncodingDecl.rule_name.clone(), EncodingDecl);

    let mut EncodingDecl_optional = ParsingRule::new("EncodingDecl?".to_owned(),
                                                     RuleType::Optional);
    EncodingDecl_optional.children_names.push("EncodingDecl".to_owned());
    rule_nameRegistry.insert(EncodingDecl_optional.rule_name.clone(),
                             EncodingDecl_optional);





    // [81] EncName
    let mut EncName_az = ParsingRule::new("[A-Za-z]".to_owned(), RuleType::Chars);
    EncName_az.expected_char_ranges.push(('a', 'z'));
    EncName_az.expected_char_ranges.push(('A', 'Z'));
    rule_nameRegistry.insert(EncName_az.rule_name.clone(), EncName_az);

    let mut EncName_az09 = ParsingRule::new("[A-Za-z0-9._]".to_owned(), RuleType::Chars);
    EncName_az09.expected_char_ranges.push(('a', 'z'));
    EncName_az09.expected_char_ranges.push(('A', 'Z'));
    EncName_az09.expected_char_ranges.push(('0', '9'));
    EncName_az09.expected_chars.push('.');
    EncName_az09.expected_chars.push('_');
    rule_nameRegistry.insert(EncName_az09.rule_name.clone(), EncName_az09);

    let mut EncName_hyphen = ParsingRule::new("-".to_owned(), RuleType::Chars);
    EncName_hyphen.expected_chars.push('-');
    rule_nameRegistry.insert(EncName_hyphen.rule_name.clone(), EncName_hyphen);

    let mut EncName_part2_single = ParsingRule::new("([A-Za-z0-9._] | '-')".to_owned(),
                                                    RuleType::Or);
    EncName_part2_single.children_names.push("-".to_owned());
    EncName_part2_single.children_names.push("[A-Za-z0-9._]".to_owned());
    rule_nameRegistry.insert(EncName_part2_single.rule_name.clone(), EncName_part2_single);

    let mut EncName_part2 = ParsingRule::new("([A-Za-z0-9._] | '-')*".to_owned(),
                                             RuleType::ZeroOrMore);
    EncName_part2.children_names.push("([A-Za-z0-9._] | '-')".to_owned());
    rule_nameRegistry.insert(EncName_part2.rule_name.clone(), EncName_part2);

    let mut EncName = ParsingRule::new("EncName".to_owned(), RuleType::Sequence);
    EncName.children_names.push("[A-Za-z]".to_owned());
    EncName.children_names.push("([A-Za-z0-9._] | '-')*".to_owned());
    rule_nameRegistry.insert(EncName.rule_name.clone(), EncName);


    for (rule_name, rule) in rule_nameRegistry.into_iter() {

        ruleRegistry.insert(rule_name, rule_vec.len());
        rule_vec.push(rule);
    }

    for rule in &mut rule_vec {
        for child_rule_name in &rule.children_names {
            match ruleRegistry.get(child_rule_name) {
                Some(child_rule_id) => rule.children.push(*child_rule_id),
                None => println!("{} rule not found", child_rule_name),
            }


        }
    }

    return Parser {
        rule_registry: ruleRegistry,
        rule_vec: rule_vec,
    };
}

pub enum ParsingResult {
    Pass(usize, usize),
    Fail,
    EOF,
}

pub trait ParsingPassLogStream {
    fn try(&mut self, rule_name: String, starting_pos: usize) -> ();
    fn pass(&mut self,
            rule_name: String,
            chars: &Vec<char>,
            starting_pos: usize,
            ending_pos: usize)
            -> ();
}

// state_vec (child_no,child starting pos,no backtrack required)
pub fn parse_with_rule<T: ParsingPassLogStream>(rule_vec: &Vec<ParsingRule>,
                                                rule: &ParsingRule,
                                                char_vector: &Vec<char>,
                                                starting_pos: usize,
                                                // use where char vec is used
                                                offset: usize,
                                                // eof
                                                resume_state_vec: &mut Vec<(usize, usize, bool)>,
                                                state_vec: &mut Vec<(usize, usize, bool)>,
                                                mut logger: T)
                                                -> (T, ParsingResult) {

    logger.try(rule.rule_name.clone(), starting_pos);

    match rule.rule_type {
        RuleType::Chars => {
            if starting_pos - offset >= char_vector.len() {
                return (logger, ParsingResult::EOF);
            }
            let c = char_vector[starting_pos - offset];


            for range in &rule.expected_char_ranges {


                if range.0 <= c && c <= range.1 {
                    logger.pass(rule.rule_name.clone(),
                                char_vector,
                                starting_pos,
                                starting_pos + 1);
                    return (logger, ParsingResult::Pass(starting_pos, starting_pos + 1));
                }
            }
            for check_char in &rule.expected_chars {
                if *check_char == c {
                    logger.pass(rule.rule_name.clone(),
                                char_vector,
                                starting_pos,
                                starting_pos + 1);
                    return (logger, ParsingResult::Pass(starting_pos, starting_pos + 1));
                }
            }

            (logger, ParsingResult::Fail)
        }

        RuleType::CharsNot => {
            if starting_pos - offset >= char_vector.len() {
                return (logger, ParsingResult::EOF);
            }
            let c = char_vector[starting_pos - offset];

            for range in &rule.expected_char_ranges {
                if range.0 <= c && c <= range.1 {
                    return (logger, ParsingResult::Fail);
                }
            }
            for check_char in &rule.expected_chars {
                if *check_char == c {
                    return (logger, ParsingResult::Fail);
                }
            }
            logger.pass(rule.rule_name.clone(),
                        char_vector,
                        starting_pos,
                        starting_pos + 1);
            return (logger, ParsingResult::Pass(starting_pos, starting_pos + 1));
        }

        RuleType::CharSequence => {
            let mut new_starting_pos = starting_pos;
            for check_char in &rule.expected_chars {
                if new_starting_pos - offset >= char_vector.len() {
                    return (logger, ParsingResult::EOF);
                }
                let c = char_vector[new_starting_pos - offset];

                if *check_char == c {
                    new_starting_pos += 1;
                } else {
                    return (logger, ParsingResult::Fail);
                }
            }
            logger.pass(rule.rule_name.clone(),
                        char_vector,
                        starting_pos,
                        new_starting_pos);
            return (logger, ParsingResult::Pass(starting_pos, new_starting_pos));
        }
        RuleType::ZeroOrMore => {
            let new_rule = &rule_vec[rule.children[0]];
            let mut fail = false;
            let mut new_starting_pos = starting_pos;

            while !fail {
                let result = parse_with_rule(rule_vec,
                                             new_rule,
                                             &char_vector,
                                             new_starting_pos,
                                             offset,
                                             resume_state_vec,
                                             state_vec,
                                             logger);
                logger = result.0;
                match result.1 {
                    ParsingResult::Fail => fail = true,
                    ParsingResult::Pass(_, e_pos) => new_starting_pos = e_pos,
                    ParsingResult::EOF => {
                        return (logger, ParsingResult::EOF);
                    }
                }
            }
            logger.pass(rule.rule_name.clone(),
                        char_vector,
                        starting_pos,
                        new_starting_pos);
            return (logger, ParsingResult::Pass(starting_pos, new_starting_pos));
        }

        RuleType::Sequence => {
            let mut child_no: usize;
            let mut new_starting_pos = starting_pos;

            match resume_state_vec.pop() {
                Some((no, resume_starting_pos, _)) => {
                    child_no = no;
                    new_starting_pos = resume_starting_pos;
                }
                None => child_no = 0,
            }

            for (no, rule_id) in rule.children.iter().skip(child_no).enumerate() {
                let mut child_no2 = no + child_no;
                // sequence always goes forward or fails, so no need for backtracking
                // if rule.rule_name =="STag" || rule.rule_name == "ETag"{
                //     state_vec.push((child_no2,new_starting_pos,false));
                // }else{
                //     state_vec.push((child_no2,new_starting_pos,true));
                // }
                if rule.is_chunkable {
                    state_vec.push((child_no2, new_starting_pos, true));
                } else {
                    state_vec.push((child_no2, new_starting_pos, false));
                }


                let new_rule = &rule_vec[*rule_id];

                let result = parse_with_rule(rule_vec,
                                             new_rule,
                                             &char_vector,
                                             new_starting_pos,
                                             offset,
                                             resume_state_vec,
                                             state_vec,
                                             logger);
                logger = result.0;
                match result.1 {
                    ParsingResult::Fail => {
                        state_vec.pop();
                        return (logger, ParsingResult::Fail);
                    }
                    ParsingResult::Pass(_, e_pos) => new_starting_pos = e_pos,
                    ParsingResult::EOF => {
                        // dont call state_vec.pop();
                        logger.pass(rule.rule_name.clone(), char_vector, starting_pos, 0);
                        return (logger, ParsingResult::EOF);
                    }
                }
                state_vec.pop();
            }
            logger.pass(rule.rule_name.clone(),
                        char_vector,
                        starting_pos,
                        new_starting_pos);
            return (logger, ParsingResult::Pass(starting_pos, new_starting_pos));
        }

        RuleType::Or => {
            let mut child_no: usize;
            let mut rule_starting_pos: usize = starting_pos;
            match resume_state_vec.pop() {
                Some((no, state_starting_pos, _)) => {
                    child_no = no;
                    rule_starting_pos = state_starting_pos;
                }
                None => child_no = 0,
            }

            for (no, rule_id) in rule.children.iter().skip(child_no).enumerate() {
                let mut child_no2 = no + child_no;
                let mut no_backtrack_required = false;
                if child_no2 == rule.children.len() - 1 {
                    no_backtrack_required = true;
                }
                state_vec.push((child_no2, rule_starting_pos, no_backtrack_required));
                let new_rule = &rule_vec[*rule_id];
                let result = parse_with_rule(rule_vec,
                                             new_rule,
                                             &char_vector,
                                             rule_starting_pos,
                                             offset,
                                             resume_state_vec,
                                             state_vec,
                                             logger);
                logger = result.0;
                match result.1 {

                    ParsingResult::Pass(s_pos, e_pos) => {
                        logger.pass(rule.rule_name.clone(), char_vector, s_pos, e_pos);
                        state_vec.pop();
                        return (logger, result.1);
                    }
                    ParsingResult::Fail => (),
                    ParsingResult::EOF => {
                        // dont call state_vec.pop();
                        // state_vec.pop();
                        return (logger, ParsingResult::EOF);
                    }
                }
                state_vec.pop();
            }
            return (logger, ParsingResult::Fail);
        }

        RuleType::WithException => {
            // first test should pass
            // second test should fail
            // i cant think of any case that needs pass or fail location for second test
            let first_rule = &rule_vec[rule.children[0]];
            let second_rule = &rule_vec[rule.children[1]];
            let result = parse_with_rule(rule_vec,
                                         first_rule,
                                         &char_vector,
                                         starting_pos,
                                         offset,
                                         resume_state_vec,
                                         state_vec,
                                         logger);
            match result.1 {
                ParsingResult::Pass(s_pos, e_pos) => {
                    let result2 = parse_with_rule(rule_vec,
                                                  second_rule,
                                                  &char_vector,
                                                  starting_pos,
                                                  offset,
                                                  resume_state_vec,
                                                  state_vec,
                                                  result.0);
                    logger = result2.0;
                    match result2.1 {
                        ParsingResult::Fail => {
                            logger.pass(rule.rule_name.clone(), char_vector, s_pos, e_pos);
                            return (logger, result.1);
                        }
                        ParsingResult::Pass(_, _) => return (logger, ParsingResult::Fail),
                        ParsingResult::EOF => {

                            return (logger, ParsingResult::EOF);
                        }
                    }

                }
                ParsingResult::Fail => return (result.0, ParsingResult::Fail),
                ParsingResult::EOF => {
                    // dont call state_vec.pop();
                    return (result.0, ParsingResult::EOF);
                }
            }

        }
        RuleType::Optional => {

            let result = parse_with_rule(rule_vec,
                                         &rule_vec[rule.children[0]],
                                         &char_vector,
                                         starting_pos,
                                         offset,
                                         resume_state_vec,
                                         state_vec,
                                         logger);
            logger = result.0;
            match result.1 {

                ParsingResult::Pass(s_pos, e_pos) => {
                    logger.pass(rule.rule_name.clone(), char_vector, s_pos, e_pos);
                    return (logger, result.1);
                }
                ParsingResult::Fail => {
                    logger.pass(rule.rule_name.clone(),
                                char_vector,
                                starting_pos,
                                starting_pos);
                    return (logger, ParsingResult::Pass(starting_pos, starting_pos));
                }
                ParsingResult::EOF => {
                    // burada state_vec.pop(); cagirmiyoruz
                    return (logger, ParsingResult::EOF);
                }
            }

        }
        // unreachable
          _ => {
         println!("UNIMPLEMENTED PARSER FOR TYPE!" );
         return (logger, ParsingResult::Fail);
         
         }
    }
}