use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    board::{Action, ActionResult, ActionType, Board, TileState},
    setup::UISizing,
    AgentState, BotButton, Difficulty, FaceButton, FaceButtonState, GameState,
    Record,
};

pub mod agent;

pub fn restart(
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    app_state: ResMut<State<GameState>>,
    mut q_record: Query<&mut Record>,
) {
    let mut board = q_board.single_mut();
    // avoid repeated restart
    if !board.first_uncovered() {
        return;
    }
    let mut record = q_record.single_mut();
    if matches!(app_state.get(), GameState::Playing) {
        end_game(&mut record, &ActionResult::Continue, &board);
    } else {
        next_app_state.set(GameState::Playing);
    }
    board.reset(None);
}

pub fn check_restart(
    difficulty: Res<State<Difficulty>>,
    mut next_difficulty: ResMut<NextState<Difficulty>>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    app_state: ResMut<State<GameState>>,
    mut q_face_buttons: Query<(&FaceButton, &crate::Button)>,
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_board: Query<&mut Board>,
    q_record: Query<&mut Record>,
) {
    for (&FaceButton(new_difficulty), button) in &mut q_face_buttons {
        if button.just_released(q_windows.single(), &mouse) {
            next_agent_state.set(AgentState::Resting);
            if new_difficulty != **difficulty {
                next_difficulty.set(new_difficulty);
                next_app_state.set(GameState::Playing);
            } else {
                restart(q_board, next_app_state, app_state, q_record);
            }
            return;
        }
    }
}

pub fn check_player_action(
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut q_record: Query<&mut Record>,
    ui_sizing: Res<UISizing>,
) {
    let mut board = q_board.single_mut();
    let mut record = q_record.single_mut();
    if let Some(position) = q_windows.single().cursor_position() {
        let action_type = if mouse.just_released(MouseButton::Left) {
            Some(ActionType::Uncover)
        } else if mouse.just_pressed(MouseButton::Right) {
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
    mut q_board: Query<&mut Board>,
    mut next_app_state: ResMut<NextState<GameState>>,
    mut next_agent_state: ResMut<NextState<AgentState>>,
    agent_state: ResMut<State<AgentState>>,
    app_state: ResMut<State<GameState>>,
    mut q_record: Query<&mut Record>,
    mut q_bot_buttons: Query<(&crate::Button, &BotButton)>,
    mouse: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_face_buttons: Query<(&mut TextureAtlasSprite, &FaceButton)>,
) {
    let mut record = q_record.single_mut();
    let window = q_windows.single();
    for (button, bot_button) in &mut q_bot_buttons {
        if button.just_released(window, &mouse) {
            if !matches!(app_state.get(), GameState::Playing) {
                restart(q_board, next_app_state, app_state, q_record);
                next_agent_state.set(bot_button.bot_effect);
                return;
            }
            match agent_state.get() {
                AgentState::Resting => {
                    next_agent_state.set(bot_button.bot_effect)
                }
                _ => next_agent_state.set(AgentState::Resting),
            }
        }
    }
    let mut board = q_board.single_mut();
    if !matches!(agent_state.get(), AgentState::Resting) {
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
            if result != ActionResult::Continue
                || matches!(agent_state.get(), AgentState::ThinkingOneMoveOnly)
            {
                next_agent_state.set(AgentState::Resting);
                return;
            }
        }
        for (mut sprite, button) in &mut q_face_buttons {
            sprite.index = button.sheet_index(FaceButtonState::Playing);
        }
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
        ActionResult::Win => {
            end_game(record, &result, board);
            next_app_state.set(GameState::Won);
        }
        ActionResult::Lose => {
            end_game(record, &result, board);
            next_app_state.set(GameState::Lost);
        }
        ActionResult::Continue => {}
    }
    result
}
