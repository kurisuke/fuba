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

use clap;

const DEF_SIM_ITER: u32 = 10000;

pub enum CmdlineConfig {
    Sim {
        config_file: String,
        iter: u32,
        num_threads: u32,
        match_rounds: Vec<String>,
    },
    File {
        config_file: String,
        ignore_seed: bool,
    },
    None,
}

pub fn get_config() -> CmdlineConfig {
    let a = clap::App::new("fuba")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Simulate football (soccer) matches and tournaments")
        .subcommand(
            clap::SubCommand::with_name("sim")
                .about("Monte carlo simulation of competition setup")
                .arg(
                    clap::Arg::with_name("config-file")
                        .help("The configuration file to use")
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name("iter")
                        .short("i")
                        .long("iter")
                        .value_name("N")
                        .help("Number of iterations")
                        .takes_value(true),
                )
                .arg(
                    clap::Arg::with_name("num-threads")
                        .short("n")
                        .long("num-threads")
                        .value_name("N")
                        .help("Number of threads to use")
                        .takes_value(true),
                )
                .arg(
                    clap::Arg::with_name("match-rounds")
                        .short("r")
                        .long("match-rounds")
                        .value_name("MATCH")
                        .help("Match round IDs for display, comma separated")
                        .takes_value(true),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("file")
                .about("Run competition from file")
                .arg(
                    clap::Arg::with_name("config-file")
                        .help("The configuration file to use")
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name("ignore-seed")
                        .short("S")
                        .long("ignore-seed")
                        .help("Ignore seed from configuration file"),
                ),
        );

    let m = a.get_matches();
    match m.subcommand() {
        ("sim", Some(sub_m)) => {
            let config_file = sub_m.value_of("config-file").unwrap_or("");
            let iter: u32 = match sub_m.value_of("iter") {
                Some(x) => x.parse().unwrap_or(DEF_SIM_ITER),
                None => DEF_SIM_ITER,
            };
            let num_threads = match sub_m.value_of("num-threads") {
                Some(x) => x.parse().unwrap_or(::num_cpus::get() as u32),
                None => ::num_cpus::get() as u32,
            };
            let match_rounds = match sub_m.value_of("match-rounds") {
                Some(x) => x.split(",").map(|x| String::from(x)).collect(),
                None => vec![],
            };
            CmdlineConfig::Sim {
                config_file: String::from(config_file),
                iter,
                num_threads,
                match_rounds,
            }
        }
        ("file", Some(sub_m)) => {
            let config_file = sub_m.value_of("config-file").unwrap_or("");
            let ignore_seed = sub_m.is_present("ignore-seed");
            CmdlineConfig::File {
                config_file: String::from(config_file),
                ignore_seed,
            }
        }
        (_, _) => {
            eprintln!("Specify subcommand: file, sim");
            CmdlineConfig::None
        }
    }
}
