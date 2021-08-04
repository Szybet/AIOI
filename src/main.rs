extern crate log;

use std::env;
use log::info;
use log::{debug, error, log_enabled, Level};
use pretty_env_logger::env_logger;

mod cpu;
mod disk;

fn main() {
    env_logger::init();
    let mut cliarg: Vec<String> = env::args().collect();
    cliarg.remove(0);
    let cliarg_iter = cliarg.iter();
    for cliarg in cliarg_iter {
        match &cliarg[..] {
            "cpu" => cpu::Cpuinfo::cpu(cpu::Cpuinfo::new()),
            "disk" => disk::Disksinfo::disk(disk::Disksinfo::new()),
            _ => println!("błędna opcja: {:?}", { &cliarg }),
        }
    }
    disk::Disksinfo::disk(disk::Disksinfo::new());
}
