
use parser::ParsingPassLogStream;
use parser::parse_with_rule;
use parser::prepare_rules;
use parsertidy;

use std::io::Read;
use std::collections::HashMap;

use char_iter;
use std::char;

use xml_sax::*;

use tree::*;

use indextree::Arena;
use std::ops::Index;
use std::ops::IndexMut;

pub struct SaxParser<'a> {
    content_handler: Option<&'a mut ContentHandler>,
    counter: i64,
    element_names: Vec<String>,
    attribute_values: Vec<String>,
    attributes: Attributes,
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

impl<'a> ParsingPassLogStream for SaxParser<'a> {
    fn try(&mut self, rulename: String, starting_pos: usize) -> () {

        if rulename == "STag" || rulename == "EmptyElemTag" {
            self.element_names.clear();
            self.attribute_values.clear();
            self.attributes.clear();
        }
    }

    fn pass(&mut self,
            rulename: String,
            chars: &Vec<char>,
            starting_pos: usize,
            ending_pos: usize)
            -> () {


        if starting_pos > ending_pos || ending_pos == 0 {
            return;
        }
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
            let name: String = self.element_names.pop().unwrap();
            self.content_handler.as_mut().unwrap().start_element(&name, &self.attributes);
        }

        if rulename == "EmptyElemTag" {

            let name: String = self.element_names.pop().unwrap();
            self.content_handler.as_mut().unwrap().start_element(&name, &self.attributes);
        }

        if rulename == "ETag" {
            let name: String = self.element_names.pop().unwrap();
            self.content_handler.as_mut().unwrap().end_element(&name);
        }
        if rulename == "CharData?" {
            let s: String = chars[starting_pos..ending_pos].into_iter().cloned().collect();
            self.content_handler.as_mut().unwrap().characters(&s);
        }

        // [18] CDSect
        if rulename == "CDSect" {
            let s: String = chars[starting_pos + 9..ending_pos - 3].into_iter().cloned().collect();
            self.content_handler.as_mut().unwrap().characters(&s);
        }

        // rule 67
        if rulename == "Reference" {
            let s: String = chars[starting_pos..ending_pos].into_iter().cloned().collect();
            let result: String;
            // rule 66 CharRef
            if s.starts_with("&#x") {

                // parse hex
                let hex_val: String = s.chars()
                    .filter(|&n| n != '&' && n != '#' && n != 'x' && n != ';')
                    .collect();
                // todo dont panic give error.
                result =
                    char::from_u32(u32::from_str_radix(&hex_val, 16).unwrap()).unwrap().to_string();
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
            self.content_handler.as_mut().unwrap().characters(&result);
        }


    }
}

impl<'a> SaxParser<'a> {
    pub fn new() -> SaxParser<'a> {
        return SaxParser {
            content_handler: None,
            counter: 0,
            element_names: Vec::new(),
            attribute_values: Vec::new(),
            attributes: Attributes::new(),
        };
    }
    pub fn set_content_handler<T: ContentHandler>(&mut self, content_handler: &'a mut T) {
        self.content_handler = Some(content_handler);
    }

    pub fn parse<R: Read>(mut self, read: R) {



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


                    let result = parse_with_rule(&parser_rules.rule_vec,
                                                 &rule,
                                                 &chars,
                                                 0,
                                                 offset,
                                                 &mut resume_vec,
                                                 &mut state_vec,
                                                 self);
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
                    &self.content_handler.as_mut().unwrap().offset(erasable_pos);
                    chars = chars.split_off(erasable_pos);

                }
            } else {

                break;
            }
        }

        // last bit
        let result = parse_with_rule(&parser_rules.rule_vec,
                                     &rule,
                                     &chars,
                                     0,
                                     offset,
                                     &mut resume_vec,
                                     &mut state_vec,
                                     self);
        self = result.0;


        &self.content_handler.as_mut().unwrap().offset(chars.len());


    }

     pub fn parse2<R: Read>(mut self, read: R) {



        let mut parser_rules = prepare_rules();
        parsertidy::remove_optional(&mut parser_rules);
        parsertidy::remove_zeroormore(&mut parser_rules);
        let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("document").unwrap()];


        //let mut chars: Vec<char> = Vec::new();
        let citer = char_iter::chars(read);

 let mut arena = &mut Arena::new();


let node=  PNode{    rulename: "document".to_owned(),
                    state: StateType::Init,
                    ruletype : rule.rule_type.clone(),
                    current_sequence: 0};

    // Add some new nodes to the arena
    let a = arena.new_node(node);

        for (i,ch) in citer.enumerate() {

            if ch.is_ok() {
//parse with state char by char
                println!("" );
                PNode::new_char(&parser_rules,a, arena, ch.unwrap());
                println!("PRINTING");

               {
                    let pnode : &PNode = &arena.index(a).data;
                     println!("{:?}|sta:{:?}|seq:{:?}|typ:{:?}", pnode.rulename,pnode.state,pnode.current_sequence,pnode.ruletype); //arena.index(n).data
               }
                PNode::print(a, arena,0);
                //node.new_char(&mut arena, ch.unwrap());
               
               /*if i==5{
               break;
               }*/
               
            } else {

                break;
            }
        }
         //PNode::print(a, arena);

     }
 
}
