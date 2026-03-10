use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use shared::protocol::{ChatChannel, ChatIntent};

use crate::network;

const MAX_HISTORY_LINES: usize = 8;
const MAX_INPUT_LEN: usize = 160;

#[derive(Resource, Debug, Clone, Default)]
pub struct ChatUiState {
    pub focused: bool,
    pub input: String,
    pub history: Vec<String>,
}

#[derive(Component)]
pub struct ChatHistoryTextUi;

#[derive(Component)]
pub struct ChatInputTextUi;

pub fn setup_chat_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            bottom: Val::Px(170.0),
            width: Val::Px(420.0),
            height: Val::Px(210.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(10.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.85)),
        children![
            (
                ChatHistoryTextUi,
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.92, 0.94, 0.98))
            ),
            (
                ChatInputTextUi,
                Text::new("> (Press Enter to chat)"),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.7, 0.78, 0.9))
            )
        ],
    ));
}

pub fn chat_focus_and_send_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    network: Option<Res<network::ClientNetwork>>,
    chat_state: Option<ResMut<ChatUiState>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(mut chat_state) = chat_state else {
        return;
    };

    if !chat_state.focused {
        chat_state.focused = true;
        return;
    }

    let raw = chat_state.input.trim().to_string();
    chat_state.focused = false;
    chat_state.input.clear();
    if raw.is_empty() {
        return;
    }

    if let Some(ref network) = network {
        if handle_guild_command(network, &raw, &mut chat_state) {
            return;
        }
    }

    let (channel, target, message) = parse_chat_command(&raw);
    if message.is_empty() {
        return;
    }

    if let Some(ref network) = network {
        network::send_chat_intent(
            network,
            ChatIntent {
                channel,
                target,
                message,
            },
        );
    }
}

pub fn chat_text_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut keyboard_inputs: MessageReader<KeyboardInput>,
    chat_state: Option<ResMut<ChatUiState>>,
) {
    let Some(mut chat_state) = chat_state else {
        return;
    };
    if !chat_state.focused {
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        chat_state.focused = false;
        chat_state.input.clear();
        return;
    }
    if keyboard.just_pressed(KeyCode::Backspace) {
        chat_state.input.pop();
    }

    for key in keyboard_inputs.read() {
        if key.state != ButtonState::Pressed || key.repeat {
            continue;
        }
        let Some(text) = key.text.as_deref() else {
            continue;
        };
        if text.chars().all(|ch| ch.is_control()) {
            continue;
        }
        if chat_state.input.chars().count() >= MAX_INPUT_LEN {
            break;
        }
        chat_state.input.push_str(text);
    }
}

pub fn update_chat_ui_system(
    chat_state: Option<Res<ChatUiState>>,
    mut history_query: Query<&mut Text, (With<ChatHistoryTextUi>, Without<ChatInputTextUi>)>,
    mut input_query: Query<&mut Text, (With<ChatInputTextUi>, Without<ChatHistoryTextUi>)>,
) {
    let Some(chat_state) = chat_state else {
        return;
    };
    if !chat_state.is_changed() {
        return;
    }

    let Ok(mut history_text) = history_query.single_mut() else {
        return;
    };
    let Ok(mut input_text) = input_query.single_mut() else {
        return;
    };

    *history_text = Text::new(chat_state.history.join("\n"));
    if chat_state.focused {
        *input_text = Text::new(format!("> {}", chat_state.input));
    } else {
        *input_text = Text::new("> (Press Enter to chat)");
    }
}

pub fn push_history_line(chat_state: &mut ChatUiState, line: String) {
    chat_state.history.push(line);
    if chat_state.history.len() > MAX_HISTORY_LINES {
        let overflow = chat_state.history.len() - MAX_HISTORY_LINES;
        chat_state.history.drain(0..overflow);
    }
}

pub fn push_system_line(chat_state: &mut ChatUiState, line: String) {
    push_history_line(chat_state, format!("[System] {}", line));
}

fn parse_chat_command(raw: &str) -> (ChatChannel, Option<String>, String) {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed
        .strip_prefix("/guildchat ")
        .or_else(|| trimmed.strip_prefix("/g "))
    {
        return (ChatChannel::Guild, None, truncate(rest.trim()));
    }
    if let Some(rest) = trimmed
        .strip_prefix("/shout ")
        .or_else(|| trimmed.strip_prefix("/sh "))
    {
        return (ChatChannel::Shout, None, truncate(rest.trim()));
    }

    if let Some(rest) = trimmed
        .strip_prefix("/whisper ")
        .or_else(|| trimmed.strip_prefix("/w "))
    {
        let mut parts = rest.trim().splitn(2, ' ');
        let Some(target) = parts.next() else {
            return (ChatChannel::Whisper, None, String::new());
        };
        let message = parts.next().unwrap_or("").trim();
        return (
            ChatChannel::Whisper,
            Some(target.to_string()),
            truncate(message),
        );
    }

    if let Some(rest) = trimmed
        .strip_prefix("/say ")
        .or_else(|| trimmed.strip_prefix("/s "))
    {
        return (ChatChannel::Say, None, truncate(rest.trim()));
    }

    (ChatChannel::Say, None, truncate(trimmed))
}

fn handle_guild_command(
    network: &network::ClientNetwork,
    raw: &str,
    chat_state: &mut ChatUiState,
) -> bool {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("/guild ") else {
        return false;
    };
    let mut parts = rest.split_whitespace();
    let Some(action) = parts.next() else {
        push_history_line(
            chat_state,
            "[System] Usage: /guild create|invite|leave|disband|accept|deny".to_string(),
        );
        return true;
    };

    match action {
        "create" => {
            let guild_name = parts.collect::<Vec<&str>>().join(" ");
            if guild_name.trim().is_empty() {
                push_history_line(
                    chat_state,
                    "[System] Usage: /guild create <name>".to_string(),
                );
            } else {
                network::send_create_guild_intent(network, guild_name.trim().to_string());
            }
        }
        "invite" => {
            let Some(target) = parts.next() else {
                push_history_line(
                    chat_state,
                    "[System] Usage: /guild invite <player>".to_string(),
                );
                return true;
            };
            network::send_invite_to_guild_intent(network, target.to_string());
        }
        "leave" => network::send_leave_guild_intent(network),
        "disband" => network::send_disband_guild_intent(network),
        "accept" => network::send_respond_guild_invite_intent(network, true),
        "deny" => network::send_respond_guild_invite_intent(network, false),
        _ => {
            push_history_line(
                chat_state,
                "[System] Usage: /guild create|invite|leave|disband|accept|deny".to_string(),
            );
        }
    }
    true
}

fn truncate(text: &str) -> String {
    text.chars().take(MAX_INPUT_LEN).collect()
}
