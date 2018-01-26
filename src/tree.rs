use parser::ParsingRule;
use indextree::Arena;
use indextree;
use std::ops::IndexMut;
use std::ops::Index;
use parser::Parser;
use parser::RuleType;
//https://rust-leipzig.github.io/architecture/2016/12/20/idiomatic-trees-in-rust/

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateType {
    Init,
    Pass,
    Fail,
    Wait,
}


#[derive(Clone, Debug)]
pub struct PNode {
    pub rulename: String,
    pub ruletype: RuleType,
    //pub children: Vec<Node>,
    pub state: StateType,
    pub current_sequence: usize, //pub data: String
}

impl PNode {
    pub fn new_char(
        parser_rules: &Parser,
        id: indextree::NodeId,
        arena: &mut Arena<PNode>,
        c: char,
    ) -> (StateType, bool) {
        let mut children_names: Vec<String> = Vec::new();
        let mut pnode2: PNode;

        //HANDLE IF ITS CHAR RELATED
        {
            let mut pnode: &mut PNode;
            let mut node = arena.index_mut(id);
            pnode = &mut node.data;
            pnode2 = pnode.clone();
            println!("starting..{:?}", pnode.rulename);
            //let state = &mut pnode.state;
            let rule: &ParsingRule =
                &parser_rules.rule_vec[*parser_rules.rule_registry.get(&pnode.rulename).unwrap()];

            //CHAR RELATED
            match rule.rule_type {
                RuleType::Chars => {
                    for range in &rule.expected_char_ranges {
                        if range.0 <= c && c <= range.1 {
                            //TODO logger pass?
                            println!("PASS");
                            pnode.state = StateType::Pass;
                            return (StateType::Pass, true);
                        }
                    }
                    for check_char in &rule.expected_chars {
                        if *check_char == c {
                            //TODO logger pass?
                            println!("PASS");
                            pnode.state = StateType::Pass;
                            return (StateType::Pass, true);
                        }
                    }
                    pnode.state = StateType::Fail;
                    return (StateType::Fail, false);
                }
                RuleType::CharsNot => {
                    for range in &rule.expected_char_ranges {
                        if range.0 <= c && c <= range.1 {
                            pnode.state = StateType::Fail;
                            return (StateType::Fail, false);
                        }
                    }
                    for check_char in &rule.expected_chars {
                        if *check_char == c {
                            pnode.state = StateType::Fail;
                            return (StateType::Fail, false);
                        }
                    }
                    pnode.state = StateType::Pass;
                    return (StateType::Pass, true);
                }
                RuleType::CharSequence => {
                    let c2 = &rule.expected_chars[pnode.current_sequence];
                    println!("CharSequence: {:?} =? {:?}", c,c2 );
                    if c == *c2 {
                        pnode.current_sequence += 1;
                        if pnode.current_sequence >= rule.expected_chars.len() {
                            pnode.state = StateType::Pass;
                            return (StateType::Pass, true);
                        } else {
                             println!("AHOYY" );
                             pnode.state = StateType::Wait;
                            return (StateType::Wait, true);
                        }
                    } else {
                        pnode.state = StateType::Fail;
                        return (StateType::Fail, true);
                    }
                }
                _ => {
                    // do nothing?
                    // children_names = rule.children_names.clone();

                }
            }
            ()
        }

        //prepare children if it is not char and INIT:
        {
            let mut pnode: &mut PNode;
            let mut node = arena.index_mut(id);
            pnode = &mut node.data;
            pnode2 = pnode.clone();
            println!("children for..{:?}", pnode.rulename);
            //let state = &mut pnode.state;

            //get children rule names
            match pnode.state {
                StateType::Init => {
                    //prepare children nodes
                    let rule: &ParsingRule = &parser_rules.rule_vec
                        [*parser_rules.rule_registry.get(&pnode.rulename).unwrap()];

                    match rule.rule_type {
                        //this dont have children:
                        RuleType::Chars | RuleType::CharsNot | RuleType::CharSequence => {}
                        _ => {
                            children_names = rule.children_names.clone();
                        }
                    }
                }
                _ => {}
            };

            //mystr.push('a');
            //mystr.clear();
            //mystr.push_str("aa"); // =  &mut "zaa".to_owned();
        }

        let rule2: &ParsingRule =
            &parser_rules.rule_vec[*parser_rules.rule_registry.get(&pnode2.rulename).unwrap()];

        //init children?
       
        if pnode2.state == StateType::Init {
            {
                 let rule: &ParsingRule = &parser_rules.rule_vec
                        [*parser_rules.rule_registry.get(&pnode2.rulename).unwrap()];

                    //println!("CN{:?}", children_names);
                    for rulename in children_names.iter() {
                        let rule: &ParsingRule = &parser_rules.rule_vec
                        [*parser_rules.rule_registry.get(rulename).unwrap()];
                        let node = PNode {
                            rulename: rulename.clone(),
                            ruletype: rule.rule_type.clone(),
                            state: StateType::Init,
                            current_sequence: 0,
                        };

                        // Add some new nodes to the arena
                        let cid = arena.new_node(node);
                        id.append(cid, arena);
                    
                }
            }


        }

        //get children
        let mut vec = Vec::new();
            {
                let iter = id.children(&arena);
                for cid in iter {
                    vec.push(cid);
                }
            }

        //new char children rule nodes
        //only first if this is sequence?

        let children_count:usize = vec.len();
        match rule2.rule_type {
            RuleType::Sequence => {
                for (i, cid) in vec.into_iter().enumerate() {
                println!("i:{:?} , current_seq:{:?}", i, pnode2.current_sequence);
                
                {
                let mut pnode = &mut arena.index_mut(id).data;
                if pnode.current_sequence > i {
                    continue;
                }
                }
                let result : (StateType , bool);
                {
                   //TEST current sequence children:
                    result = PNode::new_char(parser_rules, cid, arena, c);
                }
                

                if result.0 == StateType::Fail{
                    {   let mut pnode = &mut arena.index_mut(id).data;
                        pnode.state= StateType::Fail;
                    }
                    return (StateType::Fail,false);
                }
                //pass or wait... sure
                // char moved?
                if result.0 == StateType::Pass && result.1 == true{
                    //this char is used.. children passed... if this is the last one congratulations!
                    let curseq:usize;
                   {  let mut pnode = &mut arena.index_mut(id).data;
                    pnode.current_sequence += 1;
                    curseq = pnode.current_sequence.clone();
                   }
                   if children_count<= curseq{ 
                       {  let mut pnode = &mut arena.index_mut(id).data;
                            pnode.state= StateType::Pass;
                    
                         }
                       return (StateType::Pass, true) 
                   
                   }else{

                       {   let mut pnode = &mut arena.index_mut(id).data;
                        pnode.state= StateType::Wait;
                        }
                        return (StateType::Wait,true);
                   
                   }
                }
                if result.0 == StateType::Wait && result.1 ==true{
                    //children waits for another char, but this char is used.
                    {   let mut pnode = &mut arena.index_mut(id).data;
                        pnode.state= StateType::Wait;
                    }
                     return (StateType::Wait,true);
                }
                if result.0 == StateType::Pass && result.1 ==false{
                    //go on loop you have 1 char to spend...
                     let mut pnode = &mut arena.index_mut(id).data;
                    pnode.current_sequence += 1;

                    //if this is the last one dont go on loop
                     if children_count<= pnode.current_sequence{ 
                         pnode.state= StateType::Pass;
                       return (StateType::Pass, false) 
                       }
                }
                } //end for loop
                
            },
            RuleType::Optional => {
                //not used!
                let cid = vec[0];
                 let mut result : (StateType , bool);
                {
                    result = PNode::new_char(parser_rules, cid, arena, c);
                }
                if result.0 == StateType::Fail{
                    result.0 = StateType::Pass;
                }
                {   let mut pnode = &mut arena.index_mut(id).data;
                        pnode.state= result.0.clone();
                    }
                return (result.0, result.1 );

            }
            RuleType::Or => {

                let mut shouldwait = false;
                let children_count = vec.len();
                for (i, cid) in vec.into_iter().enumerate() {
                    let child_state: StateType; 
                    {   let mut pnode :&mut PNode = &mut arena.index_mut(cid).data;
                        child_state = pnode.state.clone();
                    }
                    if(child_state==StateType::Fail){
                        continue;
                    }

                    let result : (StateType , bool);
                    {
                        result = PNode::new_char(parser_rules, cid, arena, c);
                    }
                    //there is no optional and OR rule type, thus if it pass it moves char
                    if result.0 == StateType::Pass{
                        {   let mut pnode = &mut arena.index_mut(id).data;
                            pnode.state= StateType::Pass;
                        }
                        return (StateType::Pass,true);
                    }
                    if result.0 == StateType::Wait{
                        shouldwait=true;
                         {   let mut pnode = &mut arena.index_mut(id).data;
                            pnode.state= StateType::Wait;
                        }
                        
                    }

                }
                if shouldwait{
                    return (StateType::Wait,true);
                }else{
                    //if or has one child it should pass
                    if children_count<=1{
                        return  (StateType::Pass,false)
                    }
                    return (StateType::Fail,false);
                }
            }
            
            RuleType::WithException => {}
            RuleType::ZeroOrMore => {}
            _ => {}
        }
        /*for (i, cid) in vec.into_iter().enumerate() {
            println!("i:{:?} , current_seq:{:?}", i, pnode2.current_sequence);
            if rule2.rule_type == RuleType::Sequence {
                if pnode2.current_sequence != i {
                    break;
                }
            }
            //println!("{:?}", cid);

            /* match rule.rule_type {
                RuleType::Sequence => {
                    PNode::new_char(parser_rules, cid, arena, c);
                }
                _ => {}
            }*/
            println!("new_char called");
            PNode::new_char(parser_rules, cid, arena, c);
        }*/


        println!("NO BRANCH?");
         {
            let mut pnode: &mut PNode;
            let mut node = arena.index_mut(id);
            pnode = &mut node.data;
            println!("{:?} typ:{:?} sta:{:?}", pnode.rulename,pnode.ruletype,pnode.state);
        }
        
        println!("//NO BRANCH?");
        (StateType::Fail, false)
    }

    pub fn print(id: indextree::NodeId, arena: &Arena<PNode>, depth: usize) -> () {
        
        let pnode : &PNode = &arena.index(id).data;
        if pnode.state == StateType::Wait ||pnode.state==StateType::Pass{
        for n in id.children(arena) {
            for x in 0..depth {
                print!("-");
            }
            let pnode : &PNode = &arena.index(n).data;
            println!("{:?}|sta:{:?}|seq:{:?}|typ:{:?}", pnode.rulename,pnode.state,pnode.current_sequence,pnode.ruletype); //arena.index(n).data
            PNode::print(n, arena, depth + 1);
        }
        }
    }
}