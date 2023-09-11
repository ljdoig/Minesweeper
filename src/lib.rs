use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};
use instant::Instant;
use itertools::Itertools;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fmt::{self, Display, Formatter};
use std::slice::Iter;

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
            .add_systems(First, (update_bot_buttons, update_face_buttons))
            .add_systems(Update, (check_bot_action, close_on_esc))
            .add_systems(
                Update,
                check_player_action.run_if(
                    in_state(GameState::Playing)
                        .and_then(in_state(AgentState::Resting)),
                ),
            )
            .add_systems(PostUpdate, check_restart)
            .add_systems(PostUpdate, resize.after(check_restart))
            .add_systems(
                Last,
                (sync_board_with_tile_sprites, sync_bomb_counter),
            );
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Won,
    Lost,
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AgentState {
    #[default]
    Resting,
    Thinking,
    ThinkingOneMoveOnly,
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
    pub fn iter() -> Iter<'static, Difficulty> {
        static VALS: [Difficulty; 3] =
            [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
        VALS.iter()
    }

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

#[derive(Component)]
pub struct Button {
    location: Rect,
}

impl Button {
    fn pressed(
        &self,
        window: &Window,
        mouse: &Res<Input<MouseButton>>,
    ) -> bool {
        mouse.pressed(MouseButton::Left) && self.mouse_over(window)
    }

    fn just_released(
        &self,
        window: &Window,
        mouse: &Res<Input<MouseButton>>,
    ) -> bool {
        mouse.just_released(MouseButton::Left) && self.mouse_over(window)
    }

    fn mouse_over(&self, window: &Window) -> bool {
        if let Some(mouse_from_corner) = window.cursor_position() {
            let centre = Vec2::new(window.width(), window.height()) / 2.0;
            let mouse_pos = (mouse_from_corner - centre) * Vec2::new(1.0, -1.0);
            if self.location.contains(mouse_pos) {
                return true;
            }
        }
        false
    }
}

#[derive(Component)]
pub struct BotButton {
    bot_effect: AgentState,
    pressed_index: usize,
    unpressed_index: usize,
}

#[derive(Component)]
pub struct FaceButton(Difficulty);

impl FaceButton {
    fn sheet_index(&self, state: FaceButtonState) -> usize {
        let difficulty = self.0;
        let offset = Difficulty::iter()
            .find_position(|x| **x == difficulty)
            .unwrap()
            .0
            * 5;
        offset
            + match state {
                FaceButtonState::Unpressed => 0,
                FaceButtonState::Pressed => 1,
                FaceButtonState::Playing => 2,
                FaceButtonState::Win => 3,
                FaceButtonState::Loss => 4,
            }
    }
}

#[derive(Clone, Copy)]
pub enum FaceButtonState {
    Unpressed,
    Pressed,
    Playing,
    Win,
    Loss,
}

fn update_bot_buttons(
    mut q_buttons: Query<(&mut TextureAtlasSprite, &Button, &BotButton)>,
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    for (mut sprite, button, bot_button) in q_buttons.iter_mut() {
        sprite.index = bot_button.unpressed_index;
        if button.pressed(q_windows.single(), &mouse) {
            sprite.index = bot_button.pressed_index;
        }
    }
}

fn update_face_buttons(
    mut q_face_buttons: Query<(&mut TextureAtlasSprite, &Button, &FaceButton)>,
    app_state: ResMut<State<GameState>>,
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    for (mut sprite, button, face_button) in q_face_buttons.iter_mut() {
        let face_button_state = if button.pressed(q_windows.single(), &mouse) {
            FaceButtonState::Pressed
        } else {
            match app_state.get() {
                GameState::Won => FaceButtonState::Win,
                GameState::Lost => FaceButtonState::Loss,
                GameState::Playing => FaceButtonState::Unpressed,
            }
        };
        sprite.index = face_button.sheet_index(face_button_state)
    }
}

#[derive(Component)]
pub struct BombCounterDigit;

impl BombCounterDigit {
    fn sheet_index(c: char) -> usize {
        if let Some(x) = c.to_digit(10) {
            return x as usize;
        }
        match c {
            '-' => 10,
            ' ' => 11,
            _ => panic!(),
        }
    }
}

fn sync_bomb_counter(
    q_board: Query<&Board>,
    mut q_digits: Query<(&mut TextureAtlasSprite, &BombCounterDigit)>,
) {
    if let Ok(board) = q_board.get_single() {
        format!("{:#03}", board.num_bombs_left())
            .chars()
            .map(BombCounterDigit::sheet_index)
            .zip(q_digits.iter_mut())
            .for_each(|(index, (mut sprite, _))| {
                sprite.index = index;
            });
    }
}

fn sync_board_with_tile_sprites(
    q_board: Query<&Board>,
    mut q_tile_sprites: Query<(&mut TextureAtlasSprite, &TilePos)>,
    app_state: ResMut<State<GameState>>,
    agent_state: Res<State<AgentState>>,
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    ui_sizing: Res<UISizing>,
    mut q_face_buttons: Query<
        (&mut TextureAtlasSprite, &FaceButton),
        Without<TilePos>,
    >,
) {
    if let Ok(board) = q_board.get_single() {
        // check if mouse is down over a tile
        let mut pressed = None;
        if mouse.pressed(MouseButton::Left) {
            if let Some(position) = q_windows.single().cursor_position() {
                pressed = ui_sizing.clicked_tile_pos(position);
            }
        };
        // update tile appearence
        for (mut sprite, &pos) in &mut q_tile_sprites {
            let tile_state = board.tile_state(pos);
            if let Some(pressed_pos) = pressed {
                if matches!(app_state.get(), GameState::Playing)
                    && matches!(tile_state, TileState::Covered)
                    && matches!(**agent_state, AgentState::Resting)
                    && pos == pressed_pos
                {
                    let index = TileState::UncoveredSafe(0).sheet_index();
                    sprite.index = index;
                    for (mut sprite, button) in &mut q_face_buttons {
                        sprite.index =
                            button.sheet_index(FaceButtonState::Playing);
                    }
                    continue;
                }
            }
            let index = tile_state.sheet_index();
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
            "{}ms per game, {:.2}s in total, longest game took {:.2}s",
            (1000.0 * start.elapsed().as_secs_f32() / i as f32) as usize,
            start.elapsed().as_secs_f32(),
            longest_game,
        );
        println!(
            "Simulation {:.2}% complete\n",
            100.0 * (i as f64 / n as f64)
        );
    }
}
