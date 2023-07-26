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

fn num_covered_around(board: &Board, col: usize, row: usize) -> u8 {
    covered_neighbours(board, col, row).len() as u8
}

pub fn get_all_actions(board: &Board) -> Vec<Action> {
    let mut output = get_trivial_actions(board);
    output.append(&mut get_non_trivial_actions(board));
    deduplicate(output)
}

pub fn get_trivial_actions(board: &Board) -> Vec<Action> {
    if board.tile_states.iter().all(|&x| x == TileState::Covered) {
        let action = Action {
            col: board.width / 2,
            row: board.height / 2,
            action_type: ActionType::Uncover,
        };
        return vec![action];
    }

    let mut output = vec![];
    for col in 0..board.width {
        for row in 0..board.height {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let num_bombs = num_bombs_around(&board, col, row);
                let num_covered = num_covered_around(&board, col, row);
                // uncover all neighbours
                if num_bombs == n {
                    covered_neighbours(&board, col, row)
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
    let (min_bombs, max_bombs) = get_subset_bounds(&board);
    // check each uncovered tile to see if we have helpful adjacent subsets
    for col in 0..board.width {
        for row in 0..board.height {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let n = n - num_bombs_around(&board, col, row);
                let covered = covered_neighbours(&board, col, row);
                let num_covered = covered.len();
                if num_covered == 0 {
                    continue;
                }
                for subset in Subset::subsets(&covered, num_covered - 1) {
                    if let Some(max) = max_bombs.get(&subset) {
                        // if max bound is low enough we flag the rest
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
                    }
                    if let Some(min) = min_bombs.get(&subset) {
                        // if min bound is high enough we flag the rest
                        if *min == n {
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
                }
            }
        }
    }

    deduplicate(output)
}

fn get_subset_bounds(
    board: &Board,
) -> (HashMap<Subset, u8>, HashMap<Subset, u8>) {
    let mut min_bombs: HashMap<Subset, u8> = HashMap::new();
    let mut max_bombs: HashMap<Subset, u8> = HashMap::new();
    for col in 0..board.width {
        for row in 0..board.height {
            if let TileState::UncoveredSafe(n) = board.tile_state(col, row) {
                let n = n - num_bombs_around(&board, col, row);
                let covered = covered_neighbours(&board, col, row);
                let num_covered = covered.len();
                for subset in Subset::subsets(&covered, num_covered) {
                    // record max number of bombs in subset
                    if subset.0.len() > n as usize {
                        if let Some(max) = max_bombs.get(&subset) {
                            if n < *max {
                                max_bombs.insert(subset.clone(), n);
                            }
                        } else {
                            max_bombs.insert(subset.clone(), n);
                        }
                    }
                    // record min number of bombs in subset
                    let num_omitted = (num_covered - subset.0.len()) as u8;
                    if n > num_omitted {
                        if let Some(min) = min_bombs.get(&subset) {
                            if n > *min {
                                min_bombs.insert(subset, n - num_omitted);
                            }
                        } else {
                            min_bombs.insert(subset, n - num_omitted);
                        }
                    }
                }
            }
        }
    }
    // max_bombs
    //     .iter()
    //     .for_each(|(subset, max)| println!("max {max} in {:?}", subset));
    // min_bombs
    //     .iter()
    //     .for_each(|(subset, min)| println!("min {min} in {:?}", subset));

    (min_bombs, max_bombs)
}

#[derive(Hash, Debug, Eq, PartialEq, Clone)]
struct Subset(Vec<(usize, usize)>);

impl Subset {
    fn new(mut elts: Vec<(usize, usize)>) -> Subset {
        elts.sort();
        Subset(elts)
    }

    fn subsets(elts: &Vec<(usize, usize)>, max_size: usize) -> Vec<Subset> {
        (2..=max_size)
            .flat_map(|k| elts.iter().combinations(k))
            .map(|combination| {
                Subset::new(combination.iter().cloned().cloned().collect())
            })
            .collect()
    }

    // fn partitions(elts: Vec<(usize, usize)>) -> Vec<(Subset, Subset)> {
    //     Self::subsets(elts, min_size).iter().map(f)
    // }
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
