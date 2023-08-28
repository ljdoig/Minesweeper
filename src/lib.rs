use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};
use bevy_framepace::{FramepaceSettings, Limiter};
use std::f32::consts::PI;
use std::time::Instant;

mod agent;
mod board;

use board::*;

pub const WINDOW_HEIGHT: f32 = 600.0;
const TILE_SPRITE_SIZE: f32 = 16.0;
const EDGE_PADDING_SIZE: f32 = 12.0;
const TOP_PADDING_SIZE: f32 = 12.0;

const UNSCALED_HEIGHT: f32 = GRID_SIZE.1 as f32 * TILE_SPRITE_SIZE
    + TOP_PADDING_SIZE
    + EDGE_PADDING_SIZE;
const SCALE: f32 = WINDOW_HEIGHT / UNSCALED_HEIGHT;
const TILE_SIZE: f32 = TILE_SPRITE_SIZE * SCALE;
const EDGE_PADDING: f32 = EDGE_PADDING_SIZE * SCALE;
const TOP_PADDING: f32 = TOP_PADDING_SIZE * SCALE;
const BOARD_HEIGHT: f32 = WINDOW_HEIGHT - TOP_PADDING - EDGE_PADDING;
const BOARD_WIDTH: f32 = TILE_SIZE * GRID_SIZE.0 as f32;
pub const WINDOW_WIDTH: f32 = BOARD_WIDTH + 2.0 * EDGE_PADDING;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_state::<AgentState>()
            .add_systems(Startup, setup_game)
            .add_systems(Update, close_on_esc)
            .add_systems(Update, check_restart)
            .add_systems(
                Update,
                (check_bot_action, check_player_action)
                    .run_if(in_state(GameState::Game)),
            )
            .add_systems(
                Last,
                sync_board_with_tile_sprites.after(check_player_action),
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

#[derive(Component, Debug, Default)]
pub struct Record {
    win: usize,
    loss: usize,
    dnf: usize,
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut settings: ResMut<FramepaceSettings>,
) {
    settings.limiter = Limiter::from_framerate(60.0);
    commands.spawn(Camera2dBundle::default());
    let texture_handle =
        asset_server.load("spritesheets/minesweeper_tiles.png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::splat(TILE_SPRITE_SIZE),
        4,
        4,
        None,
        None,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    for col in 0..GRID_SIZE.0 {
        for row in 0..GRID_SIZE.1 {
            let tile_sprite = TilePos { col, row };
            let sprite_sheet_index = sprite_sheet_index(TileState::Covered);
            commands.spawn((
                SpriteSheetBundle {
                    texture_atlas: texture_atlas_handle.clone(),
                    sprite: TextureAtlasSprite::new(sprite_sheet_index),
                    transform: Transform::from_translation(
                        tile_sprite.screen_pos().extend(0.0),
                    )
                    .with_scale(Vec3::splat(SCALE)),
                    ..default()
                },
                tile_sprite,
            ));
        }
    }

    spawn_padding(&mut commands, asset_server);
    let board = Board::new(GRID_SIZE);
    commands.spawn(board);
    commands.spawn(Record::default());
}

fn spawn_padding(commands: &mut Commands, asset_server: Res<AssetServer>) {
    // verticals
    let horizontal_offset =
        Vec2::new(BOARD_WIDTH / 2.0 + EDGE_PADDING / 2.0, 0.0);
    spawn_padding_piece(commands, &asset_server, horizontal_offset, false);
    spawn_padding_piece(commands, &asset_server, -horizontal_offset, false);
    // horizontals
    let board_centre =
        Vec2::new(0.0, WINDOW_HEIGHT / 2.0 - TOP_PADDING - BOARD_HEIGHT / 2.0);
    let vertical_offset =
        Vec2::new(0.0, BOARD_HEIGHT / 2.0 + EDGE_PADDING / 2.0);
    spawn_padding_piece(
        commands,
        &asset_server,
        board_centre + vertical_offset,
        true,
    );
    spawn_padding_piece(
        commands,
        &asset_server,
        board_centre - vertical_offset,
        true,
    );
}

fn spawn_padding_piece(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    centre: Vec2,
    horizontal: bool,
) {
    let (rotation, scale) = if horizontal {
        (
            Quat::from_rotation_z(-PI / 2.0),
            Vec2::new(
                SCALE,
                (BOARD_WIDTH + EDGE_PADDING * 2.0) / TILE_SPRITE_SIZE,
            ),
        )
    } else {
        (
            Quat::IDENTITY,
            Vec2::new(SCALE, WINDOW_HEIGHT / TILE_SPRITE_SIZE),
        )
    };
    commands.spawn(SpriteBundle {
        texture: asset_server.load("padding.png"),
        transform: Transform {
            rotation,
            scale: scale.extend(1.0),
            translation: centre.extend(1.0),
        },
        ..default()
    });
}

#[derive(
    Component, Debug, PartialEq, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct TilePos {
    col: usize,
    row: usize,
}

impl TilePos {
    pub fn new(col: usize, row: usize, board: &Board) -> Option<TilePos> {
        (col < board.width() && row < board.height())
            .then_some(TilePos { col, row })
    }

    pub fn squared_distance(self, other: TilePos) -> usize {
        let col1 = self.col as isize;
        let row1 = self.row as isize;
        let col2 = other.col as isize;
        let row2 = other.row as isize;
        ((col1 - col2).pow(2) + (row1 - row2).pow(2)) as usize
    }
}

impl TilePos {
    fn screen_pos(&self) -> Vec2 {
        let translation_x =
            TILE_SIZE * (self.col as f32 - (GRID_SIZE.0 - 1) as f32 / 2.0);
        let translation_y = TILE_SIZE
            * -(self.row as f32 - (GRID_SIZE.1 - 1) as f32 / 2.0)
            - (TOP_PADDING - EDGE_PADDING) / 2.0;
        Vec2::new(translation_x, translation_y)
    }
}

fn sprite_sheet_index(state: TileState) -> usize {
    match state {
        TileState::Covered => 0,
        TileState::Flagged => 1,
        TileState::ExplodedBomb => 14,
        TileState::UncoveredBomb => 2,
        TileState::UncoveredSafe(n) => 3 + n as usize,
        TileState::Misflagged => 13,
    }
}

fn check_restart(
    keys: Res<Input<KeyCode>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    app_state: ResMut<State<GameState>>,
    mut record_query: Query<&mut Record>,
) {
    let replay = keys.just_pressed(KeyCode::R);
    if keys.just_pressed(KeyCode::Return) || replay {
        let mut record = record_query.get_single_mut().unwrap();
        if matches!(app_state.get(), GameState::Game) {
            end_game(&mut record, &ActionResult::Continue)
        }
        next_app_state.set(GameState::Game);
        next_agent_state.set(AgentState::Resting);
        let mut board = board_query.get_single_mut().unwrap();
        let seed = replay.then_some(board.seed());
        board.reset(seed);
    }
}

fn check_player_action(
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    agent_state: ResMut<State<AgentState>>,
    mut record_query: Query<&mut Record>,
) {
    if matches!(agent_state.get(), AgentState::Thinking) {
        return;
    }
    let mut board = board_query.get_single_mut().unwrap();
    let mut record = record_query.get_single_mut().unwrap();
    if let Some(position) = q_windows.single().cursor_position() {
        let action_type = if buttons.just_released(MouseButton::Left) {
            Some(ActionType::Uncover)
        } else if buttons.just_pressed(MouseButton::Right) {
            Some(ActionType::Flag)
        } else {
            None
        };
        if let Some(action_type) = action_type {
            // this ensures we can't click slightly above the first row/col
            if position.x > EDGE_PADDING && position.y > TOP_PADDING {
                let col = ((position.x - EDGE_PADDING) / TILE_SIZE) as usize;
                let row = ((position.y - TOP_PADDING) / TILE_SIZE) as usize;
                let pos = TilePos::new(col, row, &board);
                if let Some(pos) = pos {
                    if !matches!(
                        board.tile_state(pos),
                        TileState::UncoveredSafe(_)
                    ) {
                        let action = Action { pos, action_type };
                        complete_action(
                            &mut board,
                            action,
                            &mut next_app_state,
                            &mut record,
                        );
                    }
                }
            }
        }
    }
}

fn check_bot_action(
    keys: Res<Input<KeyCode>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    agent_state: ResMut<State<AgentState>>,
    mut record_query: Query<&mut Record>,
) {
    let mut board = board_query.get_single_mut().unwrap();
    let mut record = record_query.get_single_mut().unwrap();
    if keys.just_pressed(KeyCode::Space) {
        next_agent_state.set(AgentState::Thinking)
    }
    if matches!(agent_state.get(), AgentState::Thinking) {
        let actions = agent::get_all_actions(&board);
        if actions.is_empty() {
            next_agent_state.set(AgentState::Resting)
        }
        for action in actions {
            let result = complete_action(
                &mut board,
                action,
                &mut next_app_state,
                &mut record,
            );
            if result != ActionResult::Continue {
                next_agent_state.set(AgentState::Resting);
                return;
            }
        }
    }
    let trivial = keys.just_pressed(KeyCode::Key1);
    let non_trivial = keys.just_pressed(KeyCode::Key2);
    if trivial || non_trivial {
        let mut actions = if trivial {
            agent::get_trivial_actions(&board)
        } else {
            agent::deductions::get_non_trivial_actions(&board)
        };
        while !actions.is_empty() {
            for action in actions {
                let result = complete_action(
                    &mut board,
                    action,
                    &mut next_app_state,
                    &mut record,
                );
                if result != ActionResult::Continue {
                    return;
                }
            }
            actions = if trivial {
                agent::get_trivial_actions(&board)
            } else {
                break;
            };
        }
    }
    if keys.just_pressed(KeyCode::Key3) {
        let action = agent::guesses::make_guess(&board);
        complete_action(&mut board, action, &mut next_app_state, &mut record);
    }
}

fn end_game(record: &mut Record, result: &ActionResult) {
    match result {
        ActionResult::Win => {
            record.win += 1;
            println!("You won!");
        }
        ActionResult::Lose => {
            record.loss += 1;
            println!("You lost");
        }
        ActionResult::Continue => {
            record.dnf += 1;
            println!("You didn't finish the game...");
        }
    }
    let win_rate =
        record.win as f64 / (record.win + record.loss + record.dnf) as f64;
    println!(
        "Record: ({}/{}) ({:.2}%)\n",
        record.win,
        record.loss,
        100.0 * win_rate
    );
}

fn complete_action(
    board: &mut Board,
    action: Action,
    next_app_state: &mut ResMut<NextState<GameState>>,
    record: &mut Record,
) -> ActionResult {
    let result = board.apply_action(action);
    match result {
        ActionResult::Win | ActionResult::Lose => {
            end_game(record, &result);
            next_app_state.set(GameState::GameOver);
        }
        ActionResult::Continue => {}
    }
    result
}

fn sync_board_with_tile_sprites(
    board_query: Query<&Board>,
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TilePos)>,
    app_state: ResMut<State<GameState>>,
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(board) = board_query.get_single() {
        let mut pressed = None;
        if buttons.pressed(MouseButton::Left) {
            if let Some(position) = q_windows.single().cursor_position() {
                if position.x > EDGE_PADDING && position.y > TOP_PADDING {
                    let col = (position.x - EDGE_PADDING) / TILE_SIZE;
                    let row = (position.y - TOP_PADDING) / TILE_SIZE;
                    pressed = TilePos::new(col as usize, row as usize, &board);
                }
            }
        };
        for (mut sprite, &pos) in &mut tile_sprites_query {
            let tile_state = board.tile_state(pos);
            if let Some(pressed_pos) = pressed {
                if matches!(app_state.get(), GameState::Game)
                    && matches!(tile_state, TileState::Covered)
                    && pos == pressed_pos
                {
                    let index = sprite_sheet_index(TileState::UncoveredSafe(0));
                    sprite.index = index;
                    continue;
                }
            }
            let index = sprite_sheet_index(tile_state);
            sprite.index = index;
        }
    }
}

pub fn simulate_n_games(n: usize) {
    println!("Simulating {n} games:\n");
    let mut record = Record::default();
    let mut longest_game: f32 = 0.0;
    let start = Instant::now();
    for i in 1..=n {
        let mut board = Board::new(GRID_SIZE);
        let game_start = Instant::now();
        'game: loop {
            for action in agent::get_all_actions(&board) {
                let result = board.apply_action(action);
                match result {
                    ActionResult::Win | ActionResult::Lose => {
                        end_game(&mut record, &result);
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
