use std::{collections::{HashMap, HashSet}, fmt::{Write as _}, fs::File, io::{Read, Write as _}, path::Path};

use fasthash::spooky::Hash128;

const WILDCARD:         &'static str = "*";
const NO_STATE_CHANGE:  &'static str = ".";
const NO_WRITE:         &'static str = ".";
const BLANK:            &'static str = "_";
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
            if self.rules.final_states.contains(&self.state) {
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
    pub final_states: HashSet<usize, Hash128>,
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

        let machine_name = path.as_ref().to_str().unwrap()
            .split("/").last().unwrap()
            .trim_end_matches(".tur").to_owned();

        let (
            state_names, 
            sym_names, 
            final_state_names, 
            initial_state_name, 
            table,
        ) = extract_tokens(&path);

        // State parsing starts here
        let mut state2idx = HashMap::with_hasher(Hash128);

        // The HALT state is always defined with index 0
        state2idx.insert(format!("{}::HALT", machine_name), 0);

        for (i, state) in state_names.iter().enumerate() {
            state2idx.insert(format!("{}::{}", machine_name, state), i + 1);
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

        // Defining initial_state and final_states
        let initial_state = *state2idx.get(&format!("{}::{}", machine_name, initial_state_name))
            .expect(&format!("Initial state '{}' was not defined \
            with a 'states' command.", initial_state_name));

        let mut final_states = HashSet::with_hasher(Hash128);
        for state_name in &final_state_names {
            let hash = format!("{}::{}", machine_name, state_name);
            final_states.insert(*state2idx.get(&hash)
                .expect(&format!("Final state '{}' was not defined \
                with a 'states' command", state_name)));
        }

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
                    let statename1 = format!("{}::{}", machine_name, s1);
                    let statename2 = format!("{}::{}", machine_name, s2);
                    first = *state2idx.get(&statename1)
                        .expect(&format!("State '{}' has not been defined.", s1));
                    last = *state2idx.get(&statename2)
                        .expect(&format!("State '{}' has not been defined.", s2));
                    if first > last {
                        panic!("State '{}' was defined before state '{}', \
                            did you put them in the wrong order?", s2, s1);
                    }
                } else if tokens[0] == WILDCARD {
                    first = 0;
                    last = state2idx.len() - 1;
                } else {
                    let statename = format!("{}::{}", machine_name, s);
                    first = *state2idx.get(&statename)
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
                Some(*state2idx.get(&format!("{}::{}", machine_name, tokens[2])).unwrap())
            };
            let write =  if tokens[3].ends_with(NO_WRITE) {
                None
            } else {
                Some(*sym2idx.get(tokens[3]).unwrap())
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
            final_states,
            idx2sym,
            sym2idx,
            transition_map,
        }
    }
}

/// Makes a new machine at `outpath`, which takes the output from the 
/// machine at `filepath1` as input to the machine at `filepath2`.
pub fn chain<P, T, L>(filepath1: P, filepath2: T, outpath: L) 
    where P: AsRef<Path>, T: AsRef<Path>, L: AsRef<Path>
{
    
    let m2_prefix = filepath2.as_ref().to_str().unwrap()
        .split("/").last().unwrap()
        .trim_end_matches(".tur").to_owned();

    let (
        mut states, 
        mut syms, 
        _, 
        m1_init, 
        m1_table,
    ) = extract_tokens(&filepath1);

    let (
        m2_states, 
        m2_syms, 
        m2_final, 
        mut m2_init, 
        mut m2_table,
    ) = extract_tokens(&filepath2);

    let mut output = File::create(&outpath).unwrap();

    while states.contains(&m2_init) {
        m2_init = format!("{}::{}", m2_prefix, m2_init);
    }

    // Add all m2 states to m1. If there is a name clash prepend 
    // the name of m2 to the state name until there is no clash.
    for state in m2_states {
        let mut s = state.clone();
        while states.contains(&s) {
            s = format!("{}::{}", m2_prefix, s);
        }
        // Replace all instances of the state in the table 
        // to the new state name.
        m2_table = m2_table.replace(&state, &s);
        states.push(s);
    }

    // Change all HALTs in m1 to m2, so that m2 starts when m1 is done.
    let m1_table = m1_table.replace("HALT", &m2_init);

    write!(output, "states {}\n", states.join(" ")).unwrap();

    // This would probably be better as a Set, but then ordering may be messed up
    for sym in m2_syms {
        if !syms.contains(&sym) {
            syms.push(sym);
        }
    }

    write!(output, "syms {}\n", syms.join(" ")).unwrap();
    write!(output, "initstate {}\n", m1_init).unwrap();

    let mut m2_final: Vec<String> = m2_final.into_iter().collect();
    for f in &mut m2_final {
        if f.ends_with("HALT") {
            *f = "HALT".to_owned();
        }
    }

    write!(output, "finalstates {}\n", m2_final.join(" ")).unwrap();
    write!(output, "table\n{}\n{}", m1_table, m2_table).unwrap();

    println!("Wrote machine to '{}'.", outpath.as_ref().to_str().unwrap());
}

pub fn branch<P, T, L>(
    entry: P, 
    syms: &[String], 
    machines: &[T],
    outpath: L
) where P: AsRef<Path>, T: AsRef<Path>, L: AsRef<Path> {

    let (
        mut out_states, 
        mut out_syms, 
        _, 
        init_state, 
        mut out_table,
    ) = extract_tokens(&entry);

    out_table = out_table.replace("HALT", "BRANCH");
    out_states.push("BRANCH".to_owned());

    let mut init_states = Vec::with_capacity(syms.len());

    for path in machines {
        let prefix = path.as_ref().to_str().unwrap()
            .split("/").last().unwrap()
            .trim_end_matches(".tur").to_owned();
        
        let machine = extract_tokens(path);
        let (table, init_state) = merge_tokens(&mut out_states, &mut out_syms, machine, prefix);

        init_states.push(init_state);
        out_table.push_str(&format!("\n{}", table));
    }

    for (sym, init_state) in syms.iter().zip(&init_states) {
        out_table.push_str(&format!("BRANCH {} {} . R\n", sym, init_state));
    }

    write_to_file(outpath, out_states, out_syms, init_state, out_table);
}

pub fn loop_while<P: AsRef<Path>, L: AsRef<Path>>(
    entry: P, 
    loop_syms: &[String],
    outpath: L
) {
    let (
        mut states, 
        syms, 
        _, 
        init_state, 
        table,
    ) = extract_tokens(&entry);

    states.push("CHECK".to_owned());

    let mut table = table.replace("HALT", "CHECK");
    table.push_str(&format!("\nCHECK {} {} . N", loop_syms.join(","), init_state));
    table.push_str(&format!("\nCHECK * HALT . N"));

    write_to_file(outpath, states, syms, init_state, table)
}

fn merge_tokens<T>(
    states: &mut Vec<String>, syms: &mut Vec<String>,
    m2: (Vec<String>, Vec<String>, T, String, String), 
    prefix: String,
) -> (String, String) {
    let mut table = m2.4.to_owned();
    let mut init_state = m2.3.to_owned();
    for state in m2.0 {
        let mut s = state.clone();
        while states.contains(&s) {
            if s == init_state {
                init_state = format!("{}::{}", prefix, init_state);
            }
            s = format!("{}::{}", prefix, s);
        }
        // Replace all instances of the state in the table 
        // to the new state name.
        table = table.replace(&format!("{} ", state), &format!("{} ", s));
        table = table.replace(&format!("{},", state), &format!("{},", s));
        table = table.replace(&format!("{}-", state), &format!("{}-", s));
        states.push(s);
    }

    for sym in m2.1 {
        if !syms.contains(&sym) {
            syms.push(sym);
        }
    }
    (table, init_state)
}

fn write_to_file<P: AsRef<Path>>(
    path: P,
    states: Vec<String>,
    syms: Vec<String>,
    init_state: String,
    table: String,
) {
    let mut out = File::create(&path)
        .expect("Failed when creating file for machine.");

    write!(out, "states {}\n", states.join(" ")).unwrap();
    write!(out, "syms {}\n", syms.join(" ")).unwrap();
    write!(out, "initstate {}\n", init_state).unwrap();
    write!(out, "finalstates HALT\n").unwrap();
    write!(out, "table\n{}", table).unwrap();

    println!("Wrote machine to '{}'.", path.as_ref().to_str().unwrap());
}

/// Returns tokens in the order:
/// - states
/// - symbols
/// - final states
/// - initial state
/// - table
fn extract_tokens<P: AsRef<Path>>(path: P) -> (
    Vec<String>,
    Vec<String>,
    HashSet<String, Hash128>,
    String,
    String,
) {
    let mut machine = String::new();
    File::open(&path)
        .expect(&format!("Failed when opening file at '{}'.", 
        path.as_ref().to_str().expect("Failed to parse path as string."))
        ).read_to_string(&mut machine)
        .expect("Failed when reading file to string.");

    let mut machine = machine.lines();

    let mut states = Vec::new();
    let mut syms = Vec::new();
    let mut initial_state = String::new();
    let mut final_states = HashSet::with_hasher(Hash128);

    for line in &mut machine {
        let line = if let Some(split) = line.split_once("#") {
            split.0
        } else {
            line
        };
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("states") => {
                for token in tokens {
                    states.push(token.to_owned())
                }
            },
            Some("syms") => {
                for token in tokens {
                    syms.push(token.to_owned());
                }
            },
            Some("initstate") => {
                initial_state.push_str(&tokens.next().unwrap());
            }
            Some("finalstates") => {
                for token in tokens {
                    final_states.insert(token.to_owned());
                }
            },
            Some("table") => {
                break;
            },
            _ => {},
        }
    }

    let mut table = String::new();

    for line in machine {
        let line = if let Some(split) = line.split_once("#") {
            split.0
        } else {
            line
        };
        table.push_str(&format!("{}\n", line));
    }

    (states, syms, final_states, initial_state, table.trim_end().to_owned())
}

// pub fn clean_machine<P: AsRef<Path>>(filepath: P, output_path: P) {
//     todo!();
// }