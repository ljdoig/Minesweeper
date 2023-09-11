use crate::{board::*, Action, TilePos};

pub mod deductions;
pub mod guesses;

use deductions::get_non_trivial_actions;
use guesses::make_guess;

pub fn num_bombs_around(board: &Board, pos: TilePos) -> u8 {
    board
        .neighbours(pos)
        .iter()
        .filter(|&&pos| board.tile_state(pos) == TileState::Flagged)
        .count() as u8
}

pub fn covered_neighbours(board: &Board, pos: TilePos) -> Vec<TilePos> {
    board
        .neighbours(pos)
        .iter()
        .filter(|&&pos| board.tile_state(pos) == TileState::Covered)
        .cloned()
        .collect()
}

pub fn uncovered_neighbours(board: &Board, pos: TilePos) -> Vec<TilePos> {
    board
        .neighbours(pos)
        .iter()
        .filter(|&&pos| {
            matches!(board.tile_state(pos), TileState::UncoveredSafe(_))
        })
        .cloned()
        .collect()
}

pub fn num_covered_around(board: &Board, pos: TilePos) -> u8 {
    covered_neighbours(board, pos).len() as u8
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
        output.push(make_guess(board));
    }
    deduplicate(output)
}

 fn get_trivial_actions(board: &Board) -> Vec<Action> {
    let mut output = vec![];
    if board.tile_states().iter().all(|&x| x == TileState::Covered) {
        // first guess
        let pos = TilePos {
            col: 2,
            row: board.height() / 2,
        };
        return vec![Action::uncover(pos)];
    } else if board.num_bombs_left() == 0 {
        // no bombs left, just uncover last uncovered tiles
        for col in 0..board.width() {
            for row in 0..board.height() {
                let pos = TilePos { col, row };
                if board.tile_state(pos) == TileState::Covered {
                    output.push(Action::uncover(pos));
                }
            }
        }
        return output;
    }

    for col in 0..board.width() {
        for row in 0..board.height() {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                let num_bombs = num_bombs_around(board, pos);
                let num_covered = num_covered_around(board, pos);
                // uncover all neighbours
                if num_bombs == n {
                    covered_neighbours(board, pos)
                        .into_iter()
                        .map(Action::uncover)
                        .for_each(|x| output.push(x));
                }
                // flag all neighbours
                if n.saturating_sub(num_bombs) == num_covered {
                    covered_neighbours(board, pos)
                        .into_iter()
                        .map(Action::flag)
                        .for_each(|x| output.push(x));
                }
            }
        }
    }
    deduplicate(output)
}
