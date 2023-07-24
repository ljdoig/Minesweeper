use bevy::window::PrimaryWindow;
use bevy::{prelude::*, window::close_on_esc};
use rand::seq::index::sample;

const NUM_BOMBS: usize = 5;
const WINDOW_SIZE: (f32, f32) = (800.0, 800.0);
const GRID_SIZE: (usize, usize) = (10, 10);
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
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_esc)
        .add_systems(Update, mouse_button_input)
        .add_systems(Update, sync_board_with_tile_sprites)
        .run();
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
    commands.spawn(Board::new(GRID_SIZE));
}

#[derive(Clone, Copy)]
pub enum TileState {
    Covered,
    Flagged,
    UncoveredBomb,
    UncoveredSafe(u8),
}

fn sprite_sheet_index(state: TileState) -> usize {
    match state {
        TileState::Covered => 0,
        TileState::Flagged => 1,
        TileState::UncoveredBomb => 2,
        TileState::UncoveredSafe(n) => 3 + n as usize,
    }
}

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

#[derive(Component)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub tile_states: Vec<TileState>,
    bombs: Vec<bool>,
}

impl Board {
    fn new((width, height): (usize, usize)) -> Board {
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

    fn apply_action(&mut self, action: Action) -> ActionResult {
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

fn mouse_button_input(
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut board_query: Query<&mut Board>,
) {
    let mut board = board_query.get_single_mut().unwrap();
    if let Some(position) = q_windows.single().cursor_position() {
        let (col, row) = (position.as_uvec2() / TILE_SIZE as u32).into();
        let (col, row) = (col as usize, row as usize);
        if buttons.just_pressed(MouseButton::Left) {
            let action = Action {
                col,
                row,
                action_type: ActionType::Uncover,
            };
            board.apply_action(action);
        }
        if buttons.just_pressed(MouseButton::Right) {
            let action = Action {
                col,
                row,
                action_type: ActionType::Flag,
            };
            board.apply_action(action);
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
