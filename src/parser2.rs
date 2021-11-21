#![allow(warnings, unused)]

use char_iter;
use itertools;
use std::collections::HashMap;
use std::io::Read;

use parser;
use xml_sax::*;

use crate::parser::{peek_collect_start_end, peek_nth, remove_nth, ParsingRule, RuleType};

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[derive(Debug)]
pub enum ParsingResult {
    Pass(usize, usize),
    Fail,
    EOF,
}
pub struct RuleResult<'a> {
    rulename: &'a str,
    offset: usize,
    result: ParsingResult,
}

pub fn parse_with_rule3<'a, R: Read>(
    rule_vec: &Vec<ParsingRule>,
    rule: &'a ParsingRule,
    iter: &mut itertools::MultiPeek<char_iter::Chars<R>>,
    //char_vector: &Vec<char>,
    starting_pos: usize,
    // use where char vec is used
    mut offset: usize,
    // eof
    // state_vec: &mut Vec<(usize, usize, bool)>,
) -> RuleResult<'a> {
    let mut offset_plus = 0;

    // println!("{:?}", rule);
    match rule.rule_type {
        RuleType::Chars => {
            /*if starting_pos >= char_vector.len() {
                return ( ParsingResult::EOF);
            }*/
            //let c = char_vector[starting_pos ];
            let c = peek_nth(iter, starting_pos - offset);

            for range in &rule.expected_char_ranges {
                if range.0 <= c && c <= range.1 {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Pass(starting_pos, starting_pos + 1),
                        rulename: &rule.rule_name,
                    };
                }
            }
            for check_char in &rule.expected_chars {
                if *check_char == c {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Pass(starting_pos, starting_pos + 1),
                        rulename: &rule.rule_name,
                    };
                }
            }

            RuleResult {
                offset: offset_plus,
                result: ParsingResult::Fail,
                rulename: &rule.rule_name,
            }
        }

        RuleType::CharsNot => {
            /*if starting_pos >= char_vector.len() {
                return ( ParsingResult::EOF);
            }*/
            //let c = char_vector[starting_pos ];
            let c = peek_nth(iter, starting_pos - offset);
            for range in &rule.expected_char_ranges {
                if range.0 <= c && c <= range.1 {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Fail,
                        rulename: &rule.rule_name,
                    };
                }
            }
            for check_char in &rule.expected_chars {
                if *check_char == c {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Fail,
                        rulename: &rule.rule_name,
                    };
                }
            }

            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Pass(starting_pos, starting_pos + 1),
                rulename: &rule.rule_name,
            };
        }

        RuleType::CharSequence => {
            let mut new_starting_pos = starting_pos;
            for check_char in &rule.expected_chars {
                /*if new_starting_pos  >= char_vector.len() {
                    return ( ParsingResult::EOF);
                }*/
                //let c = char_vector[new_starting_pos ];
                let c = peek_nth(iter, new_starting_pos - offset);
                if *check_char == c {
                    new_starting_pos += 1;
                } else {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Fail,
                        rulename: &rule.rule_name,
                    };
                }
            }

            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Pass(starting_pos, new_starting_pos),
                rulename: &rule.rule_name,
            };
        }
        RuleType::ZeroOrMore => {
            let new_rule = &rule_vec[rule.children[0]];
            let mut fail = false;
            let mut new_starting_pos = starting_pos;

            while !fail {
                let result = parse_with_rule3(
                    rule_vec,
                    new_rule,
                    iter,
                    //&char_vector,
                    new_starting_pos,
                    offset,
                );
                //if(result.0;

                match result.result {
                    ParsingResult::Fail => fail = true,
                    ParsingResult::Pass(_, e_pos) => {
                        offset_plus += result.offset;
                        offset += result.offset;
                        new_starting_pos = e_pos
                    }
                    ParsingResult::EOF => {
                        return RuleResult {
                            offset: offset_plus,
                            result: ParsingResult::EOF,
                            rulename: &rule.rule_name,
                        };
                    }
                }
            }

            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Pass(starting_pos, new_starting_pos),
                rulename: &rule.rule_name,
            };
        }

        RuleType::Sequence => {
            let mut child_no: usize;
            let mut new_starting_pos = starting_pos;

            // match resume_state_vec.pop() {
            //     Some((no, resume_starting_pos, _)) => {
            //         child_no = no;
            //         new_starting_pos = resume_starting_pos;
            //     }
            //     None => child_no = 0,
            // }

            child_no = 0;

            for (no, rule_id) in rule.children.iter().skip(child_no).enumerate() {
                let mut child_no2 = no + child_no;
                // sequence always goes forward or fails, so no need for backtracking
                // if rule.rule_name =="STag" || rule.rule_name == "ETag"{
                //     state_vec.push((child_no2,new_starting_pos,false));
                // }else{
                //     state_vec.push((child_no2,new_starting_pos,true));
                // }

                // if rule.is_chunkable {
                //     state_vec.push((child_no2, new_starting_pos, true));
                // } else {
                //     state_vec.push((child_no2, new_starting_pos, false));
                // }

                let new_rule = &rule_vec[*rule_id];

                let result = parse_with_rule3(
                    rule_vec,
                    new_rule,
                    iter,
                    //&char_vector,
                    new_starting_pos,
                    offset,
                );

                match result.result {
                    ParsingResult::Fail => {
                        return RuleResult {
                            offset: offset_plus,
                            result: ParsingResult::Fail,
                            rulename: &rule.rule_name,
                        };
                    }
                    ParsingResult::Pass(_, e_pos) => {
                        offset_plus += result.offset;
                        offset += result.offset;
                        new_starting_pos = e_pos
                    }
                    ParsingResult::EOF => {
                        // dont call state_vec.pop();
                        return RuleResult {
                            offset: 0,
                            result: ParsingResult::EOF,
                            rulename: &rule.rule_name,
                        };
                    }
                }
            }

            //println!("******{:?},{:?},{:?},{:?}",rule.rule_name.clone(),starting_pos,offset,new_starting_pos );
            //deleting unnecessary chars causes this:

            if rule.rule_name == "STag"
                || rule.rule_name == "EmptyElemTag"
                || rule.rule_name == "ETag"
            {
                offset_plus = new_starting_pos - offset;
                //println!("offset_plus: {:?}",offset_plus );
                remove_nth(iter, offset_plus);
            }

            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Pass(starting_pos, new_starting_pos),
                rulename: &rule.rule_name,
            };
        }

        RuleType::Or => {
            let mut child_no: usize;
            let mut rule_starting_pos: usize = starting_pos;
            // match resume_state_vec.pop() {
            //     Some((no, state_starting_pos, _)) => {
            //         child_no = no;
            //         rule_starting_pos = state_starting_pos;
            //     }
            //     None => child_no = 0,
            // }
            child_no = 0;

            for (no, rule_id) in rule.children.iter().skip(child_no).enumerate() {
                let mut child_no2 = no + child_no;
                let mut no_backtrack_required = false;
                if child_no2 == rule.children.len() - 1 {
                    no_backtrack_required = true;
                }
                // state_vec.push((child_no2, rule_starting_pos, no_backtrack_required));
                let new_rule = &rule_vec[*rule_id];
                let result = parse_with_rule3(
                    rule_vec,
                    new_rule,
                    iter,
                    //&char_vector,
                    rule_starting_pos,
                    offset,
                );

                match result.result {
                    ParsingResult::Pass(s_pos, e_pos) => {
                        offset_plus += result.offset;
                        offset += result.offset;

                        return RuleResult {
                            offset: offset_plus,
                            result: result.result,
                            rulename: &rule.rule_name,
                        };
                    }
                    ParsingResult::Fail => (),
                    ParsingResult::EOF => {
                        // dont call state_vec.pop();
                        // state_vec.pop();
                        return RuleResult {
                            offset: offset_plus,
                            result: ParsingResult::EOF,
                            rulename: &rule.rule_name,
                        };
                    }
                }
            }
            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Fail,
                rulename: &rule.rule_name,
            };
        }

        RuleType::WithException => {
            // first test should pass
            // second test should fail
            // i cant think of any case that needs pass or fail location for second test
            let first_rule = &rule_vec[rule.children[0]];
            let second_rule = &rule_vec[rule.children[1]];
            let result = parse_with_rule3(
                rule_vec,
                first_rule,
                iter,
                //&char_vector,
                starting_pos,
                offset,
            );
            match result.result {
                ParsingResult::Pass(s_pos, e_pos) => {
                    let result2 = parse_with_rule3(
                        rule_vec,
                        second_rule,
                        iter,
                        //&char_vector,
                        starting_pos,
                        offset,
                    );

                    match result2.result {
                        ParsingResult::Pass(_, _) => {
                            return RuleResult {
                                offset: 0,
                                result: ParsingResult::Fail,
                                rulename: &rule.rule_name,
                            }
                        }
                        ParsingResult::EOF | ParsingResult::Fail => {
                            return RuleResult {
                                offset: offset_plus,
                                result: result2.result,
                                rulename: &rule.rule_name,
                            };
                        }
                    }
                }
                ParsingResult::Fail => {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Fail,
                        rulename: &rule.rule_name,
                    }
                }
                ParsingResult::EOF => {
                    // dont call state_vec.pop();
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::EOF,
                        rulename: &rule.rule_name,
                    };
                }
            }
        }
        RuleType::Optional => {
            let result = parse_with_rule3(
                rule_vec,
                &rule_vec[rule.children[0]],
                iter,
                //&char_vector,
                starting_pos,
                offset,
            );

            offset_plus += result.offset;
            offset += result.offset;

            match result.result {
                ParsingResult::Fail => {
                    offset_plus = 0;
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::Pass(starting_pos, starting_pos),
                        rulename: &rule.rule_name,
                    };
                }
                ParsingResult::Pass(s_pos, e_pos) => {
                    return RuleResult {
                        offset: offset_plus,
                        result: result.result,
                        rulename: &rule.rule_name,
                    };
                }
                ParsingResult::EOF => {
                    return RuleResult {
                        offset: offset_plus,
                        result: ParsingResult::EOF,
                        rulename: &rule.rule_name,
                    };
                }
            }
        }
        // unreachable
        _ => {
            println!("UNIMPLEMENTED PARSER FOR TYPE!");

            return RuleResult {
                offset: offset_plus,
                result: ParsingResult::Fail,
                rulename: &rule.rule_name,
            };
        }
    }
}

// fn parse(read: R) -> Event {
//     Event::StartDocument
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_xml1() {
        let data = "<aaa/>".as_bytes();
        let parser_rules = parser::prepare_rules();
        let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("EmptyElemTag").unwrap()];

        // let mut state_vec = Vec::new();
        let mut citer = itertools::multipeek(char_iter::chars(data));

        let res = parse_with_rule3(
            &parser_rules.rule_vec,
            rule,
            &mut citer,
            0,
            0,
            // &mut resume_state_vec,
            // &mut state_vec,
        );
        println!("{:?}", res.rulename);
        println!("{:?}", res.result);
        // println!("{:?}", state_vec);

        // parse equals "'<!--'"
    }

    #[test]
    fn test_xml2() {
        let data = "<root><A/><B/><C/></root>".as_bytes();
        let parser_rules = parser::prepare_rules();
        let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("document").unwrap()];

        // let mut state_vec = Vec::new();
        let mut citer = itertools::multipeek(char_iter::chars(data));

        let res = parse_with_rule3(
            &parser_rules.rule_vec,
            rule,
            &mut citer,
            0,
            0,
            // &mut resume_state_vec,
            // &mut state_vec,
        );
        println!("{:?}", res.rulename);
        println!("{:?}", res.result);
        // println!("{:?}", state_vec);

        // parse equals "'<!--'"
    }
}
