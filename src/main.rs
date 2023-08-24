use bevy::prelude::*;
use bevy::DefaultPlugins;
use bevy_framepace;
// ToDo: Replace bevy_game with your new crate name.
use minesweeper::{GamePlugin, WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::rgb(0.75, 0.75, 0.75)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minesweeper".to_string(), // ToDo
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                // Bind to canvas included in `index.html`
                canvas: Some("#bevy".to_owned()),
                // Tells wasm not to override default event handling, like F5 and Ctrl+R
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((GamePlugin, bevy_framepace::FramepacePlugin))
        .run();
}
