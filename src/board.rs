use crate::Difficulty;
use bevy::prelude::*;
use rand::{rngs::StdRng, seq::index::sample, Rng, SeedableRng};

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

impl TileState {
    pub fn sheet_index(&self) -> usize {
        match self {
            TileState::Covered => 0,
            TileState::Flagged => 1,
            TileState::UncoveredBomb => 2,
            TileState::UncoveredSafe(n) => 3 + *n as usize,
            TileState::Misflagged => 12,
            TileState::ExplodedBomb => 13,
        }
    }
}

#[derive(
    Component, Debug, PartialEq, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct TilePos {
    pub col: usize,
    pub row: usize,
}

impl TilePos {
    pub fn squared_distance(self, other: TilePos) -> usize {
        self.col.abs_diff(other.col).pow(2)
            + self.row.abs_diff(other.row).pow(2)
    }
}

#[derive(Component, Clone)]
pub struct Board {
    width: usize,
    height: usize,
    tile_states: Vec<TileState>,
    bombs: Vec<bool>,
    num_bombs_left: isize,
    num_bombs_total: usize,
    first_uncovered: bool,
    seed: u64,
}

impl Board {
    pub fn new(difficulty: Difficulty, seed: Option<u64>) -> Board {
        let (width, height) = difficulty.grid_size();
        let mut board = Board {
            width,
            height,
            tile_states: vec![],
            bombs: vec![],
            num_bombs_left: 0,
            num_bombs_total: difficulty.num_bombs(),
            first_uncovered: false,
            seed: 0,
        };
        board.reset(seed);
        board
    }

    pub fn reset(&mut self, seed: Option<u64>) {
        println!("Beginning game with {} bombs", self.num_bombs_total);
        self.tile_states = vec![TileState::Covered; self.width * self.height];
        self.sample_bombs(seed);
        self.num_bombs_left = self.num_bombs_total as isize;
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

    pub fn num_bombs_total(&self) -> usize {
        self.num_bombs_total
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn first_uncovered(&self) -> bool {
        self.first_uncovered
    }

    fn sample_bombs(&mut self, seed: Option<u64>) {
        self.bombs = vec![false; self.width * self.height];

        // Set board seed randomly if it is not supplied
        self.seed = seed.unwrap_or(rand::thread_rng().gen());

        let mut rng: StdRng = SeedableRng::seed_from_u64(self.seed);
        // Randomly sample grid tiles without replacement
        let sample =
            sample(&mut rng, self.width * self.height, self.num_bombs_total)
                .into_vec();

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
        // flagging or unflagging changes num_bombs_left
        self.num_bombs_left += match (self.tile_states[index], state) {
            (x, y) if x == y => return,
            (TileState::Covered, TileState::Flagged) => -1,
            (TileState::Flagged, _) => 1,
            _ => 0,
        };
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
            self.seed += 1;
            self.sample_bombs(Some(self.seed));
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
            }
            // unflag
            (TileState::Flagged, ActionType::Flag) => {
                self.set(pos, TileState::Covered);
            }
            // uncover
            (TileState::Covered, ActionType::Uncover) => {
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
            (TileState::Flagged, ActionType::Uncover) => {}
            _ => {}
        }
        ActionResult::Continue
    }
}
