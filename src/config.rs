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
}

#[derive(Deserialize, Clone, PartialEq)]
pub enum Mode {
    #[serde(rename = "roundrobin")]
    RoundRobin,
    #[serde(rename = "playoff")]
    Playoff,
    #[serde(rename = "ranking")]
    Ranking,
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

pub enum Entrant {
    Team(Rc<RefCell<Team>>),
    Prev(Rc<RefCell<Round>>, u32),
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
                if e.pos.is_none() {
                    panic!(
                        "Must set 'pos' when setting 'prev' in entrant for round: {}",
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
            entrant.push(Entrant::Team(team.clone()));
        } else if let Some(ref prev) = e.prev {
            let round = ex_rounds
                .iter()
                .find(|x| &(*x.borrow().id) == prev)
                .unwrap();
            entrant.push(Entrant::Prev(round.clone(), e.pos.unwrap()));
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
