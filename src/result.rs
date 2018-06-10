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

use common::MatchWinner;
use config::Team;
use sim::MatchResult;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::f64::EPSILON;
use std::rc::Rc;

pub struct RoundResult {
    pub id: String,
    pub name: String,
    pub mode: ::config::Mode,
    pub pairings: Vec<PairingResult>,
    pub stats: Vec<RoundStats>,
    pub flags: HashMap<String, Rc<RefCell<Team>>>,
}

pub struct RoundStats {
    pub team: Rc<RefCell<Team>>,
    pub set_flag: Option<::config::SetFlag>,
    pub points: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub vs_points: HashMap<String, u32>,
    pub vs_goals_for: HashMap<String, u32>,
    pub vs_goals_against: HashMap<String, u32>,
}

impl RoundStats {
    pub fn goal_diff(&self) -> i32 {
        self.goals_for as i32 - self.goals_against as i32
    }

    pub fn goal_quot(&self) -> f64 {
        let div = if self.goals_against == 0 {
            EPSILON
        } else {
            self.goals_against as f64
        };
        self.goals_for as f64 / div
    }

    pub fn vs_goal_diff(&self, id: &str) -> i32 {
        *self.vs_goals_for.get(id).unwrap_or(&0) as i32
            - *self.vs_goals_against.get(id).unwrap_or(&0) as i32
    }

    pub fn vs_goal_quot(&self, id: &str) -> f64 {
        let ga = *self.vs_goals_against.get(id).unwrap_or(&0);
        let div = if ga == 0 {
            EPSILON
        } else {
            self.goals_against as f64
        };
        *self.vs_goals_for.get(id).unwrap_or(&0) as f64 / div
    }
}

pub struct PairingResult {
    pub teams: (Rc<RefCell<Team>>, Rc<RefCell<Team>>),
    pub match_results: Vec<Match>,
    pub winner: Option<MatchWinner>,
    pub needs_winner: bool,
}

impl PairingResult {
    fn total_goals(&self) -> (u32, u32) {
        let mut goals = (0, 0);
        for m in self.match_results.iter() {
            if let Some(ref r) = m.result {
                goals.0 += r.total().0;
                goals.1 += r.total().1;
            }
        }
        goals
    }

    fn away_goals(&self) -> (u32, u32) {
        let mut goals = (0, 0);
        for m in self.match_results.iter() {
            if let Some(ref r) = m.result {
                match m.location {
                    MatchLocation::Home1 => {
                        goals.1 += r.total().1;
                    }
                    MatchLocation::Home2 => {
                        goals.0 += r.total().0;
                    }
                    MatchLocation::Neutral => {}
                }
            }
        }
        goals
    }

    fn last_match_result(&self) -> &::sim::MatchResult {
        self.match_results.last().unwrap().result.as_ref().unwrap()
    }

    fn last_match_result_mut(&mut self) -> &mut ::sim::MatchResult {
        self.match_results
            .last_mut()
            .unwrap()
            .result
            .as_mut()
            .unwrap()
    }
}

pub struct Match {
    pub location: MatchLocation,
    pub result: Option<MatchResult>,
}

struct TeamWithFlags {
    pub team: Rc<RefCell<Team>>,
    pub set_flag: Option<::config::SetFlag>,
    pub prev_stats: Option<PrevStats>,
}

struct PrevStats {
    points: u32,
    goals_for: u32,
    goals_against: u32,
}

#[derive(Clone, Copy)]
pub enum MatchLocation {
    Home1,
    Home2,
    Neutral,
}

pub fn calc(config: ::config::Config, sim: &mut ::sim::Sim) -> Vec<RoundResult> {
    let mut rounds_finished = HashMap::<String, RoundResult>::new();

    for r in config.round.iter() {
        let round = r.borrow();

        //   update entrants (from rounds_finished)
        let format = round.format.clone();
        let twf = resolve_entrants(&(*round.entrant), &rounds_finished, format.borrow().copy);

        //   generate matches & stats
        let mut result = RoundResult {
            id: round.id.clone(),
            name: round.name.clone(),
            mode: format.borrow().mode.clone(),
            pairings: gen_pairings(&format.borrow(), &twf),
            stats: gen_stats(&twf),
            flags: HashMap::<String, Rc<RefCell<Team>>>::new(),
        };

        // run round
        result.calc(sim, format.borrow().weight, &format.borrow().rank_by);

        // update stats
        result.update_stats(
            format.borrow().points_for_win,
            format.borrow().points_for_draw,
        );
        result.sort_stats(&format.borrow().rank_by);
        result.set_flags();

        //   move round to rounds_finished
        rounds_finished.insert(round.id.clone(), result);
    }

    let mut result_vec = vec![];
    for r in config.round.iter() {
        result_vec.push(rounds_finished.remove(&r.borrow().id).unwrap());
    }
    result_vec
}

fn gen_pairings(format: &::config::Format, twf: &[TeamWithFlags]) -> Vec<PairingResult> {
    let mut pairings = vec![];

    let location = match format.neutral {
        true => MatchLocation::Neutral,
        false => MatchLocation::Home1,
    };

    if format.mode == ::config::Mode::RoundRobin {
        let o = match format.order {
            Some(ref o) => o.clone(),
            None => {
                ::gen_pairing::generate_round_robin(twf.len() as u32, format.legs, format.random)
            }
        };

        for p in o {
            let matches = vec![Match {
                location,
                result: None,
            }];

            pairings.push(PairingResult {
                teams: (
                    twf[(p[0] - 1) as usize].team.clone(),
                    twf[(p[1] - 1) as usize].team.clone(),
                ),
                match_results: matches,
                winner: None,
                needs_winner: false,
            });
        }
    } else if format.mode == ::config::Mode::Playoff {
        for i in 0..twf.len() / 2 {
            let matches = vec![Match {
                location,
                result: None,
            }];

            pairings.push(PairingResult {
                teams: (twf[2 * i].team.clone(), twf[2 * i + 1].team.clone()),
                match_results: matches,
                winner: None,
                needs_winner: true,
            });
        }
    } else if format.mode == ::config::Mode::Ranking {
        // no games needed
    }

    pairings
}

fn gen_stats(twf: &[TeamWithFlags]) -> Vec<RoundStats> {
    let mut stats = vec![];

    for t in twf {
        let prev_stats = match t.prev_stats {
            Some(ref prev_stats) => PrevStats {
                points: prev_stats.points,
                goals_for: prev_stats.goals_for,
                goals_against: prev_stats.goals_against,
            },
            None => PrevStats {
                points: 0,
                goals_for: 0,
                goals_against: 0,
            },
        };
        stats.push(RoundStats {
            team: t.team.clone(),
            set_flag: t.set_flag.clone(),
            points: prev_stats.points,
            goals_for: prev_stats.goals_for,
            goals_against: prev_stats.goals_against,
            vs_points: HashMap::new(),
            vs_goals_for: HashMap::new(),
            vs_goals_against: HashMap::new(),
        });
    }

    stats
}

fn resolve_entrants(
    entrants: &[::config::Entrant],
    rounds_finished: &HashMap<String, RoundResult>,
    copy: bool,
) -> Vec<TeamWithFlags> {
    let mut teams = vec![];

    for entrant in entrants {
        match entrant.t {
            ::config::EntrantType::Prev(ref rc_round, rank) => {
                let round_id = &(*rc_round.borrow().id);
                match rounds_finished.get(round_id) {
                    Some(finished_round_result) => {
                        if rank as usize > finished_round_result.stats.len() {
                            panic!("Index too large: {}", rank)
                        }
                        let stat_entry = &(finished_round_result.stats[(rank - 1) as usize]);
                        teams.push(TeamWithFlags {
                            team: stat_entry.team.clone(),
                            set_flag: entrant.set_flag.clone(),
                            prev_stats: if copy {
                                Some(PrevStats {
                                    points: stat_entry.points,
                                    goals_for: stat_entry.goals_for,
                                    goals_against: stat_entry.goals_against,
                                })
                            } else {
                                None
                            },
                        });
                    }
                    None => {
                        panic!("Round not completed: {}", round_id);
                    }
                }
            }
            ::config::EntrantType::PrevFlag(ref rc_round, ref flag_checks) => {
                let round_id = &(*rc_round.borrow().id);
                match rounds_finished.get(round_id) {
                    Some(finished_round_result) => {
                        let flags = finished_round_result.flags.keys().cloned().collect();
                        let mut found = false;
                        for fc in flag_checks.iter() {
                            let checker = &fc.0;
                            let flag_to_get = &fc.1;
                            if checker.check(&flags) == Ok(true) {
                                found = true;
                                let team_rc = finished_round_result.flags.get(flag_to_get).unwrap();
                                teams.push(TeamWithFlags {
                                    team: team_rc.clone(),
                                    set_flag: entrant.set_flag.clone(),
                                    prev_stats: if copy {
                                        let stat_entry = finished_round_result
                                            .stats
                                            .iter()
                                            .find(|x| x.team.borrow().id == team_rc.borrow().id)
                                            .unwrap();
                                        Some(PrevStats {
                                            points: stat_entry.points,
                                            goals_for: stat_entry.goals_for,
                                            goals_against: stat_entry.goals_against,
                                        })
                                    } else {
                                        None
                                    },
                                });
                                break;
                            }
                        }
                        if found == false {
                            panic!(
                                "No matching entry for flags found! {:?}, number checkers: {}",
                                flags,
                                flag_checks.len()
                            );
                        }
                    }
                    None => {
                        panic!("Round not completed: {}", round_id);
                    }
                }
            }
            ::config::EntrantType::Team(ref team_rc) => {
                teams.push(TeamWithFlags {
                    team: team_rc.clone(),
                    set_flag: entrant.set_flag.clone(),
                    prev_stats: None,
                });
            }
        }
    }

    teams
}

impl RoundResult {
    fn calc(&mut self, sim: &mut ::sim::Sim, weight: f64, rank_by: &Vec<::config::RankBy>) -> () {
        for pairing in self.pairings.iter_mut() {
            let mut cur_elos = (pairing.teams.0.borrow().elo, pairing.teams.1.borrow().elo);
            let mut last_elos = cur_elos;

            // run originally planned matches
            for m in pairing.match_results.iter_mut() {
                let r = sim.simulate(cur_elos);

                // Update ELOs
                last_elos = cur_elos;
                cur_elos = ::sim::calculate_elo(cur_elos, r.total_after_extra(), weight);

                m.result = Some(r);
            }

            // resolve if pairing needs a winner
            pairing.winner = Some(get_winner(pairing.total_goals()));

            if pairing.needs_winner {
                // away goals
                if pairing.winner == Some(MatchWinner::Draw) {
                    if let Some(_) = rank_by.iter().find(|&x| x == &::config::RankBy::AwayGoals) {
                        pairing.winner = Some(get_winner(pairing.away_goals()));
                    }
                }

                // extra time
                if pairing.winner == Some(MatchWinner::Draw) {
                    if let Some(_) = rank_by.iter().find(|&x| x == &::config::RankBy::Extra) {
                        {
                            let mut r = pairing.last_match_result_mut();
                            sim.add_extra(r, last_elos);
                        }

                        pairing.winner = Some(get_winner(pairing.total_goals()));
                        if pairing.winner != Some(MatchWinner::Draw) {
                            let r = pairing.last_match_result();
                            cur_elos =
                                ::sim::calculate_elo(last_elos, r.total_after_extra(), weight);
                        }
                    }
                }

                // replay once
                if pairing.winner == Some(MatchWinner::Draw) {
                    if let Some(_) = rank_by.iter().find(|&x| x == &::config::RankBy::ReplayOnce) {
                        pairing.match_results.push(Match {
                            location: MatchLocation::Neutral,
                            result: Some(sim.simulate(cur_elos)),
                        });
                        pairing.winner = Some(get_winner(pairing.total_goals()));

                        // Update ELOs
                        let r = pairing.last_match_result();
                        last_elos = cur_elos;
                        cur_elos = ::sim::calculate_elo(cur_elos, r.total_after_extra(), weight);
                    }

                    // extra time
                    if pairing.winner == Some(MatchWinner::Draw) {
                        if let Some(_) = rank_by.iter().find(|&x| x == &::config::RankBy::Extra) {
                            {
                                let mut r = pairing.last_match_result_mut();
                                sim.add_extra(r, last_elos);
                            }

                            pairing.winner = Some(get_winner(pairing.total_goals()));
                            if pairing.winner != Some(MatchWinner::Draw) {
                                let r = pairing.last_match_result();
                                cur_elos =
                                    ::sim::calculate_elo(last_elos, r.total_after_extra(), weight);
                            }
                        }
                    }
                }

                // penalties
                if pairing.winner == Some(MatchWinner::Draw) {
                    if let Some(_) = rank_by.iter().find(|&x| x == &::config::RankBy::Penalties) {
                        {
                            let mut r = pairing.last_match_result_mut();
                            sim.add_penalties(r);
                        }

                        pairing.winner = Some(get_winner(pairing.total_goals()));
                        if pairing.winner != Some(MatchWinner::Draw) {
                            let r = pairing.last_match_result();
                            cur_elos =
                                ::sim::calculate_elo(last_elos, r.total_after_extra(), weight);
                        }
                    }
                }

                // replay always
                if pairing.winner == Some(MatchWinner::Draw) {
                    if let Some(_) = rank_by
                        .iter()
                        .find(|&x| x == &::config::RankBy::ReplayAlways)
                    {
                        loop {
                            pairing.match_results.push(Match {
                                location: MatchLocation::Neutral,
                                result: Some(sim.simulate(cur_elos)),
                            });
                            pairing.winner = Some(get_winner(pairing.total_goals()));

                            // Update ELOs
                            {
                                let r = pairing.last_match_result();
                                last_elos = cur_elos;
                                cur_elos =
                                    ::sim::calculate_elo(cur_elos, r.total_after_extra(), weight);
                            }

                            if pairing.winner != Some(MatchWinner::Draw) {
                                break;
                            } else {
                                // extra time
                                if let Some(_) =
                                    rank_by.iter().find(|&x| x == &::config::RankBy::Extra)
                                {
                                    {
                                        let mut r = pairing.last_match_result_mut();
                                        sim.add_extra(r, last_elos);
                                    }

                                    pairing.winner = Some(get_winner(pairing.total_goals()));
                                    if pairing.winner != Some(MatchWinner::Draw) {
                                        let r = pairing.last_match_result();
                                        cur_elos = ::sim::calculate_elo(
                                            last_elos,
                                            r.total_after_extra(),
                                            weight,
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                // drawing of lots
                if pairing.winner == Some(MatchWinner::Draw) {
                    if ::rand::random() {
                        pairing.winner = Some(MatchWinner::WinTeam1);
                    } else {
                        pairing.winner = Some(MatchWinner::WinTeam2);
                    }
                }

                assert_ne!(pairing.winner, Some(MatchWinner::Draw));
            }

            pairing.teams.0.borrow_mut().elo = cur_elos.0;
            pairing.teams.1.borrow_mut().elo = cur_elos.1;
        }
    }

    fn update_stats(&mut self, points_for_win: u32, points_for_draw: u32) -> () {
        for pairing in self.pairings.iter() {
            let pt = &pairing.teams;
            let opponent_id = (pt.1.borrow().id.clone(), pt.0.borrow().id.clone());

            match pairing.winner.as_ref().unwrap() {
                MatchWinner::WinTeam1 => {
                    let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                    mod_team.points += points_for_win;
                    let v = mod_team.vs_points.entry(opponent_id.0.clone()).or_insert(0);
                    *v += points_for_win;
                }
                MatchWinner::WinTeam2 => {
                    let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                    mod_team.points += points_for_win;
                    let v = mod_team.vs_points.entry(opponent_id.1.clone()).or_insert(0);
                    *v += points_for_win;
                }
                MatchWinner::Draw => {
                    {
                        let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                        mod_team.points += points_for_draw;
                        let v = mod_team.vs_points.entry(opponent_id.0.clone()).or_insert(0);
                        *v += points_for_draw;
                    }
                    {
                        let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                        mod_team.points += points_for_draw;
                        let v = mod_team.vs_points.entry(opponent_id.1.clone()).or_insert(0);
                        *v += points_for_draw;
                    }
                }
            };

            {
                let mut mod_team = get_stat_line(&mut self.stats, &pt.0);
                mod_team.goals_for += pairing.total_goals().0;
                mod_team.goals_against += pairing.total_goals().1;
                let v = mod_team
                    .vs_goals_for
                    .entry(opponent_id.0.clone())
                    .or_insert(0);
                *v += pairing.total_goals().0;
                let v = mod_team
                    .vs_goals_against
                    .entry(opponent_id.0.clone())
                    .or_insert(0);
                *v += pairing.total_goals().1;
            }
            {
                let mut mod_team = get_stat_line(&mut self.stats, &pt.1);
                mod_team.goals_for += pairing.total_goals().1;
                mod_team.goals_against += pairing.total_goals().0;
                let v = mod_team
                    .vs_goals_for
                    .entry(opponent_id.1.clone())
                    .or_insert(0);
                *v += pairing.total_goals().1;
                let v = mod_team
                    .vs_goals_against
                    .entry(opponent_id.1.clone())
                    .or_insert(0);
                *v += pairing.total_goals().0;
            }
        }
    }

    fn sort_stats(&mut self, rank_by: &Vec<::config::RankBy>) -> () {
        match self.mode {
            ::config::Mode::RoundRobin => {
                let sort_func = |x: &RoundStats, y: &RoundStats| {
                    let opponent_id = (&x.team.borrow().id, &y.team.borrow().id);
                    let mut o = Ordering::Equal;
                    for r in rank_by.iter() {
                        o = match r {
                            ::config::RankBy::Points => y.points.cmp(&x.points),
                            ::config::RankBy::GoalDiff => y.goal_diff().cmp(&x.goal_diff()),
                            ::config::RankBy::GoalQuot => {
                                y.goal_quot().partial_cmp(&x.goal_quot()).unwrap()
                            }
                            ::config::RankBy::Goals => y.goals_for.cmp(&x.goals_for),
                            ::config::RankBy::VsPoints => y.vs_points
                                .get(opponent_id.0)
                                .unwrap_or(&0)
                                .cmp(&x.vs_points.get(opponent_id.1).unwrap_or(&0)),
                            ::config::RankBy::VsGoalDiff => y.vs_goal_diff(opponent_id.0)
                                .cmp(&x.vs_goal_diff(opponent_id.1)),
                            ::config::RankBy::VsGoalQuot => y.vs_goal_quot(opponent_id.0)
                                .partial_cmp(&x.vs_goal_quot(opponent_id.1))
                                .unwrap(),
                            ::config::RankBy::VsGoals => y.vs_goals_for
                                .get(opponent_id.0)
                                .unwrap_or(&0)
                                .cmp(&x.vs_goals_for.get(opponent_id.1).unwrap_or(&0)),
                            _ => o,
                        };
                        if o != Ordering::Equal {
                            break;
                        }
                    }
                    o
                };
                self.stats.sort_by(|x, y| sort_func(x, y));
            }
            ::config::Mode::Playoff => {
                let (mut winners, mut losers): (Vec<RoundStats>, Vec<RoundStats>) =
                    self.stats.drain(..).partition(|x| x.points > 0);
                self.stats.append(&mut winners);
                self.stats.append(&mut losers);
            }
            ::config::Mode::Ranking => (),
            ::config::Mode::RankingSort => {
                self.stats.sort_by(|a, b| {
                    let a_key = (
                        &a.points,
                        (a.goals_for as i32 - a.goals_against as i32),
                        &a.goals_for,
                    );
                    let b_key = (
                        &b.points,
                        (b.goals_for as i32 - b.goals_against as i32),
                        &b.goals_for,
                    );
                    b_key.cmp(&a_key)
                });
            }
        }
    }

    fn set_flags(&mut self) -> () {
        for (i, s) in self.stats.iter().enumerate() {
            if let Some(ref c) = s.set_flag {
                match c.cond {
                    ::config::Cond::RankMin => {
                        if i < c.value as usize {
                            self.flags.insert(c.flag.clone(), s.team.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn print(&self) -> () {
        println!("Round: {}", self.name);
        self.print_matches();
        match self.mode {
            ::config::Mode::RoundRobin => {
                println!();
                self.print_table(true);
            }
            ::config::Mode::Ranking => {
                println!();
                self.print_table(false);
            }
            ::config::Mode::RankingSort => {
                println!();
                self.print_table(true);
            }
            ::config::Mode::Playoff => {}
        }
        println!();
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
                " {:3}-{:<3} {:3} {:3}",
                self.goals_for,
                self.goals_against,
                self.goal_diff(),
                self.points
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

fn get_winner(g: (u32, u32)) -> MatchWinner {
    if g.0 > g.1 {
        MatchWinner::WinTeam1
    } else if g.1 > g.0 {
        MatchWinner::WinTeam2
    } else {
        MatchWinner::Draw
    }
}
