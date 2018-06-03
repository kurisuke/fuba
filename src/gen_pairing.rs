use rand;
use rand::Rng;

pub fn generate_round_robin(num_teams: u32, num_legs: u32, randomize: bool) -> Vec<Vec<u32>> {
    let dummy_team = if num_teams % 2 != 0 { num_teams + 1 } else { 0 };

    let num_teams_in_calc = if dummy_team != 0 {
        num_teams + 1
    } else {
        num_teams
    };

    let number_of_rounds = num_teams_in_calc - 1;
    let games_per_round = num_teams_in_calc / 2;

    let mut column_a: Vec<u32> = (1..games_per_round + 1).collect();
    let mut column_b: Vec<u32> = (games_per_round + 1..num_teams_in_calc + 1).collect();

    let mut pairings = vec![];

    for _ in 0..number_of_rounds {
        for i in 0..games_per_round {
            let j = i as usize;
            if (column_a[j] != dummy_team) && (column_b[j] != dummy_team) {
                pairings.push(vec![column_a[j], column_b[j]]);
            }
        }

        // rotate elements

        let tmp1 = column_a[column_a.len() - 1];
        let mut tmp2: Vec<_> = column_a[1..column_a.len() - 1].to_vec();

        let tmp3 = column_b[0];
        let tmp4: Vec<_> = column_b[1..].to_vec();

        column_a = vec![1];
        column_a.push(tmp3);
        column_a.append(&mut tmp2);

        column_b = tmp4;
        column_b.push(tmp1);
    }

    if num_legs > 1 {
        let orig_pairings = pairings.clone();
        let mirror_pairings: Vec<Vec<u32>> = pairings.iter().map(|x| vec![x[1], x[0]]).collect();
        for x in 1..num_legs {
            if x % 2 != 0 {
                pairings.append(&mut mirror_pairings.clone());
            } else {
                pairings.append(&mut orig_pairings.clone());
            }
        }
    }

    if randomize {
        let mut rng = rand::thread_rng();
        let real_games_per_round = if dummy_team > 0 {
            games_per_round - 1
        } else {
            games_per_round
        };

        let mut shuffled_idx: Vec<usize> =
            (0..number_of_rounds as usize * num_legs as usize).collect();
        {
            let slice: &mut [usize] = &mut shuffled_idx;
            rng.shuffle(slice);
        }

        let mut shuffled_pairings = vec![];
        for s in shuffled_idx.iter() {
            let start = s * real_games_per_round as usize;
            let end = start + real_games_per_round as usize;
            shuffled_pairings.append(&mut pairings[start..end].to_vec());
        }
        pairings = shuffled_pairings;
    }

    pairings
}
