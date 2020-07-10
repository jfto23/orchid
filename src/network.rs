use crate::entities::Ship;
use crate::entities::Bullet;

use std::net::SocketAddr;

use serde::{Serialize, Deserialize};

use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum Wrapper {
    BulletWrapper(Bullet),
    ShipWrapper(Ship),
    AddressWrapper(SocketAddr),
    AddressesWrapper(Vec<SocketAddr>),
    ShipUpdateWrapper(ShipUpdate),
    ConnectSignal,
    StartSignal,
    RestartSignal,
    WinSignal,
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
