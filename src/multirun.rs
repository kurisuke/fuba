use std::collections::HashMap;

pub fn multirun(config: ::config::Config, sim: &mut ::sim::Sim, n: u32) {
    let mut winners = HashMap::<String, u32>::new();

    for _i in 0..n {
        let c = config.clone();
        let round_results = ::result::calc(c, sim);

        let final_round = round_results.last().unwrap();
        let w = winners
            .entry(final_round.stats[0].team.borrow().name.clone())
            .or_insert(0);
        *w += 1;
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
