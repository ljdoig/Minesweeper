use crate::board::*;

pub mod deductions;
pub mod guesses;

use deductions::get_non_trivial_actions;
use guesses::get_high_probability_guess;

pub fn num_bombs_around(board: &Board, col: usize, row: usize) -> u8 {
    board
        .neighbours(col, row)
        .iter()
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Flagged)
        .count() as u8
}

pub fn covered_neighbours(
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

pub fn uncovered_neighbours(
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

pub fn num_covered_around(board: &Board, col: usize, row: usize) -> u8 {
    covered_neighbours(board, col, row).len() as u8
}

pub fn deduplicate(output: Vec<Action>) -> Vec<Action> {
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
