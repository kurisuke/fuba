extern crate rand;

mod sim;

use std::env;
use std::process;

fn parse_config(mut args: std::env::Args) -> Result<sim::MatchOpts, &'static str> {
    args.next();

    let elo1 = match args.next() {
        Some(arg) => match arg.parse::<i32>() {
            Ok(x) => x,
            Err(_) => return Err("Error parsing elo of team 1"),
        },
        None => return Err("Missing elo of team 1"),
    };

    let elo2 = match args.next() {
        Some(arg) => match arg.parse::<i32>() {
            Ok(x) => x,
            Err(_) => return Err("Error parsing elo of team 2"),
        },
        None => return Err("Missing elo of team 2"),
    };

    let mut extra = false;
    let mut penalties = false;

    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "e" => {
                extra = true;
            }
            "p" => {
                penalties = true;
            }
            _ => {}
        }
    }

    Ok(sim::MatchOpts {
        elo: (elo1, elo2),
        weight: 60.,
        extra: extra,
        penalties: penalties,
    })
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let res = sim.simulate(parse_config(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    }));

    println!("Result: {}", res.result_str());
    println!("New ELOS:\n {} - {}", res.elo.0, res.elo.1);
}
