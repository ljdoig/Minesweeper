use crate::board::*;

fn num_bombs_around(board: &mut Board, col: usize, row: usize) -> u8 {
    board
        .neighbours(col, row)
        .into_iter()
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Flagged)
        .count() as u8
}

fn num_covered_around(board: &mut Board, col: usize, row: usize) -> u8 {
    board
        .neighbours(col, row)
        .into_iter()
        .filter(|(col, row)| board.tile_state(*col, *row) == TileState::Covered)
        .count() as u8
}

pub fn get_actions(mut board: Board) -> Vec<Action> {
    if (&board.tile_states)
        .into_iter()
        .all(|&x| x == TileState::Covered)
    {
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
                let num_bombs = num_bombs_around(&mut board, col, row);
                let num_covered = num_covered_around(&mut board, col, row);
                // uncover all neighbours
                if num_bombs == n {
                    board
                        .neighbours(col, row)
                        .into_iter()
                        .filter(|(col, row)| {
                            board.tile_state(*col, *row) == TileState::Covered
                        })
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

fn deduplicate(output: Vec<Action>) -> Vec<Action> {
    let mut deduplicated = vec![];
    for action in output {
        if !(&deduplicated).into_iter().any(|x: &Action| x == &action) {
            deduplicated.push(action);
        }
    }
    deduplicated
}
