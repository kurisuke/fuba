use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;

use petgraph::Graph;
use petgraph::algo::toposort;
use toml;

/// public structs

pub struct Config {
    pub name: String,
    pub seed: Option<u32>,
    pub root: Rc<RefCell<Round>>,
    pub team: Vec<Rc<RefCell<Team>>>,
    pub format: Vec<Rc<RefCell<Format>>>,
    pub round: Vec<Rc<RefCell<Round>>>,
}

#[derive(Deserialize, Clone)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub elo: u32,
}

#[derive(Deserialize, Clone)]
pub struct Format {
    pub id: String,
    pub mode: Mode,
    pub order: Option<Vec<Vec<u32>>>,
    pub neutral: Option<bool>,
    pub random: Option<bool>,
    pub rank_by: Vec<RankBy>,
    #[serde(default = "def_false")]
    pub copy: bool,
    #[serde(default = "def_weight")]
    pub weight: f64,
}

fn def_false() -> bool {
    return false;
}

fn def_weight() -> f64 {
    return 60.;
}

#[derive(Deserialize, Clone, PartialEq)]
pub enum Mode {
    #[serde(rename = "roundrobin")]
    RoundRobin,
    #[serde(rename = "playoff")]
    Playoff,
    #[serde(rename = "ranking")]
    Ranking,
    #[serde(rename = "ranking_sort")]
    RankingSort,
}

#[derive(Deserialize, Clone)]
pub enum RankBy {
    #[serde(rename = "points")]
    Points,
    #[serde(rename = "goaldiff")]
    GoalDiff,
    #[serde(rename = "goals")]
    Goals,
    #[serde(rename = "vspoints")]
    VsPoints,
    #[serde(rename = "vsgoaldiff")]
    VsGoalDiff,
    #[serde(rename = "vsgoals")]
    VsGoals,
    #[serde(rename = "extra")]
    Extra,
    #[serde(rename = "penalties")]
    Penalties,
}

pub struct Round {
    pub id: String,
    pub name: String,
    pub format: Rc<RefCell<Format>>,
    pub entrant: Vec<Entrant>,
}

pub struct Entrant {
    pub t: EntrantType,
    pub set_flag: Option<SetFlag>,
}

pub enum EntrantType {
    Team(Rc<RefCell<Team>>),
    Prev(Rc<RefCell<Round>>, u32),
    PrevFlag(Rc<RefCell<Round>>, Vec<(::flagcheck::FlagCheck, String)>),
}

#[derive(Deserialize, Clone)]
pub struct SetFlag {
    pub cond: Cond,
    pub value: u32,
    pub flag: String,
}

#[derive(Deserialize, Clone, PartialEq)]
pub enum Cond {
    #[serde(rename = "rankmin")]
    RankMin,
}

impl Clone for Config {
    fn clone(&self) -> Config {
        let team: Vec<Rc<RefCell<Team>>> = self.team
            .iter()
            .map(|x| Rc::new(RefCell::new(x.borrow().clone())))
            .collect();
        let format: Vec<Rc<RefCell<Format>>> = self.format
            .iter()
            .map(|x| Rc::new(RefCell::new(x.borrow().clone())))
            .collect();

        let mut round: Vec<Rc<RefCell<Round>>> = vec![];

        for r in self.round.iter() {
            let rb = r.borrow();
            let f = format
                .iter()
                .find(|x| x.borrow().id == rb.format.borrow().id)
                .unwrap()
                .clone();

            let mut entrant = vec![];
            for e in rb.entrant.iter() {
                match e.t {
                    EntrantType::Team(ref t) => {
                        entrant.push(Entrant {
                            t: EntrantType::Team(
                                team.iter()
                                    .find(|x| x.borrow().id == t.borrow().id)
                                    .unwrap()
                                    .clone(),
                            ),
                            set_flag: e.set_flag.clone(),
                        });
                    }
                    EntrantType::Prev(ref prev_round, ref n) => {
                        entrant.push(Entrant {
                            t: EntrantType::Prev(
                                round
                                    .iter()
                                    .find(|x| x.borrow().id == prev_round.borrow().id)
                                    .unwrap()
                                    .clone(),
                                *n,
                            ),
                            set_flag: e.set_flag.clone(),
                        });
                    }
                    EntrantType::PrevFlag(ref prev_round, ref checks) => {
                        entrant.push(Entrant {
                            t: EntrantType::PrevFlag(
                                round
                                    .iter()
                                    .find(|x| x.borrow().id == prev_round.borrow().id)
                                    .unwrap()
                                    .clone(),
                                (*checks).clone(),
                            ),
                            set_flag: e.set_flag.clone(),
                        });
                    }
                }
            }

            round.push(Rc::new(RefCell::new(Round {
                id: rb.id.clone(),
                name: rb.name.clone(),
                format: f,
                entrant,
            })));
        }

        let root = round
            .iter()
            .find(|x| x.borrow().id == self.root.borrow().id)
            .unwrap()
            .clone();

        Config {
            name: self.name.clone(),
            seed: self.seed.clone(),
            team,
            format,
            round,
            root,
        }
    }
}

/// internal structs (for parsing)

#[derive(Deserialize)]
#[serde(rename = "config")]
struct ConfigPars {
    pub name: String,
    pub seed: Option<u32>,
    pub root: String,
    pub team: Vec<Team>,
    pub format: Vec<Format>,
    pub round: Vec<RoundPars>,
}

#[derive(Deserialize)]
#[serde(rename = "round")]
struct RoundPars {
    pub id: String,
    pub name: String,
    pub format: String,
    pub entrant: Vec<EntrantPars>,
}

#[derive(Deserialize)]
#[serde(rename = "entrant")]
struct EntrantPars {
    pub id: Option<String>,
    pub prev: Option<String>,
    pub pos: Option<u32>,
    pub set_flag: Option<SetFlag>,
    pub if_flag: Option<Vec<String>>,
}

/// functions

pub fn read_config(filename: &str) -> Result<Config, &str> {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("error reading file");

    let config: ConfigPars = toml::from_str(&contents).unwrap();

    match verify(&config) {
        Ok(sort_order) => {
            let team = config
                .team
                .iter()
                .map(|x| Rc::new(RefCell::new(x.clone())))
                .collect::<Vec<_>>();

            let format = config
                .format
                .iter()
                .map(|x| Rc::new(RefCell::new(x.clone())))
                .collect::<Vec<_>>();

            let round = RefCell::new(vec![]);

            for s in sort_order.iter() {
                let r = config.round.iter().find(|x| &x.id == s).unwrap();

                let tmp = convert_round(r, &team, &format, &round.borrow());
                round.borrow_mut().push(tmp);
            }

            let root = round
                .borrow()
                .iter()
                .find(|x| *x.borrow().id == config.root)
                .unwrap()
                .clone();

            return Ok(Config {
                name: config.name,
                seed: config.seed,
                root,
                team,
                format,
                round: round.into_inner(),
            });
        }
        Err(x) => {
            return Err(x);
        }
    }
}

fn verify<'a>(config: &ConfigPars) -> Result<Vec<String>, &'a str> {
    let team_ids: Vec<_> = config.team.iter().map(|t| t.id.clone()).collect();
    let team_ids = verify_uniq(&team_ids).unwrap_or_else(|e| {
        panic!("Duplicate team ID: {}", e);
    });

    let format_ids: Vec<_> = config.format.iter().map(|t| t.id.clone()).collect();
    let format_ids = verify_uniq(&format_ids).unwrap_or_else(|e| {
        panic!("Duplicate format ID: {}", e);
    });

    let round_ids: Vec<_> = config.round.iter().map(|t| t.id.clone()).collect();
    let round_ids = verify_uniq(&round_ids).unwrap_or_else(|e| {
        panic!("Duplicate round ID: {}", e);
    });

    for r in config.round.iter() {
        format_ids
            .iter()
            .find(|&x| x == &(r.format))
            .unwrap_or_else(|| {
                panic!("Format ID not found: {}", r.format);
            });

        for e in r.entrant.iter() {
            if e.id.is_some() && e.prev.is_some() {
                panic!(
                    "Cannot set both 'id' and 'prev' in entrant for round: {}",
                    r.id
                );
            }
            if e.id.is_none() && e.prev.is_none() {
                panic!(
                    "Must set either 'id' or 'prev' in entrant for round: {}",
                    r.id
                );
            }

            if let Some(ref id) = e.id {
                team_ids.iter().find(|&x| x == id).unwrap_or_else(|| {
                    panic!("Entrant team ID not found: {}", id);
                });
            } else if let Some(ref prev) = e.prev {
                if prev == &r.id {
                    panic!(
                        "Prev round ID cannot be the same as current round: {}",
                        prev
                    );
                }
                if !(e.pos.is_none() ^ e.if_flag.is_none()) {
                    panic!(
                        "Must set either 'pos' or 'if_flag' when setting 'prev' in entrant for round: {}",
                        r.id
                    );
                }
                round_ids.iter().find(|&x| x == prev).unwrap_or_else(|| {
                    panic!("Prev round ID not found: {}", prev);
                });
            }
        }
    }

    // topological sorting
    let mut g = Graph::<&str, ()>::new();
    let mut h = HashMap::<String, ::petgraph::prelude::NodeIndex>::new();
    for r in config.round.iter() {
        h.insert(r.id.clone(), g.add_node(&r.id));
    }

    for r in config.round.iter() {
        for e in r.entrant.iter() {
            if let Some(ref prev) = e.prev {
                g.add_edge(*h.get(&r.id).unwrap(), *h.get(prev).unwrap(), ());
            }
        }
    }
    let sorted_nodes =
        toposort(&g, None).unwrap_or_else(|e| panic!("Cycle in rounds graph at node: {:?}", e));
    Ok(sorted_nodes
        .iter()
        .rev()
        .map(|&n| String::from(g[n]))
        .collect::<Vec<_>>())
}

fn verify_uniq(ids: &Vec<String>) -> Result<Vec<String>, String> {
    let mut uniq = HashMap::new();
    for id in ids.iter() {
        let id = id.clone();
        match uniq.insert(id.clone(), ()) {
            None => (),
            Some(_) => {
                return Err(id);
            }
        }
    }
    Ok(uniq.keys().map(|k| k.clone()).collect())
}

fn convert_round(
    round_p: &RoundPars,
    ex_teams: &Vec<Rc<RefCell<Team>>>,
    ex_formats: &Vec<Rc<RefCell<Format>>>,
    ex_rounds: &Vec<Rc<RefCell<Round>>>,
) -> Rc<RefCell<Round>> {
    let mut entrant = vec![];
    for e in round_p.entrant.iter() {
        if let Some(ref id) = e.id {
            let team = ex_teams.iter().find(|x| &(*x.borrow().id) == id).unwrap();
            entrant.push(Entrant {
                t: EntrantType::Team(team.clone()),
                set_flag: e.set_flag.clone(),
            });
        } else if let Some(ref prev) = e.prev {
            let round = ex_rounds
                .iter()
                .find(|x| &(*x.borrow().id) == prev)
                .unwrap();

            if let Some(pos) = e.pos {
                entrant.push(Entrant {
                    t: EntrantType::Prev(round.clone(), pos),
                    set_flag: e.set_flag.clone(),
                });
            } else if let Some(ref if_flag) = e.if_flag {
                let mut iter = if_flag.iter();
                let mut checks = vec![];
                while let Some(check_str) = iter.next() {
                    let cond = ::flagcheck::FlagCheck::new(check_str).unwrap();
                    let flag = iter.next().unwrap().clone();
                    checks.push((cond, flag));
                }
                entrant.push(Entrant {
                    t: EntrantType::PrevFlag(round.clone(), checks),
                    set_flag: e.set_flag.clone(),
                })
            }
        }
    }

    let format = ex_formats
        .iter()
        .find(|x| *x.borrow().id == round_p.format)
        .unwrap()
        .clone();

    Rc::new(RefCell::new(Round {
        id: round_p.id.clone(),
        name: round_p.name.clone(),
        format,
        entrant,
    }))
}
