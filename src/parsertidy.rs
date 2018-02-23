use std::collections::HashMap;
use parser::prepare_rules;
use parser::*;

pub fn get() {
    let mut parser_rules = prepare_rules();
    //let rule = &parser_rules.rule_vec[*parser_rules.rule_registry.get("document").unwrap()];
    remove_optional(&mut parser_rules);
    remove_zeroormore(&mut parser_rules);
    remove_withexception(&mut parser_rules);
    print_rules(&parser_rules, "content".to_owned(), 0);
}

pub fn remove_optional(parser: &mut Parser) {
    let mut rule_id = 0;

    let parser_rule_vec_2 = parser.rule_vec.clone();
    let mut rule_name_registry_queue: HashMap<String, ParsingRule> = HashMap::new();

    for rule in parser.rule_vec.iter_mut() {
        let rule: &mut ParsingRule = rule;
        if rule.rule_type == RuleType::Sequence {
            //check if it contains optional?
            let mut containsoptional = false;
            let mut optionals_str: String = "".to_owned();
            for child_rule_name in &rule.children_names {
                let crule: &ParsingRule =
                    &parser_rule_vec_2[*parser.rule_registry.get(child_rule_name).unwrap()];
                if crule.rule_type == RuleType::Optional {
                    containsoptional = true;
                    optionals_str = optionals_str + "+" + &crule.rule_name.clone();
                }
            }
            if containsoptional {
                let mut child_vec: Vec<Vec<String>> = vec![vec![]];
                for rulenamec in &rule.children_names {
                    let rulec: &ParsingRule =
                        &parser_rule_vec_2[*parser.rule_registry.get(rulenamec).unwrap()];
                    if rulec.rule_type == RuleType::Optional {
                        //optionals child must be used
                        let mandatory_child = rulec.children_names[0].clone();
                        let mut append_child_vec = child_vec.clone();
                        for vecih in append_child_vec.iter_mut() {
                            vecih.push(mandatory_child.clone());
                        }
                        child_vec.append(&mut append_child_vec);
                    } else {
                        for vecih in child_vec.iter_mut() {
                            vecih.push(rulec.rule_name.clone());
                        }
                    }
                }

                //sequence becomes or
                rule.rule_type = RuleType::Or;
                rule.children_names.clear();

                for vecih in child_vec.iter_mut() {
                    //create one sequence rule for each new-children
                    let mut child = ParsingRule::new(
                        "option_child_".to_owned() + &optionals_str + &rule_id.to_string(),
                        RuleType::Sequence,
                    );
                    rule_id += 1;
                    for crule_name in vecih {
                        child.children_names.push(crule_name.clone());
                    }
                    let name = child.rule_name.clone();
                    rule_name_registry_queue.insert(name.clone(), child);
                    rule.children_names.push(name.clone());
                }
            }
        }
    }

    for (rule_name, rule) in rule_name_registry_queue.into_iter() {
        //println!("{:?}", rule_name);
        parser
            .rule_registry
            .insert(rule_name, parser.rule_vec.len());
        parser.rule_vec.push(rule);
    }
}

pub fn remove_zeroormore(parser: &mut Parser) {
    let mut rule_id = 0;

    //some explicit copying & extra attention, cannot read & write to same memory area(ok rust):
    let parser_rule_vec_2 = parser.rule_vec.clone();
    //queue to work on the same objects again
    // generated_zom_name -> ( zom_child, sequence_splitted_children )
    let mut zom_queue: HashMap<String, (String, Vec<String>)> = HashMap::new();

    //newly generated "flattened" zoms
    let mut rule_name_registry_queue: HashMap<String, ParsingRule> = HashMap::new();

    for rule in parser.rule_vec.iter_mut() {
        let rule: &mut ParsingRule = rule;
        if rule.rule_type == RuleType::Sequence {
            //check if it contains zeroormore?
            let mut containszeroormore = false;
            for child_rule_name in &rule.children_names {
                let crule: &ParsingRule =
                    &parser_rule_vec_2[*parser.rule_registry.get(child_rule_name).unwrap()];
                if crule.rule_type == RuleType::ZeroOrMore {
                    containszeroormore = true;
                }
            }
            if containszeroormore {
                let mut index = 0;
                let mut zom_rule_name: String = "".to_owned();
                let mut zom_orig_child_rule_name: String = "".to_owned();

                let mut child_vec: Vec<Vec<String>> = vec![vec![]];
                for (i, rulenamec) in rule.children_names.iter().enumerate() {
                    let rulec: &ParsingRule =
                        &parser_rule_vec_2[*parser.rule_registry.get(rulenamec).unwrap()];
                    if rulec.rule_type == RuleType::ZeroOrMore {
                        index = i;

                        rule_id += 1;
                        zom_orig_child_rule_name = rulec.children_names[0].clone();
                        zom_rule_name =
                            zom_orig_child_rule_name.clone() + "-zomgen" + &rule_id.to_string();
                        break;
                    }
                }

                //CHANGE RULE
                //give children to zom :(
                let mut zom_children: Vec<String> = rule.children_names.split_off(index + 1);
                rule.children_names[index] = zom_rule_name.clone();
                zom_queue.insert(zom_rule_name, (zom_orig_child_rule_name, zom_children));
            }
        }
    }

    for (zom_name, (zom_orig, zom_child)) in zom_queue.clone().into_iter() {
        change_zom_rule_rec(&mut zom_queue, parser, zom_name, zom_orig, zom_child, 0);
    }

    for (zom_name, (zom_orig, zom_child)) in zom_queue.clone().into_iter() {
        let seq1_name = zom_name.clone() + "-seq1";

        let mut zom = ParsingRule::new(zom_name.clone(), RuleType::Or);
        zom.children_names.push(seq1_name.clone());

        //child 1
        let mut seq1 = ParsingRule::new(seq1_name.clone(), RuleType::Sequence);
        //zom original and new zom "OR" rule
        seq1.children_names.push(zom_orig);
        seq1.children_names.push(zom_name.clone());
        rule_name_registry_queue.insert(seq1_name, seq1);

        //dont create empty sequence
        if zom_child.len() > 0 {
            let seq2_name = zom_name.clone() + "-seq2";
            zom.children_names.push(seq2_name.clone());
            //child 2
            let mut seq2 = ParsingRule::new(seq2_name.clone(), RuleType::Sequence);
            //zom original and new zom "OR" rule
            seq2.children_names.append(&mut zom_child.clone());
            rule_name_registry_queue.insert(seq2_name, seq2);
        }

        rule_name_registry_queue.insert(zom_name.clone(), zom);
    }

    for (rule_name, rule) in rule_name_registry_queue.into_iter() {
        //println!("{:?}", rule_name);
        parser
            .rule_registry
            .insert(rule_name, parser.rule_vec.len());
        parser.rule_vec.push(rule);
    }
    /*zom_queue
 change_zeroormore_rule(parser, zom_rule_name, zom_children);
*/
}

pub fn change_zom_rule_rec(
    zom_queue: &mut HashMap<String, (String, Vec<String>)>,
    parser: &mut Parser,
    zom_gen_rule_name: String,
    zom_orig_child_name: String,
    mut zom_children: Vec<String>,
    depth: usize,
) {
    //check if this zom contains another zom... this is the reason this function exists... recursion

    let mut containszeroormore = false;
    for child_rule_name in &zom_children {
        let crule: &ParsingRule =
            &parser.rule_vec[*parser.rule_registry.get(child_rule_name).unwrap()];
        if crule.rule_type == RuleType::ZeroOrMore {
            containszeroormore = true;
        }
    }
    if !containszeroormore {
        zom_queue.insert(zom_gen_rule_name, (zom_orig_child_name, zom_children));
        return;
    }

    let mut index = 0;
    let mut zom_rule_name_new: String = "".to_owned();
    let mut zom_orig_child_rule_name_new: String = "".to_owned();

    for (i, rulenamec) in zom_children.iter().enumerate() {
        let rulec: &ParsingRule = &parser.rule_vec[*parser.rule_registry.get(rulenamec).unwrap()];
        if rulec.rule_type == RuleType::ZeroOrMore {
            index = i;
            zom_rule_name_new = zom_gen_rule_name.clone() + "-" + &depth.to_string();
            zom_orig_child_rule_name_new = rulec.children_names[0].clone();
            break;
        }
    }

    let zom_children_new: Vec<String> = zom_children.split_off(index + 1);
    zom_children[index] = zom_rule_name_new.clone();

    change_zom_rule_rec(
        zom_queue,
        parser,
        zom_rule_name_new,
        zom_orig_child_rule_name_new,
        zom_children_new,
        depth + 1,
    );

    zom_queue.insert(zom_gen_rule_name, (zom_orig_child_name, zom_children));
}

pub fn remove_withexception(parser: &mut Parser) {
    /*for rule in parser.rule_vec.iter_mut() {
        let rule: &mut ParsingRule = rule;
        if rule.rule_type == RuleType::WithException {
            rule.rule_type = RuleType::Or;
            rule.children_names.split_off(1);
        }
    }*/
}

pub fn print_rules(parser: &Parser, rule_name: String, depth: usize) {
    if depth > 8 {
        return;
    }
    let rule: &ParsingRule = &parser.rule_vec[*parser.rule_registry.get(&rule_name).unwrap()];
    for x in 0..depth {
        print!("-");
    }
    println!("name: {:?}, type:{:?}", rule.rule_name, rule.rule_type);
    for cname in &rule.children_names {
        //print!("***{:?}***", cname);
        print_rules(parser, cname.clone(), depth + 1);
    }
}
