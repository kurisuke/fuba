extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod config;
mod result;
mod sim;

fn main() {
    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let config = config::read_config("res/tournaments/wc2018.toml").unwrap();

    result::calc(config, &mut sim);
}
