use bevy::prelude::*;
use bevy::DefaultPlugins;
use minesweeper::{simulate_n_games, GamePlugin, WINDOW_HEIGHT, WINDOW_WIDTH};
use std::env;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {
        let n: usize = args[1].parse().unwrap();
        simulate_n_games(n);
        return;
    }
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.75, 0.75, 0.75)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minesweeper".to_string(), // ToDo
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                // Bind to canvas included in `index.html`
                canvas: Some("#bevy".to_owned()),
                // Tells wasm not to override default event handling, like F5 and Ctrl+R
                prevent_default_event_handling: false,
                // fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((GamePlugin, bevy_framepace::FramepacePlugin))
        .run();
}
