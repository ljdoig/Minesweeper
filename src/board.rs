use bevy::prelude::*;
use rand::seq::index::sample;

const NUM_BOMBS: usize = 5;

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

#[derive(Clone, Copy)]
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
    pub tile_states: Vec<TileState>,
    bombs: Vec<bool>,
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
            println!("{} {} is a bomb", index / width, index % width);
            bombs[index] = true;
        }

        Board {
            width,
            height,
            tile_states,
            bombs,
        }
    }

    fn index(&self, col: usize, row: usize) -> usize {
        self.width * row + col
    }

    pub fn tile_state(&self, col: usize, row: usize) -> TileState {
        self.tile_states[self.index(col, row)]
    }

    fn set(&mut self, col: usize, row: usize, state: TileState) {
        let index = self.index(col, row);
        self.tile_states[index] = state;
    }

    pub fn apply_action(&mut self, action: Action) -> ActionResult {
        println!("{:?}", action);
        match action.action_type {
            ActionType::Flag => {
                self.set(action.col, action.row, TileState::Flagged);
            }
            ActionType::Uncover => {
                self.set(action.col, action.row, TileState::UncoveredSafe(0));
            }
        }
        ActionResult::Continue
    }
}
