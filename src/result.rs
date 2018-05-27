use config::Team;
use sim::MatchResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct RoundResult {
    pub id: String,
    pub pairings: Vec<PairingResult>,
    pub stats: Vec<RoundStats>,
}

struct RoundStats {
    pub team: Rc<RefCell<Team>>,
    pub points: u32,
    pub goals_for: u32,
    pub goals_against: u32,
}

struct PairingResult {
    pub teams: (Rc<RefCell<Team>>, Rc<RefCell<Team>>),
    pub match_results: Vec<Match>,
    pub winner: Option<bool>,
}

struct Match {
    pub location: MatchLocation,
    pub extra: bool,
    pub penalties: bool,
    pub result: Option<MatchResult>,
}

enum MatchLocation {
    Home1,
    Home2,
    Neutral,
}

pub fn calc(config: ::config::Config, sim: &mut ::sim::Sim) {
    let mut rounds_finished = HashMap::<String, RoundResult>::new();

    for r in config.round.iter() {
        let round = r.borrow();
        println!("Run round: {}", &(*round.name));

        //   update entrants (from rounds_finished)
        let teams = resolve_entrants(&(*round.entrant), &rounds_finished);
        let format = round.format.clone();

        //   generate matches & stats
        let mut result = RoundResult {
            id: round.id.clone(),
            pairings: gen_pairings(&format.borrow(), &teams),
            stats: gen_stats(&teams),
        };

        // run round
        result.calc(sim);

        // update stats
        result.update_stats();
        result.sort_stats(&format.borrow().mode, &format.borrow().rank_by);

        // print stuff
        result.print_matches();
        if format.borrow().mode == ::config::Mode::RoundRobin {
            result.print_table(true);
        } else if format.borrow().mode == ::config::Mode::Ranking {
            result.print_table(false);
        }

        //   move round to rounds_finished
        rounds_finished.insert(round.id.clone(), result);
    }
}

fn gen_pairings(format: &::config::Format, teams: &[Rc<RefCell<Team>>]) -> Vec<PairingResult> {
    let mut pairings = vec![];

    if format.mode == ::config::Mode::RoundRobin {
        if let Some(ref o) = format.order {
            for p in o {
                let location = match format.neutral {
                    Some(true) => MatchLocation::Neutral,
                    Some(false) => MatchLocation::Home1,
                    None => MatchLocation::Home1,
                };
                let matches = vec![
                    Match {
                        location,
                        extra: false,
                        penalties: false,
                        result: None,
                    },
                ];

                pairings.push(PairingResult {
                    teams: (
                        teams[(p[0] - 1) as usize].clone(),
                        teams[(p[1] - 1) as usize].clone(),
                    ),
                    match_results: matches,
                    winner: None,
                });
            }
        }
    } else if format.mode == ::config::Mode::Playoff {
        // not implemented yet
    } else if format.mode == ::config::Mode::Ranking {
        // no games needed
    }

    pairings
}

fn gen_stats(teams: &[Rc<RefCell<Team>>]) -> Vec<RoundStats> {
    let mut stats = vec![];

    for team in teams {
        stats.push(RoundStats {
            team: team.clone(),
            points: 0,
            goals_for: 0,
            goals_against: 0,
        });
    }

    stats
}

fn resolve_entrants(
    entrants: &[::config::Entrant],
    rounds_finished: &HashMap<String, RoundResult>,
) -> Vec<Rc<RefCell<Team>>> {
    let mut teams = vec![];

    for entrant in entrants {
        match entrant {
            &::config::Entrant::Prev(ref rc_round, rank) => {
                let round_id = &(*rc_round.borrow().id);
                match rounds_finished.get(round_id) {
                    Some(finished_round_result) => {
                        if rank as usize > finished_round_result.stats.len() {
                            panic!("Index too large: {}", rank)
                        }
                        let team_rc = &(finished_round_result.stats[(rank - 1) as usize].team);
                        teams.push(team_rc.clone());
                    }
                    None => {
                        panic!("Round not completed: {}", round_id);
                    }
                }
            }
            &::config::Entrant::Team(ref team_rc) => {
                teams.push(team_rc.clone());
            }
        }
    }

    teams
}

impl RoundResult {
    fn calc(&mut self, sim: &mut ::sim::Sim) -> () {
        for pairing in self.pairings.iter_mut() {
            for m in pairing.match_results.iter_mut() {
                m.result = Some(sim.simulate(::sim::MatchOpts {
                    elo: (pairing.teams.0.borrow().elo, pairing.teams.1.borrow().elo),
                    weight: 60.,
                    extra: m.extra,
                    penalties: m.penalties,
                }));

                // Update ELOs
                let res = m.result.as_ref().unwrap();
                pairing.teams.0.borrow_mut().elo = res.elo.0;
                pairing.teams.1.borrow_mut().elo = res.elo.1;
            }
        }
    }

    fn update_stats(&mut self) -> () {
        for pairing in self.pairings.iter() {
            let pt = &pairing.teams;
            for m in pairing.match_results.iter() {
                let res = m.result.as_ref().unwrap();
                match res.winner() {
                    ::sim::MatchWinner::WinTeam1 => {
                        let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                        mod_team.points += 3;
                    }
                    ::sim::MatchWinner::WinTeam2 => {
                        let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                        mod_team.points += 3;
                    }
                    ::sim::MatchWinner::Draw => {
                        {
                            let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                            mod_team.points += 1;
                        }
                        {
                            let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                            mod_team.points += 1;
                        }
                    }
                };

                {
                    let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                    mod_team.goals_for += res.goals.total().0;
                    mod_team.goals_against += res.goals.total().1;
                }
                {
                    let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                    mod_team.goals_for += res.goals.total().1;
                    mod_team.goals_against += res.goals.total().0;
                }
            }
        }
    }

    fn sort_stats(&mut self, mode: &::config::Mode, rank_by: &Vec<::config::RankBy>) -> () {
        match mode {
            ::config::Mode::RoundRobin => {
                self.stats.sort_by(|a, b| {
                    let a_key = (
                        &a.points,
                        (a.goals_for as i32 - a.goals_against as i32),
                        (&a.goals_for),
                    );
                    let b_key = (
                        &b.points,
                        (b.goals_for as i32 - b.goals_against as i32),
                        (&b.goals_for),
                    );
                    b_key.cmp(&a_key)
                });
            }
            ::config::Mode::Playoff => (),
            ::config::Mode::Ranking => (),
        }
    }

    fn print_matches(&self) -> () {
        for pairing in self.pairings.iter() {
            let pt = &pairing.teams;
            for m in pairing.match_results.iter() {
                let res = m.result.as_ref().unwrap();
                let mut n1 = pt.0.borrow().name.clone();
                let mut n2 = pt.1.borrow().name.clone();
                n1.truncate(32);
                n2.truncate(32);
                println!("{:32} - {:32}   {}", n1, n2, res.result_str());
            }
        }
    }

    fn print_table(&self, with_stats: bool) -> () {
        for (i, stats_line) in self.stats.iter().enumerate() {
            println!("{:2} {}", i + 1, stats_line.table_line_str(with_stats));
        }
    }
}

impl RoundStats {
    pub fn table_line_str(&self, with_stats: bool) -> String {
        let mut n = self.team.borrow().name.clone();
        n.truncate(32);
        let mut s = format!("{:32}", n);
        if with_stats {
            s += &format!(
                " {:2}-{:2} {:2}",
                self.goals_for, self.goals_against, self.points
            );
        }
        s
    }
}

fn get_stat_line<'a>(
    stats: &'a mut Vec<RoundStats>,
    team: &Rc<RefCell<Team>>,
) -> &'a mut RoundStats {
    stats
        .iter_mut()
        .find(|x| Rc::ptr_eq(&x.team, team))
        .unwrap()
}
