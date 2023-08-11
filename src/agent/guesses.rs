use super::*;
use itertools::Itertools;
use std::time::Instant;

fn tile_vec_to_u32(
    tile_vec: &Vec<&(usize, usize)>,
    covered: &Vec<(usize, usize)>,
) -> u32 {
    let mut tracker: u32 = 0;
    for (i, covered_tile) in covered.iter().enumerate() {
        if tile_vec.contains(&covered_tile) {
            tracker |= 1 << i;
        }
    }
    tracker
}

fn u32_to_tile_vec(
    bits: &u32,
    covered: &Vec<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let mut tile_vec = Vec::with_capacity(bits.count_ones() as usize);
    for (i, &covered_tile) in covered.iter().enumerate() {
        if bits & (1 << i) != 0 {
            tile_vec.push(covered_tile);
        }
    }
    tile_vec
}

fn get_boundary_constraints(
    board: &Board,
    covered: &Vec<(usize, usize)>,
) -> Vec<(u8, u32)> {
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let num_bombs = num_bombs_around(board, col, row);
                let n = n - num_bombs;
                let covered_neighbours = covered_neighbours(board, col, row);
                if !covered_neighbours.is_empty() {
                    let covered_neighbours_u32 = tile_vec_to_u32(
                        &covered_neighbours.iter().collect(),
                        covered,
                    );
                    return Some((n, covered_neighbours_u32));
                }
            }
            None
        })
        .collect()
}

fn get_safety_probability(
    tile: &(usize, usize),
    legal_bomb_cases: &Vec<Vec<(usize, usize)>>,
) -> f64 {
    legal_bomb_cases
        .iter()
        .filter(|bombs| !bombs.contains(tile))
        .count() as f64
        / legal_bomb_cases.len() as f64
}

fn get_high_probability_guess_all_covered(
    all_covered: Vec<(usize, usize)>,
    board: &Board,
) -> Action {
    // generate and test possible locations of bombs
    let start = Instant::now();
    let boundary_constraints = get_boundary_constraints(board, &all_covered);
    let total_num_bombs = board.num_bombs_left() as u32;
    let max_val = ((1_u64 << all_covered.len()) - 1) as u32;
    let legal_bomb_cases: Vec<Vec<(usize, usize)>> = (0..=max_val)
        .into_iter()
        .filter_map(|bombs| {
            if bombs.count_ones() != total_num_bombs {
                return None;
            }
            for (n, covered_neighbours) in &boundary_constraints {
                let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
                if *n != num_bombs {
                    return None;
                }
            }
            Some(u32_to_tile_vec(&bombs, &all_covered))
        })
        .collect();
    println!(
        "Iterating combinations of bombs took: {:.2}s ({} tiles with {} bombs)",
        start.elapsed().as_secs_f32(),
        all_covered.len(),
        total_num_bombs
    );
    let tile = all_covered
        .iter()
        .max_by_key(|tile| {
            (1000.0 * get_safety_probability(tile, &legal_bomb_cases)) as usize
        })
        .unwrap();
    let tile_safety_prob = get_safety_probability(&tile, &legal_bomb_cases);
    println!("Best odds: {:2.1}%", tile_safety_prob * 100.0);
    Action {
        col: tile.0,
        row: tile.1,
        action_type: ActionType::Uncover,
    }
}

fn average_length<T>(vecs: Vec<Vec<T>>) -> f64 {
    let sum_of_lengths: usize = vecs.iter().map(|vec| vec.len()).sum();
    sum_of_lengths as f64 / vecs.len() as f64
}

fn get_high_probability_guess_covered_boundary(
    covered_boundary: Vec<(usize, usize)>,
    all_covered: Vec<(usize, usize)>,
    board: &Board,
) -> Action {
    // generate and test possible bombs positions around boundary
    let start = Instant::now();
    let boundary_constraints =
        get_boundary_constraints(board, &covered_boundary);
    let max_val = ((1_u64 << covered_boundary.len()) - 1) as u32;
    let legal_bomb_cases: Vec<Vec<(usize, usize)>> = (0..=max_val)
        .into_iter()
        .filter_map(|bombs| {
            for (n, covered_neighbours) in &boundary_constraints {
                let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
                if *n != num_bombs {
                    return None;
                }
            }
            Some(u32_to_tile_vec(&bombs, &covered_boundary))
        })
        .collect();
    println!(
        "Iterating combinations of bombs took: {:.2}s ({} tiles)",
        start.elapsed().as_secs_f32(),
        covered_boundary.len()
    );

    // evaluate legal bomb cases around boundary
    let boundary_tile = covered_boundary
        .iter()
        .max_by_key(|tile| {
            (1000.0 * get_safety_probability(tile, &legal_bomb_cases)) as usize
        })
        .unwrap();
    let boundary_safety_prob =
        get_safety_probability(&boundary_tile, &legal_bomb_cases);
    println!(
        "Best odds on boundary:       {:2.1}%",
        boundary_safety_prob * 100.0
    );

    // consider if there are better odds for a non-boundary tile
    let num_non_boundary_bombs =
        board.num_bombs_left() as f64 - average_length(legal_bomb_cases);
    let num_non_boundary_covered = all_covered.len() - covered_boundary.len();
    let non_boundary_safety_prob =
        1.0 - num_non_boundary_bombs / num_non_boundary_covered as f64;
    println!(
        "Best odds on non boundary:   {:2.1}%    ({:.1} bombs in {} tiles)",
        non_boundary_safety_prob * 100.0,
        num_non_boundary_bombs,
        num_non_boundary_covered
    );

    let (col, row) = if boundary_safety_prob > non_boundary_safety_prob {
        println!(
            "Best odds are from boundary: {:2.1}% -> {:?}",
            boundary_safety_prob * 100.0,
            boundary_tile,
        );
        boundary_tile
    } else {
        let non_boundary_tile = all_covered
            .iter()
            .find(|tile| !covered_boundary.contains(tile))
            .unwrap();
        println!(
            "Best odds are from boundary: {:2.1}% -> {:?}",
            non_boundary_safety_prob * 100.0,
            non_boundary_tile,
        );
        non_boundary_tile
    };

    Action {
        col: *col,
        row: *row,
        action_type: ActionType::Uncover,
    }
}

pub fn get_high_probability_guess(board: &Board) -> Action {
    // if we're out of ideas, just permute until we find a compatible option
    let all_covered = (0..board.width())
        .cartesian_product(0..board.height())
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Covered)
        .collect_vec();
    let covered_boundary = all_covered
        .iter()
        .filter(|(col, row)| {
            !uncovered_neighbours(board, *col, *row).is_empty()
        })
        .cloned()
        .collect_vec();

    if all_covered.len() <= 30 {
        return get_high_probability_guess_all_covered(all_covered, board);
    } else if covered_boundary.len() <= 26 {
        return get_high_probability_guess_covered_boundary(
            covered_boundary,
            all_covered,
            board,
        );
    }

    // to avoid combinatorics, we just take the tile with the best odds
    let (min_bombs, max_bombs) = deductions::get_subset_bounds(board);
    let (col, row) = covered_boundary
        .iter()
        .min_by_key(|(col, row)| {
            min_bombs
                .iter()
                .filter(|(&ref subset, _)| {
                    subset.contains(&(*col, *row))
                        && min_bombs.get(subset) == max_bombs.get(subset)
                })
                .map(|(subset, n)| {
                    ((*n as f64 / subset.len() as f64) * 10000.0) as usize
                })
                .max()
                .unwrap()
        })
        .unwrap();
    println!("Guessing: {col}, {row}");
    return Action {
        col: *col,
        row: *row,
        action_type: ActionType::Uncover,
    };
}
