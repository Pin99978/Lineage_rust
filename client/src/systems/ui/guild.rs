use bevy::prelude::*;

#[derive(Component)]
pub struct GuildPanelRoot;

#[derive(Component)]
pub struct GuildInfoText;

pub fn setup_guild_ui(commands: &mut Commands) {
    let panel = commands
        .spawn((
            GuildPanelRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(24.0),
                width: Val::Px(420.0),
                height: Val::Px(150.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.05, 0.9)),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).with_children(|parent| {
        parent.spawn((
            Text::new("Guild [G]"),
            TextFont::from_font_size(20.0),
            TextColor(Color::srgb(0.9, 0.92, 0.97)),
        ));
        parent.spawn((
            GuildInfoText,
            Text::new("Use chat commands:\n/guild create <name>\n/guild invite <player>\n/guild leave | /guild disband"),
            TextFont::from_font_size(14.0),
            TextColor(Color::srgb(0.85, 0.88, 0.93)),
        ));
    });
}

pub fn toggle_guild_window_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    chat_state: Option<Res<super::chat::ChatUiState>>,
    windows_state: Option<ResMut<super::inventory::UiWindowsState>>,
) {
    if chat_state
        .as_ref()
        .map(|state| state.focused)
        .unwrap_or(false)
    {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyG) {
        return;
    }
    let Some(mut windows_state) = windows_state else {
        return;
    };
    windows_state.guild_open = !windows_state.guild_open;
}

pub fn apply_guild_visibility_system(
    windows_state: Option<Res<super::inventory::UiWindowsState>>,
    mut panels: Query<&mut Visibility, With<GuildPanelRoot>>,
) {
    let Some(windows_state) = windows_state else {
        return;
    };

    for mut visibility in &mut panels {
        *visibility = if windows_state.guild_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn refresh_guild_ui_system(
    hud_state: Option<Res<super::HudState>>,
    mut texts: Query<&mut Text, With<GuildInfoText>>,
) {
    let Some(hud_state) = hud_state else {
        return;
    };
    if !hud_state.is_changed() {
        return;
    }

    let guild_line = if let Some(guild_name) = hud_state.guild_name.as_ref() {
        let role = hud_state
            .guild_role
            .map(|value| format!("{:?}", value))
            .unwrap_or_else(|| "Member".to_string());
        format!("Guild: {} ({})", guild_name, role)
    } else {
        "Guild: None".to_string()
    };
    let members = if hud_state.guild_members.is_empty() {
        "Online members: none".to_string()
    } else {
        format!("Online members: {}", hud_state.guild_members.join(", "))
    };

    for mut text in &mut texts {
        *text = Text::new(format!(
            "{}\n{}\nUse chat commands:\n/guild create <name> | /guild invite <player>\n/guild leave | /guild disband | /guild accept | /guild deny",
            guild_line, members
        ));
    }
}
