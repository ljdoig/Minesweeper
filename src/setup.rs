use bevy::{prelude::*, window::PrimaryWindow};
use bevy_framepace::{FramepaceSettings, Limiter};
use std::f32::consts::PI;

const WINDOW_HEIGHT: f32 = 700.0;
const TILE_SPRITE_SIZE: f32 = 16.0;
const EDGE_PADDING_SPRITE_SIZE: f32 = 12.0;
const TOP_PADDING_SPRITE_SIZE: f32 = 60.0;
const DIGIT_SPRITE_SIZE: (f32, f32) = (13.0, 23.0);

use crate::{
    board::{Board, TileState},
    tile_sheet_index, BombCounterDigit, Difficulty, Record, TilePos,
};

#[derive(Resource, Debug)]
pub struct UISizing {
    pub window_size: (f32, f32),
    pub board_size: (f32, f32),
    pub grid_size: (usize, usize),
    pub tile_size: f32,
    pub edge_padding: f32,
    pub top_padding: f32,
    pub scale: f32,
}

impl UISizing {
    pub fn new((width, height): (usize, usize)) -> Self {
        let unscaled_height = height as f32 * TILE_SPRITE_SIZE
            + EDGE_PADDING_SPRITE_SIZE
            + TOP_PADDING_SPRITE_SIZE;
        let scale = WINDOW_HEIGHT / unscaled_height;
        let tile_size = TILE_SPRITE_SIZE * scale;
        let edge_padding = EDGE_PADDING_SPRITE_SIZE * scale;
        let top_padding = TOP_PADDING_SPRITE_SIZE * scale;
        let board_width = tile_size * width as f32;
        let board_height = tile_size * height as f32;
        let window_width = board_width + 2.0 * edge_padding;
        UISizing {
            window_size: (window_width, WINDOW_HEIGHT),
            board_size: (board_width, board_height),
            grid_size: (width, height),
            tile_size,
            edge_padding,
            top_padding,
            scale,
        }
    }

    pub fn pos_on_board(&self, &TilePos { col, row }: &TilePos) -> Vec3 {
        let &UISizing {
            tile_size,
            grid_size,
            ..
        } = self;
        let translation_x =
            tile_size * (col as f32 - (grid_size.0 - 1) as f32 / 2.0);
        let translation_y =
            tile_size * -(row as f32 - (grid_size.1 - 1) as f32 / 2.0);
        Vec3::new(translation_x, translation_y, 0.0)
    }

    pub fn clicked_tile_pos(&self, position: Vec2) -> Option<TilePos> {
        let &UISizing {
            edge_padding,
            top_padding,
            tile_size,
            grid_size,
            ..
        } = self;
        if position.x > edge_padding && position.y > top_padding {
            let col = ((position.x - edge_padding) / tile_size) as usize;
            let row = ((position.y - top_padding) / tile_size) as usize;
            return (col < grid_size.0 && row < grid_size.1)
                .then_some(TilePos { col, row });
        }
        None
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut settings: ResMut<FramepaceSettings>,
    q_windows: Query<&mut Window, With<PrimaryWindow>>,
    ui_sizing: Res<UISizing>,
    difficulty: Res<State<Difficulty>>,
) {
    settings.limiter = Limiter::from_framerate(50.0);
    setup_game(
        &mut commands,
        asset_server,
        texture_atlases,
        q_windows,
        ui_sizing,
        difficulty.num_bombs(),
    );
}

pub fn resize(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut ui_sizing: ResMut<UISizing>,
    game_objects: Query<Entity, Without<Window>>,
    next_difficulty: ResMut<NextState<Difficulty>>,
    difficulty: Res<State<Difficulty>>,
) {
    let new_difficulty = {
        if let Some(next_difficulty) = next_difficulty.0 {
            if next_difficulty != **difficulty {
                next_difficulty
            } else {
                return;
            }
        } else {
            return;
        }
    };
    println!("\nChanging difficulty level to {}\n", new_difficulty);
    *ui_sizing = UISizing::new(new_difficulty.grid_size());
    setup_game(
        &mut commands,
        asset_server,
        texture_atlases,
        q_windows,
        ui_sizing.into(),
        new_difficulty.num_bombs(),
    );
    // despawn old
    for entity in &game_objects {
        commands.entity(entity).despawn();
    }
}

fn setup_game(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    ui_sizing: Res<UISizing>,
    num_bombs: usize,
) {
    let (width, height) = ui_sizing.window_size;
    q_windows.single_mut().resolution.set(width, height);
    commands.spawn(Camera2dBundle::default());
    spawn_board(
        commands,
        &asset_server,
        &mut texture_atlases,
        ui_sizing.grid_size,
        num_bombs,
        &ui_sizing,
    );
    spawn_bomb_display(
        commands,
        &asset_server,
        &mut texture_atlases,
        &ui_sizing,
    );
    spawn_padding(commands, &asset_server, &ui_sizing);
    commands.spawn(Record::default());
}

fn spawn_board(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    grid_size: (usize, usize),
    num_bombs: usize,
    ui_sizing: &UISizing,
) {
    let &UISizing {
        edge_padding,
        top_padding,
        scale,
        ..
    } = ui_sizing;
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
    let board = Board::new(grid_size, num_bombs);
    let (width, height) = (board.width(), board.height());
    commands
        .spawn(board)
        .insert(SpatialBundle::from(Transform::from_translation(
            Vec3::Y * -(top_padding - edge_padding) / 2.0,
        )))
        .with_children(|parent| {
            for col in 0..width {
                for row in 0..height {
                    let tile_sprite = TilePos { col, row };
                    let sprite_sheet_index =
                        tile_sheet_index(TileState::Covered);
                    parent.spawn((
                        SpriteSheetBundle {
                            texture_atlas: texture_atlas_handle.clone(),
                            sprite: TextureAtlasSprite::new(sprite_sheet_index),
                            transform: Transform {
                                translation: ui_sizing
                                    .pos_on_board(&tile_sprite),
                                scale: Vec3::splat(scale),
                                ..default()
                            },
                            ..default()
                        },
                        tile_sprite,
                    ));
                }
            }
        });
}

fn spawn_bomb_display(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    &UISizing {
        window_size,
        top_padding,
        scale,
        ..
    }: &UISizing,
) {
    let texture_handle = asset_server.load("spritesheets/numbers.png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(DIGIT_SPRITE_SIZE.0, DIGIT_SPRITE_SIZE.1),
        12,
        1,
        Some(Vec2::new(1.0, DIGIT_SPRITE_SIZE.1)),
        None,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    let transform = Transform {
        translation: Vec3::new(0.0, (window_size.1 - top_padding) / 2.0, 1.0),
        scale: Vec3::splat(scale),
        ..default()
    };
    commands
        .spawn(SpatialBundle::from_transform(transform))
        .with_children(|parent| {
            let digit_spacing = Vec3::X * (DIGIT_SPRITE_SIZE.0 - 0.5);
            for i in -1..=1 {
                let new_digit = (
                    SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        transform: Transform::from_translation(
                            digit_spacing * i as f32,
                        ),
                        ..default()
                    },
                    BombCounterDigit,
                );
                parent.spawn(new_digit);
            }
        });
}

fn spawn_padding(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    &UISizing {
        window_size,
        board_size,
        edge_padding,
        top_padding,
        scale,
        ..
    }: &UISizing,
) {
    // verticals
    let vertical_length = window_size.1 / TILE_SPRITE_SIZE;
    let horizontal_offset =
        Vec2::new(board_size.0 / 2.0 + edge_padding / 2.0, 0.0);
    spawn_padding_piece(
        commands,
        asset_server,
        horizontal_offset,
        false,
        vertical_length,
        scale,
    );
    spawn_padding_piece(
        commands,
        asset_server,
        -horizontal_offset,
        false,
        vertical_length,
        scale,
    );
    // horizontals
    let horizontal_length = window_size.0 / TILE_SPRITE_SIZE;
    let window_vertical_offset =
        Vec2::Y * (window_size.1 / 2.0 - edge_padding / 2.0);
    // very top
    spawn_padding_piece(
        commands,
        asset_server,
        window_vertical_offset,
        true,
        horizontal_length,
        scale,
    );
    let board_centre =
        Vec2::new(0.0, window_size.1 / 2.0 - top_padding - board_size.1 / 2.0);
    let board_vertical_offset =
        Vec2::new(0.0, board_size.1 / 2.0 + edge_padding / 2.0);
    spawn_padding_piece(
        commands,
        asset_server,
        board_centre + board_vertical_offset,
        true,
        horizontal_length,
        scale,
    );
    spawn_padding_piece(
        commands,
        asset_server,
        board_centre - board_vertical_offset,
        true,
        horizontal_length,
        scale,
    );
}

fn spawn_padding_piece(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    centre: Vec2,
    horizontal: bool,
    length: f32,
    scale: f32,
) {
    let rotation = if horizontal {
        Quat::from_rotation_z(-PI / 2.0)
    } else {
        Quat::IDENTITY
    };
    commands.spawn(SpriteBundle {
        texture: asset_server.load("padding/padding.png"),
        transform: Transform {
            rotation,
            scale: Vec2::new(scale, length).extend(1.0),
            translation: centre.extend(1.0),
        },
        ..default()
    });
}
