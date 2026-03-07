use bevy::prelude::*;
use shared::protocol::LoginRequest;
use shared::{EquipmentMap, Health};

use crate::{network, Player};

pub mod chat;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    LoginMenu,
    InGame,
}

#[derive(Resource)]
pub struct LoginName {
    pub username: String,
    pub submitted: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct HudState {
    pub mana_current: i32,
    pub mana_max: i32,
    pub equipment: EquipmentMap,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            mana_current: 60,
            mana_max: 60,
            equipment: EquipmentMap::default(),
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct DialogState {
    pub text: String,
    pub timer: Timer,
    pub visible: bool,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            text: String::new(),
            timer: Timer::from_seconds(4.0, TimerMode::Once),
            visible: false,
        }
    }
}

#[derive(Component)]
pub struct LoginMenuRoot;

#[derive(Component)]
pub struct PlayerHealthBarUi;

#[derive(Component)]
pub struct PlayerManaBarUi;

#[derive(Component)]
pub struct EquipmentTextUi;

#[derive(Component)]
pub struct DialogTextUi;

#[derive(Component)]
pub struct DialogPanelUi;

pub fn setup_login_menu(mut commands: Commands, login_name: Option<Res<LoginName>>) {
    let username = login_name
        .map(|value| value.username.clone())
        .unwrap_or_else(|| "adventurer".to_string());
    commands.spawn((
        LoginMenuRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.08, 0.08, 0.09)),
        children![(
            Node {
                width: Val::Px(560.0),
                height: Val::Px(160.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.14, 0.9)),
            children![
                (Text::new("Login Menu"), TextFont::from_font_size(32.0)),
                (
                    Text::new(format!("Username: {}", username)),
                    TextFont::from_font_size(22.0)
                ),
                (
                    Text::new("Press Enter to Login"),
                    TextFont::from_font_size(18.0)
                )
            ]
        )],
    ));
}

pub fn login_submit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    login_name: Option<ResMut<LoginName>>,
    network: Option<Res<network::ClientNetwork>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(mut login_name) = login_name else {
        return;
    };
    if login_name.submitted {
        return;
    }
    let Some(network) = network else {
        return;
    };

    login_name.submitted = true;
    network::send_login_request(
        &network,
        LoginRequest {
            username: login_name.username.clone(),
        },
    );
}

pub fn cleanup_login_menu(mut commands: Commands, roots: Query<Entity, With<LoginMenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

pub fn setup_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                width: Val::Px(340.0),
                height: Val::Px(130.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceEvenly,
                padding: UiRect::all(Val::Px(12.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.85)),
        ))
        .id();

    let health_bg = commands
        .spawn((
            Node {
                width: Val::Px(260.0),
                height: Val::Px(16.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.12, 0.02, 0.02)),
        ))
        .id();

    let health_fill = commands
        .spawn((
            PlayerHealthBarUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.86, 0.12, 0.12)),
        ))
        .id();

    let mana_bg = commands
        .spawn((
            Node {
                width: Val::Px(260.0),
                height: Val::Px(14.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.02, 0.06, 0.12)),
        ))
        .id();

    let mana_fill = commands
        .spawn((
            PlayerManaBarUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.12, 0.45, 0.9)),
        ))
        .id();

    let equipment_text = commands
        .spawn((
            EquipmentTextUi,
            Text::new("Equip: Weapon=None Armor=None"),
            TextFont::from_font_size(16.0),
            TextColor(Color::srgb(0.9, 0.92, 0.95)),
        ))
        .id();

    let hint_text = commands
        .spawn((
            Text::new("1 Fireball | 2 Heal | E/R Equip | Q/W Unequip"),
            TextFont::from_font_size(14.0),
            TextColor(Color::srgb(0.75, 0.8, 0.9)),
        ))
        .id();

    commands.entity(root).add_child(health_bg);
    commands.entity(health_bg).add_child(health_fill);
    commands.entity(root).add_child(mana_bg);
    commands.entity(mana_bg).add_child(mana_fill);
    commands.entity(root).add_child(equipment_text);
    commands.entity(root).add_child(hint_text);

    commands.spawn((
        DialogPanelUi,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            left: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-180.0),
                ..Default::default()
            },
            width: Val::Px(360.0),
            height: Val::Px(72.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.85)),
        Visibility::Hidden,
        children![(
            DialogTextUi,
            Text::new(""),
            TextFont::from_font_size(20.0),
            TextColor(Color::srgb(0.95, 0.95, 0.9))
        )],
    ));

    chat::setup_chat_ui(&mut commands);
}

pub fn update_player_health_hud(
    player_health: Query<&Health, (With<Player>, Changed<Health>)>,
    mut bar_query: Query<&mut Node, With<PlayerHealthBarUi>>,
) {
    let Ok(health) = player_health.single() else {
        return;
    };
    let Ok(mut bar_node) = bar_query.single_mut() else {
        return;
    };

    let ratio = if health.max > 0 {
        (health.current as f32 / health.max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    bar_node.width = Val::Percent(ratio * 100.0);
}

pub fn update_player_mana_hud(
    hud_state: Option<Res<HudState>>,
    mut bar_query: Query<&mut Node, With<PlayerManaBarUi>>,
) {
    let Some(hud_state) = hud_state else {
        return;
    };
    let Ok(mut bar_node) = bar_query.single_mut() else {
        return;
    };

    let ratio = if hud_state.mana_max > 0 {
        (hud_state.mana_current as f32 / hud_state.mana_max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    bar_node.width = Val::Percent(ratio * 100.0);
}

pub fn update_equipment_text_hud(
    hud_state: Option<Res<HudState>>,
    mut text_query: Query<&mut Text, With<EquipmentTextUi>>,
) {
    let Some(hud_state) = hud_state else {
        return;
    };
    if !hud_state.is_changed() {
        return;
    }
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    *text = Text::new(format!(
        "Equip: Weapon={:?} Armor={:?}",
        hud_state.equipment.weapon, hud_state.equipment.armor
    ));
}

#[allow(clippy::type_complexity)]
pub fn update_dialog_hud(
    time: Res<Time>,
    dialog_state: Option<ResMut<DialogState>>,
    mut nodes: Query<(&mut Visibility, &Children), With<DialogPanelUi>>,
    mut texts: Query<&mut Text, With<DialogTextUi>>,
) {
    let Some(mut dialog_state) = dialog_state else {
        return;
    };

    if dialog_state.visible {
        dialog_state.timer.tick(time.delta());
        if dialog_state.timer.is_finished() {
            dialog_state.visible = false;
        }
    }

    for (mut visibility, children) in &mut nodes {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                if dialog_state.visible {
                    *visibility = Visibility::Visible;
                    *text = Text::new(dialog_state.text.clone());
                } else {
                    *visibility = Visibility::Hidden;
                }
                return;
            }
        }
    }
}
