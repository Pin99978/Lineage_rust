use bevy::prelude::*;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct TargetPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct MoveSpeed {
    pub value: f32,
}
