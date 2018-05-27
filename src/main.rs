#[macro_use]
extern crate clap;
extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod config;
mod result;
mod sim;

fn main() {
    let a = clap::App::new("fuba")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Simulate football (soccer) matches and tournaments")
        .args_from_usage("<CONFIG>       'Configuration file (toml)'")
        .get_matches();

    let config_file = a.value_of("CONFIG").unwrap();

    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let config = config::read_config(&config_file).unwrap();

    result::calc(config, &mut sim);
}
