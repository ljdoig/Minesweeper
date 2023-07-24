use bevy::prelude::*;
use rand::seq::index::sample;

const NUM_BOMBS: usize = 99;
pub const GRID_SIZE: (usize, usize) = (30, 16);

#[derive(Debug, PartialEq)]
pub struct Action {
    pub col: usize,
    pub row: usize,
    pub action_type: ActionType,
}

#[derive(Debug, PartialEq)]
pub enum ActionType {
    Flag,
    Uncover,
}

#[derive(PartialEq)]
pub enum ActionResult {
    Win,
    Lose,
    Continue,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TileState {
    Covered,
    Flagged,
    ExplodedBomb,
    UncoveredBomb,
    UncoveredSafe(u8),
}

#[derive(Component, Clone)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub tile_states: Vec<TileState>,
    bombs: Vec<bool>,
    num_bombs_left: usize,
    first_uncovered: bool,
}

impl Board {
    pub fn new((width, height): (usize, usize)) -> Board {
        let mut board = Board {
            width,
            height,
            tile_states: vec![],
            bombs: vec![],
            num_bombs_left: 0,
            first_uncovered: false,
        };
        board.reset();
        board
    }

    pub fn reset(&mut self) {
        self.tile_states = vec![TileState::Covered; self.width * self.height];
        self.sample_bombs();
        self.num_bombs_left = NUM_BOMBS;
        self.first_uncovered = false;
    }

    pub fn tile_state(&self, col: usize, row: usize) -> TileState {
        self.tile_states[self.index(col, row)]
    }

    pub fn num_bombs_left(&self) -> usize {
        self.num_bombs_left
    }

    fn sample_bombs(&mut self) {
        self.bombs = vec![false; self.width * self.height];

        // Randomly sample grid tiles without replacement
        let mut rng = rand::thread_rng();
        let sample =
            sample(&mut rng, self.width * self.height, NUM_BOMBS).into_vec();

        // Mark the corresponding tiles as bombs
        for &index in &sample {
            self.bombs[index] = true;
        }
    }

    fn index(&self, col: usize, row: usize) -> usize {
        self.width * row + col
    }

    fn bomb(&self, col: usize, row: usize) -> bool {
        self.bombs[self.index(col, row)]
    }

    fn set(&mut self, col: usize, row: usize, state: TileState) {
        let index = self.index(col, row);
        self.tile_states[index] = state;
    }

    fn on_board(&self, col: usize, row: usize) -> bool {
        col < self.width && row < self.height
    }

    pub fn neighbours(
        &mut self,
        col: usize,
        row: usize,
    ) -> Vec<(usize, usize)> {
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

    fn uncover_first(&mut self, col: usize, row: usize) {
        while self.num_bombs_around(col, row) > 0 {
            self.sample_bombs();
        }
        self.uncover_safe(col, row);
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

    fn uncover_bombs(&mut self, col: usize, row: usize) {
        for col in 0..self.width {
            for row in 0..self.height {
                if self.bomb(col, row) {
                    self.set(col, row, TileState::UncoveredBomb);
                }
            }
        }
        self.set(col, row, TileState::ExplodedBomb);
    }

    fn flag_remaining(&mut self) {
        for col in 0..self.width {
            for row in 0..self.height {
                if self.bomb(col, row) {
                    self.set(col, row, TileState::Flagged);
                }
            }
        }
    }

    fn check_win(&self) -> bool {
        for col in 0..self.width {
            for row in 0..self.height {
                // if there is a safe tile yet to be uncovered, haven't won yet
                let safe = !self.bomb(col, row);
                match self.tile_state(col, row) {
                    TileState::Covered | TileState::Flagged => {
                        if safe {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        true
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
            // flag
            (TileState::Covered, ActionType::Flag) => {
                self.set(col, row, TileState::Flagged);
                self.num_bombs_left -= 1;
            }
            // unflag
            (TileState::Flagged, ActionType::Flag) => {
                self.set(col, row, TileState::Covered);
                self.num_bombs_left += 1;
            }
            // uncover
            (_, ActionType::Uncover) => {
                if !self.first_uncovered {
                    self.uncover_first(col, row);
                    self.first_uncovered = true;
                } else if self.bombs[self.index(col, row)] {
                    self.uncover_bombs(col, row);
                    return ActionResult::Lose;
                } else {
                    self.uncover_safe(col, row);
                    if self.check_win() {
                        self.flag_remaining();
                        return ActionResult::Win;
                    }
                }
            }
            _ => {}
        }
        ActionResult::Continue
    }
}
