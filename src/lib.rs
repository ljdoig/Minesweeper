use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};
use instant::Instant;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fmt::{self, Display, Formatter};

// redirect println! to console.log in wasm
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[cfg(target_family = "wasm")]
custom_print::define_macros!({ cprintln }, concat, unsafe fn (crate::log)(&str));

#[cfg(target_family = "wasm")]
macro_rules! println { ($($args:tt)*) => { cprintln!($($args)*); } }

mod actions;
mod board;
pub mod setup;

use actions::{agent, *};
use board::*;
use setup::{resize, setup, UISizing};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_state::<AgentState>()
            .add_state::<Difficulty>()
            .add_systems(Startup, setup)
            .add_systems(Update, close_on_esc)
            .add_systems(
                Update,
                check_bot_action.run_if(in_state(GameState::Game)),
            )
            .add_systems(
                Update,
                check_player_action.run_if(
                    in_state(GameState::Game)
                        .and_then(in_state(AgentState::Resting)),
                ),
            )
            .add_systems(PostUpdate, (check_restart, check_change_difficulty))
            .add_systems(PostUpdate, resize.after(check_change_difficulty))
            .add_systems(
                Last,
                (sync_board_with_tile_sprites, sync_bomb_counter),
            );
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Game,
    GameOver,
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AgentState {
    #[default]
    Resting,
    Thinking,
}

#[derive(
    States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, clap::ValueEnum,
)]
pub enum Difficulty {
    Easy,
    Medium,
    #[default]
    Hard,
}

impl Display for Difficulty {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(format!("{:?}", self).as_ref())
    }
}

impl Difficulty {
    pub fn num_bombs(&self) -> usize {
        match self {
            Difficulty::Easy => 10,
            Difficulty::Medium => 40,
            Difficulty::Hard => 99,
        }
    }

    pub fn grid_size(&self) -> (usize, usize) {
        match self {
            Difficulty::Easy => (10, 10),
            Difficulty::Medium => (16, 16),
            Difficulty::Hard => (30, 16),
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct Record {
    win: usize,
    loss: usize,
    dnf: usize,
    total_bombs_cleared: usize,
    total_bombs: usize,
    difficulty: Difficulty,
}

impl Record {
    fn new(difficulty: Difficulty) -> Self {
        Record {
            difficulty,
            ..default()
        }
    }
    fn win_rate(&self) -> f64 {
        self.win as f64 / (self.win + self.loss + self.dnf) as f64
    }

    fn clearance_rate(&self) -> f64 {
        self.total_bombs_cleared as f64 / self.total_bombs as f64
    }
}

impl Display for Record {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let string = format!(
            "{}-{}-{} on {:?} ({:.2}% win rate, {:.2}% bombs cleared)",
            self.win,
            self.loss,
            self.dnf,
            self.difficulty,
            100.0 * self.win_rate(),
            100.0 * self.clearance_rate(),
        );
        f.write_str(string.as_ref())
    }
}

fn tile_sheet_index(state: TileState) -> usize {
    match state {
        TileState::Covered => 0,
        TileState::Flagged => 1,
        TileState::ExplodedBomb => 14,
        TileState::UncoveredBomb => 2,
        TileState::UncoveredSafe(n) => 3 + n as usize,
        TileState::Misflagged => 13,
    }
}

fn digit_sheet_index(c: char) -> usize {
    if let Some(x) = c.to_digit(10) {
        return x as usize;
    }
    match c {
        '-' => 10,
        ' ' => 11,
        _ => panic!(),
    }
}

#[derive(Component)]
pub struct BombCounterDigit;

fn sync_bomb_counter(
    q_board: Query<&Board>,
    mut digits: Query<(&mut TextureAtlasSprite, &BombCounterDigit)>,
) {
    if let Ok(board) = q_board.get_single() {
        let display_string = format!("{:#03}", board.num_bombs_left());
        let iter = display_string
            .chars()
            .map(digit_sheet_index)
            .zip(digits.iter_mut());
        for (index, (mut sprite, _)) in iter {
            sprite.index = index;
        }
    }
}

fn sync_board_with_tile_sprites(
    q_board: Query<&Board>,
    mut q_tile_sprites: Query<(&mut TextureAtlasSprite, &TilePos)>,
    app_state: ResMut<State<GameState>>,
    agent_state: Res<State<AgentState>>,
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    ui_sizing: Res<UISizing>,
) {
    if let Ok(board) = q_board.get_single() {
        // check if mouse is down over a tile
        let mut pressed = None;
        if buttons.pressed(MouseButton::Left) {
            if let Some(position) = q_windows.single().cursor_position() {
                pressed = ui_sizing.clicked_tile_pos(position);
            }
        };
        // update tile appearence
        for (mut sprite, &pos) in &mut q_tile_sprites {
            let tile_state = board.tile_state(pos);
            if let Some(pressed_pos) = pressed {
                if matches!(app_state.get(), GameState::Game)
                    && matches!(tile_state, TileState::Covered)
                    && matches!(**agent_state, AgentState::Resting)
                    && pos == pressed_pos
                {
                    let index = tile_sheet_index(TileState::UncoveredSafe(0));
                    sprite.index = index;
                    continue;
                }
            }
            let index = tile_sheet_index(tile_state);
            sprite.index = index;
        }
    }
}

pub fn simulate_n_games(n: usize, difficulty: Difficulty, seed: u64) {
    println!("Simulating {n} games on {difficulty}:\n");
    let mut record = Record::new(difficulty);
    let mut longest_game: f32 = 0.0;
    let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
    let start = Instant::now();
    for i in 1..=n {
        let mut board = Board::new(difficulty, Some(rng.gen::<u64>()));
        let game_start = Instant::now();
        'game: loop {
            for action in agent::get_all_actions(&board) {
                let result = board.apply_action(action);
                match result {
                    ActionResult::Win | ActionResult::Lose => {
                        end_game(&mut record, &result, &board);
                        break 'game;
                    }
                    _ => {}
                }
            }
        }
        longest_game = longest_game.max(game_start.elapsed().as_secs_f32());
        println!(
            "Game {i} finished in {:.2}s (seed: {})",
            game_start.elapsed().as_secs_f32(),
            board.seed()
        );
        println!(
            "{:.2}s per game, {:.2}s in total, longest game took {:.2}s",
            start.elapsed().as_secs_f32() / i as f32,
            start.elapsed().as_secs_f32(),
            longest_game,
        );
        println!(
            "Simulation {:.2}% complete\n",
            100.0 * (i as f64 / n as f64)
        );
    }
}
