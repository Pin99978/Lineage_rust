use bevy::prelude::*;

pub mod components;
pub mod protocol;

pub use components::combat::{ActionState, CombatStats, Health};
pub use components::movement::{MoveSpeed, Position, TargetPosition};

pub struct MovementComponentsPlugin;

impl Plugin for MovementComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Position>()
            .register_type::<TargetPosition>()
            .register_type::<MoveSpeed>()
            .register_type::<Health>()
            .register_type::<CombatStats>()
            .register_type::<ActionState>();
    }
}
