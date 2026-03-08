use bevy::prelude::*;
use shared::{QuestId, QuestStatus};

use super::HudState;

#[derive(Component)]
pub struct QuestLogTextUi;

pub fn setup_quest_log_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(20.0),
            width: Val::Px(320.0),
            min_height: Val::Px(80.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.80)),
        children![(
            QuestLogTextUi,
            Text::new("Quest Log\n- No active quests"),
            TextFont::from_font_size(16.0),
            TextColor(Color::srgb(0.90, 0.95, 0.98))
        )],
    ));
}

pub fn update_quest_log_ui(
    hud_state: Option<Res<HudState>>,
    mut quest_log_text: Query<&mut Text, With<QuestLogTextUi>>,
) {
    let Some(hud_state) = hud_state else {
        return;
    };
    if !hud_state.is_changed() {
        return;
    }
    let Ok(mut text) = quest_log_text.single_mut() else {
        return;
    };

    if hud_state.quest_entries.is_empty() {
        *text = Text::new("Quest Log\n- No active quests");
        return;
    }

    let mut lines = vec!["Quest Log".to_string()];
    for (quest_id, status) in &hud_state.quest_entries {
        let label = match quest_id {
            QuestId::KillSlimes => "Kill Slimes",
        };
        let status_text = match status {
            QuestStatus::NotStarted => "Not started".to_string(),
            QuestStatus::InProgress { count, target } => {
                format!("[{}/{}] In Progress", count, target)
            }
            QuestStatus::ReadyToTurnIn => "Ready to turn in".to_string(),
            QuestStatus::Completed => "Completed".to_string(),
        };
        lines.push(format!("- {}: {}", label, status_text));
    }
    *text = Text::new(lines.join("\n"));
}
