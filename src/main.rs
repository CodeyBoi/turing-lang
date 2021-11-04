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
        _ => eprintln!("Error: Not a valid command."),
    }
    
}
