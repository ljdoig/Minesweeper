use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    board::{Action, ActionResult, ActionType, Board, TileState},
    setup::UISizing,
    AgentState, Difficulty, GameState, Record,
};

pub mod agent;

pub fn check_restart(
    keys: Res<Input<KeyCode>>,
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    app_state: ResMut<State<GameState>>,
    mut q_record: Query<&mut Record>,
) {
    let mut board = q_board.get_single_mut().unwrap();
    // avoid repeated restart
    if !board.first_uncovered() {
        return;
    }
    let replay = keys.just_pressed(KeyCode::R);
    if keys.just_pressed(KeyCode::Return) || replay {
        let mut record = q_record.get_single_mut().unwrap();
        if matches!(app_state.get(), GameState::Game) {
            end_game(&mut record, &ActionResult::Continue, &board);
        }
        next_app_state.set(GameState::Game);
        next_agent_state.set(AgentState::Resting);
        let seed = replay.then_some(board.seed());
        board.reset(seed);
    }
}

pub fn check_change_difficulty(
    keys: Res<Input<KeyCode>>,
    difficulty: Res<State<Difficulty>>,
    mut next_difficulty: ResMut<NextState<Difficulty>>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
) {
    let mut set_difficulty = |new_difficulty| {
        if new_difficulty != **difficulty {
            next_difficulty.set(new_difficulty);
            next_app_state.set(GameState::Game);
            next_agent_state.set(AgentState::Resting);
        }
    };
    if keys.just_pressed(KeyCode::Key8) {
        set_difficulty(Difficulty::Easy)
    } else if keys.just_pressed(KeyCode::Key9) {
        set_difficulty(Difficulty::Medium)
    } else if keys.just_pressed(KeyCode::Key0) {
        set_difficulty(Difficulty::Hard)
    };
}

pub fn check_player_action(
    buttons: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    agent_state: ResMut<State<AgentState>>,
    mut q_record: Query<&mut Record>,
    ui_sizing: Res<UISizing>,
) {
    if matches!(agent_state.get(), AgentState::Thinking) {
        return;
    }
    let mut board = q_board.get_single_mut().unwrap();
    let mut record = q_record.get_single_mut().unwrap();
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
            if let Some(pos) = ui_sizing.clicked_tile_pos(position) {
                if !matches!(board.tile_state(pos), TileState::UncoveredSafe(_))
                {
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

pub fn check_bot_action(
    keys: Res<Input<KeyCode>>,
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    agent_state: ResMut<State<AgentState>>,
    mut q_record: Query<&mut Record>,
) {
    let mut board = q_board.get_single_mut().unwrap();
    let mut record = q_record.get_single_mut().unwrap();
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

pub fn end_game(record: &mut Record, result: &ActionResult, board: &Board) {
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
    record.total_bombs_cleared +=
        board.num_bombs_total() - board.num_bombs_left() as usize;
    record.total_bombs += board.num_bombs_total();
    println!("Record: {}\n", record);
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
            end_game(record, &result, board);
            next_app_state.set(GameState::GameOver);
        }
        ActionResult::Continue => {}
    }
    result
}
