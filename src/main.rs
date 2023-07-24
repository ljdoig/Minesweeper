use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};

mod board;
use board::*;

const WINDOW_SIZE: (f32, f32) = (800.0, 800.0);
const TILE_SPRITE_SIZE: f32 = 128.0;
const TILE_SIZE: f32 = WINDOW_SIZE.1 as f32 / GRID_SIZE.1 as f32;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()).set(
            WindowPlugin {
                primary_window: Some(Window {
                    resolution: [WINDOW_SIZE.0, WINDOW_SIZE.1].into(),
                    title: "Minesweeper".to_string(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            },
        ))
        .add_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_esc)
        .add_systems(Update, check_restart)
        .add_systems(Update, check_action.run_if(in_state(GameState::Game)))
        .add_systems(Update, sync_board_with_tile_sprites)
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
) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle = asset_server.load("minesweeper_tiles.png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::splat(TILE_SPRITE_SIZE),
        4,
        3,
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
        TileState::ExplodedBomb => 2,
        TileState::UncoveredBomb => 2,
        TileState::UncoveredSafe(n) => 3 + n as usize,
    }
}

fn check_restart(
    keys: Res<Input<KeyCode>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Return) {
        let mut board = board_query.get_single_mut().unwrap();
        board.reset();
        next_app_state.set(GameState::Game);
        println!("Beginning game with {} bombs", board.num_bombs_left());
    }
}

fn check_action(
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut board_query: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
) {
    if let Some(position) = q_windows.single().cursor_position() {
        let action_type = if buttons.just_pressed(MouseButton::Left) {
            Some(ActionType::Uncover)
        } else if buttons.just_pressed(MouseButton::Right) {
            Some(ActionType::Flag)
        } else {
            None
        };
        if let Some(action_type) = action_type {
            let (col, row) = (position.as_uvec2() / TILE_SIZE as u32).into();
            let action = Action {
                col: col as usize,
                row: row as usize,
                action_type,
            };
            let mut board = board_query.get_single_mut().unwrap();
            complete_action(&mut board, action, next_app_state);
        }
    }
}

fn complete_action(
    board: &mut Board,
    action: Action,
    mut next_app_state: ResMut<NextState<GameState>>,
) {
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
}

fn sync_board_with_tile_sprites(
    board_query: Query<&Board>,
    mut tile_sprites_query: Query<(&mut TextureAtlasSprite, &TileSprite)>,
) {
    if let Ok(board) = board_query.get_single() {
        for (mut sprite, TileSprite { col, row }) in &mut tile_sprites_query {
            let tile_state = board.tile_state(*col, *row);
            let index = sprite_sheet_index(tile_state);
            sprite.index = index;
        }
    }
}
