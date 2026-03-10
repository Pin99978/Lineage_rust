use bevy::prelude::*;
use shared::{spell_def, EquipmentSlot, ItemType, SpellType};

use crate::network;

#[derive(Resource, Debug, Clone, Default)]
pub struct LocalEquipmentState {
    pub weapon: Option<ItemType>,
    pub armor: Option<ItemType>,
}

#[derive(Component)]
pub struct PaperdollPanelRoot;

#[derive(Component, Debug, Clone, Copy)]
pub struct PaperdollSlotButton {
    pub slot: EquipmentSlot,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct PaperdollSlotText {
    pub slot: EquipmentSlot,
}

#[derive(Component)]
pub struct PaperdollStatsText;

#[derive(Component)]
pub struct PaperdollKnownSpellsText;

pub fn setup_paperdoll_ui(commands: &mut Commands) {
    let panel = commands
        .spawn((
            PaperdollPanelRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(360.0),
                bottom: Val::Px(24.0),
                width: Val::Px(280.0),
                height: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.9)),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).with_children(|parent| {
        parent.spawn((
            Text::new("Character [C]"),
            TextFont::from_font_size(20.0),
            TextColor(Color::srgb(0.93, 0.94, 0.98)),
        ));

        for slot in [EquipmentSlot::Weapon, EquipmentSlot::Armor] {
            parent
                .spawn((
                    Button,
                    PaperdollSlotButton { slot },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(54.0),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        padding: UiRect::left(Val::Px(10.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.15, 0.92)),
                ))
                .with_children(|button| {
                    button.spawn((
                        PaperdollSlotText { slot },
                        Text::new(""),
                        TextFont::from_font_size(17.0),
                        TextColor(Color::srgb(0.88, 0.9, 0.96)),
                    ));
                });
        }

        parent.spawn((
            PaperdollStatsText,
            Text::new("[Knight] LV 1  EXP 0/100\nSTR 15  DEX 15  INT 15  CON 15"),
            TextFont::from_font_size(15.0),
            TextColor(Color::srgb(0.85, 0.9, 0.95)),
        ));

        parent.spawn((
            PaperdollKnownSpellsText,
            Text::new("Known Spells:\n-"),
            TextFont::from_font_size(14.0),
            TextColor(Color::srgb(0.8, 0.86, 0.92)),
        ));
    });
}

pub fn toggle_paperdoll_window_system(
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
    if !keyboard.just_pressed(KeyCode::KeyC) {
        return;
    }
    let Some(mut windows_state) = windows_state else {
        return;
    };
    windows_state.paperdoll_open = !windows_state.paperdoll_open;
}

pub fn apply_paperdoll_visibility_system(
    windows_state: Option<Res<super::inventory::UiWindowsState>>,
    mut panels: Query<&mut Visibility, With<PaperdollPanelRoot>>,
) {
    let Some(windows_state) = windows_state else {
        return;
    };

    for mut visibility in &mut panels {
        *visibility = if windows_state.paperdoll_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn refresh_paperdoll_ui_system(
    hud_state: Option<Res<super::HudState>>,
    equipment_state: Option<Res<LocalEquipmentState>>,
    mut slot_texts: Query<
        (&PaperdollSlotText, &mut Text),
        (Without<PaperdollStatsText>, Without<PaperdollKnownSpellsText>),
    >,
    mut stats_texts: Query<
        &mut Text,
        (
            With<PaperdollStatsText>,
            Without<PaperdollSlotText>,
            Without<PaperdollKnownSpellsText>,
        ),
    >,
    mut known_spells_texts: Query<
        &mut Text,
        (
            With<PaperdollKnownSpellsText>,
            Without<PaperdollSlotText>,
            Without<PaperdollStatsText>,
        ),
    >,
) {
    let Some(hud_state) = hud_state else {
        return;
    };
    let Some(equipment_state) = equipment_state else {
        return;
    };
    if !equipment_state.is_changed() && !hud_state.is_changed() {
        return;
    }

    for (marker, mut text) in &mut slot_texts {
        let equipped = match marker.slot {
            EquipmentSlot::Weapon => equipment_state.weapon,
            EquipmentSlot::Armor => equipment_state.armor,
        };
        *text = Text::new(format!("{:?}: {:?}", marker.slot, equipped));
    }

    if let Ok(mut stats_text) = stats_texts.single_mut() {
        *stats_text = Text::new(format!(
            "[{}] LV {}  EXP {}/{}\nSTR {}  DEX {}  INT {}  CON {}",
            super::class_label(hud_state.class),
            hud_state.level,
            hud_state.exp_current,
            hud_state.exp_next,
            hud_state.str_stat,
            hud_state.dex,
            hud_state.int_stat,
            hud_state.con
        ));
    }

    if let Ok(mut known_spells_text) = known_spells_texts.single_mut() {
        let lines = if hud_state.known_spells.is_empty() {
            "-".to_string()
        } else {
            hud_state
                .known_spells
                .iter()
                .map(|spell| format!("{} (Lv {})", spell_label(*spell), spell_def(*spell).req_level))
                .collect::<Vec<String>>()
                .join("\n")
        };
        *known_spells_text = Text::new(format!("Known Spells:\n{}", lines));
    }
}

#[allow(clippy::type_complexity)]
pub fn paperdoll_click_unequip_system(
    network: Option<Res<network::ClientNetwork>>,
    windows_state: Option<Res<super::inventory::UiWindowsState>>,
    equipment_state: Option<Res<LocalEquipmentState>>,
    interactions: Query<
        (&Interaction, &PaperdollSlotButton),
        (
            Changed<Interaction>,
            With<Button>,
            Without<super::inventory::InventoryItemButton>,
        ),
    >,
) {
    let Some(network) = network else {
        return;
    };
    let Some(windows_state) = windows_state else {
        return;
    };
    let Some(equipment_state) = equipment_state else {
        return;
    };
    if !windows_state.paperdoll_open {
        return;
    }

    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let has_item = match button.slot {
            EquipmentSlot::Weapon => equipment_state.weapon.is_some(),
            EquipmentSlot::Armor => equipment_state.armor.is_some(),
        };
        if !has_item {
            continue;
        }
        network::unequip_slot_by_hotkey(&network, button.slot);
    }
}

fn spell_label(spell: SpellType) -> &'static str {
    match spell {
        SpellType::Fireball => "Fireball",
        SpellType::Heal => "Heal",
        SpellType::Lightning => "Lightning",
        SpellType::PoisonArrow => "Poison Arrow",
        SpellType::Bless => "Bless",
    }
}
