use bevy::prelude::*;
use rand::seq::index::sample;

const NUM_BOMBS: usize = 10;
pub const GRID_SIZE: (usize, usize) = (10, 10);

#[derive(Debug)]
pub struct Action {
    pub col: usize,
    pub row: usize,
    pub action_type: ActionType,
}

#[derive(Debug)]
pub enum ActionType {
    Flag,
    Uncover,
}

pub enum ActionResult {
    Win,
    Lose,
    Continue,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TileState {
    Covered,
    Flagged,
    UncoveredBomb,
    UncoveredSafe(u8),
}

#[derive(Component)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    tile_states: Vec<TileState>,
    bombs: Vec<bool>,
    num_bombs_left: usize,
}

impl Board {
    pub fn new((width, height): (usize, usize)) -> Board {
        let tile_states = vec![TileState::Covered; width * height];
        let mut bombs = vec![false; width * height];

        // Randomly sample grid tiles without replacement
        let mut rng = rand::thread_rng();
        let sample = sample(&mut rng, width * height, NUM_BOMBS).into_vec();

        // Mark the corresponding tiles as bombs
        for &index in &sample {
            bombs[index] = true;
        }

        Board {
            width,
            height,
            tile_states,
            bombs,
            num_bombs_left: NUM_BOMBS,
        }
    }

    fn index(&self, col: usize, row: usize) -> usize {
        self.width * row + col
    }

    fn bomb(&self, col: usize, row: usize) -> bool {
        self.bombs[self.index(col, row)]
    }

    pub fn tile_state(&self, col: usize, row: usize) -> TileState {
        self.tile_states[self.index(col, row)]
    }

    fn set(&mut self, col: usize, row: usize, state: TileState) {
        let index = self.index(col, row);
        self.tile_states[index] = state;
    }

    fn on_board(&self, col: usize, row: usize) -> bool {
        col < self.width && row < self.height
    }

    fn neighbours(&mut self, col: usize, row: usize) -> Vec<(usize, usize)> {
        let mut neighbours = vec![];
        for neighbour_col in col.saturating_sub(1)..=col + 1 {
            for neighbour_row in row.saturating_sub(1)..=row + 1 {
                if self.on_board(neighbour_col, neighbour_row)
                    && !(neighbour_col == col && neighbour_row == row)
                {
                    neighbours.push((neighbour_col, neighbour_row))
                }
            }
        }
        neighbours
    }

    fn num_bombs_around(&mut self, col: usize, row: usize) -> u8 {
        self.neighbours(col, row)
            .into_iter()
            .filter(|(col, row)| self.bomb(*col, *row))
            .count() as u8
    }

    fn uncover_safe(&mut self, col: usize, row: usize) {
        let num_bombs = self.num_bombs_around(col, row);
        self.set(col, row, TileState::UncoveredSafe(num_bombs));
        if num_bombs == 0 {
            for (col, row) in self.neighbours(col, row) {
                if self.tile_state(col, row) == TileState::Covered {
                    self.uncover_safe(col, row)
                }
            }
        }
    }

    fn uncover_bombs(&mut self) {
        for col in 0..self.width {
            for row in 0..self.height {
                if self.bomb(col, row) {
                    self.set(col, row, TileState::UncoveredBomb);
                }
            }
        }
    }

    fn uncover_remaining(&mut self) {
        for col in 0..self.width {
            for row in 0..self.height {
                if !self.bomb(col, row) {
                    self.uncover_safe(col, row);
                }
            }
        }
    }

    pub fn apply_action(
        &mut self,
        Action {
            col,
            row,
            action_type,
        }: Action,
    ) -> ActionResult {
        match (self.tile_state(col, row), action_type) {
            (TileState::Covered, ActionType::Flag) => {
                self.set(col, row, TileState::Flagged);
                self.num_bombs_left -= 1;
                if self.num_bombs_left == 0 {
                    self.uncover_remaining();
                    return ActionResult::Win;
                }
                println!("Num bombs left: {}", self.num_bombs_left);
            }
            (TileState::Flagged, ActionType::Flag) => {
                self.set(col, row, TileState::Covered);
                self.num_bombs_left += 1;
                println!("Num bombs left: {}", self.num_bombs_left);
            }
            (_, ActionType::Uncover) => {
                if self.bombs[self.index(col, row)] {
                    self.uncover_bombs();
                    return ActionResult::Lose;
                } else {
                    self.uncover_safe(col, row);
                }
            }
            _ => {}
        }
        ActionResult::Continue
    }
}
