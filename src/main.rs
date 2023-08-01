use std::f32::consts::PI;

use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};

pub mod board;
use board::*;

mod agent;

const WINDOW_HEIGHT: f32 = 750.0;
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
const BOARD_WIDTH: f32 = TILE_SIZE * GRID_SIZE.0 as f32;
const WINDOW_WIDTH: f32 = BOARD_WIDTH + 2.0 * EDGE_PADDING;
const BOARD_HEIGHT: f32 = WINDOW_HEIGHT - TOP_PADDING - EDGE_PADDING;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.7569, 0.7569, 0.7569)))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()).set(
                WindowPlugin {
                    primary_window: Some(Window {
                        resolution: [WINDOW_WIDTH, WINDOW_HEIGHT].into(),
                        title: "Minesweeper".to_string(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                },
            ),
            bevy_framepace::FramepacePlugin,
        ))
        .add_state::<GameState>()
        .add_state::<AgentState>()
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_esc)
        .add_systems(Update, check_restart)
        .add_systems(Update, check_action.run_if(in_state(GameState::Game)))
        .run();
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut settings: ResMut<bevy_framepace::FramepaceSettings>,
) {
    settings.limiter = bevy_framepace::Limiter::from_framerate(40.0);
    commands.spawn(Camera2dBundle::default());

    let texture_handle = asset_server.load("minesweeper_tiles.png");
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
            let tile_sprite = TileSprite { col, row };
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
    println!("Beginning game with {} bombs", board.num_bombs_left());
    commands.spawn(board);
    commands.spawn(Record::default());
}

fn spawn_padding(commands: &mut Commands, asset_server: Res<AssetServer>) {
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
    // verticals
    let horizontal_offset =
        Vec2::new(BOARD_WIDTH / 2.0 + EDGE_PADDING / 2.0, 0.0);
    spawn_padding_piece(
        commands,
        &asset_server,
        board_centre + horizontal_offset,
        false,
    );
    spawn_padding_piece(
        commands,
        &asset_server,
        board_centre - horizontal_offset,
        false,
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
            Vec2::new(SCALE, BOARD_WIDTH / TILE_SPRITE_SIZE),
        )
    } else {
        (
            Quat::IDENTITY,
            Vec2::new(SCALE, BOARD_HEIGHT / TILE_SPRITE_SIZE),
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

#[derive(Component)]
pub struct TileSprite {
    col: usize,
    row: usize,
}

impl TileSprite {
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
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TileSprite)>,
    mut record_query: Query<&mut Record>,
) {
    let replay = keys.just_pressed(KeyCode::R);
    if keys.just_pressed(KeyCode::Return) || replay
    // || matches!(app_state.get(), GameState::GameOver)
    {
        let mut board = board_query.get_single_mut().unwrap();
        let seed = replay.then_some(board.seed());
        board.reset(seed);
        let mut record = record_query.get_single_mut().unwrap();
        if let GameState::Game = app_state.get() {
            record.dnf += 1;
            println!("{:?}", *record);
        }
        next_app_state.set(GameState::Game);
        next_agent_state.set(AgentState::Resting);
        println!("Beginning game with {} bombs", board.num_bombs_left());
        sync_board_with_tile_sprites(&mut board, &mut tile_sprites_query);
    }
}

fn check_action(
    buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    agent_state: ResMut<State<AgentState>>,
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TileSprite)>,
    mut record_query: Query<&mut Record>,
) {
    let mut board = board_query.get_single_mut().unwrap();
    if let Some(position) = q_windows.single().cursor_position() {
        let action_type = if buttons.just_pressed(MouseButton::Left) {
            Some(ActionType::Uncover)
        } else if buttons.just_pressed(MouseButton::Right) {
            Some(ActionType::Flag)
        } else {
            None
        };
        if let Some(action_type) = action_type {
            let col = ((position.x - EDGE_PADDING) / TILE_SIZE) as usize;
            let row = ((position.y - TOP_PADDING) / TILE_SIZE) as usize;
            if col < board.width()
                && row < board.height()
                && !matches!(
                    board.tile_state(col, row),
                    TileState::UncoveredSafe(_)
                )
            {
                let action = Action {
                    col,
                    row,
                    action_type,
                };
                complete_action(
                    &mut board,
                    action,
                    &mut next_app_state,
                    &mut tile_sprites_query,
                    &mut record_query,
                );
            }
        }
    }

    // use bot
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
                &mut tile_sprites_query,
                &mut record_query,
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
            agent::get_non_trivial_actions(&board)
        };
        while !actions.is_empty() {
            for action in actions {
                let result = complete_action(
                    &mut board,
                    action,
                    &mut next_app_state,
                    &mut tile_sprites_query,
                    &mut record_query,
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
        let mut actions = agent::get_non_trivial_actions(&board);
        if let Some(action) = actions.pop() {
            complete_action(
                &mut board,
                action,
                &mut next_app_state,
                &mut tile_sprites_query,
                &mut record_query,
            );
        }
    }
}

fn complete_action(
    board: &mut Board,
    action: Action,
    next_app_state: &mut ResMut<NextState<GameState>>,
    tile_sprites_query: &mut Query<(&mut TextureAtlasSprite, &TileSprite)>,
    record_query: &mut Query<&mut Record>,
) -> ActionResult {
    let result = board.apply_action(action);
    let mut record = record_query.get_single_mut().unwrap();
    match result {
        ActionResult::Win => {
            record.win += 1;
            println!("You won!");
            println!("{:?}", *record);
            next_app_state.set(GameState::GameOver);
        }
        ActionResult::Lose => {
            record.loss += 1;
            println!("You lost...");
            println!("{:?}", *record);
            next_app_state.set(GameState::GameOver);
        }
        ActionResult::Continue => {
            println!("Num bombs left: {}", board.num_bombs_left());
        }
    }
    sync_board_with_tile_sprites(board, tile_sprites_query);
    result
}

fn sync_board_with_tile_sprites(
    board: &mut Board,
    tile_sprites_query: &mut Query<(&mut TextureAtlasSprite, &TileSprite)>,
) {
    for (mut sprite, TileSprite { col, row }) in tile_sprites_query {
        let tile_state = board.tile_state(*col, *row);
        let index = sprite_sheet_index(tile_state);
        sprite.index = index;
    }
}
