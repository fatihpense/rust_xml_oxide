use parser::ParsingPassLogStream;
use parser::parse_with_rule;
use parser::parse_with_rule2;
use parser::prepare_rules;

use std::io::Read;
use std::collections::HashMap;

use char_iter;
use std::char;

use xml_sax::*;

use itertools;

use std::rc::Rc;
use std::cell::RefCell;

pub struct SaxParser {
    content_handler: Option<Rc<RefCell<ContentHandler>>>,
    stats_handler: Option<Rc<RefCell<StatsHandler>>>,
    counter: i64,
    element_names: Vec<String>,
    attribute_values: Vec<String>,
    attributes: Attributes,
    namespaces: Vec<(usize, String, String)>, //element depth, prefix, uri
    element_depth: usize,
}

#[derive(Clone)]
struct Attribute {
    uri: String,
    qualified_name: String,
    local_name: String,
    value: String,
}

impl SAXAttribute for Attribute {
    fn get_value(&self) -> &str {
        &self.value
    }

    fn get_qualified_name(&self) -> &str {
        &self.qualified_name
    }
    fn get_local_name(&self) -> &str {
        &self.local_name
    }
    fn get_uri(&self) -> &str {
        &self.uri
    }
}

impl Attribute {
    fn new() -> Attribute {
        Attribute {
            uri: String::new(),
            qualified_name: String::new(),
            local_name: String::new(),
            value: String::new(),
        }
    }
}

struct Attributes {
    attribute_vec: Vec<Box<Attribute>>,
    attribute_qname_map: HashMap<String, usize>,
    attribute_uri_local_map: HashMap<String, usize>,
}
impl SAXAttributes for Attributes {
    fn get_length(&self) -> usize {
        self.attribute_vec.len()
    }

    fn get_by_index(&self, index: usize) -> Option<Box<SAXAttribute>> {
        match self.attribute_vec.get(index) {
            // Some(val) => Some(Box::new((*val).clone())),
            Some(val) => {
                let x = *val.clone();
                Some(Box::new(x) as Box<SAXAttribute>)
            }
            None => None,
        }
    }

    fn iter(&self) -> Box<Iterator<Item = Box<SAXAttribute>>> {
        let x = AttributesIter {
            attribute_vec: self.attribute_vec.clone(),
            cur: 0,
        };
        return Box::new(x);
    }
}

pub struct AttributesIter {
    attribute_vec: Vec<Box<Attribute>>,
    cur: usize,
}

impl Iterator for AttributesIter {
    type Item = Box<SAXAttribute>;
    fn next(&mut self) -> Option<Box<SAXAttribute>> {
        let r = match self.attribute_vec.get(self.cur) {
            Some(val) => Some((*val).clone() as Box<SAXAttribute>),
            None => None,
        };

        self.cur += 1;
        return r;
    }
}

impl Attributes {
    fn new() -> Attributes {
        Attributes {
            attribute_vec: Vec::new(),
            attribute_qname_map: HashMap::new(),
            attribute_uri_local_map: HashMap::new(),
        }
    }
    fn clear(&mut self) {
        self.attribute_vec.clear();
        self.attribute_qname_map.clear();
        self.attribute_uri_local_map.clear();
    }
}

impl<'a> ParsingPassLogStream for SaxParser {
    fn offset(&mut self, offset: usize) {
        //self.content_handler.
        //&self.content_handler.as_mut().unwrap().offset(offset);
        {
            if let &Some(_) = &self.stats_handler {
                self.stats_handler
                    .as_mut()
                    .unwrap()
                    .borrow_mut()
                    .offset(offset);
            }
        }
    }
    fn try(&mut self, rulename: String, _: usize) -> () {
        //starting_pos _
        //println!("try rule: {:?}",rulename );
        if rulename == "STag" || rulename == "EmptyElemTag" {
            self.element_names.clear();
            self.attribute_values.clear();
            self.attributes.clear();
        }
    }

    fn pass(
        &mut self,
        rulename: String,
        chars: &Vec<char>,
        mut starting_pos: usize,
        mut ending_pos: usize,
    ) -> () {
        //println!("pass rule: {:?}, {:?},{:?}",rulename,starting_pos,ending_pos );

        if starting_pos > ending_pos || ending_pos == 0 {
            return;
        }

        //normalize for parser3
        ending_pos = ending_pos - starting_pos;
        starting_pos = 0;

        if rulename == "Name" {
            let s: String = chars[starting_pos..ending_pos]
                .into_iter()
                .cloned()
                .collect();
            self.element_names.push(s);
        }

        if rulename == "AttValue" {
            let s: String = chars[starting_pos..ending_pos]
                .into_iter()
                .skip(1)
                .take(ending_pos - starting_pos - 2)
                .cloned()
                .collect();
            self.attribute_values.push(s);
        }

        if rulename == "Attribute" {
            let name: String = self.element_names.pop().unwrap();
            let value: String = self.attribute_values.pop().unwrap();
            let mut attr: Attribute = Attribute::new();
            attr.qualified_name = name;
            attr.value = value;
            self.attributes.attribute_vec.push(Box::new(attr));
        }

        if rulename == "element" {
            self.counter = self.counter + 1;
        }

        if rulename == "STag" {
            self.element_depth += 1;
            //start prefix mapping

            //set xmlns: values (should merge#1)
            for attr in self.attributes.attribute_vec.iter() {
                if attr.get_qualified_name().starts_with("xmlns:") {
                    let v: Vec<&str> = attr.get_qualified_name().clone().split(':').collect();
                    // let prefix = v[1];
                    //attr.get_value();
                    self.namespaces.push((
                        self.element_depth,
                        v[1].to_owned(),
                        attr.get_value().to_owned(),
                    ));
                }
                if attr.get_qualified_name() == "xmlns" {
                    self.namespaces.push((
                        self.element_depth,
                        "".to_owned(),
                        attr.get_value().to_owned(),
                    ));
                }
            }

            //attribute code should merge #1
            for attr in self.attributes.attribute_vec.iter_mut() {
                let prefix: String;
                let local_name: String;
                let uri: String;
                if attr.get_qualified_name().contains(':') {
                    {
                        let v: Vec<&str> = attr.get_qualified_name().clone().split(':').collect();
                        prefix = v[0].to_owned();
                        local_name = v[1].to_owned();
                    }
                    //search it.
                    let default = &(0, "".to_owned(), "".to_owned());
                    let result = self.namespaces
                        .iter()
                        .rev()
                        .find(|&x| x.1 == prefix)
                        .unwrap_or(default);
                    //attr.prefix = prefix;
                    uri = result.2.clone();
                } else {
                    //  ""
                    //https://stackoverflow.com/questions/3312390/xml-default-namespaces-for-unqualified-attribute-names
                    local_name = attr.get_qualified_name().to_owned();
                    //prefix = "".to_owned();
                    uri = "".to_owned();
                }
                attr.local_name = local_name;
                attr.uri = uri;
            }
            //elementname code should merge #1
            let name: String = self.element_names.pop().unwrap();

            let prefix: String;
            let local_name: String;
            let uri: String;
            if name.contains(':') {
                {
                    let v: Vec<&str> = name.split(':').collect();
                    prefix = v[0].to_owned();
                    local_name = v[1].to_owned();
                }
            } else {
                //  ""
                local_name = name.to_owned();
                prefix = "".to_owned();
            }
            //search it.
            let default = &(0, "".to_owned(), "".to_owned());
            let result = self.namespaces
                .iter()
                .rev()
                .find(|&x| x.1 == prefix)
                .unwrap_or(default);
            //attr.prefix = prefix;
            uri = result.2.clone();

            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .start_element(&uri, &local_name, &name, &self.attributes);
        }

        if rulename == "EmptyElemTag" {
            self.element_depth += 1;
            //start prefix mapping

            //set xmlns: values (should merge#1)
            for attr in self.attributes.attribute_vec.iter() {
                if attr.get_qualified_name().starts_with("xmlns:") {
                    let v: Vec<&str> = attr.get_qualified_name().clone().split(':').collect();
                    // let prefix = v[1];
                    //attr.get_value();
                    self.namespaces.push((
                        self.element_depth,
                        v[1].to_owned(),
                        attr.get_value().to_owned(),
                    ));
                }
                if attr.get_qualified_name() == "xmlns" {
                    self.namespaces.push((
                        self.element_depth,
                        "".to_owned(),
                        attr.get_value().to_owned(),
                    ));
                }
            }
            //attribute code should merge #2
            for attr in self.attributes.attribute_vec.iter_mut() {
                let prefix: String;
                let local_name: String;
                let uri: String;
                if attr.get_qualified_name().contains(':') {
                    {
                        let v: Vec<&str> = attr.get_qualified_name().clone().split(':').collect();
                        prefix = v[0].to_owned();
                        local_name = v[1].to_owned();
                    }
                    //search it.
                    let default = &(0, "".to_owned(), "".to_owned());
                    let result = self.namespaces
                        .iter()
                        .rev()
                        .find(|&x| x.1 == prefix)
                        .unwrap_or(default);
                    //attr.prefix = prefix;
                    uri = result.2.clone();
                } else {
                    //  ""
                    //https://stackoverflow.com/questions/3312390/xml-default-namespaces-for-unqualified-attribute-names
                    local_name = attr.get_qualified_name().to_owned();
                    //prefix = "".to_owned();
                    uri = "".to_owned();
                }
                attr.local_name = local_name;
                attr.uri = uri;
            }

            //elementname code should merge #2
            let name: String = self.element_names.pop().unwrap();

            let prefix: String;
            let local_name: String;
            let uri: String;
            if name.contains(':') {
                {
                    let v: Vec<&str> = name.split(':').collect();
                    prefix = v[0].to_owned();
                    local_name = v[1].to_owned();
                }
            } else {
                //  ""
                local_name = name.to_owned();
                prefix = "".to_owned();
            }
            //search it.
            let default = &(0, "".to_owned(), "".to_owned());
            let result = self.namespaces
                .iter()
                .rev()
                .find(|&x| x.1 == prefix)
                .unwrap_or(default)
                .to_owned();

            //attr.prefix = prefix;
            uri = result.2.clone();

            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .start_element(&uri, &local_name, &name, &self.attributes);

            self.element_depth -= 1;
            //end prefix mapping

            /* loop {
                match self.namespaces.last().clone() {
                    Some(ns_tuple) => {
                        if ns_tuple.0 > self.element_depth {
                            self.namespaces.pop();
                        }
                    }
                    None => {
                        break;
                    }
                };
            }*/
            let depth = self.element_depth;
            self.namespaces.retain(|ref x| x.0 <= depth);
        }

        if rulename == "ETag" {
            let name: String = self.element_names.pop().unwrap();
            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .end_element("", "", &name);

            self.element_depth -= 1;
            //end prefix mapping

            let depth = self.element_depth;
            self.namespaces.retain(|ref x| x.0 <= depth);
        }
        if rulename == "CharData?" {
            let s: String = chars[starting_pos..ending_pos]
                .into_iter()
                .cloned()
                .collect();
            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .characters(&s);
        }

        // [18] CDSect
        if rulename == "CDSect" {
            let s: String = chars[starting_pos + 9..ending_pos - 3]
                .into_iter()
                .cloned()
                .collect();
            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .characters(&s);
        }

        // rule 67
        if rulename == "Reference" {
            let s: String = chars[starting_pos..ending_pos]
                .into_iter()
                .cloned()
                .collect();
            let result: String;
            // rule 66 CharRef
            if s.starts_with("&#x") {
                // parse hex
                let hex_val: String = s.chars()
                    .filter(|&n| n != '&' && n != '#' && n != 'x' && n != ';')
                    .collect();
                // todo dont panic give error.
                result = char::from_u32(u32::from_str_radix(&hex_val, 16).unwrap())
                    .unwrap()
                    .to_string();
            // .collect::<Vec<char>>(); also working but vec
            } else if s.starts_with("&#") {
                // parse scalar
                let scalar_val: String = s.chars()
                    .filter(|&n| n != '&' && n != '#' && n != ';')
                    .collect();
                // todo dont panic give error.
                result = char::from_u32(u32::from_str_radix(&scalar_val, 10).unwrap())
                    .unwrap()
                    .to_string();
            } else {
                // rule 68 EntityRef
                result = match s.as_ref() {
                    "&quot;" => '"'.to_string(),
                    "&amp;" => '&'.to_string(),
                    "&apos;" => '\''.to_string(),
                    "&lt;" => '<'.to_string(),
                    "&gt;" => '>'.to_string(),

                    _ => "".to_string(), //TODO give error
                };
            }
            self.content_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .characters(&result);
        }
    }
}

impl<'a> SaxParser {
    pub fn new() -> SaxParser {
        return SaxParser {
            content_handler: None,
            stats_handler: None,
            counter: 0,
            element_names: Vec::new(),
            attribute_values: Vec::new(),
            attributes: Attributes::new(),
            namespaces: Vec::new(),
            element_depth: 0,
        };
    }
    pub fn set_content_handler<T: ContentHandler + 'static>(
        &mut self,
        content_handler: Rc<RefCell<T>>,
    ) {
        self.content_handler = Some(content_handler);
    }

    pub fn set_stats_handler<T: StatsHandler + 'static>(&mut self, stats_handler: Rc<RefCell<T>>) {
        self.stats_handler = Some(stats_handler);
    }

    pub fn parse_old<R: Read>(mut self, read: R) {
        let parser_rules = prepare_rules();
        let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("document").unwrap()];

        let mut resume_vec = Vec::new();
        let mut state_vec = Vec::new();
        let offset: usize = 0;

        let mut chars: Vec<char> = Vec::new();
        let citer = char_iter::chars(read);

        let mut chunk_count: usize = 0;
        for ch in citer {
            if ch.is_ok() {
                chars.push(ch.unwrap());
                chunk_count += 1;
                if chunk_count >= 10000 {
                    chunk_count = 0;

                    let result = parse_with_rule(
                        &parser_rules.rule_vec,
                        &rule,
                        &chars,
                        0,
                        offset,
                        &mut resume_vec,
                        &mut state_vec,
                        self,
                    );
                    self = result.0;

                    let mut erasable_pos: usize = 0;
                    let mut split_pos: usize = 0;
                    for (index, state) in state_vec.iter().enumerate() {
                        if state.2 {
                            // no backtrack req true
                            erasable_pos = state.1; //pos
                            split_pos = index + 1;
                        } else {
                            split_pos = index;
                            break;
                        }
                    }

                    for state in &mut state_vec {
                        state.1 = 0;
                    }
                    state_vec.split_off(split_pos);

                    state_vec.reverse();
                    resume_vec = state_vec;

                    state_vec = Vec::new();
                    // starting position can be different erasable_pos?
                    //&self.content_handler.as_mut().unwrap().offset(erasable_pos);
                    // .unwrap()
                    {
                        if let Some(_) = self.stats_handler {
                            self.stats_handler
                                .as_mut()
                                .unwrap()
                                .borrow_mut()
                                .offset(erasable_pos);
                        }
                    }

                    chars = chars.split_off(erasable_pos);
                }
            } else {
                break;
            }
        }

        // last bit
        let result = parse_with_rule(
            &parser_rules.rule_vec,
            &rule,
            &chars,
            0,
            offset,
            &mut resume_vec,
            &mut state_vec,
            self,
        );
        self = result.0;

        // .unwrap()
        if let Some(_) = self.stats_handler {
            self.stats_handler
                .as_mut()
                .unwrap()
                .borrow_mut()
                .offset(chars.len());
        }
    }

    pub fn parse<R: Read>(mut self, read: R) {
        let parser_rules = prepare_rules();
        //parsertidy::remove_optional(&mut parser_rules);
        //parsertidy::remove_zeroormore(&mut parser_rules);
        let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("document").unwrap()];

        //let mut chars: Vec<char> = Vec::new();

        let mut citer = itertools::multipeek(char_iter::chars(read)); //.into_iter().multipeek();
   /* {
    &citer.peek();
    }
*/
        let mut resume_vec = Vec::new();
        let mut state_vec = Vec::new();
        self.content_handler
            .as_mut()
            .unwrap()
            .borrow_mut()
            .start_document();
        let result = parse_with_rule2(
            &parser_rules.rule_vec,
            &rule,
            &mut citer,
            //&Vec::new(),
            0,
            0,
            &mut resume_vec,
            &mut state_vec,
            self,
        );
        self = result.1;
        self.content_handler
            .as_mut()
            .unwrap()
            .borrow_mut()
            .end_document();
        //println!("result:{:?}",result.2 );
    }
}
