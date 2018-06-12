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
use std::collections::HashMap;

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
        ::cmdline::CmdlineConfig::Match {
            elo,
            extra,
            penalties,
            iter,
        } => {
            let mut rng = XorShiftRng::from_entropy();
            let mut sim = sim::Sim::new(&mut rng);

            if iter == 1 {
                let m = run_single_match(&mut sim, elo, extra, penalties);
                println!("Result: {}", m.result_str());
            } else {
                run_single_match_iter(&mut sim, elo, extra, penalties, iter);
            }
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

fn run_single_match(
    sim: &mut ::sim::Sim,
    elo: (u32, u32),
    extra: bool,
    penalties: bool,
) -> ::sim::MatchResult {
    let mut m = sim.simulate(elo);
    if extra && m.winner() == ::common::MatchWinner::Draw {
        sim.add_extra(&mut m, elo);
    }
    if penalties && m.winner() == ::common::MatchWinner::Draw {
        sim.add_penalties(&mut m);
    }
    m
}

fn run_single_match_iter(
    sim: &mut ::sim::Sim,
    elo: (u32, u32),
    extra: bool,
    penalties: bool,
    iter: u32,
) {
    let mut wdl = (0, 0, 0);
    let mut goals = (0, 0);
    let mut result_map = HashMap::<String, u32>::new();
    for _ in 0..iter {
        let m = run_single_match(sim, elo, extra, penalties);
        match m.winner() {
            common::MatchWinner::WinTeam1 => {
                wdl.0 += 1;
            }
            common::MatchWinner::Draw => {
                wdl.1 += 1;
            }
            common::MatchWinner::WinTeam2 => {
                wdl.2 += 1;
            }
        }
        goals.0 += m.total_after_extra().0;
        goals.1 += m.total_after_extra().1;

        let result_str = m.result_str();
        let result_key = result_str.split_terminator(" ").next().unwrap();
        let rc = result_map.entry(String::from(result_key)).or_insert(0);
        *rc += 1;
    }
    println!("Win Team 1: {}", wdl.0 as f64 / iter as f64);
    println!("Draw      : {}", wdl.1 as f64 / iter as f64);
    println!("Win Team 2: {}", wdl.2 as f64 / iter as f64);

    println!(
        "Avg goals : {:4} - {:4}",
        goals.0 as f64 / iter as f64,
        goals.1 as f64 / iter as f64
    );

    let mut results_v: Vec<_> = result_map.iter().collect();
    results_v.sort_by_key(|x| x.1);
    results_v.reverse();

    println!("\n Most probable results: ");
    let max_el = std::cmp::min(5, results_v.len());
    for i in 0..max_el {
        println!(
            "{} {}",
            results_v[i].0,
            *results_v[i].1 as f64 / iter as f64
        );
    }
}
