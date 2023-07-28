use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};

pub mod board;
use board::*;

mod agent;

const WINDOW_HEIGHT: f32 = 750.0;
const TILE_SPRITE_SIZE: f32 = 128.0;

const ASPECT_RATIO: f32 = GRID_SIZE.0 as f32 / GRID_SIZE.1 as f32;
const WINDOW_SIZE: (f32, f32) = (WINDOW_HEIGHT * ASPECT_RATIO, WINDOW_HEIGHT);
const TILE_SIZE: f32 = WINDOW_SIZE.1 / GRID_SIZE.1 as f32;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()).set(
                WindowPlugin {
                    primary_window: Some(Window {
                        resolution: [WINDOW_SIZE.0, WINDOW_SIZE.1].into(),
                        title: "Minesweeper".to_string(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                },
            ),
            bevy_framepace::FramepacePlugin,
        ))
        .add_state::<GameState>()
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut settings: ResMut<bevy_framepace::FramepaceSettings>,
) {
    settings.limiter = bevy_framepace::Limiter::from_framerate(20.0);
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
                    .with_scale(Vec3::splat(TILE_SIZE / TILE_SPRITE_SIZE)),
                    ..default()
                },
                tile_sprite,
            ));
        }
    }
    let board = Board::new(GRID_SIZE);
    println!("Beginning game with {} bombs", board.num_bombs_left());
    commands.spawn(board);
    // commands.spawn(Agent {});
}

#[derive(Component)]
pub struct TileSprite {
    pub col: usize,
    pub row: usize,
}

impl TileSprite {
    fn screen_pos(&self) -> Vec2 {
        let translation_x =
            TILE_SIZE * (self.col as f32 - (GRID_SIZE.0 - 1) as f32 / 2.0);
        let translation_y =
            TILE_SIZE * -(self.row as f32 - (GRID_SIZE.1 - 1) as f32 / 2.0);
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
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TileSprite)>,
) {
    let replay = keys.just_pressed(KeyCode::R);
    if keys.just_pressed(KeyCode::Return) || replay {
        let mut board = board_query.get_single_mut().unwrap();
        let seed = replay.then_some(board.seed());
        board.reset(seed);
        next_app_state.set(GameState::Game);
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
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TileSprite)>,
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
            let action = Action {
                col: (position.x / TILE_SIZE) as usize,
                row: (position.y / TILE_SIZE) as usize,
                action_type,
            };
            complete_action(
                &mut board,
                action,
                &mut next_app_state,
                &mut tile_sprites_query,
            );
        }
    }
    // use bot

    if keys.just_pressed(KeyCode::Space) {
        let mut actions = agent::get_all_actions(&board);
        while !actions.is_empty() {
            for action in actions {
                let result = complete_action(
                    &mut board,
                    action,
                    &mut next_app_state,
                    &mut tile_sprites_query,
                );
                if result != ActionResult::Continue {
                    return;
                }
            }
            actions = agent::get_all_actions(&board);
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
            );
        }
    }
}

fn complete_action(
    board: &mut Board,
    action: Action,
    next_app_state: &mut ResMut<NextState<GameState>>,
    tile_sprites_query: &mut Query<(&mut TextureAtlasSprite, &TileSprite)>,
) -> ActionResult {
    let result = board.apply_action(action);
    match result {
        ActionResult::Win => {
            println!("You won!");
            next_app_state.set(GameState::GameOver);
        }
        ActionResult::Lose => {
            println!("You lost...");
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
