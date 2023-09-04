use bevy::prelude::*;
use bevy::DefaultPlugins;
use minesweeper::setup::UISizing;
use minesweeper::{simulate_n_games, Difficulty, GamePlugin};
use std::env;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {
        let n: usize = args[1].parse().unwrap();
        simulate_n_games(n);
        return;
    }
    let ui_sizing = UISizing::new(Difficulty::default().grid_size());
    let window_size = ui_sizing.window_size;
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.75, 0.75, 0.75)))
        .insert_resource(ui_sizing)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minesweeper".to_string(), // ToDo
                resolution: window_size.into(),
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
