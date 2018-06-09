/* fuba - Simulate football (soccer) match & tournament results.
 *
 * Copyright (C) 2018  Peter Helbing
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.
 *
 */

#[macro_use]
extern crate clap;
extern crate num_cpus;
extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod common;
mod config;
mod flagcheck;
mod gen_pairing;
mod multirun;
mod result;
mod sim;

use rand::prng::XorShiftRng;
use rand::{FromEntropy, SeedableRng};

fn convert_to_seed(s: &String) -> [u8; 16] {
    let mut v: Vec<u8> = s.bytes().collect();
    v.resize(16, 0);

    let mut seed: [u8; 16] = Default::default();
    seed.copy_from_slice(&v[..]);
    seed
}

fn main() {
    let a = clap::App::new("fuba")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Simulate football (soccer) matches and tournaments")
        .args_from_usage(
            "-s, --simulate=[N]    'Simulate N runs and print statistics'
             -n, --num-threads=[N] 'Number of Threads (for simulation mode)'
             -i, --ignore-seed     'Ignore random seed (for tournament mode)'
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
            let config = config::read_config(&config_file).unwrap();

            let mut rng = if let Some(ref x) = config.seed {
                if a.is_present("ignore-seed") {
                    XorShiftRng::from_entropy()
                } else {
                    XorShiftRng::from_seed(convert_to_seed(x))
                }
            } else {
                XorShiftRng::from_entropy()
            };
            let mut sim = sim::Sim::new(&mut rng);

            let round_results = result::calc(config, &mut sim);
            for r in round_results {
                r.print();
            }
        }
    }
}
