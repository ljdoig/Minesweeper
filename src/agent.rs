use crate::board::*;
use itertools::Itertools;
use std::collections::HashMap;

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

pub fn get_all_actions(board: &Board) -> Vec<Action> {
    let mut output = get_trivial_actions(board);
    if output.is_empty() {
        output.append(&mut get_non_trivial_actions(board));
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
                if n - num_bombs == num_covered {
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
            for subset in Subset::subsets(&covered, num_covered - 1) {
                let subset =
                    Subset::new(subset.iter().cloned().cloned().collect_vec());
                // need so few bombs in subset that the rest must be bombs
                let max = max_in_subset(&subset, &mut max_bombs);
                let rest_size = (num_covered - subset.0.len()) as u8;
                if max + rest_size == n {
                    covered
                        .iter()
                        .filter(|x| !subset.0.contains(x))
                        .map(|(col, row)| Action {
                            col: *col,
                            row: *row,
                            action_type: ActionType::Flag,
                        })
                        .for_each(|x| output.push(x));
                }

                // need at least n bombs in the subset, then rest are safe
                let min = min_in_subset(&subset, &mut min_bombs);
                if min == n {
                    covered
                        .iter()
                        .filter(|x| !subset.0.contains(x))
                        .map(|(col, row)| Action {
                            col: *col,
                            row: *row,
                            action_type: ActionType::Uncover,
                        })
                        .for_each(|x| output.push(x));
                }
            }
        });
    if !output.is_empty() {
        return deduplicate(output);
    }

    // min_bombs
    //     .iter()
    //     .for_each(|(subset, min)| println!("min in {:?}: {}", subset, min));
    // max_bombs
    //     .iter()
    //     .for_each(|(subset, max)| println!("max in {:?}: {}", subset, max));

    // if we're out of ideas, just permute until we find a compatible option
    let num_bombs = board.num_bombs_left() as u128;
    let boundary_conditions: Vec<_> = (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let num_bombs = num_bombs_around(board, col, row);
                let n = n - num_bombs;
                let covered_neighbours = covered_neighbours(board, col, row);
                if !covered_neighbours.is_empty() {
                    return Some((n, covered_neighbours));
                }
            }
            None
        })
        .collect();
    let mut covered = (0..board.width())
        .cartesian_product(0..board.height())
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Covered)
        .collect_vec();
    let covered_boundary = covered
        .iter()
        .filter(|(col, row)| {
            !uncovered_neighbours(board, *col, *row).is_empty()
        })
        .cloned()
        .collect_vec();
    if covered_boundary.len() > 20 {
        // to avoid combinatorics, we just take the tile with the best odds
        let (col, row) = covered_boundary
            .iter()
            .min_by_key(|(col, row)| {
                min_bombs
                    .iter()
                    .filter(|(subset, _)| {
                        subset.0.contains(&(*col, *row))
                            && min_bombs.get(subset) == max_bombs.get(subset)
                    })
                    .map(|(subset, n)| {
                        ((*n as f64 / subset.0.len() as f64) * 10000.0) as usize
                    })
                    .max()
                    .unwrap()
            })
            .unwrap();
        println!("Guessing: {col}, {row}");
        return vec![Action {
            col: *col,
            row: *row,
            action_type: ActionType::Uncover,
        }];
    }
    // which tiles to check for bombs - either all uncovered or just boundary
    let combinations = if covered.len() > 25 {
        covered = covered_boundary;
        Subset::subsets(&covered, covered.len())
    } else {
        covered
            .iter()
            .combinations(num_bombs as usize)
            .collect_vec()
    };

    let mut legal_bomb_combos = vec![];
    'combos: for bombs in combinations {
        for (n, covered_neighbours) in &boundary_conditions {
            let num_bombs = covered_neighbours
                .iter()
                .filter(|neighbour| bombs.contains(neighbour))
                .count();
            if *n != num_bombs as u8 {
                continue 'combos;
            }
        }
        legal_bomb_combos.push(bombs);
    }
    let (col, row) = covered
        .iter()
        .max_by_key(|tile| {
            legal_bomb_combos
                .iter()
                .filter(|bombs| !bombs.contains(&tile))
                .count()
        })
        .unwrap();
    println!("Best odds from iterating: {col}, {row}");
    vec![Action {
        col: *col,
        row: *row,
        action_type: ActionType::Uncover,
    }]
}

fn max_in_subset(tiles: &Subset, max_bombs: &mut HashMap<Subset, u8>) -> u8 {
    let mut smallest_max = if let Some(&max) = max_bombs.get(tiles) {
        max
    } else {
        tiles.0.len() as u8
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.0.len() <= 2 {
        return smallest_max;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.0.len().saturating_sub(1);
    for subset in Subset::subsets(&tiles.0, max_size) {
        let subset = Subset::new(subset.iter().cloned().cloned().collect_vec());
        if let Some(&sub_max) = max_bombs.get(&subset) {
            let rest = tiles.not_in(&subset);
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

fn min_in_subset(tiles: &Subset, min_bombs: &mut HashMap<Subset, u8>) -> u8 {
    let mut biggest_min = if let Some(&min) = min_bombs.get(tiles) {
        min
    } else {
        0
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.0.len() <= 2 {
        return biggest_min;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.0.len().saturating_sub(1);
    for subset in Subset::subsets(&tiles.0, max_size) {
        let subset = Subset::new(subset.iter().cloned().cloned().collect_vec());
        if let Some(&sub_min) = min_bombs.get(&subset) {
            let rest = tiles.not_in(&subset);
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
) -> (HashMap<Subset, u8>, HashMap<Subset, u8>) {
    let mut min_bombs: HashMap<Subset, u8> = HashMap::new();
    let mut max_bombs: HashMap<Subset, u8> = HashMap::new();
    for _ in 0..3 {
        update_subset_bounds(board, &mut min_bombs, &mut max_bombs);
    }
    (min_bombs, max_bombs)
}

fn update_subset_bounds(
    board: &Board,
    min_bombs: &mut HashMap<Subset, u8>,
    max_bombs: &mut HashMap<Subset, u8>,
) {
    for col in 0..board.width() {
        for row in 0..board.height() {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let n = n - num_bombs_around(board, col, row);
                let covered = covered_neighbours(board, col, row);
                let num_covered = covered.len();
                let covered_subset = Subset::new(covered.clone());
                for subset in Subset::subsets(&covered, num_covered) {
                    let subset = Subset::new(
                        subset.iter().cloned().cloned().collect_vec(),
                    );
                    // rule 1: at most n bombs in all subsets around the tile
                    if subset.0.len() > n as usize {
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
                    let rest = covered_subset.not_in(&subset);
                    let max_omitted = max_in_subset(&rest, max_bombs);
                    if n > max_omitted {
                        if let Some(min) = min_bombs.get(&subset) {
                            if n > *min {
                                min_bombs.insert(subset, n - max_omitted);
                            }
                        } else {
                            min_bombs.insert(subset, n - max_omitted);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Hash, Debug, Eq, PartialEq, Clone)]
struct Subset(Vec<(usize, usize)>);

impl Subset {
    fn new(mut elts: Vec<(usize, usize)>) -> Subset {
        elts.sort();
        Subset(elts)
    }

    fn subsets(
        elts: &[(usize, usize)],
        max_size: usize,
    ) -> Vec<Vec<&(usize, usize)>> {
        (2..=max_size)
            .flat_map(|k| elts.iter().combinations(k))
            .collect()
    }

    fn not_in(&self, other: &Subset) -> Subset {
        let elts = self
            .0
            .iter()
            .filter(|x| !other.0.contains(x))
            .cloned()
            .collect();
        Subset::new(elts)
    }
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
