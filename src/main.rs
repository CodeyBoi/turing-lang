use turing::TuringMachine;

pub mod turing;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "compute" => {
            let filepath = &args[2];
            let input = args[3].clone();
            let turing = TuringMachine::from_file(filepath).input(input);
            for tmove in turing {
                println!("{}", tmove);
            }
        }
        "combine" => {
            let (m1, m2, out) = (args[2].as_str(), args[3].as_str(), args[4].as_str());
            turing::combine_machines(m1, m2, out);
        }
        "add" => {
            let n: u16 = args[2].parse().unwrap();
            let name = format!("machines/add{}.tur", n);
            let add_m = "machines/add_one.tur";
            turing::combine_machines(
                add_m, 
                add_m, 
                &name,
            );
            for _ in 0..n - 2 {
                turing::combine_machines(
                    name.as_str(), 
                    add_m, 
                    name.as_str()
                );
            }
        }
        _ => println!("Error: Not a valid command."),
    }
    
}
