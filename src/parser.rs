
use std::collections::HashMap;

#[derive(Debug)]
enum RuleType {
    Sequence,
    Or,
    ZeroOrMore,
    Chars, // directly chars
    CharSequence, // easy char sequence for <!-- cdata etc. TODO
    CharsNot,
    WithException,
    Optional,
}

pub struct Parser<'a> {
    pub rule_vec: Vec<ParsingRule<'a>>,
    pub rule_registry: HashMap<&'a str, usize>,
}

pub struct ParsingRule<'a> {
    rule_name: &'a str,
    rule_type: RuleType,
    children: Vec<usize>,
    children_names: Vec<&'a str>,
    expected_char_ranges: Vec<(char, char)>,
    expected_chars: Vec<char>,
    is_chunkable: bool,
}

impl<'a> ParsingRule<'a> {
    fn new(rule_name: &str, rule_type: RuleType) -> ParsingRule {
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
pub fn prepare_rules<'a>() -> Parser<'a> {
    let mut rule_vec = Vec::new();

    let mut ruleRegistry: HashMap<&str, usize> = HashMap::new();
    let mut rule_nameRegistry: HashMap<&str, ParsingRule> = HashMap::new();


    // general rule implementations
    let mut AposChar = ParsingRule::new("'", RuleType::Chars);
    AposChar.expected_chars.push('\'');
    rule_nameRegistry.insert(AposChar.rule_name, AposChar);

    let mut QuoteChar = ParsingRule::new("\"", RuleType::Chars);
    QuoteChar.expected_chars.push('"');
    rule_nameRegistry.insert(QuoteChar.rule_name, QuoteChar);

    let mut charLT = ParsingRule::new("<", RuleType::Chars);
    charLT.expected_chars.push('<');
    rule_nameRegistry.insert(charLT.rule_name, charLT);


    let mut charBT = ParsingRule::new(">", RuleType::Chars);
    charBT.expected_chars.push('>');
    rule_nameRegistry.insert(charBT.rule_name, charBT);


    let mut endTagToken = ParsingRule::new("'</'", RuleType::CharSequence);
    endTagToken.expected_chars.push('<');
    endTagToken.expected_chars.push('/');
    rule_nameRegistry.insert(endTagToken.rule_name, endTagToken);

    let mut percent_char = ParsingRule::new("'%'", RuleType::Chars);
    percent_char.expected_chars.push('%');
    rule_nameRegistry.insert(percent_char.rule_name, percent_char);

    let mut ampersand_char = ParsingRule::new("'&'", RuleType::Chars);
    ampersand_char.expected_chars.push('&');
    rule_nameRegistry.insert(ampersand_char.rule_name, ampersand_char);

    let mut semicolon_char = ParsingRule::new("';'", RuleType::Chars);
    semicolon_char.expected_chars.push(';');
    rule_nameRegistry.insert(semicolon_char.rule_name, semicolon_char);


    let mut notLTorAmp = ParsingRule::new("[^<&]", RuleType::CharsNot);
    notLTorAmp.expected_chars.push('<');
    notLTorAmp.expected_chars.push('&');
    rule_nameRegistry.insert(notLTorAmp.rule_name, notLTorAmp);


    let mut notLTorAmpZeroOrMore = ParsingRule::new("[^<&]*", RuleType::ZeroOrMore);
    notLTorAmpZeroOrMore.children_names.push("[^<&]");

    rule_nameRegistry.insert(notLTorAmpZeroOrMore.rule_name, notLTorAmpZeroOrMore);

    let mut not_lt_amp_quote = ParsingRule::new("[^<&\"]", RuleType::CharsNot);
    not_lt_amp_quote.expected_chars.push('<');
    not_lt_amp_quote.expected_chars.push('&');
    not_lt_amp_quote.expected_chars.push('\"');
    rule_nameRegistry.insert(not_lt_amp_quote.rule_name, not_lt_amp_quote);

    let mut not_lt_amp_quote_zom = ParsingRule::new("[^<&\"]*", RuleType::ZeroOrMore);
    not_lt_amp_quote_zom.children_names.push("[^<&\"]");
    rule_nameRegistry.insert(not_lt_amp_quote_zom.rule_name, not_lt_amp_quote_zom);

    //  ([^<&]* ']]>' [^<&]*)

    let mut CdataEnd = ParsingRule::new("']]>'", RuleType::CharSequence);
    CdataEnd.expected_chars.push(']');
    CdataEnd.expected_chars.push(']');
    CdataEnd.expected_chars.push('>');

    rule_nameRegistry.insert(CdataEnd.rule_name, CdataEnd);



    // Parser Rules organized by W3C Spec

    // [1] document ::= prolog element Misc*
    // TODO prolog Misc
    let mut document = ParsingRule::new("document", RuleType::Sequence);
    document.children_names.push("XMLDecl?");
    // TODO remove S? as it is contained in prolog
    document.children_names.push("S?");
    document.children_names.push("element");
    rule_nameRegistry.insert(document.rule_name, document);

    // [3] S ::= (#x20 | #x9 | #xD | #xA)+
    let mut S_single = ParsingRule::new("S_single", RuleType::Chars);
    S_single.expected_chars = vec!['\x20', '\x09', '\x0D', '\x0A'];
    rule_nameRegistry.insert(S_single.rule_name, S_single);

    let mut S_ZeroOrMore = ParsingRule::new("S_single*", RuleType::ZeroOrMore);
    S_ZeroOrMore.children_names.push("S_single");
    rule_nameRegistry.insert(S_ZeroOrMore.rule_name, S_ZeroOrMore);

    let mut S = ParsingRule::new("S", RuleType::Sequence);
    S.children_names.push("S_single");
    S.children_names.push("S_single*");
    rule_nameRegistry.insert(S.rule_name, S);

    let mut S_optional = ParsingRule::new("S?", RuleType::Optional);
    S_optional.children_names.push("S");
    rule_nameRegistry.insert(S_optional.rule_name, S_optional);

    // [4] NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |
    // [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] |
    // [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]
    let mut NameStartChar = ParsingRule::new("NameStartChar", RuleType::Chars);

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

    rule_nameRegistry.insert(NameStartChar.rule_name, NameStartChar);


    // [4a] NameChar ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
    let mut NameCharExtraRule = ParsingRule::new("NameCharExtra", RuleType::Chars);
    let name_char_extra_rule_ranges_arr =
        [('0', '9'), ('a', 'z'), ('\u{0300}', '\u{036F}'), ('\u{203F}', '\u{2040}')];
    NameCharExtraRule.expected_char_ranges.extend(name_char_extra_rule_ranges_arr.iter().cloned());
    NameCharExtraRule.expected_chars.push('-');
    NameCharExtraRule.expected_chars.push('.');
    NameCharExtraRule.expected_chars.push('\u{B7}');
    rule_nameRegistry.insert(NameCharExtraRule.rule_name, NameCharExtraRule);

    let mut NameChar = ParsingRule::new("NameChar", RuleType::Or);
    NameChar.children_names.push("NameStartChar");
    NameChar.children_names.push("NameCharExtra");
    rule_nameRegistry.insert(NameChar.rule_name, NameChar);

    let mut NameCharZeroOrMore = ParsingRule::new("NameChar*", RuleType::ZeroOrMore);
    NameCharZeroOrMore.children_names.push("NameChar");
    rule_nameRegistry.insert(NameCharZeroOrMore.rule_name, NameCharZeroOrMore);

    let mut Name = ParsingRule::new("Name", RuleType::Sequence);
    Name.children_names.push("NameStartChar");
    Name.children_names.push("NameChar*");
    rule_nameRegistry.insert(Name.rule_name, Name);


    // [10] AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"
    let mut AttValue_char_or_ref = ParsingRule::new("([^<&\"] | Reference)", RuleType::Or);
    AttValue_char_or_ref.children_names.push("[^<&\"]");
    AttValue_char_or_ref.children_names.push("Reference");
    rule_nameRegistry.insert(AttValue_char_or_ref.rule_name, AttValue_char_or_ref);

    let mut AttValue_char_or_ref_zom = ParsingRule::new("([^<&\"] | Reference)*",
                                                        RuleType::ZeroOrMore);
    AttValue_char_or_ref_zom.children_names.push("([^<&\"] | Reference)");
    rule_nameRegistry.insert(AttValue_char_or_ref_zom.rule_name, AttValue_char_or_ref_zom);

    let mut AttValue_alt_1 = ParsingRule::new("AttValue_alt_1", RuleType::Sequence);
    AttValue_alt_1.children_names.push("\"");
    AttValue_alt_1.children_names.push("([^<&\"] | Reference)*");
    AttValue_alt_1.children_names.push("\"");
    rule_nameRegistry.insert(AttValue_alt_1.rule_name, AttValue_alt_1);

    let mut AttValue_alt_2 = ParsingRule::new("AttValue_alt_2", RuleType::Sequence);
    AttValue_alt_2.children_names.push("'");
    AttValue_alt_2.children_names.push("([^<&\"] | Reference)*");
    AttValue_alt_2.children_names.push("'");
    rule_nameRegistry.insert(AttValue_alt_2.rule_name, AttValue_alt_2);

    let mut AttValue = ParsingRule::new("AttValue", RuleType::Or);
    AttValue.children_names.push("AttValue_alt_1");
    AttValue.children_names.push("AttValue_alt_2");
    rule_nameRegistry.insert(AttValue.rule_name, AttValue);





    // [14] CharData ::= [^<&]* - ([^<&]* ']]>' [^<&]*)

    // let mut CharDataExceptionSeq = ParsingRule::new("([^<&]* ']]>' [^<&]*)", RuleType::Sequence);
    // CharDataExceptionSeq.children_names.push("[^<&]*");
    // CharDataExceptionSeq.children_names.push("']]>'");
    // CharDataExceptionSeq.children_names.push("[^<&]*");
    // rule_nameRegistry.insert(CharDataExceptionSeq.rule_name, CharDataExceptionSeq);

    let mut charDataSingleBeforeException = ParsingRule::new("([^<&] - ']]>')",
                                                             RuleType::WithException);
    charDataSingleBeforeException.children_names.push("[^<&]");
    charDataSingleBeforeException.children_names.push("']]>'");
    rule_nameRegistry.insert(charDataSingleBeforeException.rule_name,
                             charDataSingleBeforeException);

    let mut charDataBeforeException = ParsingRule::new("([^<&] - ']]>')*", RuleType::ZeroOrMore);
    charDataBeforeException.children_names.push("([^<&] - ']]>')");
    rule_nameRegistry.insert(charDataBeforeException.rule_name, charDataBeforeException);


    let mut CharDataException = ParsingRule::new("([^<&] - ']]>')* ']]>' [^<&]*",
                                                 RuleType::Sequence);
    CharDataException.children_names.push("([^<&] - ']]>')*");
    CharDataException.children_names.push("']]>'");
    CharDataException.children_names.push("[^<&]*");
    rule_nameRegistry.insert(CharDataException.rule_name, CharDataException);


    let mut CharData = ParsingRule::new("CharData", RuleType::WithException);
    CharData.children_names.push("[^<&]*");
    CharData.children_names.push("([^<&] - ']]>')* ']]>' [^<&]*");
    rule_nameRegistry.insert(CharData.rule_name, CharData);


    let mut CharDataOptional = ParsingRule::new("CharData?", RuleType::Optional);
    CharDataOptional.children_names.push("CharData");
    rule_nameRegistry.insert(CharDataOptional.rule_name, CharDataOptional);



    // [23] XMLDecl ::= '<?xml' VersionInfo EncodingDecl? SDDecl? S? '?>'
    let mut XMLDecl_start = ParsingRule::new("'<?xml'", RuleType::CharSequence);
    XMLDecl_start.expected_chars = "<?xml".chars().collect();
    rule_nameRegistry.insert(XMLDecl_start.rule_name, XMLDecl_start);

    let mut XMLDecl_end = ParsingRule::new("'?>'", RuleType::CharSequence);
    XMLDecl_end.expected_chars = "?>".chars().collect();
    rule_nameRegistry.insert(XMLDecl_end.rule_name, XMLDecl_end);

    let mut XMLDecl = ParsingRule::new("XMLDecl", RuleType::Sequence);
    XMLDecl.children_names.push("'<?xml'");
    XMLDecl.children_names.push("VersionInfo");
    XMLDecl.children_names.push("EncodingDecl?");
    // TODO MISSING SDDecl?
    XMLDecl.children_names.push("S?");
    XMLDecl.children_names.push("'?>'");
    rule_nameRegistry.insert(XMLDecl.rule_name, XMLDecl);

    let mut XMLDecl_optional = ParsingRule::new("XMLDecl?", RuleType::Optional);
    XMLDecl_optional.children_names.push("XMLDecl");
    rule_nameRegistry.insert(XMLDecl_optional.rule_name, XMLDecl_optional);



    // [24] VersionInfo ::= S 'version' Eq ("'" VersionNum "'" | '"' VersionNum '"')
    let mut VersionInfo_version = ParsingRule::new("'version'", RuleType::CharSequence);
    VersionInfo_version.expected_chars = "version".chars().collect();
    rule_nameRegistry.insert(VersionInfo_version.rule_name, VersionInfo_version);


    let mut VersionInfo_VNum1 = ParsingRule::new("\"'\" VersionNum \"'\"", RuleType::Sequence);
    VersionInfo_VNum1.children_names.push("'");
    VersionInfo_VNum1.children_names.push("VersionNum");
    VersionInfo_VNum1.children_names.push("'");
    rule_nameRegistry.insert(VersionInfo_VNum1.rule_name, VersionInfo_VNum1);

    let mut VersionInfo_VNum2 = ParsingRule::new("'\"' VersionNum '\"'", RuleType::Sequence);
    VersionInfo_VNum2.children_names.push("\"");
    VersionInfo_VNum2.children_names.push("VersionNum");
    VersionInfo_VNum2.children_names.push("\"");
    rule_nameRegistry.insert(VersionInfo_VNum2.rule_name, VersionInfo_VNum2);

    let mut VersionInfo_VNum = ParsingRule::new("VersionInfo_VersionNum", RuleType::Or);
    VersionInfo_VNum.children_names.push("\"'\" VersionNum \"'\"");
    VersionInfo_VNum.children_names.push("'\"' VersionNum '\"'");
    rule_nameRegistry.insert(VersionInfo_VNum.rule_name, VersionInfo_VNum);

    let mut VersionInfo = ParsingRule::new("VersionInfo", RuleType::Sequence);
    VersionInfo.children_names.push("S");
    VersionInfo.children_names.push("'version'");
    VersionInfo.children_names.push("Eq");
    VersionInfo.children_names.push("VersionInfo_VersionNum");
    rule_nameRegistry.insert(VersionInfo.rule_name, VersionInfo);


    // [25] Eq ::= S? '=' S?
    let mut equalsCharRule = ParsingRule::new("'='", RuleType::Chars);
    equalsCharRule.expected_chars.push('=');
    rule_nameRegistry.insert(equalsCharRule.rule_name, equalsCharRule);

    let mut _Eq = ParsingRule::new("Eq", RuleType::Sequence);
    _Eq.children_names.push("S?");
    _Eq.children_names.push("'='");
    _Eq.children_names.push("S?");
    rule_nameRegistry.insert(_Eq.rule_name, _Eq);

    // [26] VersionNum ::= '1.' [0-9]+
    let mut VersionNum_1 = ParsingRule::new("'1.'", RuleType::CharSequence);
    VersionNum_1.expected_chars = "1.".to_owned().chars().collect();
    rule_nameRegistry.insert(VersionNum_1.rule_name, VersionNum_1);

    let mut VersionNum_09 = ParsingRule::new("[0-9]", RuleType::Chars);
    VersionNum_09.expected_char_ranges.push(('0', '9'));
    rule_nameRegistry.insert(VersionNum_09.rule_name, VersionNum_09);

    let mut VersionNum_09_zom = ParsingRule::new("[0-9]*", RuleType::ZeroOrMore);
    VersionNum_09_zom.children_names.push("[0-9]");
    rule_nameRegistry.insert(VersionNum_09_zom.rule_name, VersionNum_09_zom);

    let mut VersionNum_09_oom = ParsingRule::new("[0-9]+", RuleType::Sequence);
    VersionNum_09_oom.children_names.push("[0-9]");
    VersionNum_09_oom.children_names.push("[0-9]*");
    rule_nameRegistry.insert(VersionNum_09_oom.rule_name, VersionNum_09_oom);

    let mut VersionNum = ParsingRule::new("VersionNum", RuleType::Sequence);
    VersionNum.children_names.push("'1.'");
    VersionNum.children_names.push("[0-9]+");
    rule_nameRegistry.insert(VersionNum.rule_name, VersionNum);




    // [39] element ::= EmptyElemTag | STag content ETag
    // TODO spec incomplete
    let mut element_notempty = ParsingRule::new("STag content ETag", RuleType::Sequence);
    element_notempty.children_names.push("STag");
    element_notempty.children_names.push("content");
    element_notempty.children_names.push("ETag");

    rule_nameRegistry.insert(element_notempty.rule_name, element_notempty);

    let mut element = ParsingRule::new("element", RuleType::Or);
    element.children_names.push("EmptyElemTag");
    element.children_names.push("STag content ETag");
    rule_nameRegistry.insert(element.rule_name, element);



    // [40] STag ::= '<' Name (S Attribute)* S? '>'
    let mut STag = ParsingRule::new("STag", RuleType::Sequence);
    STag.is_chunkable = false;
    STag.children_names.push("<");  //'<' Name (S Attribute)* S? '>'
    STag.children_names.push("Name");
    STag.children_names.push("(S Attribute)*");
    STag.children_names.push("S?");
    STag.children_names.push(">");
    rule_nameRegistry.insert(STag.rule_name, STag);


    // [41] Attribute ::= Name Eq AttValue
    let mut Attribute = ParsingRule::new("Attribute", RuleType::Sequence);
    Attribute.children_names.push("Name");
    Attribute.children_names.push("Eq");
    Attribute.children_names.push("AttValue");
    rule_nameRegistry.insert(Attribute.rule_name, Attribute);

    let mut Attribute_optional = ParsingRule::new("Attribute?", RuleType::Optional);
    Attribute_optional.children_names.push("Attribute");
    rule_nameRegistry.insert(Attribute_optional.rule_name, Attribute_optional);

    // (S Attribute)
    let mut Attribute_after_s = ParsingRule::new("(S Attribute)", RuleType::Sequence);
    Attribute_after_s.children_names.push("S");
    Attribute_after_s.children_names.push("Attribute");
    rule_nameRegistry.insert(Attribute_after_s.rule_name, Attribute_after_s);

    // (S Attribute)*
    let mut Attribute_after_s_zom = ParsingRule::new("(S Attribute)*", RuleType::ZeroOrMore);
    Attribute_after_s_zom.children_names.push("(S Attribute)");
    rule_nameRegistry.insert(Attribute_after_s_zom.rule_name, Attribute_after_s_zom);


    // [42] ETag ::= '</' Name S? '>'
    let mut ETag = ParsingRule::new("ETag", RuleType::Sequence);
    ETag.is_chunkable = false;
    ETag.children_names.push("'</'");
    ETag.children_names.push("Name");
    ETag.children_names.push("S?");
    ETag.children_names.push(">");

    rule_nameRegistry.insert(ETag.rule_name, ETag);




    // [43] content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
    // TODO unimplemented spec
    // need to seperate circular ref-seperated..
    // put element to last child for reducing backtrack needs
    let mut content_inside = ParsingRule::new("(element | Reference | CDSect | PI | Comment)",
                                              RuleType::Or);
    content_inside.children_names.push("Reference");
    content_inside.children_names.push("element");
    // TODO add child here
    rule_nameRegistry.insert(content_inside.rule_name, content_inside);


    let mut content_inside_and_CharData = ParsingRule::new("(content_inside CharData?)",
                                                           RuleType::Sequence);
    // ruleRegistry.insert(elementAndCharData.rule_name, rule_vec.len());
    content_inside_and_CharData.children_names
        .push("(element | Reference | CDSect | PI | Comment)");
    content_inside_and_CharData.children_names.push("CharData?");
    rule_nameRegistry.insert(content_inside_and_CharData.rule_name,
                             content_inside_and_CharData);

    let mut content_inside_and_CharData_zom = ParsingRule::new("(content_inside CharData?)*",
                                                               RuleType::ZeroOrMore);
    content_inside_and_CharData_zom.children_names.push("(content_inside CharData?)");
    rule_nameRegistry.insert(content_inside_and_CharData_zom.rule_name,
                             content_inside_and_CharData_zom);

    let mut content = ParsingRule::new("content", RuleType::Sequence);
    content.children_names.push("CharData?");
    content.children_names.push("(content_inside CharData?)*");
    rule_nameRegistry.insert(content.rule_name, content);

    // [44] EmptyElemTag ::= '<' Name (S Attribute)* S? '/>'

    let mut EmptyElemTag_end = ParsingRule::new("'/>'", RuleType::CharSequence);
    EmptyElemTag_end.expected_chars.push('/');
    EmptyElemTag_end.expected_chars.push('>');
    rule_nameRegistry.insert(EmptyElemTag_end.rule_name, EmptyElemTag_end);

    let mut EmptyElemTag = ParsingRule::new("EmptyElemTag", RuleType::Sequence);
    EmptyElemTag.is_chunkable = false;
    EmptyElemTag.children_names.push("<");
    EmptyElemTag.children_names.push("Name");
    EmptyElemTag.children_names.push("(S Attribute)*");
    EmptyElemTag.children_names.push("S?");
    EmptyElemTag.children_names.push("'/>'");

    rule_nameRegistry.insert(EmptyElemTag.rule_name, EmptyElemTag);


    // [66] CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'
    let mut CharRef_ampdial = ParsingRule::new("'&#'", RuleType::CharSequence);
    CharRef_ampdial.expected_chars.push('&');
    CharRef_ampdial.expected_chars.push('#');
    rule_nameRegistry.insert(CharRef_ampdial.rule_name, CharRef_ampdial);

    let mut CharRef_ampx = ParsingRule::new("'&#x'", RuleType::CharSequence);
    CharRef_ampx.expected_chars.push('&');
    CharRef_ampx.expected_chars.push('#');
    CharRef_ampx.expected_chars.push('x');
    rule_nameRegistry.insert(CharRef_ampx.rule_name, CharRef_ampx);

    let mut CharRef_09 = ParsingRule::new("[0-9]", RuleType::Chars);
    CharRef_09.expected_char_ranges.push(('0', '9'));
    rule_nameRegistry.insert(CharRef_09.rule_name, CharRef_09);

    let mut CharRef_09_zom = ParsingRule::new("[0-9]*", RuleType::ZeroOrMore);
    CharRef_09_zom.children_names.push("[0-9]");
    rule_nameRegistry.insert(CharRef_09_zom.rule_name, CharRef_09_zom);

    let mut CharRef_09_oom = ParsingRule::new("[0-9]+", RuleType::Sequence);
    CharRef_09_oom.children_names.push("[0-9]");
    CharRef_09_oom.children_names.push("[0-9]*");
    rule_nameRegistry.insert(CharRef_09_oom.rule_name, CharRef_09_oom);

    let mut CharRef_alt_1 = ParsingRule::new("CharRef_alt_1", RuleType::Sequence);
    CharRef_alt_1.children_names.push("'&#'");
    CharRef_alt_1.children_names.push("[0-9]+");
    CharRef_alt_1.children_names.push("';'");
    rule_nameRegistry.insert(CharRef_alt_1.rule_name, CharRef_alt_1);

    let mut CharRef_09af = ParsingRule::new("[0-9a-fA-F]", RuleType::Chars);
    CharRef_09af.expected_char_ranges.push(('0', '9'));
    CharRef_09af.expected_char_ranges.push(('a', 'f'));
    CharRef_09af.expected_char_ranges.push(('A', 'F'));
    rule_nameRegistry.insert(CharRef_09af.rule_name, CharRef_09af);

    let mut CharRef_09af_zom = ParsingRule::new("[0-9a-fA-F]*", RuleType::ZeroOrMore);
    CharRef_09af_zom.children_names.push("[0-9a-fA-F]");
    rule_nameRegistry.insert(CharRef_09af_zom.rule_name, CharRef_09af_zom);

    let mut CharRef_09af_oom = ParsingRule::new("[0-9a-fA-F]+", RuleType::Sequence);
    CharRef_09af_oom.children_names.push("[0-9a-fA-F]");
    CharRef_09af_oom.children_names.push("[0-9a-fA-F]*");
    rule_nameRegistry.insert(CharRef_09af_oom.rule_name, CharRef_09af_oom);

    // '&#x' [0-9a-fA-F]+ ';'
    let mut CharRef_alt_2 = ParsingRule::new("CharRef_alt_2", RuleType::Sequence);
    CharRef_alt_2.children_names.push("'&#x'");
    CharRef_alt_2.children_names.push("[0-9a-fA-F]+");
    CharRef_alt_2.children_names.push("';'");
    rule_nameRegistry.insert(CharRef_alt_2.rule_name, CharRef_alt_2);

    let mut CharRef = ParsingRule::new("CharRef", RuleType::Or);
    CharRef.children_names.push("CharRef_alt_1");
    CharRef.children_names.push("CharRef_alt_2");
    rule_nameRegistry.insert(CharRef.rule_name, CharRef);


    // [67] Reference ::= EntityRef | CharRef
    let mut Reference = ParsingRule::new("Reference", RuleType::Or);
    Reference.children_names.push("EntityRef");
    Reference.children_names.push("CharRef");
    rule_nameRegistry.insert(Reference.rule_name, Reference);

    // [68] EntityRef ::= '&' Name ';'
    let mut EntityRef = ParsingRule::new("EntityRef", RuleType::Sequence);
    EntityRef.children_names.push("'&'");
    EntityRef.children_names.push("Name");
    EntityRef.children_names.push("';'");
    rule_nameRegistry.insert(EntityRef.rule_name, EntityRef);

    // [69] PEReference ::= '%' Name ';'
    let mut PEReference = ParsingRule::new("PEReference", RuleType::Sequence);
    PEReference.children_names.push("'%'");
    PEReference.children_names.push("Name");
    PEReference.children_names.push("';'");
    rule_nameRegistry.insert(PEReference.rule_name, PEReference);

    // [80] EncodingDecl EncodingDecl	   ::=   	S 'encoding' Eq ('"' EncName '"' | "'" EncName "'" )
    let mut EncodingDecl_encoding = ParsingRule::new("'encoding'", RuleType::CharSequence);
    EncodingDecl_encoding.expected_chars = "encoding".to_owned().chars().collect();
    rule_nameRegistry.insert(EncodingDecl_encoding.rule_name, EncodingDecl_encoding);

    let mut EncodingDecl_encname_1 = ParsingRule::new("EncodingDecl_encname_1", RuleType::Sequence);
    EncodingDecl_encname_1.children_names.push("\"");
    EncodingDecl_encname_1.children_names.push("EncName");
    EncodingDecl_encname_1.children_names.push("\"");
    rule_nameRegistry.insert(EncodingDecl_encname_1.rule_name, EncodingDecl_encname_1);

    let mut EncodingDecl_encname_2 = ParsingRule::new("EncodingDecl_encname_2", RuleType::Sequence);
    EncodingDecl_encname_2.children_names.push("'");
    EncodingDecl_encname_2.children_names.push("EncName");
    EncodingDecl_encname_2.children_names.push("'");
    rule_nameRegistry.insert(EncodingDecl_encname_2.rule_name, EncodingDecl_encname_2);

    let mut EncodingDecl_encname = ParsingRule::new("EncodingDecl_encname", RuleType::Or);
    EncodingDecl_encname.children_names.push("EncodingDecl_encname_1");
    EncodingDecl_encname.children_names.push("EncodingDecl_encname_2");
    rule_nameRegistry.insert(EncodingDecl_encname.rule_name, EncodingDecl_encname);

    let mut EncodingDecl = ParsingRule::new("EncodingDecl", RuleType::Sequence);
    EncodingDecl.children_names.push("S");
    EncodingDecl.children_names.push("'encoding'");
    EncodingDecl.children_names.push("Eq");
    EncodingDecl.children_names.push("EncodingDecl_encname");
    rule_nameRegistry.insert(EncodingDecl.rule_name, EncodingDecl);

    let mut EncodingDecl_optional = ParsingRule::new("EncodingDecl?", RuleType::Optional);
    EncodingDecl_optional.children_names.push("EncodingDecl");
    rule_nameRegistry.insert(EncodingDecl_optional.rule_name, EncodingDecl_optional);





    // [81] EncName
    let mut EncName_az = ParsingRule::new("[A-Za-z]", RuleType::Chars);
    EncName_az.expected_char_ranges.push(('a', 'z'));
    EncName_az.expected_char_ranges.push(('A', 'Z'));
    rule_nameRegistry.insert(EncName_az.rule_name, EncName_az);

    let mut EncName_az09 = ParsingRule::new("[A-Za-z0-9._]", RuleType::Chars);
    EncName_az09.expected_char_ranges.push(('a', 'z'));
    EncName_az09.expected_char_ranges.push(('A', 'Z'));
    EncName_az09.expected_char_ranges.push(('0', '9'));
    EncName_az09.expected_chars.push('.');
    EncName_az09.expected_chars.push('_');
    rule_nameRegistry.insert(EncName_az09.rule_name, EncName_az09);

    let mut EncName_hyphen = ParsingRule::new("-", RuleType::Chars);
    EncName_hyphen.expected_chars.push('-');
    rule_nameRegistry.insert(EncName_hyphen.rule_name, EncName_hyphen);

    let mut EncName_part2_single = ParsingRule::new("([A-Za-z0-9._] | '-')", RuleType::Or);
    EncName_part2_single.children_names.push("-");
    EncName_part2_single.children_names.push("[A-Za-z0-9._]");
    rule_nameRegistry.insert(EncName_part2_single.rule_name, EncName_part2_single);

    let mut EncName_part2 = ParsingRule::new("([A-Za-z0-9._] | '-')*", RuleType::ZeroOrMore);
    EncName_part2.children_names.push("([A-Za-z0-9._] | '-')");
    rule_nameRegistry.insert(EncName_part2.rule_name, EncName_part2);

    let mut EncName = ParsingRule::new("EncName", RuleType::Sequence);
    EncName.children_names.push("[A-Za-z]");
    EncName.children_names.push("([A-Za-z0-9._] | '-')*");
    rule_nameRegistry.insert(EncName.rule_name, EncName);


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
    fn try(&mut self, rule_name: &str, starting_pos: usize) -> ();
    fn pass(&mut self,
            rule_name: &str,
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

    logger.try(rule.rule_name, starting_pos);

    match rule.rule_type {
        RuleType::Chars => {
            if starting_pos - offset >= char_vector.len() {
                return (logger, ParsingResult::EOF);
            }
            let c = char_vector[starting_pos - offset];


            for range in &rule.expected_char_ranges {


                if range.0 <= c && c <= range.1 {
                    logger.pass(rule.rule_name, char_vector, starting_pos, starting_pos + 1);
                    return (logger, ParsingResult::Pass(starting_pos, starting_pos + 1));
                }
            }
            for check_char in &rule.expected_chars {
                if *check_char == c {
                    logger.pass(rule.rule_name, char_vector, starting_pos, starting_pos + 1);
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
            logger.pass(rule.rule_name, char_vector, starting_pos, starting_pos + 1);
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
            logger.pass(rule.rule_name, char_vector, starting_pos, new_starting_pos);
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
            logger.pass(rule.rule_name, char_vector, starting_pos, new_starting_pos);
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
                        logger.pass(rule.rule_name, char_vector, starting_pos, 0);
                        return (logger, ParsingResult::EOF);
                    }
                }
                state_vec.pop();
            }
            logger.pass(rule.rule_name, char_vector, starting_pos, new_starting_pos);
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
                        logger.pass(rule.rule_name, char_vector, s_pos, e_pos);
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
                            logger.pass(rule.rule_name, char_vector, s_pos, e_pos);
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
                    logger.pass(rule.rule_name, char_vector, s_pos, e_pos);
                    return (logger, result.1);
                }
                ParsingResult::Fail => {
                    logger.pass(rule.rule_name, char_vector, starting_pos, starting_pos);
                    return (logger, ParsingResult::Pass(starting_pos, starting_pos));
                }
                ParsingResult::EOF => {
                    // burada state_vec.pop(); cagirmiyoruz
                    return (logger, ParsingResult::EOF);
                }
            }

        }
        // unreachable
        //        _ => {
        // println!("UNIMPLEMENTED PARSER FOR TYPE!");
        //
        // ParsingResult::Fail
        // }
    }
}