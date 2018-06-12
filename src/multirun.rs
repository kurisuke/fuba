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

use rand::prng::XorShiftRng;
use rand::FromEntropy;
use std::collections::HashMap;
use std::process;
use std::sync::mpsc;
use std::thread;

struct Stats {
    pub winner: HashMap<String, u32>,
    pub participant: HashMap<String, u32>,
}

struct RoundResultForStats {
    pub name: String,
    pub winner: String,
    pub participants: Vec<String>,
}

pub fn multirun(config_file: String, n: u32, num_threads: u32, match_rounds: Vec<String>) {
    // test if config file parses
    ::config::read_config(&config_file).unwrap_or_else(|x| {
        eprintln!("{}", x);
        process::exit(1);
    });

    let mut round_stats = HashMap::<String, Stats>::new();

    let (tx, rx) = mpsc::channel();

    let mut thread_handles = vec![];

    for _i in 0..num_threads {
        let local_tx = mpsc::Sender::clone(&tx);
        let local_config_file = config_file.clone();
        thread_handles.push(thread::spawn(move || {
            let mut rng = XorShiftRng::from_entropy();
            let mut sim = ::sim::Sim::new(&mut rng);

            let config = ::config::read_config(&local_config_file).unwrap();

            for _i in 0..(n / num_threads) {
                let c = config.clone();
                let round_results = ::result::calc(c, &mut sim);

                let rr_for_stats: Vec<RoundResultForStats> = round_results
                    .iter()
                    .map(|r| RoundResultForStats {
                        name: r.name.clone(),
                        winner: r.stats[0].team.borrow().name.clone(),
                        participants: r.stats
                            .iter()
                            .map(|x| x.team.borrow().name.clone())
                            .collect(),
                    })
                    .collect();

                local_tx.send(rr_for_stats).unwrap();
            }
        }));
    }
    drop(tx);

    for rr_for_stats in rx {
        for round_result in rr_for_stats.iter() {
            let round_entry = round_stats
                .entry(round_result.name.clone())
                .or_insert(Stats {
                    winner: HashMap::<String, u32>::new(),
                    participant: HashMap::<String, u32>::new(),
                });

            let w = round_entry
                .winner
                .entry(round_result.winner.clone())
                .or_insert(0);
            *w += 1;

            for pname in round_result.participants.iter() {
                let p = round_entry.participant.entry(pname.clone()).or_insert(0);
                *p += 1;
            }
        }
    }

    for h in thread_handles.into_iter() {
        h.join().unwrap();
    }

    for (k, v) in round_stats {
        if match_rounds.is_empty() {
            print_round_result(&k, &v, n);
        } else if let Some(_) = match_rounds.iter().find(|&x| x == &k) {
            print_round_result(&k, &v, n);
        }
    }
}

fn print_round_result(name: &str, r: &Stats, n: u32) {
    println!("Stats for round: {}\n", name);

    println!("Winners:");
    print_histo(&r.winner, n);
    println!();

    println!("Participants:");
    print_histo(&r.participant, n);
    println!();
}

fn print_histo(m: &HashMap<String, u32>, n: u32) {
    let mut winners_v: Vec<_> = m.iter().collect();
    winners_v.sort_by_key(|x| x.1);
    winners_v.reverse();
    for w in winners_v {
        let mut tn = w.0.clone();
        tn.truncate(32);
        println!("{:32} {}", tn, *(w.1) as f64 / n as f64);
    }
}
