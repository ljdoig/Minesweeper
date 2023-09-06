use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::DefaultPlugins;
use clap::Parser;
use minesweeper::setup::UISizing;
use minesweeper::{simulate_n_games, Difficulty, GamePlugin};

/// Minesweeper game: only need to pass arguments to run simulations
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of games to simulate
    #[arg(short, long, default_value_t)]
    num_games: usize,

    /// Difficulty of simulated games
    #[arg(short, long, value_enum, default_value_t)]
    difficulty: Difficulty,

    /// Seed for simulated games
    #[arg(short, long, default_value_t)]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    if args.num_games > 0 {
        simulate_n_games(args.num_games, args.difficulty, args.seed);
        return;
    }
    let ui_sizing = UISizing::new(Difficulty::default().grid_size());
    let window_size = ui_sizing.window_size;
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.75, 0.75, 0.75)))
        .insert_resource(ui_sizing)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Minesweeper".to_string(),
                        resolution: window_size.into(),
                        // Bind to canvas included in `index.html`
                        canvas: Some("#bevy".to_owned()),
                        // Tells wasm not to override default event handling
                        prevent_default_event_handling: false,
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: bevy::log::Level::ERROR,
                    ..default()
                }),
        )
        .add_plugins((GamePlugin, bevy_framepace::FramepacePlugin))
        .run();
}
