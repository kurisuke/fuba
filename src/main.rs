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

mod cmdline;
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
    let cmdline_cfg = ::cmdline::get_config();

    match cmdline_cfg {
        ::cmdline::CmdlineConfig::File {
            config_file,
            ignore_seed,
        } => {
            check_config_file(&config_file);
            let config = config::read_config(&config_file).unwrap_or_else(|x| {
                eprintln!("{}", x);
                std::process::exit(1);
            });

            let mut rng = if let Some(ref x) = config.seed {
                if ignore_seed {
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
        ::cmdline::CmdlineConfig::Sim {
            config_file,
            iter,
            num_threads,
            match_rounds,
        } => {
            check_config_file(&config_file);
            println!(
                "Launch monte carlo simulation mode, {} iterations, {} threads",
                iter, num_threads
            );
            ::multirun::multirun(config_file, iter, num_threads, match_rounds);
        }
        ::cmdline::CmdlineConfig::None => {
            std::process::exit(1);
        }
    };
}

fn check_config_file(config_file: &str) {
    if config_file.is_empty() {
        eprintln!("Configuration file name required!");
        std::process::exit(1);
    }

    if !std::path::Path::new(config_file).exists() {
        eprintln!("File does not exist: {}", config_file);
        std::process::exit(1);
    }
}
