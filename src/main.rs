#[macro_use]
extern crate clap;
extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod condition;
mod config;
mod multirun;
mod result;
mod sim;

fn main() {
    let a = clap::App::new("fuba")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Simulate football (soccer) matches and tournaments")
        .args_from_usage(
            "-s, --simulate=[N] 'Simulate N runs and print statistics'
             <CONFIG>           'Configuration file (toml)'",
        )
        .get_matches();

    let config_file = a.value_of("CONFIG").unwrap();

    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let config = config::read_config(&config_file).unwrap();

    match a.value_of("simulate") {
        Some(n_str) => {
            let n: u32 = n_str.parse().unwrap();
            println!("Launch multirun mode, {} runs", n);
            ::multirun::multirun(config, &mut sim, n);
        }
        None => {
            let round_results = result::calc(config, &mut sim);
            for r in round_results {
                r.print();
            }
        }
    }

    let v = vec![
        String::from("a"),
        String::from("b"),
        String::from("e"),
        String::from("f"),
    ];
    let c = "a&b&(e&f)";
    println!("Flags: {:?}", v);
    println!("Condition: {}", condition::print_parse(&c));
    println!("Check result: {:?}", condition::check_condition(&c, &v));
}
