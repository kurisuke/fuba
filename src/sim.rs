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

use rand::distributions::{Distribution, Poisson, Uniform};
use rand::prng::XorShiftRng;
use rand::Rng;
use std::string::String;

pub struct MatchResult {
    first_half: (Vec<u32>, Vec<u32>),
    second_half: (Vec<u32>, Vec<u32>),
    extra_time: Option<(Vec<u32>, Vec<u32>)>,
    penalties: Option<(Vec<u32>, Vec<u32>)>,
}

impl MatchResult {
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

    pub fn result_str(&self) -> String {
        let mut s = String::new();
        s += &format!("{}-{}", self.total().0, self.total().1);

        let goals_first = format!(
            "{}-{}",
            self.total_after_first().0,
            self.total_after_first().1
        );
        let goals_second = format!(
            "{}-{}",
            self.total_after_second().0,
            self.total_after_second().1
        );
        let goals_extra = format!(
            "{}-{}",
            self.total_after_extra().0,
            self.total_after_extra().1
        );

        if let Some(_) = self.penalties {
            if let Some(_) = self.extra_time {
                s += &format!("p ({}, {}, {})", goals_first, goals_second, goals_extra);
            } else {
                s += &format!("p ({}, {})", goals_first, goals_second);
            }
        } else if let Some(_) = self.extra_time {
            s += &format!("e ({}, {})", goals_first, goals_second);
        } else {
            s += &format!(" ({})", goals_first);
        }
        s
    }
}

pub struct Sim<'a> {
    rng: &'a mut XorShiftRng,
}

impl<'a> Sim<'a> {
    pub fn new(rng: &mut XorShiftRng) -> Sim {
        Sim { rng: rng }
    }

    pub fn simulate(&mut self, elo: (u32, u32)) -> MatchResult {
        let d_elo = (elo.0 as i32 - elo.1 as i32) as f64;
        let avg_goal = 2.3 + d_elo.abs() / 600.0;

        MatchResult {
            first_half: self.simulate_period(d_elo, 45, avg_goal / 2.),
            second_half: self.simulate_period(d_elo, 45, avg_goal / 2.),
            extra_time: None,
            penalties: None,
        }
    }

    pub fn add_extra(&mut self, res: &mut MatchResult, elo: (u32, u32)) {
        let d_elo = (elo.0 as i32 - elo.1 as i32) as f64;
        let avg_goal = 2.3 + d_elo.abs() / 600.0;

        res.extra_time = Some(self.simulate_period(d_elo, 30, avg_goal / 2.));
    }

    pub fn add_penalties(&mut self, res: &mut MatchResult) {
        res.penalties = Some(self.simulate_penalties());
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
        let r_goal = Uniform::new(0f64, 1.);
        let p_goal = 0.725;

        let mut goals = (vec![], vec![]);
        {
            // draw lots on which team starts
            let g = if self.rng.gen() {
                (&mut goals.0, &mut goals.1)
            } else {
                (&mut goals.1, &mut goals.0)
            };

            for i in 0..5 {
                // first team tries penalty, chance is 75 percent
                if g.1.len() as i32 - g.0.len() as i32 <= (5 - i) {
                    if r_goal.sample(self.rng) < p_goal {
                        g.0.push(i as u32 + 1);
                    }
                } else {
                    break;
                }
                // second team tries penalty, chance is 75 percent
                if g.0.len() as i32 - g.1.len() as i32 <= (5 - i) {
                    if r_goal.sample(self.rng) < p_goal {
                        g.1.push(i as u32 + 1);
                    }
                } else {
                    break;
                }
            }

            // if still even score after 5 shots, continue until there is a winner
            let mut i = 5;
            let mut goal_prob = 0.75;
            while g.0.len() == g.1.len() {
                i += 1;
                goal_prob *= 0.9;
                let r_goal = Uniform::new(goal_prob, 1.);
                if r_goal.sample(self.rng) < p_goal {
                    g.0.push(i as u32);
                }
                if r_goal.sample(self.rng) < p_goal {
                    g.1.push(i as u32);
                }
            }
        }

        goals
    }
}

pub fn calculate_elo(old_elo: (u32, u32), total: (u32, u32), k: f64) -> (u32, u32) {
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

#[cfg(test)]
mod tests {
    #[test]
    fn calculate_elo_equal() {
        assert_eq!(
            super::calculate_elo((2000, 2000), (0, 0), 60.),
            (2000, 2000)
        );
    }

    #[test]
    fn calculate_elo_symmetric() {
        let tmp = super::calculate_elo((2000, 1800), (0, 3), 60.);
        assert_eq!(
            (tmp.1, tmp.0),
            super::calculate_elo((1800, 2000), (3, 0), 60.)
        );
    }

    fn gen_test_result() -> super::MatchResult {
        super::MatchResult {
            first_half: (vec![10, 20], vec![30]),
            second_half: (vec![30], vec![10, 20]),
            extra_time: None,
            penalties: None,
        }
    }

    #[test]
    fn matchresult_result_str() {
        let m = gen_test_result();
        assert_eq!(m.result_str(), String::from("3-3 (2-1)"));
    }
}
