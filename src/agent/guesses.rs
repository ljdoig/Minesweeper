use crate::TilePos;

use super::*;
use itertools::Itertools;
use std::time::Instant;

fn tile_vec_to_u32(tile_vec: &Vec<&TilePos>, covered: &Vec<TilePos>) -> u32 {
    let mut tracker: u32 = 0;
    for (i, covered_tile) in covered.iter().enumerate() {
        if tile_vec.contains(&covered_tile) {
            tracker |= 1 << i;
        }
    }
    tracker
}

fn u32_to_tile_vec(bits: &u32, covered: &Vec<TilePos>) -> Vec<TilePos> {
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
    covered: &Vec<TilePos>,
) -> Vec<(u8, u32)> {
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                let num_bombs = num_bombs_around(board, pos);
                let n = n - num_bombs;
                let covered_neighbours = covered_neighbours(board, pos);
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

fn n_choose_r(n: u32, mut r: u32) -> u128 {
    if n == 0 || r == 0 || r >= n {
        return 1;
    }
    if 2 * r > n {
        r = n - r;
    }
    let n = n as u128;
    let r = r as u128;
    let mut value = 1;
    for (numerator, denominator) in (n - r + 1..=n).zip(1..=r) {
        value *= numerator;
        value /= denominator;
    }
    value
}

fn get_safety_probability_all_covered(
    tile: &TilePos,
    legal_bomb_cases: &Vec<Vec<TilePos>>,
) -> f64 {
    legal_bomb_cases
        .iter()
        .filter(|bombs| !bombs.contains(tile))
        .count() as f64
        / legal_bomb_cases.len() as f64
}

fn get_high_probability_guess_all_covered(
    all_covered: Vec<TilePos>,
    board: &Board,
) -> Action {
    // generate and test possible locations of bombs
    let start = Instant::now();
    let boundary_constraints = get_boundary_constraints(board, &all_covered);
    let total_num_bombs = board.num_bombs_left() as u32;
    let max_val = ((1_u64 << all_covered.len()) - 1) as u32;
    let legal_bomb_cases: Vec<Vec<TilePos>> = (0..=max_val)
        .into_iter()
        .filter_map(|bombs| {
            if bombs.count_ones() != total_num_bombs {
                return None;
            }
            for (bombs_needed, covered_neighbours) in &boundary_constraints {
                let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
                if *bombs_needed != num_bombs {
                    return None;
                }
            }
            // for TilePos { col, row } in u32_to_tile_vec(&bombs, &all_covered) {
            //     print!("({:2},{:2}) ", col, row);
            // }
            // println!("");
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
            (1000.0
                * get_safety_probability_all_covered(tile, &legal_bomb_cases))
                as usize
        })
        .unwrap();
    let tile_safety_prob =
        get_safety_probability_all_covered(&tile, &legal_bomb_cases);
    println!("Best odds: {:2.1}% -> {:?}", tile_safety_prob * 100.0, tile,);
    Action::uncover(*tile)
}

fn get_high_probability_guess_covered_boundary(
    covered_boundary: Vec<TilePos>,
    all_covered: Vec<TilePos>,
    board: &Board,
) -> Action {
    // generate and test possible bombs positions around boundary
    let start = Instant::now();
    let boundary_constraints =
        get_boundary_constraints(board, &covered_boundary);
    let total_num_bombs = board.num_bombs_left() as u32;
    let num_non_boundary_covered =
        (all_covered.len() - covered_boundary.len()) as u32;
    let max_val = ((1_u64 << covered_boundary.len()) - 1) as u32;
    let mut total_weights = 0;
    let use_weights = num_non_boundary_covered <= 125;
    let legal_bomb_cases = (0..=max_val)
        .into_iter()
        .filter_map(|bombs| {
            // too many bombs on boundary
            if bombs.count_ones() > total_num_bombs {
                return None;
            }
            // not enough bombs on boundary
            if bombs.count_ones() + num_non_boundary_covered < total_num_bombs {
                return None;
            }
            for (bombs_needed, covered_neighbours) in &boundary_constraints {
                let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
                if *bombs_needed != num_bombs {
                    return None;
                }
            }
            let bombs = u32_to_tile_vec(&bombs, &covered_boundary);
            let num_bombs_omitted = total_num_bombs - bombs.len() as u32;
            // println!("{} {}", num_non_boundary_covered, num_bombs_omitted);
            let weight = if use_weights {
                n_choose_r(num_non_boundary_covered, num_bombs_omitted)
            } else {
                1
            };
            total_weights += weight;
            // print!("Weight: {:2} ; ", weight);
            // for TilePos { col, row } in &bombs {
            //     print!("({:2},{:2}) ", col, row);
            // }
            // println!("");
            Some((bombs, weight))
        })
        .collect_vec();
    println!(
        "Iterating combinations of bombs took: {:.2}s ({} tiles)",
        start.elapsed().as_secs_f32(),
        covered_boundary.len()
    );

    // evaluate legal bomb cases around boundary
    let (boundary_tile, boundary_safety_prob) = covered_boundary
        .iter()
        .map(|tile| {
            let total_unsafe_weights: u128 = legal_bomb_cases
                .iter()
                .filter_map(|(bombs, weight)| {
                    (bombs.contains(&tile)).then(|| weight)
                })
                .sum();
            let proportion_unsafe =
                total_unsafe_weights as f64 / total_weights as f64;
            let proportion_safe = 1.0 - proportion_unsafe;
            (tile, proportion_safe)
        })
        .max_by_key(|(_, proportion_safe)| (10000.0 * proportion_safe) as u64)
        .unwrap();
    println!(
        "Best odds on boundary:           {:3.1}% -> {:?}",
        boundary_safety_prob * 100.0,
        boundary_tile,
    );

    // consider if there are better odds for a non-boundary tile
    let (non_boundary_tile, non_boundary_safety_prob) = all_covered
        .iter()
        .filter(|tile| !covered_boundary.contains(&tile))
        .map(|tile| {
            let proportion_safe = if use_weights {
                let total_unsafe_weights: u128 = legal_bomb_cases
                    .iter()
                    .filter_map(|(bombs, weight)| {
                        // check that not all bombs are on the boundary in this case
                        let num_omitted =
                            total_num_bombs as usize - bombs.len();
                        (num_omitted > 0).then(|| {
                            weight * num_omitted as u128
                                / num_non_boundary_covered as u128
                        })
                    })
                    .sum();
                let proportion_unsafe =
                    total_unsafe_weights as f64 / total_weights as f64;
                1.0 - proportion_unsafe
            } else {
                // no weights, just work out the expectation of bomb outside boundary
                let total_boundary_bombs: usize =
                    legal_bomb_cases.iter().map(|(bombs, _)| bombs.len()).sum();
                let mean_boundary_bombs =
                    total_boundary_bombs as f64 / legal_bomb_cases.len() as f64;
                let non_boundary_bombs =
                    total_num_bombs as f64 - mean_boundary_bombs;
                1.0 - non_boundary_bombs / num_non_boundary_covered as f64
            };
            (tile, proportion_safe)
        })
        .max_by_key(|(_, proportion_safe)| (10000.0 * proportion_safe) as u64)
        .unwrap();
    println!(
        "Best odds not on boundary:       {:3.1}% -> {:?}",
        non_boundary_safety_prob * 100.0,
        non_boundary_tile,
    );

    let pos = if boundary_safety_prob > non_boundary_safety_prob {
        println!(
            "Best odds are from boundary:     {:3.1}% -> {:?}",
            boundary_safety_prob * 100.0,
            boundary_tile,
        );
        boundary_tile
    } else {
        println!(
            "Best odds are from non-boundary: {:3.1}% -> {:?}",
            non_boundary_safety_prob * 100.0,
            non_boundary_tile,
        );
        non_boundary_tile
    };
    Action::uncover(*pos)
}

fn validate(
    bomb_subset: u64,
    boundary_constraints: &Vec<(u8, u64)>,
    mask: u64,
) -> bool {
    for &(constraint, subset) in boundary_constraints {
        let bombs_in_subset = (bomb_subset & subset).count_ones() as u8;
        // if all tiles in the subset have been considered: must match exactly
        // otherwise just make sure we don't exceed the given number of bombs
        if mask & subset == subset {
            if bombs_in_subset != constraint {
                return false;
            }
        } else if bombs_in_subset > constraint {
            return false;
        }
    }
    true
}

fn tile_vec_to_u64(tile_vec: &Vec<&TilePos>, covered: &Vec<TilePos>) -> u64 {
    let mut tracker: u64 = 0;
    for (i, covered_tile) in covered.iter().enumerate() {
        if tile_vec.contains(&covered_tile) {
            tracker |= 1 << i;
        }
    }
    tracker
}

fn u64_to_tile_vec(bits: &u64, covered: &Vec<TilePos>) -> Vec<TilePos> {
    let mut tile_vec = Vec::with_capacity(bits.count_ones() as usize);
    for (i, &covered_tile) in covered.iter().enumerate() {
        if bits & (1 << i) != 0 {
            tile_vec.push(covered_tile);
        }
    }
    tile_vec
}

fn get_boundary_constraints2(
    board: &Board,
    covered: &Vec<TilePos>,
) -> Vec<(u8, u64)> {
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                let num_bombs = num_bombs_around(board, pos);
                let n = n - num_bombs;
                let covered_neighbours = covered_neighbours(board, pos);
                if !covered_neighbours.is_empty() {
                    let covered_neighbours_u64 = tile_vec_to_u64(
                        &covered_neighbours.iter().collect(),
                        covered,
                    );
                    return Some((n, covered_neighbours_u64));
                }
            }
            None
        })
        .collect()
}

fn legal_bomb_candidates(
    boundary_constraints: &Vec<(u8, u64)>,
    boundary_size: usize,
) -> Vec<u64> {
    let start = Instant::now();
    let mut nbits_left = boundary_size;
    let mut bins = vec![];
    while nbits_left > 0 {
        let chunk_size = nbits_left.min(4);
        nbits_left -= chunk_size;
        let max_chunk = (2_u128.pow(chunk_size as u32) - 1) as u64;
        let mut bin = vec![];
        let mask = max_chunk << nbits_left;
        for i in 0..=max_chunk {
            let bomb_subset = i << nbits_left;
            if validate(bomb_subset, boundary_constraints, mask) {
                bin.push(bomb_subset);
            }
        }
        bins.push((bin, mask));
    }
    while bins.len() >= 2 {
        let (bin1, mask1) = bins.pop().unwrap();
        let (bin2, mask2) = bins.pop().unwrap();
        let mut new_bin = vec![];
        let new_mask = mask1 | mask2;
        for (subset1, subset2) in bin1.iter().cartesian_product(bin2) {
            let combined_bomb_subset = subset1 | subset2;
            if validate(combined_bomb_subset, boundary_constraints, new_mask) {
                new_bin.push(combined_bomb_subset);
            }
        }
        bins.insert(0, (new_bin, new_mask));
    }
    let (bin, mask) = bins.pop().unwrap();
    assert!(bins.is_empty());
    assert_eq!(mask.count_ones() as usize, boundary_size);
    bin
}

fn get_high_probability_guess_covered_boundary2(
    covered_boundary: Vec<TilePos>,
    all_covered: Vec<TilePos>,
    board: &Board,
) -> Action {
    // generate and test possible bombs positions around boundary
    let start = Instant::now();
    let boundary_constraints =
        get_boundary_constraints2(board, &covered_boundary);
    let total_num_bombs = board.num_bombs_left() as u32;
    let num_non_boundary_covered =
        (all_covered.len() - covered_boundary.len()) as u32;
    let mut total_weights = 0;
    let use_weights = num_non_boundary_covered <= 125;
    let legal_bomb_cases =
        legal_bomb_candidates(&boundary_constraints, covered_boundary.len())
            .iter()
            .filter_map(|bombs| {
                // too many bombs on boundary
                if bombs.count_ones() > total_num_bombs {
                    return None;
                }
                // not enough bombs on boundary
                if bombs.count_ones() + num_non_boundary_covered
                    < total_num_bombs
                {
                    return None;
                }
                for (bombs_needed, covered_neighbours) in &boundary_constraints
                {
                    let num_bombs =
                        (bombs & covered_neighbours).count_ones() as u8;
                    if *bombs_needed != num_bombs {
                        return None;
                    }
                }
                let bombs = u64_to_tile_vec(&bombs, &covered_boundary);
                let num_bombs_omitted = total_num_bombs - bombs.len() as u32;
                // println!("{} {}", num_non_boundary_covered, num_bombs_omitted);
                let weight = if use_weights {
                    n_choose_r(num_non_boundary_covered, num_bombs_omitted)
                } else {
                    1
                };
                total_weights += weight;
                // print!("Weight: {:2} ; ", weight);
                // for TilePos { col, row } in &bombs {
                //     print!("({:2},{:2}) ", col, row);
                // }
                // println!("");
                Some((bombs, weight))
            })
            .collect_vec();
    println!(
        "Generating legal arrangements of bombs took: {:.2}s ({} scenario(s) from {} tiles)",
        start.elapsed().as_secs_f32(),
        legal_bomb_cases.len(),
        covered_boundary.len()
    );
    let start = Instant::now();

    // evaluate legal bomb cases around boundary
    let (boundary_tile, boundary_safety_prob) = covered_boundary
        .iter()
        .map(|tile| {
            let total_unsafe_weights: u128 = legal_bomb_cases
                .iter()
                .filter_map(|(bombs, weight)| {
                    (bombs.contains(&tile)).then(|| weight)
                })
                .sum();
            let proportion_unsafe =
                total_unsafe_weights as f64 / total_weights as f64;
            let proportion_safe = 1.0 - proportion_unsafe;
            (tile, proportion_safe)
        })
        .max_by_key(|(_, proportion_safe)| (10000.0 * proportion_safe) as u64)
        .unwrap();
    println!(
        "Best odds on boundary:           {:3.1}% -> {:?}",
        boundary_safety_prob * 100.0,
        boundary_tile,
    );

    // consider if there are better odds for a non-boundary tile
    let non_boundary_scenario = all_covered
        .iter()
        .filter(|tile| !covered_boundary.contains(&tile))
        .map(|tile| {
            let proportion_safe = if use_weights {
                let total_unsafe_weights: u128 = legal_bomb_cases
                    .iter()
                    .filter_map(|(bombs, weight)| {
                        // check that not all bombs are on the boundary in this case
                        let num_omitted =
                            total_num_bombs as usize - bombs.len();
                        (num_omitted > 0).then(|| {
                            weight * num_omitted as u128
                                / num_non_boundary_covered as u128
                        })
                    })
                    .sum();
                let proportion_unsafe =
                    total_unsafe_weights as f64 / total_weights as f64;
                1.0 - proportion_unsafe
            } else {
                // no weights, just work out the expectation of bomb outside boundary
                let total_boundary_bombs: usize =
                    legal_bomb_cases.iter().map(|(bombs, _)| bombs.len()).sum();
                let mean_boundary_bombs =
                    total_boundary_bombs as f64 / legal_bomb_cases.len() as f64;
                let non_boundary_bombs =
                    total_num_bombs as f64 - mean_boundary_bombs;
                1.0 - non_boundary_bombs / num_non_boundary_covered as f64
            };
            (tile, proportion_safe)
        })
        .max_by_key(|(_, proportion_safe)| (10000.0 * proportion_safe) as u64);

    let non_boundary_safety_prob =
        if let Some((non_boundary_tile, non_boundary_safety_prob)) =
            non_boundary_scenario
        {
            println!(
                "Best odds not on boundary:       {:3.1}% -> {:?}",
                non_boundary_safety_prob * 100.0,
                non_boundary_tile,
            );
            non_boundary_safety_prob
        } else {
            0.0
        };

    let &tile = if boundary_safety_prob > non_boundary_safety_prob {
        println!(
            "Best odds are from boundary:     {:3.1}% -> {:?}",
            boundary_safety_prob * 100.0,
            boundary_tile,
        );
        boundary_tile
    } else {
        let (non_boundary_tile, non_boundary_safety_prob) =
            non_boundary_scenario.unwrap();
        println!(
            "Best odds are from non-boundary: {:3.1}% -> {:?}",
            non_boundary_safety_prob * 100.0,
            non_boundary_tile,
        );
        non_boundary_tile
    };
    println!(
        "Analysing arrangements of bombs took:        {:.2}s",
        start.elapsed().as_secs_f32(),
    );
    Action::uncover(tile)
}

fn sensible_ordering(mut covered_boundary: Vec<TilePos>) -> Vec<TilePos> {
    if covered_boundary.is_empty() {
        return covered_boundary;
    }
    let mut latest = covered_boundary.pop().unwrap();
    let mut output = vec![latest];
    while !covered_boundary.is_empty() {
        let (index, tile) = covered_boundary
            .iter()
            .cloned()
            .enumerate()
            .min_by_key(|(_, tile)| tile.squared_distance(latest))
            .unwrap();
        covered_boundary.remove(index);
        output.push(tile);
        latest = tile;
    }
    output
}

pub fn get_high_probability_guess(board: &Board) -> Action {
    // if we're out of ideas, just permute until we find a compatible option
    let all_covered = (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            (board.tile_state(pos) == TileState::Covered).then(|| pos)
        })
        .collect_vec();
    let covered_boundary = all_covered
        .iter()
        .filter(|&&pos| !uncovered_neighbours(board, pos).is_empty())
        .cloned()
        .collect_vec();
    let covered_boundary = sensible_ordering(covered_boundary);

    if 0 < covered_boundary.len() && covered_boundary.len() <= 64 {
        return get_high_probability_guess_covered_boundary2(
            covered_boundary.clone(),
            all_covered.clone(),
            board,
        );
    }
    // if all_covered.len() <= 28 {
    //     if all_covered.len() > covered_boundary.len()
    //         && covered_boundary.len() > 0
    //     {
    //         let answer = get_high_probability_guess_covered_boundary(
    //             covered_boundary.clone(),
    //             all_covered.clone(),
    //             board,
    //         );
    //         println!("Alternatively: ");
    //         let answer2 = get_high_probability_guess_covered_boundary2(
    //             covered_boundary.clone(),
    //             all_covered.clone(),
    //             board,
    //         );
    //         assert_eq!(answer, answer2);
    //     }
    //     if covered_boundary.len() > 0 {
    //         return get_high_probability_guess_all_covered(all_covered, board);
    //     }
    // } else if covered_boundary.len() <= 26 {
    //     let answer = get_high_probability_guess_covered_boundary(
    //         covered_boundary.clone(),
    //         all_covered.clone(),
    //         board,
    //     );
    //     println!("Alternatively: ");
    //     let answer2 = get_high_probability_guess_covered_boundary2(
    //         covered_boundary,
    //         all_covered,
    //         board,
    //     );
    //     assert_eq!(answer, answer2);
    //     return answer;
    // }

    if covered_boundary.is_empty() {
        let &tile = all_covered.first().unwrap();
        return Action::uncover(tile);
    }

    // to avoid combinatorics, we just take the tile with the best odds
    let (min_bombs, max_bombs) = deductions::get_subset_bounds(board);
    let pos = covered_boundary
        .iter()
        .min_by_key(|pos| {
            min_bombs
                .iter()
                .filter(|(&ref subset, _)| {
                    subset.contains(pos)
                        && min_bombs.get(subset) == max_bombs.get(subset)
                })
                .map(|(subset, n)| {
                    ((*n as f64 / subset.len() as f64) * 10000.0) as usize
                })
                .max()
                .unwrap()
        })
        .unwrap();
    println!("Guessing: ({}, {})", pos.col, pos.row);
    Action::uncover(*pos)
}
