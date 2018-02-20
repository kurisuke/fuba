extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod config;
mod sim;

fn main() {
    let mut rng = rand::thread_rng();
    let mut sim = sim::Sim::new(&mut rng);

    let config = config::read_config("res/tournaments/wc2018.toml").unwrap();

    run(config);

    let res = sim.simulate(sim::MatchOpts {
        elo: (1900, 1800),
        weight: 60.,
        extra: true,
        penalties: true,
    });

    println!("Result: {}", res.result_str());
    println!("New ELOS:\n {} - {}", res.elo.0, res.elo.1);
}

fn run(config: config::Config) {
    for round in config.round.iter()
    {
        println!("Run round: {}", &(*round.borrow().name));
        //   update entrants (from rounds_finished)
        //   generate matches
        //   run round
        //   move round to rounds_finished
    }
}
