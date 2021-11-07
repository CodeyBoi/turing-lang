use std::{collections::{HashMap, HashSet}, fmt::{Write as _}, fs::File, io::{Read, Write as _}, ops::RangeBounds, path::Path};

use fasthash::spooky::Hash128;
use rand::{Rng, distributions::Uniform};
use regex::{Captures, Regex};

const WILDCARD:         &'static str = "*";
const NO_STATE_CHANGE:  &'static str = ".";
const NO_WRITE:         &'static str = ".";
const BLANK:            &'static str = "_";
const STATE_DELIMITER:  &'static str = r"([,\-\s])";
const BLANK_ID: usize = 0;

pub struct TuringMachine {
    tape: HashMap<i64, usize, Hash128>,
    state: usize,
    pointer: i64,
    rules: TuringRules,
    done: bool,
}

impl TuringMachine {

    pub fn from_file<P: AsRef<Path>>(filepath: P) -> Self {
        let rules = TuringRules::parse_file(filepath);
        let pointer = 0;
        let tape = HashMap::with_hasher(Hash128);
        let state = rules.initial_state;
        Self {
            tape,
            state,
            pointer,
            rules,
            done: false,
        }
    }

    pub fn input(mut self, tape: String) -> Self {
        for (i, token) in tape.split_whitespace().rev().enumerate() {
            self.tape.insert(-(i as i64), *self.rules.sym2idx.get(token)
                .expect("Input string contained symbols which were not defined.")
            );
        }
        self
    }

    pub fn get_string(&self) -> String {
        let width = 80;
        let pointer = (self.pointer + width / 2) as usize + 1;
        let mut string = format!("{:>left$}\n", "v", left=pointer);
        for i in -width/2..width/2 {
            if let Some(idx) = self.tape.get(&i) {
                string.write_str(
                    self.rules.idx2sym.get(idx)
                        .expect("Encountered non-defined symbol \
                            while building string representation.")
                ).unwrap();
            } else {
                string.write_str(BLANK).unwrap();
            }
        }
        string
    }
}

impl Iterator for TuringMachine {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tape = self.get_string();
        let mut done = false;
        let current_symbol = *self.tape.get(&self.pointer)
            .unwrap_or_else(|| &BLANK_ID);
        let next_op = self.rules.get_move(self.state, current_symbol);
        if let Some((new_state, write, dir)) = next_op {
            self.state = new_state;
            self.tape.insert(self.pointer, write);
            match dir {
                Direction::Left  => self.pointer -= 1,
                Direction::Right => self.pointer += 1,
                _ => {},
            }
        } else {
            if self.state == 0 {
                tape.push_str("\n    ACCEPTED");
            } else {
                tape.push_str("\n    REJECTED");
            }
            done = true;
        }
        if !self.done {
            self.done = done;
            Some(tape)
        } else {
            None
        }
    }
}

/// A turing machine halts if there is no instruction for the state and read combination.
/// We make symbols (String) into indices (usize)
struct TuringRules {
    // What state the machine is initially in
    pub initial_state: usize,
    // The set of states which are 'accepted' if the machine halts in them
    pub idx2sym: HashMap<usize, String, Hash128>,
    pub sym2idx: HashMap<String, usize, Hash128>,
    // A hashmap describing what to do when you are in state s with head read r for (s, r).
    // Format for output is (symbol to be written, direction to move the head, state to transition to)
    pub transition_map: HashMap<(usize, usize), (usize, usize, Direction), Hash128>,
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Left,
    Right,
    Stay,
}

impl Direction {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "l" | "<" | "left"  => Self::Left,
            "r" | ">" | "right" => Self::Right,
            "n" | "v" | "stay"  => Self::Stay,
            _ => Self::Stay,
        }
    }
}

impl TuringRules {

    fn get_move(&self, st: usize, sym: usize) -> Option<(usize, usize, Direction)> {
        if let Some(next_move) = self.transition_map.get(&(st, sym)) {
            Some(*next_move)
        } else {
            None
        }
    }

    fn parse_file<P: AsRef<Path>>(path: P) -> Self {

        let (
            state_names, 
            sym_names, 
            initial_state_name, 
            table,
        ) = extract_tokens(&path);

        // State parsing starts here
        let mut state2idx = HashMap::with_hasher(Hash128);

        // The HALT state is always defined with index 0
        state2idx.insert("HALT".to_string(), 0);
        for (i, state) in state_names.iter().enumerate() {
            state2idx.insert(state.to_string(), i + 1);
        }

        // Symbol parsing starts here
        let mut sym2idx = HashMap::with_hasher(Hash128);
        let mut idx2sym = HashMap::with_hasher(Hash128);

        // The blank symbol is always defined with index 0
        sym2idx.insert("_".to_owned(), 0);
        idx2sym.insert(0, "_".to_owned());

        for (i, sym) in sym_names.iter().enumerate() {
            sym2idx.insert(sym.to_owned(), i + 1);
            idx2sym.insert(i + 1, sym.to_owned());
        }

        // Defining initial_state
        let initial_state = *state2idx.get(&initial_state_name)
            .expect(&format!("Initial state '{}' was not defined \
            with a 'states' command.", initial_state_name));

        // Transition table parsing starts here!
        let mut transition_map = HashMap::with_hasher(Hash128);

        for line in table.lines() {
            if line.is_empty() {
                continue;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != 5 {
                panic!("Transition table had an entry with {} tokens, \
                    while parser only allows entries with 5 tokens.", tokens.len());
            }
            
            let mut states = Vec::new();
            for s in tokens[0].split(",") {
                let first;
                let last;
                if let Some((s1, s2)) = s.split_once("-") {
                    first = *state2idx.get(s1)
                        .expect(&format!("State '{}' has not been defined.", s1));
                    last = *state2idx.get(s2)
                        .expect(&format!("State '{}' has not been defined.", s2));
                    if first > last {
                        panic!("State '{}' was defined before state '{}', \
                            did you put them in the wrong order?", s2, s1);
                    }
                } else {
                    first = *state2idx.get(s)
                        .expect(&format!("State '{}' has not been defined", s));
                    last = first;
                }
                for id in first..last + 1 {
                    states.push(id);
                }
            }
            
            let mut syms = Vec::new();
            for s in tokens[1].split(",") {
                let first;
                let last;
                if let Some((sym1, sym2)) = s.split_once("-") {
                    first = *sym2idx.get(sym1)
                        .expect(&format!("Symbol '{}' has not been defined.", sym1));
                    last = *sym2idx.get(sym2)
                        .expect(&format!("Symbol '{}' has not been defined.", sym2));
                    if first > last {
                        panic!("Symbol '{}' was defined before symbol '{}', \
                        did you put them in the wrong order?", sym2, sym1);
                    }
                } else if tokens[1] == WILDCARD {
                    first = 0;
                    last = sym2idx.len() - 1;
                } else {
                    first = *sym2idx.get(s)
                        .expect(&format!("Symbol '{}' has not been defined.", s));
                    last = first;
                }
                for idx in first..last + 1 {
                    syms.push(idx);
                }
            }

            let new_state = if tokens[2].ends_with(NO_STATE_CHANGE) {
                None
            } else {
                Some(*state2idx.get(tokens[2])
                    .expect(&format!("State '{}' has not been defined.", tokens[2])))
            };
            let write =  if tokens[3].ends_with(NO_WRITE) {
                None
            } else {
                Some(*sym2idx.get(tokens[3])
                    .expect(&format!("Symbol '{}' has not been defined.", tokens[3])))
            };
            let d = Direction::from(tokens[4]);
            for state_idx in &states {
                for sym_idx in &syms {
                    if let None = transition_map.get(&(*state_idx, *sym_idx)) {
                        let ns = if let Some(ns) = new_state {
                            ns
                        } else {
                            *state_idx
                        };
                        let w = if let Some(w) = write {
                            w
                        } else {
                            *sym_idx
                        };
                        transition_map.insert((*state_idx, *sym_idx), (ns, w, d));
                    }
                }
            }
        }

        Self {
            initial_state,
            idx2sym,
            sym2idx,
            transition_map,
        }
    }
}

/// Makes a new machine at `outpath`, which takes the output from the 
/// machine at `machine1` as input to the machine at `machine2`.
pub fn chain<P, T, L>(machine1: P, machines: &[T], outpath: L) 
    where P: AsRef<Path>, T: AsRef<Path>, L: AsRef<Path>
{
    let (
        mut states, 
        mut syms, 
        init_state, 
        mut table,
    ) = extract_tokens(&machine1);

    for path in machines {
        let (
            states2, 
            syms2, 
            init_state2, 
            table2,
        ) = extract_tokens(&path);

        for state in states2 {
            states.push(state);
        }

        table = table.replace("HALT", &init_state2);

        for sym in syms2 {
            if !syms.contains(&sym) {
                syms.push(sym);
            }
        }

        table = format!("{}\n{}", table, table2);
    }

    write_to_file(outpath, states, syms, init_state, table);
}

pub fn branch<P, T, L>(
    entry: P, 
    branch_syms: &[String], 
    machines: &[T],
    outpath: L
) where P: AsRef<Path>, T: AsRef<Path>, L: AsRef<Path> {

    let (
        mut states, 
        mut syms, 
        init_state, 
        mut table,
    ) = extract_tokens(&entry);

    table = table.replace("HALT", "BRANCH");
    states.push("BRANCH".to_string());

    let mut init_states = Vec::with_capacity(syms.len());

    for sym in branch_syms {
        if !syms.contains(&sym) {
            syms.push(sym.to_string());
        }
    }

    for path in machines {
        let (
            cur_states,
            cur_syms,
            cur_init_state,
            cur_table,
        ) = extract_tokens(path);

        for state in cur_states {
            states.push(state);
        }

        for sym in cur_syms {
            if !syms.contains(&sym) {
                syms.push(sym);
            }
        }

        init_states.push(cur_init_state);
        table.push_str(&format!("\n{}", cur_table));
    }

    for (sym, machine_init_state) in branch_syms.iter().zip(&init_states) {
        table.push_str(&format!("BRANCH {} {} . R\n", sym, machine_init_state));
    }

    write_to_file(outpath, states, syms, init_state, table);
}

pub fn loop_while<P: AsRef<Path>, L: AsRef<Path>>(
    entry: P, 
    loop_syms: &[String],
    outpath: L
) {
    let (
        mut states, 
        mut syms, 
        init_state, 
        table,
    ) = extract_tokens(&entry);

    states.push("CHECK".to_owned());

    for sym in loop_syms {
        if !syms.contains(&sym) {
            syms.push(sym.to_string());
        }
    }

    let mut table = table.replace("HALT", "CHECK");
    table.push_str(&format!("\nCHECK {} {} . N", loop_syms.join(","), init_state));
    table.push_str(&format!("\nCHECK * HALT . N"));

    write_to_file(outpath, states, syms, init_state, table)
}

fn write_to_file<P: AsRef<Path>>(
    path: P,
    mut states: Vec<String>,
    syms: Vec<String>,
    mut init_state: String,
    mut table: String,
) {
    let mut out = File::create(&path)
        .expect("Failed when creating file for machine.");
    table = table.trim_start().trim_end().to_owned();
    table = format!("\n{}\n", table);
    let re = Regex::new(r"[^\s]*HALT").unwrap();
    table = re.replace_all(&table, "HALT").to_string();

    for (i, state) in states.iter_mut().enumerate() {
        let re = get_state_regex(state);
        // All state names are enumerated as `q{index}`
        let new_state = format!("q{}", i);
        table = re.replace_all(&table, |c: &Captures| {
            format!("{}{}{}", &c[1], new_state, &c[2])
        }).to_string();
        if init_state.eq(state) {
            init_state = new_state.to_owned();
        }
        *state = new_state;
    }

    write!(out, "states {}\n", states.join(" ")).unwrap();
    write!(out, "syms {}\n", syms.join(" ")).unwrap();
    write!(out, "initstate {}\n", init_state).unwrap();
    write!(out, "table{}", table).unwrap();

    println!("Wrote machine to '{}'.", path.as_ref().to_str().unwrap());
}

/// Returns tokens in the order:
/// - states
/// - symbols
/// - initial state
/// - table
fn extract_tokens<P: AsRef<Path>>(path: P) -> (
    Vec<String>,
    Vec<String>,
    String,
    String,
) {
    let mut text = String::new();
    File::open(&path)
        .expect(&format!("Failed when opening file at '{}'.", 
        path.as_ref().to_str().expect("Failed to parse path as string.")))
        .read_to_string(&mut text)
        .expect("Failed when reading file to string.");

    let mut prefix = String::from("");
    let mut rng = rand::thread_rng();
    for _ in 0..16 {
        prefix.push_str(&rng.gen_range(0..10).to_string());
    }
    
    let (commands, mut table) = if let Some((cmds, tbl)) = text.split_once("table") {
        // Panics if there are commands defined after the `table`-command
        if tbl.contains("states")
            || tbl.contains("syms")
            || tbl.contains("initstate")
            || tbl.contains("finalstates") 
        {
            panic!("All states and symbols must be defined before the 'table'-command.");
        }
        (cmds.to_owned(), format!("\n{}\n", tbl.trim_start().trim_end()))
    } else {
        panic!("Transition table was not defined!");
    };

    let mut states = Vec::new();
    let mut syms = Vec::new();
    let mut initial_state = String::new();

    for line in commands.lines() {
        // Filters out code comments
        let line = if let Some((l, _)) = line.split_once("#") {
            l
        } else {
            line
        };
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("states") => {
                // All state names are prepended with the prefix to avoid clashes
                for token in tokens {
                    let new_name = format!("{}::{}", prefix, token);
                    let re = get_state_regex(token);
                    // Here the state names in the table are changed
                    table = re.replace_all(&table, |c: &Captures| {
                        format!("{}{}{}", &c[1], new_name, &c[2])
                    }).to_string();
                    states.push(new_name);
                }
            },
            Some("syms") => {
                // Syms are pushed 'as is'
                for token in tokens {
                    syms.push(token.to_owned());
                }
            },
            Some("initstate") => {
                let state = tokens.next()
                    .expect("Machine must have an initstate.");
                initial_state = format!("{}::{}", prefix, state);
            }
            _ => {},
        }
    }

    (states, syms, initial_state, table.trim_end().trim_start().to_owned())
}

fn get_state_regex(state: &str) -> Regex {
    Regex::new(&format!("{}{}{}", STATE_DELIMITER, state, STATE_DELIMITER)).unwrap()
}
