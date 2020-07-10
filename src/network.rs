use crate::entities::Ship;
use crate::entities::Bullet;

use std::net::SocketAddr;

use rand_xoshiro::Xoshiro256Plus;

use serde::{Serialize, Deserialize};

use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum Wrapper {
    BulletWrapper(Bullet),
    ShipWrapper(Ship),
    AddressWrapper(SocketAddr),
    AddressesWrapper(Vec<SocketAddr>),
    ShipUpdateWrapper(ShipUpdate),
    Rng(Option<Xoshiro256Plus>),
    ConnectSignal,
    StartSignal,
    RestartSignal,
    WinSignal,
    HitSignal(Uuid, Option<u64>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShipUpdate {
    pub id: Uuid,
    pub x: f32,
    pub y: f32,
    pub shield: bool,
}

impl ShipUpdate {
    pub fn new(id: Uuid, x: f32, y: f32, shield: bool) -> ShipUpdate { 
        ShipUpdate {
            id,
            x,
            y,
            shield,
        }
    }
}

pub enum Network {
    Host,
    Peer,
}
