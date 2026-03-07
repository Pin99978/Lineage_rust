use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::{network, Player};

const DIRECTIONS: usize = 8;
const FULL_SHEET_FRAMES_PER_STATE: usize = 3;
const FULL_SHEET_STATES: usize = 4;
const DIRECTIONAL_SHEET_FRAMES: usize = 8;
const FALLBACK_FRAMES_PER_DIRECTION: usize = 6;

#[derive(Resource, Clone)]
pub struct CharacterVisualAssets {
    pub sprite_sheet: Handle<Image>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayAttackAnimation {
    pub target_id: Option<u64>,
    pub local_player: bool,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterState {
    #[default]
    Idle,
    Walking,
    Attacking,
    Dead,
}

#[derive(Component, Debug, Clone)]
pub struct AnimationController {
    pub timer: Timer,
    pub current_frame: usize,
    pub direction_row: usize,
    pub attack_timer: Timer,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            current_frame: 0,
            direction_row: 0,
            attack_timer: Timer::from_seconds(0.28, TimerMode::Once),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct LastKnownPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct AtlasApplied;

#[derive(Component)]
pub struct StaticCharacterVisual;

#[derive(Component, Debug, Clone, Copy)]
pub struct AtlasConfig {
    pub frames_per_state: usize,
    pub directions: usize,
    pub state_count: usize,
}

#[derive(Component)]
pub struct BackgroundKeyed;

pub fn setup_character_visual_assets(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };

    let sprite_sheet: Handle<Image> = asset_server.load("textures/player.png");
    commands.insert_resource(CharacterVisualAssets { sprite_sheet });
}

#[allow(clippy::type_complexity)]
pub fn attach_animation_components(
    mut commands: Commands,
    entities: Query<(
        Entity,
        &shared::Position,
        Option<&Player>,
        Option<&network::Attackable>,
        Option<&network::Lootable>,
        Option<&AnimationController>,
    )>,
) {
    for (entity, position, local_player, attackable, lootable, controller) in &entities {
        if lootable.is_some() {
            continue;
        }
        if local_player.is_none() && attackable.is_none() {
            continue;
        }
        if controller.is_some() {
            continue;
        }

        commands.entity(entity).insert((
            CharacterState::Idle,
            AnimationController::default(),
            LastKnownPosition {
                x: position.x,
                y: position.y,
            },
        ));
    }
}

#[allow(clippy::type_complexity)]
pub fn apply_character_atlas_when_ready(
    assets: Option<Res<CharacterVisualAssets>>,
    asset_server: Option<Res<AssetServer>>,
    images: Option<ResMut<Assets<Image>>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
    mut sprites: Query<
        (Entity, &mut Sprite),
        (
            With<AnimationController>,
            Without<AtlasApplied>,
            Without<BackgroundKeyed>,
            Without<StaticCharacterVisual>,
        ),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(asset_server) = asset_server else {
        return;
    };
    let Some(mut images) = images else {
        return;
    };

    let load_state = asset_server.get_load_state(assets.sprite_sheet.id());
    if !matches!(load_state, Some(LoadState::Loaded)) {
        return;
    }

    if let Some(image) = images.get_mut(&assets.sprite_sheet) {
        remove_checkerboard_background(image);
    }

    let atlas_mode = images.get(&assets.sprite_sheet).and_then(|image| {
        let width = image.texture_descriptor.size.width;
        let height = image.texture_descriptor.size.height;

        let full_sheet_ok = width % FULL_SHEET_FRAMES_PER_STATE as u32 == 0
            && height % (DIRECTIONS as u32 * FULL_SHEET_STATES as u32) == 0
            && width / FULL_SHEET_FRAMES_PER_STATE as u32
                == height / (DIRECTIONS as u32 * FULL_SHEET_STATES as u32);
        if full_sheet_ok {
            return Some(AtlasConfig {
                frames_per_state: FULL_SHEET_FRAMES_PER_STATE,
                directions: DIRECTIONS,
                state_count: FULL_SHEET_STATES,
            });
        }

        let directional_sheet_ok = width % DIRECTIONAL_SHEET_FRAMES as u32 == 0
            && height % DIRECTIONS as u32 == 0
            && width / DIRECTIONAL_SHEET_FRAMES as u32 == height / DIRECTIONS as u32;
        if directional_sheet_ok {
            return Some(AtlasConfig {
                frames_per_state: DIRECTIONAL_SHEET_FRAMES,
                directions: DIRECTIONS,
                state_count: 1,
            });
        }

        // Placeholder art can be uneven; default to 6x8 slicing before
        // falling back to a static sprite.
        if width >= FALLBACK_FRAMES_PER_DIRECTION as u32 && height >= DIRECTIONS as u32 {
            return Some(AtlasConfig {
                frames_per_state: FALLBACK_FRAMES_PER_DIRECTION,
                directions: DIRECTIONS,
                state_count: 1,
            });
        }

        None
    });

    for (entity, mut sprite) in &mut sprites {
        if let Some(config) = atlas_mode {
            let Some(image) = images.get(&assets.sprite_sheet) else {
                continue;
            };
            let width = image.texture_descriptor.size.width;
            let height = image.texture_descriptor.size.height;
            let tile_w = (width / config.frames_per_state as u32).max(1);
            let tile_h = (height / (config.directions as u32 * config.state_count as u32)).max(1);
            let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
                UVec2::new(tile_w, tile_h),
                config.frames_per_state as u32,
                (config.directions * config.state_count) as u32,
                None,
                None,
            ));
            *sprite = Sprite::from_atlas_image(
                assets.sprite_sheet.clone(),
                TextureAtlas { layout, index: 0 },
            );
            sprite.custom_size = Some(Vec2::new(tile_w as f32, tile_h as f32));
            commands.entity(entity).insert((AtlasApplied, config));
        } else {
            *sprite = Sprite::from_image(assets.sprite_sheet.clone());
            sprite.custom_size = Some(Vec2::new(96.0, 96.0));
            commands.entity(entity).insert(StaticCharacterVisual);
        }
        commands.entity(entity).insert(BackgroundKeyed);
    }
}

pub fn trigger_attack_animation(
    mut events: MessageReader<PlayAttackAnimation>,
    local_player_query: Query<Entity, With<Player>>,
    entity_map: Option<Res<network::NetworkEntityMap>>,
    mut controllers: Query<&mut AnimationController>,
) {
    let local_player_entity = local_player_query.single().ok();
    let entity_map = entity_map.as_deref();

    for event in events.read() {
        if event.local_player {
            if let Some(player_entity) = local_player_entity {
                if let Ok(mut controller) = controllers.get_mut(player_entity) {
                    controller.attack_timer.reset();
                }
            }
        }
        if let (Some(target_id), Some(entity_map)) = (event.target_id, entity_map) {
            if let Some(entity) = entity_map.entity_by_id.get(&target_id).copied() {
                if let Ok(mut controller) = controllers.get_mut(entity) {
                    controller.attack_timer.reset();
                }
            }
        }
    }
}

pub fn update_animation_state(
    time: Res<Time>,
    mut query: Query<(
        &shared::Position,
        Option<&shared::Health>,
        &mut CharacterState,
        &mut AnimationController,
        &mut LastKnownPosition,
    )>,
) {
    for (position, health, mut state, mut controller, mut last_position) in &mut query {
        controller.attack_timer.tick(time.delta());

        let dx = position.x - last_position.x;
        let dy = position.y - last_position.y;
        if dx.abs() > 0.01 || dy.abs() > 0.01 {
            controller.direction_row = direction_row_from_vector(dx, dy);
        }

        *state = if health.map(|h| h.current <= 0).unwrap_or(false) {
            CharacterState::Dead
        } else if !controller.attack_timer.is_finished() {
            CharacterState::Attacking
        } else if dx.abs() > 0.2 || dy.abs() > 0.2 {
            CharacterState::Walking
        } else {
            CharacterState::Idle
        };

        last_position.x = position.x;
        last_position.y = position.y;
    }
}

pub fn animate_sprite_system(
    time: Res<Time>,
    mut query: Query<
        (
            &CharacterState,
            &AtlasConfig,
            &mut AnimationController,
            &mut Sprite,
        ),
        With<AtlasApplied>,
    >,
) {
    for (state, config, mut controller, mut sprite) in &mut query {
        let Some(texture_atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

        controller.timer.tick(time.delta());
        if !controller.timer.just_finished() {
            continue;
        }

        controller.current_frame = (controller.current_frame + 1) % config.frames_per_state;
        let state_offset = if config.state_count >= FULL_SHEET_STATES {
            match state {
                CharacterState::Idle => 0,
                CharacterState::Walking => 1,
                CharacterState::Attacking => 2,
                CharacterState::Dead => 3,
            }
        } else {
            0
        };
        let row =
            state_offset * config.directions + controller.direction_row.min(config.directions - 1);
        texture_atlas.index = row * config.frames_per_state + controller.current_frame;
    }
}

#[allow(clippy::type_complexity)]
pub fn update_static_character_visual(
    mut query: Query<
        (
            &shared::Position,
            &mut LastKnownPosition,
            Option<&shared::Health>,
            Option<&CharacterState>,
            &mut Sprite,
        ),
        With<StaticCharacterVisual>,
    >,
) {
    for (position, mut last_position, health, state, mut sprite) in &mut query {
        let dx = position.x - last_position.x;
        if dx.abs() > 0.2 {
            sprite.flip_x = dx < 0.0;
        }
        if health.map(|h| h.current <= 0).unwrap_or(false)
            || matches!(state, Some(CharacterState::Dead))
        {
            sprite.color = Color::srgb(0.45, 0.45, 0.45);
        } else {
            sprite.color = Color::WHITE;
        }
        last_position.x = position.x;
        last_position.y = position.y;
    }
}

fn direction_row_from_vector(dx: f32, dy: f32) -> usize {
    // 0..7: E, NE, N, NW, W, SW, S, SE
    let angle = dy.atan2(dx);
    let octant = ((angle / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
    octant as usize
}

fn remove_checkerboard_background(image: &mut Image) {
    let Some(bytes) = image.data.as_mut() else {
        return;
    };

    match image.texture_descriptor.format {
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => {
            for pixel in bytes.chunks_exact_mut(4) {
                if is_checker_pixel(pixel[0], pixel[1], pixel[2]) {
                    pixel[3] = 0;
                }
            }
        }
        TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm => {
            for pixel in bytes.chunks_exact_mut(4) {
                if is_checker_pixel(pixel[2], pixel[1], pixel[0]) {
                    pixel[3] = 0;
                }
            }
        }
        _ => {}
    }
}

fn is_checker_pixel(r: u8, g: u8, b: u8) -> bool {
    let drg = (r as i16 - g as i16).abs();
    let dgb = (g as i16 - b as i16).abs();
    let base = r.max(g).max(b);
    let low = r.min(g).min(b);
    let gray_like = drg <= 10 && dgb <= 10;
    let light_checker = (170..=245).contains(&base) && (160..=240).contains(&low);
    gray_like && light_checker
}
