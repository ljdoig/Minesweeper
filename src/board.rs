use bevy::prelude::*;
use rand::{rngs::StdRng, seq::index::sample, Rng, SeedableRng};

use crate::TilePos;

const NUM_BOMBS: usize = 99;
pub const GRID_SIZE: (usize, usize) = (30, 16);

// const NUM_BOMBS: usize = 40;
// pub const GRID_SIZE: (usize, usize) = (16, 16);

#[derive(Debug, PartialEq)]
pub struct Action {
    pub pos: TilePos,
    pub action_type: ActionType,
}

impl Action {
    pub fn uncover(pos: TilePos) -> Action {
        Action {
            pos,
            action_type: ActionType::Uncover,
        }
    }
    pub fn flag(pos: TilePos) -> Action {
        Action {
            pos,
            action_type: ActionType::Flag,
        }
    }
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
    Misflagged,
}

#[derive(Component, Clone)]
pub struct Board {
    width: usize,
    height: usize,
    tile_states: Vec<TileState>,
    bombs: Vec<bool>,
    num_bombs_left: isize,
    first_uncovered: bool,
    seed: u64,
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
            seed: 0,
        };
        board.reset(None);
        board
    }

    pub fn reset(&mut self, seed: Option<u64>) {
        self.tile_states = vec![TileState::Covered; self.width * self.height];
        self.sample_bombs(seed);
        self.num_bombs_left = NUM_BOMBS as isize;
        self.first_uncovered = false;
    }

    pub fn tile_state(&self, pos: TilePos) -> TileState {
        self.tile_states[self.index(pos)]
    }

    pub fn tile_states(&self) -> &Vec<TileState> {
        &self.tile_states
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn num_bombs_left(&self) -> isize {
        self.num_bombs_left
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn sample_bombs(&mut self, seed: Option<u64>) {
        self.bombs = vec![false; self.width * self.height];

        // Set board seed randomly if it is not supplied
        self.seed = seed.unwrap_or(rand::thread_rng().gen());

        // maybe try and select a tile that is closest to a boundary tile
        // self.seed = 2148938678238164413;

        // self.seed = 11056703485085464763;

        let mut rng: StdRng = SeedableRng::seed_from_u64(self.seed);
        // Randomly sample grid tiles without replacement
        let sample =
            sample(&mut rng, self.width * self.height, NUM_BOMBS).into_vec();

        // Mark the corresponding tiles as bombs
        for &index in &sample {
            self.bombs[index] = true;
        }
    }

    fn index(&self, TilePos { col, row }: TilePos) -> usize {
        self.width * row + col
    }

    fn bomb(&self, pos: TilePos) -> bool {
        self.bombs[self.index(pos)]
    }

    fn set(&mut self, pos: TilePos, state: TileState) {
        let index = self.index(pos);
        self.tile_states[index] = state;
    }

    pub fn neighbours(&self, TilePos { col, row }: TilePos) -> Vec<TilePos> {
        let mut neighbours = vec![];
        for neighbour_col in col.saturating_sub(1)..=col + 1 {
            for neighbour_row in row.saturating_sub(1)..=row + 1 {
                if neighbour_col < self.width
                    && neighbour_row < self.height
                    && !(neighbour_col == col && neighbour_row == row)
                {
                    neighbours.push(TilePos {
                        col: neighbour_col,
                        row: neighbour_row,
                    })
                }
            }
        }
        neighbours
    }

    fn num_bombs_around(&mut self, pos: TilePos) -> u8 {
        self.neighbours(pos)
            .iter()
            .filter(|&&neighbour| {
                let index = self.index(neighbour);
                self.bombs[index]
            })
            .count() as u8
    }

    fn uncover_first(&mut self, pos: TilePos) {
        while self.num_bombs_around(pos) > 0 || self.bomb(pos) {
            self.sample_bombs(None);
        }
        println!("Board seed: {}", self.seed);
        self.uncover_safe(pos);
    }

    fn uncover_safe(&mut self, pos: TilePos) {
        let num_bombs = self.num_bombs_around(pos);
        self.set(pos, TileState::UncoveredSafe(num_bombs));
        if num_bombs == 0 {
            for neighbour in self.neighbours(pos) {
                if self.tile_state(neighbour) == TileState::Covered {
                    self.uncover_safe(neighbour)
                }
            }
        }
    }

    fn uncover_loss(&mut self, pos: TilePos) {
        for col in 0..self.width {
            for row in 0..self.height {
                let pos = TilePos { col, row };
                let flagged = self.tile_state(pos) == TileState::Flagged;
                if self.bomb(pos) && !flagged {
                    self.set(pos, TileState::UncoveredBomb);
                } else if !self.bomb(pos) && flagged {
                    self.set(pos, TileState::Misflagged);
                }
            }
        }
        self.set(pos, TileState::ExplodedBomb);
    }

    fn flag_remaining(&mut self) {
        for col in 0..self.width {
            for row in 0..self.height {
                let pos = TilePos { col, row };
                if self.bomb(pos) {
                    self.set(pos, TileState::Flagged);
                }
            }
        }
    }

    fn check_win(&self) -> bool {
        for col in 0..self.width {
            for row in 0..self.height {
                let pos = TilePos { col, row };
                // if there is a safe tile yet to be uncovered, haven't won yet
                let safe = !self.bomb(pos);
                match self.tile_state(pos) {
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
        Action { pos, action_type }: Action,
    ) -> ActionResult {
        match (self.tile_state(pos), action_type) {
            // flag
            (TileState::Covered, ActionType::Flag) => {
                self.set(pos, TileState::Flagged);
                self.num_bombs_left -= 1;
            }
            // unflag
            (TileState::Flagged, ActionType::Flag) => {
                self.set(pos, TileState::Covered);
                self.num_bombs_left += 1;
            }
            // uncover
            (_, ActionType::Uncover) => {
                if !self.first_uncovered {
                    self.uncover_first(pos);
                    self.first_uncovered = true;
                } else if self.bombs[self.index(pos)] {
                    self.uncover_loss(pos);
                    return ActionResult::Lose;
                } else {
                    self.uncover_safe(pos);
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
