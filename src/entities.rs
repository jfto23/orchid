use crate::constants::*;
use crate::assets::Assets;
use crate::Point;
use crate::states::InputState;

use uuid::Uuid;

use serde::{Serialize, Deserialize};

use std::f32::consts;

use mint;

use ggez::{graphics, Context, GameResult};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum Possession {
    Player,
    Enemy,
}

//==============================
//          BULLET
//==============================

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Bullet {
    pub possession: Possession,
    pub angle: f32,
    pub pos: Point,
    pub hit: bool,
    pub bullet_type: BulletType

}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum BulletType {
    Normal,
    Special,
}


impl Bullet {
    pub fn new(possession: Possession, angle: f32, pos: Point, bullet_type: BulletType) -> Bullet {
        let new_pos = Point{
            x: pos.x, 
            // the adjustments on y make is so the bullets don't
            // appear on the ship itself
            y: pos.y - (consts::PI/2.0-angle).sin()*20.0,
        };
        Bullet {
            possession: possession,
            angle: angle,
            pos: new_pos,
            hit: false,
            bullet_type: bullet_type,
        }
    }

    pub fn draw(&self, assets: &mut Assets, ctx: &mut Context) -> GameResult {

        let img = match self.possession {
            Possession::Player => match self.bullet_type {
                BulletType::Normal => &assets.player_bullet,
                BulletType::Special => &assets.special_bullet,
            }
            Possession::Enemy => &assets.enemy_bullet,
        };

        let drawparams = graphics::DrawParam::new()
            .dest(mint::Point2{ x: self.pos.x, y: self.pos.y })
            .offset(mint::Point2{ x:0.5, y:0.5 });

        graphics::draw(ctx,img,drawparams)
    }

    pub fn update_pos(&mut self, dt: f32) {
        match self.bullet_type {
            // if more bullet types are added, make a bullet_type struct instead to avoid copy paste
            BulletType::Normal => {
                self.pos.x += (consts::PI/2.0-self.angle).cos() * dt * BULLET_SPEED;
                self.pos.y -= (consts::PI/2.0-self.angle).sin() * dt * BULLET_SPEED;
            }
            BulletType::Special => {
                self.pos.x += (consts::PI/2.0-self.angle).cos() * dt * SPECIAL_BULLET_SPEED;
                self.pos.y -= (consts::PI/2.0-self.angle).sin() * dt * SPECIAL_BULLET_SPEED;
            }

        }
    }
}

//==============================
//          SHIP
//==============================

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Ship {
    pub health: f32,
    pub ship_type: Possession,
    pub pos: Point,
    pub angle: f32,
    pub direction: Option<f32>,
    pub shield: bool,
    pub id: Uuid,
}

impl Ship {
    pub fn new(ship_type: Possession) -> Ship {
        match ship_type {
            Possession::Player => {
                Ship {
                    health: 1.0,
                    ship_type: ship_type,
                    pos: PLAYER_SPAWN,
                    angle: 0.0,
                    direction: None,
                    shield: false,
                    id: Uuid::new_v4(),
                }
            }
            Possession::Enemy => {
                Ship {
                    health: BOSS_HEALTH,
                    ship_type: ship_type,
                    pos: Point{ x:400.0, y:50.0 },
                    angle: consts::PI,
                    direction: Some(1.0),
                    shield: false,
                    id: Uuid::new_v4(),
                }
            }
        }
    }

    pub fn reset(&mut self) {
        match self.ship_type {
            Possession::Player => {
                self.health = 1.0;
                self.pos = PLAYER_SPAWN;
                self.shield = false;
            
            },
            Possession::Enemy => {
                self.health = BOSS_HEALTH;
                self.pos = Point{ x:400.0, y:50.0};
                self.shield = false;
                self.direction = Some(1.0);
            },

        }

    }

    pub fn move_to_point(&mut self, p: Point) {
        self.pos.x = p.x;
        self.pos.y = p.y;
    }

    // the optionnal argument lets ships shoot in more directions
    // i.e bosses can shoot in diagonals
    pub fn shoot(&self, curve: Option<f32>, bullet_type: BulletType) -> Bullet {
        match curve {
            Some(angle) => Bullet::new(self.ship_type, self.angle+angle, self.pos, BulletType::Normal),
            _ => Bullet::new(self.ship_type, self.angle, self.pos, bullet_type),
        }
    }

    pub fn draw(&self, assets: &mut Assets, ctx: &mut Context, version: Option<i32>) -> GameResult {

        let img = match self.ship_type {
            Possession::Player => {
                if self.health < 0.0 {
                    &assets.player_dead
                }
                else if self.shield {
                    &assets.shield
                }
                else {
                    if let Some(1) = version {
                        &assets.player_ship
                    }
                    else {
                        &assets.other_players
                    }
                }
            },
            Possession::Enemy => &assets.enemy_ship,
        };

        let drawparams = graphics::DrawParam::new()
            .dest(mint::Point2{ x: self.pos.x, y: self.pos.y })
            .rotation(self.angle)
            .offset(mint::Point2{ x:0.5, y:0.5 })
            .scale(mint::Vector2{x: 2.0, y: 2.0});
        graphics::draw(ctx,img,drawparams)
    }

    // returns true if the ship moved, false otherwise
    pub fn update_pos(&mut self, dt: f32, input_state: &InputState, width: f32, height: f32) -> bool {
        let old_x = self.pos.x;
        let old_y = self.pos.y;
        if self.health < 0.0 {
            return false
        }
        if input_state.up && self.pos.y >= SCREEN_BORDER {
            self.pos.y -= dt *SHIP_SPEED;
        }
        if input_state.down && self.pos.y <= height - SCREEN_BORDER {
            self.pos.y += dt *SHIP_SPEED;
        }
        if input_state.right && self.pos.x <= width - SCREEN_BORDER {
            self.pos.x += dt *SHIP_SPEED;
        }
        if input_state.left && self.pos.x >= SCREEN_BORDER {
            self.pos.x -= dt *SHIP_SPEED;
        }

        self.pos.x != old_x || self.pos.y != old_y
    }

    // for boss ship
    pub fn oscillate(&mut self, dt: f32, width: f32) {
        if self.pos.x <= 0.0 + SCREEN_BORDER {
            self.direction = Some(1.0);
        }
        else if self.pos.x >= width - SCREEN_BORDER {
            self.direction = Some(-1.0);
        }

        self.pos.x += dt * BOSS_SPEED * self.direction.unwrap();

    }
}
