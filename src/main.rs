use turing::TuringMachine;

pub mod turing;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        println!("Please type a command.");
        return;
    }
    match args[1].as_str() {
        "compute" => {
            if args.len() < 4 {
                eprintln!("Too few arguments! Expected usage: compute [MACHINEPATH] [INPUT]");
                return;
            }
            let filepath = &args[2];
            let input = args[3].clone();
            let turing = TuringMachine::from_file(filepath).input(input);
            for tmove in turing {
                println!("{}", tmove);
            }
        }
        "chain" => {
            if args.len() < 5 {
                eprintln!("Expected 3 filepaths, but found {}.", args.len() - 2);
                return;
            }
            let (m1, m2, out) = (args[2].as_str(), args[3].as_str(), args[4].as_str());
            turing::chain(m1, m2, out);
        }
        "branch" => {
            if args.len() < 6 {
                eprintln!("Too few arguments! Expected usage: \
                    branch [ENTRYPOINT] [SYMS] [MACHINE_PATHS] [OUTPATH]");
                return;
            }
            let entry = &args[2];
            let syms: Vec<String> = args[3].split_whitespace()
                .map(|s| s.to_owned()).collect();
            let machines: Vec<String> = args[4].split_whitespace()
                .map(|s| s.to_owned()).collect();
            let outpath = &args[5];
            turing::branch(entry, &syms, &machines, outpath)
        }
        "loop" => {
            if args.len() < 5 {
                eprintln!("Too few arguments! Expected usage: \
                    loop [ENTRYPOINT] [SYMS] [OUTPATH]");
                return;
            }
            let entry = &args[2];
            let loop_syms: Vec<String> = args[3].split_whitespace()
                .map(|s| s.to_owned()).collect();
            let outpath = &args[4];
            turing::loop_while(entry, &loop_syms, outpath);
        }
        _ => eprintln!("Error: '{}' is not a valid command.", args[1]),
    }
    
}
