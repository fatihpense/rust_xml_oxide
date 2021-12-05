// Pull API

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
    pub attributes: Vec<Attribute<'a>>,
    pub is_empty: bool,
    // namespace aware
    pub local_name: &'a str,
    pub prefix: &'a str,
    pub namespace: &'a str,
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
