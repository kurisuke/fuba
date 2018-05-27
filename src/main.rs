extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod config;
mod result;
mod sim;

use std::env;
use std::process;

struct ParsedArgs {
    pub config_file: String,
}

impl ParsedArgs {
    pub fn new(args: &[String]) -> Result<ParsedArgs, &'static str> {
        if args.len() < 2 {
            return Err("missing argument: configuration file (*.toml)");
        }

        let config_file = args[1].clone();

        Ok(ParsedArgs { config_file })
    }
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let args: Vec<String> = env::args().collect();
    let parsed_args = ParsedArgs::new(&args).unwrap_or_else(|err| {
        eprintln!("Error parsing arguments: {}", err);
        process::exit(1);
    });

    let config = config::read_config(&parsed_args.config_file).unwrap();

    result::calc(config, &mut sim);
}
