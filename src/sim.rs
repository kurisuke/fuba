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

use rand;
use rand::distributions::{Distribution, Poisson, Uniform};
use std::string::String;

pub struct MatchOpts {
    pub elo: (u32, u32),
    pub weight: f64,
    pub extra: bool,
    pub penalties: bool,
}

pub struct Goals {
    first_half: (Vec<u32>, Vec<u32>),
    second_half: (Vec<u32>, Vec<u32>),
    extra_time: Option<(Vec<u32>, Vec<u32>)>,
    penalties: Option<(Vec<u32>, Vec<u32>)>,
}

impl Goals {
    pub fn total_after_first(&self) -> (u32, u32) {
        (
            self.first_half.0.len() as u32,
            self.first_half.1.len() as u32,
        )
    }

    pub fn total_after_second(&self) -> (u32, u32) {
        (
            self.first_half.0.len() as u32 + self.second_half.0.len() as u32,
            self.first_half.1.len() as u32 + self.second_half.1.len() as u32,
        )
    }

    pub fn total_after_extra(&self) -> (u32, u32) {
        let mut total = self.total_after_second();

        if let Some(ref x) = self.extra_time {
            total.0 += x.0.len() as u32;
            total.1 += x.1.len() as u32;
        }
        total
    }

    pub fn total(&self) -> (u32, u32) {
        let mut total = self.total_after_extra();

        if let Some(ref x) = self.penalties {
            total.0 += x.0.len() as u32;
            total.1 += x.1.len() as u32;
        }
        total
    }

    pub fn draw(&self) -> bool {
        self.total().0 == self.total().1
    }
}

pub struct MatchResult {
    pub goals: Goals,
    pub elo: (u32, u32),
}

pub enum MatchWinner {
    WinTeam1,
    WinTeam2,
    Draw,
}

impl MatchResult {
    pub fn winner(&self) -> MatchWinner {
        let r = self.goals.total().0 as i32 - self.goals.total().1 as i32;
        if r > 0 {
            MatchWinner::WinTeam1
        } else if r < 0 {
            MatchWinner::WinTeam2
        } else {
            MatchWinner::Draw
        }
    }

    pub fn result_str(&self) -> String {
        let mut s = String::new();
        s += &format!("{}-{}", self.goals.total().0, self.goals.total().1);

        let goals_first = format!(
            "{}-{}",
            self.goals.total_after_first().0,
            self.goals.total_after_first().1
        );
        let goals_second = format!(
            "{}-{}",
            self.goals.total_after_second().0,
            self.goals.total_after_second().1
        );
        let goals_extra = format!(
            "{}-{}",
            self.goals.total_after_extra().0,
            self.goals.total_after_extra().1
        );

        if let Some(_) = self.goals.penalties {
            if let Some(_) = self.goals.extra_time {
                s += &format!("p ({}, {}, {})", goals_first, goals_second, goals_extra);
            } else {
                s += &format!("p ({}, {})", goals_first, goals_second);
            }
        } else if let Some(_) = self.goals.extra_time {
            s += &format!("e ({}, {})", goals_first, goals_second);
        } else {
            s += &format!(" ({})", goals_first);
        }
        s
    }
}

pub struct Sim<'a> {
    rng: &'a mut rand::ThreadRng,
}

impl<'a> Sim<'a> {
    pub fn new(rng: &mut rand::ThreadRng) -> Sim {
        Sim { rng: rng }
    }

    pub fn simulate(&mut self, opts: MatchOpts) -> MatchResult {
        let d_elo = (opts.elo.0 as i32 - opts.elo.1 as i32) as f64;
        let avg_goal = 2.3 + d_elo.abs() / 600.0;

        let mut res = MatchResult {
            goals: Goals {
                first_half: self.simulate_period(d_elo, 45, avg_goal / 2.),
                second_half: self.simulate_period(d_elo, 45, avg_goal / 2.),
                extra_time: None,
                penalties: None,
            },
            elo: (0, 0),
        };

        if opts.extra {
            if res.goals.draw() {
                res.goals.extra_time = Some(self.simulate_period(d_elo, 30, avg_goal / 2.));
            }
        }

        if opts.penalties {
            if res.goals.draw() {
                res.goals.penalties = Some(self.simulate_penalties());
            }
        }

        res.elo = calculate_elo(opts.elo, res.goals.total_after_extra(), opts.weight);

        res
    }

    fn simulate_period(&mut self, d_elo: f64, length: u32, avg_goal: f64) -> (Vec<u32>, Vec<u32>) {
        let r_addtime = Uniform::new(1, length / 7);
        let r_minute = Uniform::new(1, length + r_addtime.sample(self.rng) + 1);

        let mut goals = (vec![], vec![]);
        let p_team = expected_result(d_elo);
        let r_team = Uniform::new(0f64, 1.);

        let poi = Poisson::new(avg_goal);

        for _ in 1..(poi.sample(&mut self.rng) + 1) {
            if r_team.sample(self.rng) <= p_team {
                goals.0.push(r_minute.sample(self.rng));
            } else {
                goals.1.push(r_minute.sample(self.rng));
            }
        }

        goals.0.sort();
        goals.1.sort();

        goals
    }

    fn simulate_penalties(&mut self) -> (Vec<u32>, Vec<u32>) {
        let r_goal = Uniform::new(1, 5);

        let mut goals = (vec![], vec![]);

        let mut i = 0;

        while (goals.0.len() == goals.1.len()) || (i <= 5) {
            i += 1;
            if r_goal.sample(self.rng) != 1 {
                goals.0.push(i);
            }
            if r_goal.sample(self.rng) != 1 {
                goals.1.push(i);
            }
        }

        goals
    }
}

fn calculate_elo(old_elo: (u32, u32), total: (u32, u32), k: f64) -> (u32, u32) {
    let g = match (total.0 as i32 - total.1 as i32).abs() {
        0...1 => 1.,
        2 => 1.5,
        n => (11. + (n as f64)) / 8.,
    };

    let w = if total.0 > total.1 {
        1.
    } else if total.0 < total.1 {
        0.
    } else {
        0.5
    };

    let d_elo = (old_elo.0 as i32 - old_elo.1 as i32) as f64;

    (
        ((old_elo.0 as f64) + k * g * (w - expected_result(d_elo))).round() as u32,
        ((old_elo.1 as f64) + k * g * (expected_result(d_elo) - w)).round() as u32,
    )
}

fn expected_result(d_elo: f64) -> f64 {
    1. / (10f64.powf(-d_elo / 400.) + 1.)
}
