use crate::board::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::time::Instant;

fn num_bombs_around(board: &Board, col: usize, row: usize) -> u8 {
    board
        .neighbours(col, row)
        .iter()
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Flagged)
        .count() as u8
}

fn covered_neighbours(
    board: &Board,
    col: usize,
    row: usize,
) -> Vec<(usize, usize)> {
    board
        .neighbours(col, row)
        .iter()
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Covered)
        .cloned()
        .collect()
}

fn uncovered_neighbours(
    board: &Board,
    col: usize,
    row: usize,
) -> Vec<(usize, usize)> {
    board
        .neighbours(col, row)
        .iter()
        .filter(|(col, row)| {
            matches!(board.tile_state(*col, *row), TileState::UncoveredSafe(_))
        })
        .cloned()
        .collect()
}

fn num_covered_around(board: &Board, col: usize, row: usize) -> u8 {
    covered_neighbours(board, col, row).len() as u8
}

fn deduplicate(output: Vec<Action>) -> Vec<Action> {
    let mut deduplicated = vec![];
    for action in output {
        if !deduplicated.iter().any(|x: &Action| x == &action) {
            deduplicated.push(action);
        }
    }
    deduplicated
}

pub fn get_all_actions(board: &Board) -> Vec<Action> {
    let mut output = get_trivial_actions(board);
    if output.is_empty() {
        output.append(&mut get_non_trivial_actions(board));
    }
    if output.is_empty() {
        output.push(get_high_probability_guess(board));
    }
    deduplicate(output)
}

pub fn get_trivial_actions(board: &Board) -> Vec<Action> {
    let mut output = vec![];
    if board.tile_states().iter().all(|&x| x == TileState::Covered) {
        // first guess, just go for the centre
        let action = Action {
            col: board.width() / 2,
            row: board.height() / 2,
            action_type: ActionType::Uncover,
        };
        return vec![action];
    } else if board.num_bombs_left() == 0 {
        // no bombs left, just uncover last uncovered tiles
        for col in 0..board.width() {
            for row in 0..board.height() {
                if board.tile_state(col, row) == TileState::Covered {
                    output.push(Action {
                        col,
                        row,
                        action_type: ActionType::Uncover,
                    });
                }
            }
        }
        return output;
    }

    for col in 0..board.width() {
        for row in 0..board.height() {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let num_bombs = num_bombs_around(board, col, row);
                let num_covered = num_covered_around(board, col, row);
                // uncover all neighbours
                if num_bombs == n {
                    covered_neighbours(board, col, row)
                        .into_iter()
                        .map(|(col, row)| Action {
                            col,
                            row,
                            action_type: ActionType::Uncover,
                        })
                        .for_each(|x| output.push(x));
                }
                // flag all neighbours
                if n.saturating_sub(num_bombs) == num_covered {
                    board
                        .neighbours(col, row)
                        .into_iter()
                        .filter(|(col, row)| {
                            board.tile_state(*col, *row) == TileState::Covered
                        })
                        .map(|(col, row)| Action {
                            col,
                            row,
                            action_type: ActionType::Flag,
                        })
                        .for_each(|x| output.push(x));
                }
            }
        }
    }
    deduplicate(output)
}

fn subsets(
    elts: &[(usize, usize)],
    max_size: usize,
) -> Vec<Vec<&(usize, usize)>> {
    (2..=max_size)
        .flat_map(|k| elts.iter().combinations(k))
        .collect()
}

fn set_difference(
    this: &Vec<(usize, usize)>,
    other: &Vec<(usize, usize)>,
) -> Vec<(usize, usize)> {
    this.iter()
        .filter(|x| !other.contains(x))
        .cloned()
        .collect()
}

fn max_in_subset(
    tiles: &Vec<(usize, usize)>,
    max_bombs: &mut HashMap<Vec<(usize, usize)>, u8>,
) -> u8 {
    let mut smallest_max = if let Some(&max) = max_bombs.get(tiles) {
        max
    } else {
        tiles.len() as u8
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.len() <= 2 {
        return smallest_max;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.len().saturating_sub(1);
    for subset in subsets(&tiles, max_size) {
        let subset = subset.iter().copied().copied().collect_vec();
        if let Some(&sub_max) = max_bombs.get(&subset) {
            let rest = set_difference(tiles, &subset);
            let tiles_max = sub_max + max_in_subset(&rest, max_bombs);
            if tiles_max < smallest_max {
                smallest_max = tiles_max;
            }
        }
    }
    if let Some(&max) = max_bombs.get(tiles) {
        if smallest_max < max {
            max_bombs.insert(tiles.clone(), smallest_max);
        }
    } else {
        max_bombs.insert(tiles.clone(), smallest_max);
    };
    smallest_max
}

fn min_in_subset(
    tiles: &Vec<(usize, usize)>,
    min_bombs: &mut HashMap<Vec<(usize, usize)>, u8>,
) -> u8 {
    let mut biggest_min = if let Some(&min) = min_bombs.get(tiles) {
        min
    } else {
        0
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.len() <= 2 {
        return biggest_min;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.len().saturating_sub(1);
    for subset in subsets(&tiles, max_size) {
        let subset = subset.iter().cloned().cloned().collect_vec();
        if let Some(&sub_min) = min_bombs.get(&subset) {
            let rest = set_difference(tiles, &subset);
            let tiles_min = sub_min + min_in_subset(&rest, min_bombs);
            if tiles_min > biggest_min {
                biggest_min = tiles_min;
            }
        }
    }
    if let Some(&min) = min_bombs.get(tiles) {
        if biggest_min > min {
            min_bombs.insert(tiles.clone(), biggest_min);
        }
    } else {
        min_bombs.insert(tiles.clone(), biggest_min);
    };
    biggest_min
}

fn get_subset_bounds(
    board: &Board,
) -> (
    HashMap<Vec<(usize, usize)>, u8>,
    HashMap<Vec<(usize, usize)>, u8>,
) {
    let mut min_bombs: HashMap<Vec<(usize, usize)>, u8> = HashMap::new();
    let mut max_bombs: HashMap<Vec<(usize, usize)>, u8> = HashMap::new();
    for _ in 0..3 {
        update_subset_bounds(board, &mut min_bombs, &mut max_bombs);
    }
    (min_bombs, max_bombs)
}

fn update_subset_bounds(
    board: &Board,
    min_bombs: &mut HashMap<Vec<(usize, usize)>, u8>,
    max_bombs: &mut HashMap<Vec<(usize, usize)>, u8>,
) {
    for col in 0..board.width() {
        for row in 0..board.height() {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let n = n - num_bombs_around(board, col, row);
                let covered = covered_neighbours(board, col, row);
                let num_covered = covered.len();
                for subset in subsets(&covered, num_covered) {
                    let subset = subset.iter().cloned().cloned().collect_vec();
                    // rule 1: at most n bombs in all subsets around the tile
                    if subset.len() > n as usize {
                        if let Some(max) = max_bombs.get(&subset) {
                            if n < *max {
                                max_bombs.insert(subset.clone(), n);
                            }
                        } else {
                            max_bombs.insert(subset.clone(), n);
                        }
                    }
                    // rule 2: if we exclude tiles with a max of k bombs there
                    // are at least n - k bombs in the remaining subset
                    let rest = set_difference(&covered.clone(), &subset);
                    let max_omitted = max_in_subset(&rest, max_bombs);
                    if n > max_omitted {
                        if let Some(min) = min_bombs.get(&subset) {
                            if n - max_omitted > *min {
                                min_bombs
                                    .insert(subset.clone(), n - max_omitted);
                            }
                        } else {
                            min_bombs.insert(subset.clone(), n - max_omitted);
                        }
                    }
                    // rule 3: if we exclude tiles with a min of k bombs there
                    // are at most n - k bombs in the remaining subset
                    let min_omitted = min_in_subset(&rest, min_bombs);
                    if n > min_omitted {
                        if let Some(max) = max_bombs.get(&subset) {
                            if n - min_omitted < *max {
                                max_bombs.insert(subset, n - min_omitted);
                            }
                        } else {
                            max_bombs.insert(subset, n - min_omitted);
                        }
                    }
                }
            }
        }
    }
}

pub fn get_non_trivial_actions(board: &Board) -> Vec<Action> {
    let mut output = vec![];
    let (mut min_bombs, mut max_bombs) = get_subset_bounds(board);
    // check each uncovered tile to see if we have helpful adjacent subsets
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                Some((col, row, n))
            } else {
                None
            }
        })
        .for_each(|(col, row, n)| {
            let n = n - num_bombs_around(board, col, row);
            let covered = covered_neighbours(board, col, row);
            let num_covered = covered.len();
            if num_covered == 0 {
                return;
            }
            for subset in subsets(&covered, num_covered - 1) {
                // need so few bombs in subset that the rest must be bombs
                let max = max_in_subset(
                    &subset.iter().cloned().cloned().collect(),
                    &mut max_bombs,
                );
                let rest_size = (num_covered - subset.len()) as u8;
                if max + rest_size == n {
                    covered
                        .iter()
                        .filter(|x| !subset.contains(x))
                        .map(|(col, row)| Action {
                            col: *col,
                            row: *row,
                            action_type: ActionType::Flag,
                        })
                        .for_each(|x| output.push(x));
                }

                // need at least n bombs in the subset, then rest are safe
                let min = min_in_subset(
                    &subset.iter().cloned().cloned().collect(),
                    &mut min_bombs,
                );
                if min == n {
                    covered
                        .iter()
                        .filter(|x| !subset.contains(x))
                        .map(|(col, row)| Action {
                            col: *col,
                            row: *row,
                            action_type: ActionType::Uncover,
                        })
                        .for_each(|x| output.push(x));
                }
            }
        });
    deduplicate(output)
}

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
    let mut legal_bomb_cases = vec![];
    let max_val = (1_u64 << all_covered.len()) - 1;
    'cases: for bombs in 0..=max_val as u32 {
        if bombs.count_ones() != total_num_bombs {
            continue;
        }
        for (n, covered_neighbours) in &boundary_constraints {
            let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
            if *n != num_bombs {
                continue 'cases;
            }
        }
        legal_bomb_cases.push(u32_to_tile_vec(&bombs, &all_covered));
    }
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
    let mut legal_bomb_cases = vec![];
    'cases: for bombs in 0..1 << covered_boundary.len() {
        for (n, covered_neighbours) in &boundary_constraints {
            let num_bombs = (bombs & covered_neighbours).count_ones() as u8;
            if *n != num_bombs {
                continue 'cases;
            }
        }
        legal_bomb_cases.push(u32_to_tile_vec(&bombs, &covered_boundary));
    }
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

    if all_covered.len() <= 32 {
        return get_high_probability_guess_all_covered(all_covered, board);
    } else if covered_boundary.len() <= 26 {
        return get_high_probability_guess_covered_boundary(
            covered_boundary,
            all_covered,
            board,
        );
    }

    // to avoid combinatorics, we just take the tile with the best odds
    let (min_bombs, max_bombs) = get_subset_bounds(board);
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
