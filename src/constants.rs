use ggez::graphics;
use crate::Point;

pub const SHIP_SPEED: f32 = 350.0;
pub const BOSS_SPEED: f32 = 125.0;
pub const BULLET_SPEED: f32 = 500.0;
pub const PLAYER_FIRE_RATE: f64 = 200.0;
pub const BOSS_FIRE_RATE: f64 = 250.0;
pub const SPECIAL_BULLET_SPEED: f32 = 250.0;
pub const SPECIAL_BULLET_COOLDOWN: f32 = 5.0;
pub const SPECIAL_BULLET_DAMAGE: f32 = 5.0;
pub const GREEN: graphics::Color = graphics::Color::new(0.0, 255.0, 0.0, 1.0);
pub const RED: graphics::Color = graphics::Color::new(255.0, 0.0, 0.0, 1.0);
pub const SCREEN_BORDER: f32 = 20.0;
pub const SHIELD_COOLDOWN: f32 = 15.0;
pub const SHIELD_DURATION: f32 = 2.0;
pub const BOSS_HEALTH: f32 = 1.0;
pub const BROADCAST_TICK: f32 = 1.0/30.0;
pub const PLAYER_SPAWN: Point = Point{ x: 400.0, y: 500.0};
