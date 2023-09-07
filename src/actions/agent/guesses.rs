use super::*;
use crate::TilePos;
use instant::Instant;
use itertools::{Itertools, MinMaxResult};

fn case_weight(
    num_bombs_omitted: usize,
    num_non_boundary_covered: usize,
    min_bombs_omitted: usize,
) -> f64 {
    // bit of combinatorics here: weight should be proportional to
    // `num_non_boundary_tiles` choose `num_bombs_omitted`
    // to save space we divide all weights by
    // `num_non_boundary_tiles` choose `min_bombs_omitted`
    // this is the result after that division:
    if num_bombs_omitted > num_non_boundary_covered {
        return 0.0;
    }
    let num_bombs_omitted = num_bombs_omitted as u128;
    let num_non_boundary_covered = num_non_boundary_covered as u128;
    let min_bombs_omitted = min_bombs_omitted as u128;

    let numerator_min = num_non_boundary_covered - num_bombs_omitted + 1;
    let numerator_max = num_non_boundary_covered - min_bombs_omitted;
    let denominator_min = min_bombs_omitted + 1;
    let denominator_max = num_bombs_omitted;

    let numerator_terms = numerator_min..=numerator_max;
    let denominator_terms = denominator_min..=denominator_max;
    let mut output = 1.0;
    for (num, denom) in numerator_terms.zip(denominator_terms) {
        output /= denom as f64;
        output *= num as f64;
    }
    output
}

fn validate(
    bomb_subset: u128,
    boundary_constraints: &Vec<(u8, u128)>,
    mask: u128,
) -> bool {
    for &(constraint, subset) in boundary_constraints {
        let bombs_in_subset = (bomb_subset & subset).count_ones() as u8;
        // if all tiles in the subset have been considered: must match exactly
        // otherwise just make the constraint is still fulifllable later
        if mask & subset == subset {
            if bombs_in_subset != constraint {
                return false;
            }
        } else if bombs_in_subset > constraint {
            return false;
        } else {
            let non_assigned_in_subset = (!mask & subset).count_ones() as u8;
            if bombs_in_subset + non_assigned_in_subset < constraint {
                return false;
            }
        }
    }
    true
}

fn validate_final(
    bomb_subset: u128,
    boundary_constraints: &Vec<(u8, u128)>,
) -> bool {
    for &(constraint, subset) in boundary_constraints {
        let bombs_in_subset = (bomb_subset & subset).count_ones() as u8;
        if bombs_in_subset != constraint {
            return false;
        }
    }
    return true;
}

fn tile_vec_to_u128(
    tile_vec: &[TilePos],
    covered_boundary: &[TilePos],
) -> u128 {
    let mut tracker: u128 = 0;
    for (i, covered_tile) in covered_boundary.iter().enumerate() {
        if tile_vec.contains(covered_tile) {
            tracker |= 1 << i;
        }
    }
    tracker
}

fn get_boundary_constraints(
    board: &Board,
    covered_boundary: &[TilePos],
) -> Vec<(u8, u128)> {
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                let covered_neighbours = covered_neighbours(board, pos);
                if !covered_neighbours.is_empty() {
                    let num_bombs = num_bombs_around(board, pos);
                    let n = n - num_bombs;
                    let covered_neighbours_u128 =
                        tile_vec_to_u128(&covered_neighbours, covered_boundary);
                    return Some((n, covered_neighbours_u128));
                }
            }
            None
        })
        .collect()
}

fn elapsed_time_string(instant: &Instant) -> String {
    let str = format!("{:3.3}", instant.elapsed().as_secs_f32());
    format!("{:>7}", str)
}

fn legal_scenario_info(
    boundary_constraints: &Vec<(u8, u128)>,
    boundary_size: usize,
) -> ([[usize; 100]; 128], [usize; 100], usize) {
    let mut nbits_left = boundary_size;
    let mut bins = vec![];
    let nbins = if boundary_size <= 32 { 2 } else { 8 };
    for bin in 0..nbins {
        let chunk_size = (nbits_left as f64 / (nbins - bin) as f64).round();
        nbits_left -= chunk_size as usize;
        let max_chunk = 2_u128.pow(chunk_size as u32) - 1;
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
    while bins.len() > 2 {
        let (bin1, mask1) = bins.pop().unwrap();
        let (bin2, mask2) = bins.pop().unwrap();
        let mut new_bin = vec![];
        let new_mask = mask1 | mask2;
        let merging_constraints = boundary_constraints
            .iter()
            .cloned()
            .filter(|(_, subset)| {
                // only need to check constraints that overlap both regions
                subset & mask1 > 0 && subset & mask2 > 0
            })
            .collect_vec();
        for (subset1, subset2) in bin1.iter().cartesian_product(bin2) {
            let combined_bomb_subset = subset1 | subset2;
            if validate(combined_bomb_subset, &merging_constraints, new_mask) {
                new_bin.push(combined_bomb_subset);
            }
        }
        bins.insert(0, (new_bin, new_mask));
    }
    // final 2
    let (bin1, mask1) = bins.pop().unwrap();
    let (bin2, mask2) = bins.pop().unwrap();
    assert_eq!(boundary_size, (mask1 | mask2).count_ones() as usize);
    assert_eq!(0, mask1 & mask2);
    let merging_constraints = boundary_constraints
        .iter()
        .cloned()
        .filter(|(_, subset)| subset & mask1 > 0 && subset & mask2 > 0)
        .collect_vec();

    let mut num_bombs_counters = [[0; 100]; 128];
    let mut total_num_bombs_counter = [0; 100];
    let mut num_scenarios = 0;
    for (subset1, subset2) in bin1.iter().cartesian_product(bin2) {
        let bomb_subset = subset1 | subset2;
        if validate_final(bomb_subset, &merging_constraints) {
            num_scenarios += 1;
            let num_bombs = bomb_subset.count_ones() as usize;
            for (i, num_bombs_counters) in
                num_bombs_counters.iter_mut().enumerate()
            {
                if bomb_subset & (1 << i) > 0 {
                    num_bombs_counters[num_bombs] += 1;
                }
            }
            total_num_bombs_counter[num_bombs] += 1;
        }
    }
    (num_bombs_counters, total_num_bombs_counter, num_scenarios)
}

fn get_high_probability_guess(
    covered_boundary: Vec<TilePos>,
    all_covered: Vec<TilePos>,
    board: &Board,
) -> Action {
    // generate and test possible bombs positions around boundary
    let start = Instant::now();
    let covered_boundary = sensible_ordering(covered_boundary);
    let boundary_constraints =
        get_boundary_constraints(board, &covered_boundary);
    let total_num_bombs_left = board.num_bombs_left() as usize;
    let num_non_boundary_covered = all_covered.len() - covered_boundary.len();
    let (num_bombs_counters, mut total_num_bombs_counter, num_scenarios) =
        legal_scenario_info(&boundary_constraints, covered_boundary.len());
    let max_bombs = total_num_bombs_left;
    let min_bombs =
        total_num_bombs_left.saturating_sub(num_non_boundary_covered);
    let filter = |num_bombs_counter: &mut [usize; 100]| {
        for (num_bombs, counter) in num_bombs_counter.iter_mut().enumerate() {
            if num_bombs < min_bombs || max_bombs < num_bombs {
                *counter = 0;
            }
        }
    };
    for mut num_bombs_counter in num_bombs_counters {
        filter(&mut num_bombs_counter);
    }
    filter(&mut total_num_bombs_counter);
    println!(
        "Analysing legal scenarios took: {}s ({} scenario(s) from {} tiles)",
        elapsed_time_string(&start),
        num_scenarios,
        covered_boundary.len()
    );

    let (min_bombs, max_bombs) = {
        let min_max = total_num_bombs_counter
            .iter()
            .enumerate()
            .filter_map(|(num_bombs, &counter)| {
                (counter > 0).then_some(num_bombs)
            })
            .minmax();
        match min_max {
            MinMaxResult::MinMax(min, max) => (min, max),
            MinMaxResult::OneElement(val) => (val, val),
            MinMaxResult::NoElements => panic!(),
        }
    };
    let min_bombs_omitted = total_num_bombs_left - max_bombs;
    let mut total_weights = 0.0;
    let bombs_present_count = (0..=99)
        .map(|num_bombs| {
            if !(min_bombs <= num_bombs && num_bombs <= max_bombs) {
                return (0, 0.0);
            }
            let weight = case_weight(
                total_num_bombs_left - num_bombs,
                num_non_boundary_covered,
                min_bombs_omitted,
            );
            let count = total_num_bombs_counter[num_bombs];
            total_weights += weight * count as f64;
            (count, weight)
        })
        .collect_vec();

    // evaluate legal bomb cases around boundary
    let (boundary_tile, boundary_safety_prob) = covered_boundary
        .iter()
        .enumerate()
        .map(|(i, tile)| {
            let unsafe_weights: f64 = num_bombs_counters[i]
                .iter()
                .enumerate()
                .map(|(num_bombs, &count)| {
                    let weight = bombs_present_count[num_bombs].1;
                    weight * count as f64
                })
                .sum();
            let proportion_safe = 1.0 - unsafe_weights / total_weights;
            (tile, proportion_safe)
        })
        .max_by(|(tile1, proportion_safe1), (tile2, proportion_safe2)| {
            proportion_safe1
                .total_cmp(proportion_safe2)
                .then(tile2.cmp(tile1))
        })
        .unwrap();

    if num_non_boundary_covered == 0 {
        // println!(
        //     "Best odds:                       {:>5.1}% -> {:?}",
        //     boundary_safety_prob * 100.0,
        //     boundary_tile,
        // );
        return Action::uncover(*boundary_tile);
    }

    // consider if there are better odds for a non-boundary tile
    let non_boundary_safety_prob = {
        let unsafe_weights: f64 = bombs_present_count
            .iter()
            .enumerate()
            .rev()
            .map(|(num_bombs, &(count, weight))| {
                if count == 0 {
                    return 0.0;
                }
                let num_bombs_omitted = total_num_bombs_left - num_bombs;
                count as f64 * weight * num_bombs_omitted as f64
                    / num_non_boundary_covered as f64
            })
            .sum();
        1.0 - unsafe_weights / total_weights
    };
    // println!(
    //     "Best odds on boundary:           {:>5.1}% -> {:?}",
    //     boundary_safety_prob * 100.0,
    //     boundary_tile,
    // );
    // println!(
    //     "Best odds not on boundary:       {:>5.1}%",
    //     non_boundary_safety_prob * 100.0,
    // );
    let &best_tile = if boundary_safety_prob > non_boundary_safety_prob {
        // println!(
        //     "Best odds are from boundary:     {:>5.1}% -> {:?}",
        //     boundary_safety_prob * 100.0,
        //     boundary_tile,
        // );
        boundary_tile
    } else {
        // unwrap here because we have already checked non-boundary tiles exist
        let non_boundary_tile = all_covered
            .iter()
            .filter(|tile| !covered_boundary.contains(tile))
            .min_by_key(|&&tile| {
                // choose tile that will keep the boundary smallest
                (
                    covered_neighbours(board, tile)
                        .into_iter()
                        .filter(|tile| !covered_boundary.contains(tile))
                        .count(),
                    tile,
                )
            })
            .unwrap();
        // println!(
        //     "Best odds are from non-boundary: {:>5.1}% -> {:?}",
        //     non_boundary_safety_prob * 100.0,
        //     non_boundary_tile,
        // );
        non_boundary_tile
    };
    Action::uncover(best_tile)
}

fn sensible_ordering(covered_boundary: Vec<TilePos>) -> Vec<TilePos> {
    if covered_boundary.len() <= 1 {
        return covered_boundary.to_vec();
    }
    let &centroid = covered_boundary
        .iter()
        .cartesian_product(&covered_boundary)
        .max_by_key(|(tile, &other_tile)| tile.squared_distance(other_tile))
        .unwrap()
        .0;
    let mut sorted = covered_boundary
        .into_iter()
        .sorted_unstable_by_key(|tile| tile.squared_distance(centroid))
        .collect_vec();

    let boundary2 = sorted.split_off(sorted.len() / 2);
    let boundary1 = sorted;
    let mut boundary1 = sensible_ordering(boundary1);
    let mut boundary2 = sensible_ordering(boundary2);

    // reorder when merging if distance is smaller
    let tail1 = boundary1.last().unwrap();
    let tail_to_head = tail1.squared_distance(*boundary2.first().unwrap());
    let tail_to_tail = tail1.squared_distance(*boundary2.last().unwrap());
    if tail_to_head > tail_to_tail {
        boundary2.reverse();
    }

    boundary1.append(&mut boundary2);
    boundary1
}

pub fn make_guess(board: &Board) -> Action {
    // if we're out of ideas, just permute until we find a compatible option
    let all_covered = (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            (board.tile_state(pos) == TileState::Covered).then_some(pos)
        })
        .collect_vec();
    let covered_boundary = all_covered
        .iter()
        .filter(|&&pos| !uncovered_neighbours(board, pos).is_empty())
        .cloned()
        .collect_vec();

    if covered_boundary.is_empty() {
        let &tile = all_covered.first().unwrap();
        return Action::uncover(tile);
    }

    if covered_boundary.len() <= 128 {
        return get_high_probability_guess(
            covered_boundary,
            all_covered,
            board,
        );
    }

    // this will almost certainly never happen, but it's an option
    let (min_bombs, max_bombs) = deductions::get_subset_bounds(board);
    let pos = covered_boundary
        .iter()
        .min_by_key(|pos| {
            min_bombs
                .iter()
                .filter(|&(subset, _)| {
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
