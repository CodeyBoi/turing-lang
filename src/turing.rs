use std::{collections::{HashMap, HashSet}, fmt::Write as _, fs::File, io::{Read, Write as _}, io::{BufRead, BufReader}, path::Path};

use fasthash::spooky::Hash128;

const WILDCARD:         &'static str = "*";
const NO_STATE_CHANGE:  &'static str = ".";
const NO_WRITE:         &'static str = ".";
const BLANK:            &'static str = "_";

pub struct TuringMachine {
    tape: HashMap<i64, usize, Hash128>,
    state: usize,
    pointer: i64,
    rules: TuringRules,
    done: bool,
}

impl TuringMachine {

    pub fn from_file<P: AsRef<Path>>(filepath: P) -> Self {
        let rules = TuringRules::parse(filepath);
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
        for (i, token) in tape.split_whitespace().enumerate() {
            self.tape.insert(i as i64, *self.rules.sym2idx.get(token).unwrap());
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
                    self.rules.idx2sym.get(idx).unwrap()
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
            .unwrap_or_else(|| &self.rules.blank_sym);
        let next_op = self.rules.get_move(self.state, current_symbol);
        if let Some((write, dir, new_state)) = next_op {
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

/// A turing machine halts if there are no instructions for the state and read combination.
/// We make symbols (String) into indices (usize)
struct TuringRules {
    pub blank_sym: usize,       // What index corresponds to the blank symbol
    // A hashmap describing what to do when you are in state s with head read r for (s, r).
    // Format for output is (symbol to be written, direction to move the head, state to transition to)
    pub transition_map: HashMap<(usize, usize), (usize, Direction, usize), Hash128>,
    pub initial_state: usize,   // What state the machine is initially in
    // The set of states which are 'accepted' if the machine halts in them
    pub final_states: HashSet<usize, Hash128>,
    pub idx2sym: HashMap<usize, String, Hash128>,
    pub sym2idx: HashMap<String, usize, Hash128>
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
            "l" | "left"  => Self::Left,
            "r" | "right" => Self::Right,
            _ => Self::Stay,
        }
    }
}

impl TuringRules {

    fn get_move(&self, st: usize, sym: usize) -> Option<(usize, Direction, usize)> {
        if let Some(next_move) = self.transition_map.get(&(st, sym)) {
            Some(*next_move)
        } else {
            None
        }
    }

    fn parse<P: AsRef<Path>>(filepath: P) -> Self {

        let machine_name = filepath.as_ref().to_str().unwrap()
            .split("/").last().unwrap()
            .split_once(".tu").unwrap().0
            .to_owned();

        let mut state2idx = HashMap::with_hasher(Hash128);
        state2idx.insert(format!("{}::HALT", machine_name), 0);
        let mut sym2idx = HashMap::with_hasher(Hash128);
        let mut idx2sym = HashMap::with_hasher(Hash128);
        let mut blank_sym = 0;
        let mut initial_state = 0;
        let mut final_states = HashSet::with_hasher(Hash128);

        let mut lines = BufReader::new(File::open(&filepath).unwrap()).lines();

        for line in &mut lines {
            let line = line.unwrap();
            let mut tokens = line.split_whitespace();
            match tokens.next() {
                Some("states") => {
                    for (i, token) in tokens.enumerate() {
                        state2idx.insert(format!("{}::{}", machine_name, token), i + 1);
                    }
                },
                Some("syms") => {
                    for token in tokens {
                        if token.contains("-") {
                            let mut split = token.split("-");
                            let start: usize = split.next().unwrap().parse().unwrap();
                            let end: usize = split.next().unwrap().parse().unwrap();
                            for t in start..end + 1 {
                                let i = sym2idx.len();
                                sym2idx.insert(t.to_string(), i);
                                idx2sym.insert(i, t.to_string());
                            }
                        } else {
                            let i = sym2idx.len();
                            sym2idx.insert(token.to_owned(), i);
                            idx2sym.insert(i, token.to_owned());
                        }
                    }
                },
                Some("blank") => blank_sym = *sym2idx.get(tokens.next().unwrap()).unwrap(),
                Some("initstate") => {
                    let token = format!("{}::{}", machine_name, tokens.next().unwrap());
                    initial_state = *state2idx.get(&token).unwrap();
                }
                Some("finalstates") => {
                    for token in tokens {
                        let token = format!("{}::{}", machine_name, token);
                        println!("{}", token);
                        final_states.insert(*state2idx.get(&token).unwrap());
                    }
                },
                Some("table") => {
                    break;
                },
                _ => {},
            }
        }

        let mut transition_map = HashMap::with_hasher(Hash128);

        for line in lines {
            let line = line.unwrap();
            println!("{}", line);
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let mut states = Vec::new();
            for s in tokens[0].split(",") {
                let first;
                let last;
                if let Some((state1, state2)) = s.split_once("-") {
                    let state1 = format!("{}::{}", machine_name, state1);
                    let state2 = format!("{}::{}", machine_name, state2);
                    first = *state2idx.get(&state1)
                        .expect("more than one '-' in a state range");
                    last = *state2idx.get(&state2)
                        .expect("more than one '-' in a state range");
                } else if tokens[0] == WILDCARD {
                    first = 0;
                    last = state2idx.len() - 1;
                } else {
                    let s = format!("{}::{}", machine_name, s);
                    first = *state2idx.get(&s).unwrap();
                    last = first;
                }
                for idx in first..last + 1 {
                    states.push(idx);
                }
            }
            
            let mut syms = Vec::new();
            for s in tokens[1].split(",") {
                let first;
                let last;
                if let Some((sym1, sym2)) = s.split_once("-") {
                    first = *sym2idx.get(sym1)
                        .expect("more than one '-' in a sym range");
                    last = *sym2idx.get(sym2)
                        .expect("more than one '-' in a sym range");
                } else if tokens[1] == WILDCARD {
                    first = 0;
                    last = sym2idx.len() - 1;
                } else {
                    first = *sym2idx.get(s).unwrap();
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
                        transition_map.insert((*state_idx, *sym_idx), (w, d, ns));
                    }
                }
            }
        }

        Self {
            blank_sym,
            transition_map,
            initial_state,
            final_states,
            idx2sym,
            sym2idx,
        }
    }
}

pub fn combine_machines<P: AsRef<Path>>(filepath1: P, filepath2: P, output_path: P) {

    let mut m1 = String::new();
    File::open(&filepath1)
        .expect("failed when opening file")
        .read_to_string(&mut m1)
        .expect("failed when reading file to string");

    let mut m2 = String::new();
    File::open(&filepath2)
        .expect("failed when opening file")
        .read_to_string(&mut m2)
        .expect("failed when reading file to string");
    let m2_prefix = filepath2.as_ref().to_str().unwrap()
        .split("/").last().unwrap()
        .split_once(".tu").unwrap().0
        .to_owned();

    let (
        mut states, 
        mut syms, 
        _, 
        m1_init, 
        m1_table,
    ) = extract_tokens(&m1);

    let (
        m2_states, 
        m2_syms, 
        mut m2_final, 
        mut m2_init, 
        mut m2_table,
    ) = extract_tokens(&m2);

    let mut output = File::create(&output_path).unwrap();

    while states.contains(&m2_init) {
        m2_init = format!("{}::{}", m2_prefix, m2_init);
    }

    for state in m2_states {
        let mut s = state.clone();
        while states.contains(&s) {
            s = format!("{}::{}", m2_prefix, s);
        }
        m2_table = m2_table.replace(&state, &s);
        states.push(s);
    }

    // We add a RETURN state to make the pointer go to the leftmost position
    // after the execution of machine 1.
    let mut ret_name = "RETURN".to_owned();
    while states.contains(&ret_name) {
        ret_name = format!("{}::{}", m2_prefix, ret_name);
    }
    let m1_table = m1_table.replace("HALT", &ret_name);
    let ret_table = format!("{0} _ {1} . R\n{0} * . . L\n", ret_name, m2_init);
    states.push(ret_name);

    write!(output, "states {}\n", states.join(" ")).unwrap();

    // This would probably be better as a Set, but then ordering may be messed up
    for sym in m2_syms {
        if !syms.contains(&sym) {
            syms.push(sym);
        }
    }
    write!(output, "syms {}\n", syms.join(" ")).unwrap();
    write!(output, "initstate {}\n", m1_init).unwrap();

    for f in &mut m2_final {
        if f.ends_with("HALT") {
            *f = "HALT".to_owned();
        }
    }

    write!(output, "finalstates {}\n", m2_final.join(" ")).unwrap();
    write!(output, "table\n{}{}{}", m1_table, ret_table, m2_table).unwrap();

    println!("Wrote new turing machine to '{}'.", output_path.as_ref().to_str().unwrap());

    /// Returns tokens in the order:
    /// - states
    /// - symbols
    /// - final states
    /// - initial state
    /// - table
    fn extract_tokens(machine: &str) -> (
        Vec<String>,
        Vec<String>,
        Vec<String>,
        String,
        String,
    ) {

        let mut machine = machine.lines();

        let mut states = Vec::new();
        let mut syms = Vec::new();
        let mut initial_state = String::new();
        let mut final_states = Vec::new();

        for line in &mut machine {
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
                        final_states.push(token.to_owned());
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
            table.push_str(&format!("{}\n", line));
        }

        (states, syms, final_states, initial_state, table)
    }

}

pub fn clean_machine<P: AsRef<Path>>(filepath: P, output_path: P) {
    todo!();
}