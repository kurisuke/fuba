use rand;
use std::collections::HashMap;
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

pub fn multirun(config_file: String, n: u32, num_threads: u32) {
    let mut round_stats = HashMap::<String, Stats>::new();

    let (tx, rx) = mpsc::channel();

    let mut thread_handles = vec![];

    for _i in 0..num_threads {
        let local_tx = mpsc::Sender::clone(&tx);
        let local_config_file = config_file.clone();
        thread_handles.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
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
        println!("Stats for round: {}\n", k);

        println!("Winners:");
        print_histo(&v.winner, n);
        println!();

        println!("Participants:");
        print_histo(&v.participant, n);
        println!();
    }
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
