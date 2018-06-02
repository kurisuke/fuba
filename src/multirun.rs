use rand;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

pub fn multirun(config_file: String, n: u32, num_threads: u32) {
    let mut winners = HashMap::<String, u32>::new();

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

                let final_round = round_results.last().unwrap();
                let winner = final_round.stats[0].team.borrow().name.clone();
                local_tx.send(winner).unwrap();
            }
        }));
    }
    drop(tx);

    for winner in rx {
        let w = winners.entry(winner.clone()).or_insert(0);
        *w += 1;
    }

    for h in thread_handles.into_iter() {
        h.join().unwrap();
    }

    let mut winners_v: Vec<_> = winners.into_iter().collect();
    winners_v.sort_by_key(|x| x.1);
    winners_v.reverse();

    for w in winners_v {
        let mut tn = w.0.clone();
        tn.truncate(32);
        println!("{:32} {}", tn, w.1 as f64 / n as f64);
    }
}
