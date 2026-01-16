//! Input handling for sail controls.
//!
//! Handles keyboard input for sail angle control.

use bevy::prelude::*;

/// Resource tracking the current sail angle.
#[derive(Resource, Default)]
pub struct SailControl {
    /// Sail angle in radians. 0 = sail perpendicular to wind, positive = clockwise rotation.
    pub angle: f32,
}

/// System to handle keyboard input for sail rotation.
pub fn handle_sail_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sail_control: ResMut<SailControl>,
) {
    const ROTATION_SPEED: f32 = 0.03; // radians per frame
    const MAX_ANGLE: f32 = 1.5; // ~85 degrees

    if keyboard.pressed(KeyCode::KeyA) {
        sail_control.angle -= ROTATION_SPEED;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        sail_control.angle += ROTATION_SPEED;
    }
    
    // Clamp to valid range
    sail_control.angle = sail_control.angle.clamp(-MAX_ANGLE, MAX_ANGLE);
}
