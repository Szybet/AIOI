use std::env;

mod cpu;

fn main() {
    let mut cliarg: Vec<String> = env::args().collect();
    cliarg.remove(0);
    let cliarg_iter = cliarg.iter();
    for cliarg in cliarg_iter {
        match &cliarg[..] {
            "cpu" => cpu::Cpuinfo::cpu(cpu::Cpuinfo::new()),
            _ => println!("błędna opcja: {:?}", { &cliarg }),
        }
    }
    cpu::Cpuinfo::cpu(cpu::Cpuinfo::new());
}
