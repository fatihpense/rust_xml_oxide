mod circular;
mod internal;
pub mod parser;

use crate::sax::parser::convert_attribute_range;
// Pull API

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attributes<'a> {
    index: usize,
    range_list: &'a Vec<internal::AttributeRange>,
    strbuffer: &'a str,
    namespace_strbuffer: &'a str,
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Attribute<'a>;
    fn next(&mut self) -> Option<Attribute<'a>> {
        match self.range_list.get(self.index) {
            Some(range) => {
                self.index += 1;
                Some(convert_attribute_range(
                    self.strbuffer,
                    self.namespace_strbuffer,
                    range.clone(),
                ))
            }
            None => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attribute<'a> {
    pub value: &'a str,
    pub name: &'a str,
    // namespace aware
    pub local_name: &'a str,
    pub prefix: &'a str,
    pub namespace: &'a str,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartElement<'a> {
    pub name: &'a str,
    // pub attributes: &'a Attributes<'a>,
    pub is_empty: bool,
    // namespace aware
    pub local_name: &'a str,
    pub prefix: &'a str,
    pub namespace: &'a str,

    range_list: &'a Vec<internal::AttributeRange>,
    strbuffer: &'a str,
    namespace_strbuffer: &'a str,
}
impl<'a> StartElement<'a> {
    pub fn attributes(&self) -> Attributes<'a> {
        Attributes {
            index: 0,
            range_list: self.range_list,
            strbuffer: self.strbuffer,
            namespace_strbuffer: self.namespace_strbuffer,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndElement<'a> {
    pub name: &'a str,
    // namespace aware
    pub local_name: &'a str,
    pub prefix: &'a str,
    pub namespace: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference<'a> {
    pub raw: &'a str,
    pub resolved: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event<'a> {
    StartDocument,
    EndDocument,

    StartElement(StartElement<'a>),
    EndElement(EndElement<'a>),
    Characters(&'a str),
    Reference(Reference<'a>),

    StartComment,
    Comment(&'a str),
    EndComment,

    StartCdataSection,
    Cdata(&'a str),
    EndCdataSection,

    DocumentTypeDeclaration(&'a str),
    ProcessingInstruction(&'a str),
    XmlDeclaration(&'a str),
    Whitespace(&'a str),
}
