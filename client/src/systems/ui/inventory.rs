use bevy::prelude::*;
use shared::{item_slot, scroll_spell, ItemType};
use std::collections::HashMap;

use crate::network;

#[derive(Resource, Debug, Clone, Default)]
pub struct LocalInventoryState {
    pub items: HashMap<ItemType, u32>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct UiWindowsState {
    pub inventory_open: bool,
    pub paperdoll_open: bool,
    pub guild_open: bool,
}

impl UiWindowsState {
    pub fn blocks_world_input(&self) -> bool {
        self.inventory_open || self.paperdoll_open || self.guild_open
    }
}

#[derive(Component)]
pub struct InventoryPanelRoot;

#[derive(Component, Debug, Clone, Copy)]
pub struct InventoryItemButton {
    pub item_type: ItemType,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct InventoryItemText {
    pub item_type: ItemType,
}

pub fn setup_inventory_ui(commands: &mut Commands) {
    let panel = commands
        .spawn((
            InventoryPanelRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                width: Val::Px(320.0),
                height: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.9)),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(panel).with_children(|parent| {
        parent.spawn((
            Text::new("Inventory [I]"),
            TextFont::from_font_size(20.0),
            TextColor(Color::srgb(0.9, 0.92, 0.97)),
        ));

        for item_type in [
            ItemType::BronzeSword,
            ItemType::LeatherArmor,
            ItemType::HealthPotion,
            ItemType::ScrollLightning,
            ItemType::ScrollPoisonArrow,
            ItemType::ScrollBless,
            ItemType::Gold,
        ] {
            parent
                .spawn((
                    Button,
                    InventoryItemButton { item_type },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        padding: UiRect::left(Val::Px(10.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.15, 0.92)),
                ))
                .with_children(|button| {
                    button.spawn((
                        InventoryItemText { item_type },
                        Text::new(""),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::srgb(0.86, 0.88, 0.93)),
                    ));
                });
        }
    });
}

pub fn toggle_inventory_window_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    chat_state: Option<Res<super::chat::ChatUiState>>,
    windows_state: Option<ResMut<UiWindowsState>>,
) {
    if chat_state
        .as_ref()
        .map(|state| state.focused)
        .unwrap_or(false)
    {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyI) {
        return;
    }
    let Some(mut windows_state) = windows_state else {
        return;
    };
    windows_state.inventory_open = !windows_state.inventory_open;
}

pub fn apply_inventory_visibility_system(
    windows_state: Option<Res<UiWindowsState>>,
    mut panels: Query<&mut Visibility, With<InventoryPanelRoot>>,
) {
    let Some(windows_state) = windows_state else {
        return;
    };

    for mut visibility in &mut panels {
        *visibility = if windows_state.inventory_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn refresh_inventory_ui_system(
    inventory_state: Option<Res<LocalInventoryState>>,
    mut texts: Query<(&InventoryItemText, &mut Text)>,
) {
    let Some(inventory_state) = inventory_state else {
        return;
    };
    if !inventory_state.is_changed() {
        return;
    }

    for (marker, mut text) in &mut texts {
        let count = inventory_state
            .items
            .get(&marker.item_type)
            .copied()
            .unwrap_or(0);
        let item_name = format!("{:?}", marker.item_type);
        let action_hint = if item_slot(marker.item_type).is_some() {
            "Click to Equip"
        } else if marker.item_type == ItemType::HealthPotion
            || scroll_spell(marker.item_type).is_some()
        {
            "Click to Use"
        } else {
            "Not Usable"
        };
        *text = Text::new(format!("{} x{}  [{}]", item_name, count, action_hint));
    }
}

#[allow(clippy::type_complexity)]
pub fn inventory_click_equip_system(
    network: Option<Res<network::ClientNetwork>>,
    windows_state: Option<Res<UiWindowsState>>,
    interactions: Query<
        (&Interaction, &InventoryItemButton),
        (
            Changed<Interaction>,
            With<Button>,
            Without<super::paperdoll::PaperdollSlotButton>,
        ),
    >,
) {
    let Some(network) = network else {
        return;
    };
    let Some(windows_state) = windows_state else {
        return;
    };
    if !windows_state.inventory_open {
        return;
    }

    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if item_slot(button.item_type).is_some() {
            network::equip_item_by_hotkey(&network, button.item_type);
        } else if button.item_type == ItemType::HealthPotion
            || scroll_spell(button.item_type).is_some()
        {
            network::send_use_item_intent(
                &network,
                shared::protocol::UseItemIntent {
                    item_type: button.item_type,
                },
            );
        }
    }
}
