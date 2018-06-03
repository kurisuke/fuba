#[macro_use]
extern crate clap;
extern crate num_cpus;
extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod config;
mod flagcheck;
mod gen_pairing;
mod multirun;
mod result;
mod sim;

fn main() {
    let a = clap::App::new("fuba")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Simulate football (soccer) matches and tournaments")
        .args_from_usage(
            "-s, --simulate=[N]    'Simulate N runs and print statistics'
             -n, --num-threads=[N] 'Number of Threads (for simulation mode)'
             <CONFIG>           'Configuration file (toml)'",
        )
        .get_matches();

    let config_file = a.value_of("CONFIG").unwrap();

    match a.value_of("simulate") {
        Some(n_str) => {
            let n: u32 = n_str.parse().unwrap();
            let num_threads: u32 = match a.value_of("num-threads") {
                Some(x) => x.parse().unwrap_or(1),
                None => num_cpus::get() as u32,
            };
            println!("Launch multirun mode, {} runs, {} threads", n, num_threads);
            ::multirun::multirun(String::from(config_file), n, num_threads);
        }
        None => {
            let mut rng = rand::thread_rng();
            let mut sim = sim::Sim::new(&mut rng);

            let config = config::read_config(&config_file).unwrap();
            let round_results = result::calc(config, &mut sim);
            for r in round_results {
                r.print();
            }
        }
    }
}
