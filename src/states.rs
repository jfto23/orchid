use crate::constants::*;
use crate::entities::{ Ship, Bullet, Possession, BulletType } ;
use crate::network::{ Wrapper, Network, ShipUpdate };
use crate::distance_2d;
use crate::assets::Assets;

use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use std::net::{UdpSocket, SocketAddr};
use std::f32::consts;

use rand::Rng;

use ggez::{graphics, Context, GameResult};
use ggez::event::{EventHandler, KeyCode, KeyMods};

use serde::{Serialize, Deserialize};

//=================
//   GAME STATE
//=================

#[derive(Serialize, Deserialize, Debug)]
pub enum State {
    Playing,
    Won,
    Lost,
    Loading,
}

//=================
//   INPUT STATE
//=================

#[derive(Serialize, Deserialize)]
pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub right: bool,
    pub left: bool,
    pub fire: bool,
    pub special: bool,
    pub shield: bool,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            up: false,
            down: false,
            right: false,
            left: false,
            fire: false,
            special: false,
            shield: false,
        }
    }
}


//=================
//   MAIN STATE
//=================

pub struct MainState {
    player_ship: Ship,
    enemy_ship: Ship,
    bullets: Vec<Bullet>,
    assets: Assets,
    input_state: InputState,
    player_fire_delay: f64,
    enemy_fire_delay: f64,
    special_timer: f32,
    shield_timer: f32,
    shield_active: f32,
    state: State,
    other_players: Vec::<Ship>,
    network_type: Network,
    socket: UdpSocket,
    peers: Vec<SocketAddr>,
    broadcast_timer: f32,
}

impl MainState {
    pub fn new(ctx: &mut Context, network_type: Network, socket: UdpSocket) -> MainState {
        MainState {
            player_ship: Ship::new(Possession::Player),
            enemy_ship: Ship::new(Possession::Enemy),
            bullets: Vec::<Bullet>::new(),
            assets: Assets::new(ctx).unwrap(),
            input_state: InputState::new(),
            player_fire_delay: 0.0,
            enemy_fire_delay: 0.0,
            special_timer: 0.0,
            shield_timer: 0.0,
            shield_active: 0.0,
            state: State::Loading,
            other_players: Vec::<Ship>::new(),
            network_type: network_type,
            socket: socket,
            peers: Vec::<SocketAddr>::new(),
            broadcast_timer: BROADCAST_TICK,
        }
    }
fn check_collisions(&mut self) {

        for bullet in &mut self.bullets {
            match bullet.possession {
                Possession::Enemy => {
                    for i in 0..(self.other_players.len()+1) {
                        let (player_distance, mut player) = if i == self.other_players.len() {
                            (distance_2d(bullet.pos, self.player_ship.pos), &mut self.player_ship)
                        }
                        else {
                            (distance_2d(bullet.pos, self.other_players[i].pos), &mut self.other_players[i])
                        };

                        let shield = player.shield;
                        if player_distance < 24.0 && self.enemy_ship.health > 0.0 && player.health > 0.0 {
                            if !shield && player.health > 0.0 {
                                player.health -= 2.0;
                            }
                            bullet.hit = true;
                            break;
                        }
                    }

                }

                Possession::Player => {
                    let enemy_distance = distance_2d(bullet.pos, self.enemy_ship.pos);
                    if enemy_distance < 40.0 {
                        match bullet.bullet_type {
                            BulletType::Normal => self.enemy_ship.health -= 1.0,
                            BulletType::Special => self.enemy_ship.health -= SPECIAL_BULLET_DAMAGE,

                        }
                        bullet.hit = true;
                    }

                }
            };
        }
    }


    fn reset(&mut self) {
            self.player_ship.reset();
            self.enemy_ship.reset();
            self.bullets = Vec::<Bullet>::new();
            self.input_state = InputState::new();
            self.player_fire_delay = 0.0;
            self.enemy_fire_delay = 0.0;
            self.special_timer = 0.0;
            self.shield_timer = 0.0;
            self.shield_active = 0.0;
            self.state = State::Loading;

            for ship in &mut self.other_players {
                ship.reset();
            }
    }


    fn draw_indicators(&mut self, ctx: &mut Context) -> GameResult {

        let special_text = graphics::Text::new(("SPECIAL(J)",self.assets.font,16.0));
        let special_color = if self.special_timer < 0.0 {
            GREEN
        }
        else {
            RED
        };
        
        let shield_text = graphics::Text::new(("SHIELD(K)",self.assets.font,16.0));
        let shield_color = if self.shield_timer < 0.0 {
            GREEN
        }
        else {
            RED
        };

        graphics::draw(ctx, &special_text, (mint::Point2{x:5.0,y:5.0}, 0.0, special_color))?;
        graphics::draw(ctx, &shield_text, (mint::Point2{x:180.0,y:5.0}, 0.0, shield_color))?;
        Ok(())
    }

    fn draw_death_screen(&mut self, ctx: &mut Context) -> GameResult {
        let text = graphics::Text::new(("YOU DIED",self.assets.font,16.0));
        graphics::draw(ctx, &text, (mint::Point2{x:350.0,y:100.0}, 0.0, graphics::WHITE))?;
        Ok(())
    }

    fn draw_win_screen(&mut self, ctx: &mut Context) -> GameResult {
        let text = graphics::Text::new(("YOU WON",self.assets.font,16.0));
        graphics::draw(ctx, &text, (mint::Point2{x:350.0,y:100.0}, 0.0, graphics::WHITE))?;
        Ok(())
    }

    fn send_to_peers(&self, msg: Wrapper) {
        let encoded = bincode::serialize(&msg).unwrap();
        for peer in self.peers.iter() {
            self.socket.send_to(&encoded, peer).unwrap();
        }
    }

    fn handle_connections(&mut self) -> Result<(), Box<dyn Error>> {

        for ship in &mut self.other_players {
            ship.move_to_point(PLAYER_SPAWN);
        }

        // first make all ships appear
        if self.peers.len() != self.other_players.len() {

            println!("loading all ships...");
            let mut buf = [0u8; 512];
            let result = self.socket.recv_from(&mut buf);

            match result {
                Ok(_) => {
                    let decoded: Wrapper = bincode::deserialize(&buf)?;
                    match decoded {
                        Wrapper::ShipWrapper(ship) => {
                            let index = self.other_players
                                .iter()
                                .position(|&x| x.id == ship.id);
                            match index {
                                Some(_) => {},
                                None => self.other_players.push(ship),
                            }
                        },
                        _ => {},
                    }
                },
                Err(_) => {},
            }

        }

        // this is horrible but it's only during
        // loading phase
        let msg  = Wrapper::ShipWrapper(self.player_ship);
        self.send_to_peers(msg);


        let mut buf = [0u8; 512];
        let result = self.socket.recv_from(&mut buf);

        match self.network_type {
            Network::Host => {
                match result {
                    // some client connected to host
                    Ok((_amt, src)) => {
                        let decoded: Wrapper = bincode::deserialize(&buf)?;
                        match decoded {
                            Wrapper::ConnectSignal => {
                                let new_address = bincode::serialize(&Wrapper::AddressWrapper(src))?;

                                for peer in self.peers.iter() {
                                    let encoded_address = bincode::serialize(&Wrapper::AddressWrapper(*peer))?;
                                    self.socket.send_to(&encoded_address, src)?;
                                    self.socket.send_to(&new_address, peer)?;
                                }
                                let encoded_host = bincode::serialize(&Wrapper::AddressWrapper(self.socket.local_addr()?))?;
                                self.socket.send_to(&encoded_host, src)?;
                                self.peers.push(src);

                            },

                            Wrapper::StartSignal => self.state = State::Playing,
                            _ => {},
                        }

                    },

                    Err(_) => {},
                }
            },

            Network::Peer => {
                match result {
                    Ok((_amt, _src)) => {
                        let decoded: Wrapper = bincode::deserialize(&buf)?;
                        match decoded {
                            Wrapper::AddressWrapper(address) => {
                                self.peers.push(address);
                            },
                            Wrapper::StartSignal => self.state = State::Playing,
                            _ => {}
                        }
                    },
                    Err(_) => {},

                }

            }
        }

        Ok(())
    }

    fn handle_updates(&mut self) -> Result<(), Box<dyn Error>> {

        let mut buf = [0u8; 128];
        let result = self.socket.recv_from(&mut buf);
        match result {
            Ok((_amt, _src)) => {
                let decoded: Wrapper = bincode::deserialize(&buf)?;

                match decoded {
                    Wrapper::ShipUpdateWrapper(ship_update) => {
                        //println!("got ship");
                        let index = self.other_players
                            .iter()
                            .position(|&x| x.id == ship_update.id);
                        match index {
                            Some(i) => {
                                self.other_players[i].pos.x = ship_update.x;
                                self.other_players[i].pos.y = ship_update.y;
                                self.other_players[i].shield = ship_update.shield;
                                // this is just a safeguard to prevent
                                // having a player both alive and dead
                                self.other_players[i].health = 1.0;
                            },
                            None => {},
                        }
                    },
                    Wrapper::BulletWrapper(bullet) => {
                        //println!("got bullet");
                        self.bullets.push(bullet);
                    },
                    Wrapper::RestartSignal => self.reset(),
                    Wrapper::WinSignal => self.state = State::Won,
                    Wrapper::DeathSignal(id) => {
                        println!("death signal");
                        let index = self.other_players
                            .iter()
                            .position(|&x| x.id == id);
                        if let Some(i) = index { 
                            self.other_players[i].health -= 2.0;
                        }
                    }
                    _ => {}
                }

            },

            Err(_) => {},

        }

        Ok(())
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {

        let dt = ggez::timer::duration_to_f64(ggez::timer::delta(ctx)) as f32;

        self.broadcast_timer -= dt;

        let (width, height) = graphics::drawable_size(ctx);

        let moved = self.player_ship.update_pos(dt, &self.input_state, width, height);

        // broadcast_timer limits the amount of time the position of the ship gets broadcasted 
        // to all peers. 
        if moved && self.broadcast_timer < 0.0 {
            let movement = Wrapper::ShipUpdateWrapper(ShipUpdate::new(
                    self.player_ship.id,
                    self.player_ship.pos.x,
                    self.player_ship.pos.y,
                    self.player_ship.shield));

            self.send_to_peers(movement);
            self.broadcast_timer = BROADCAST_TICK;
        }

        if let State::Playing | State::Lost = self.state {
            //self.enemy_ship.oscillate(dt, width);
        }

        for bullet in &mut self.bullets {
            bullet.update_pos(dt);
        }

        self.bullets.retain(|bullet| bullet.pos.y > 0.0
                            && bullet.pos.y < height
                            && !bullet.hit);

        // add delay between shots
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("couldn't get time")
            .as_millis() as f64;

        if now >= self.player_fire_delay && self.input_state.fire {
            match self.state {
                State::Playing | State::Won => {
                    let bullet = self.player_ship.shoot(None, BulletType::Normal);
                    self.bullets.push(bullet);

                    let msg = Wrapper::BulletWrapper(bullet);
                    self.send_to_peers(msg);
                    
                },
                _ => {}
            }

            self.player_fire_delay = now + PLAYER_FIRE_RATE;
        }

        self.special_timer -= dt;
        if self.input_state.special && self.special_timer < 0.0 {
            let special_bullet = self.player_ship.shoot(None, BulletType::Special);

            let msg = Wrapper::BulletWrapper(special_bullet);
            self.send_to_peers(msg);

            self.bullets.push(special_bullet);

            self.special_timer = SPECIAL_BULLET_COOLDOWN;
        }

        self.shield_timer -= dt;
        if self.input_state.shield && self.shield_timer < 0.0 {
            self.player_ship.shield = true;
            self.shield_timer = SHIELD_COOLDOWN;
            self.shield_active = SHIELD_DURATION;

            let update = Wrapper::ShipUpdateWrapper(ShipUpdate::new(
                    self.player_ship.id,
                    self.player_ship.pos.x,
                    self.player_ship.pos.y,
                    true));
            self.send_to_peers(update);
        }

        if self.shield_active > 0.0 {
            self.shield_active -= dt;
        }
        else if self.player_ship.shield {
            self.player_ship.shield = false;
            let update = Wrapper::ShipUpdateWrapper(ShipUpdate::new(
                    self.player_ship.id,
                    self.player_ship.pos.x,
                    self.player_ship.pos.y,
                    self.player_ship.shield));
            self.send_to_peers(update);
        }

        if now >= self.enemy_fire_delay {
            match self.state {
                State::Loading => {},
                State::Won => {},
                _ => {
                    self.bullets.push(self.enemy_ship.shoot(Some(consts::PI/4.0), BulletType::Normal));
                    self.bullets.push(self.enemy_ship.shoot(Some(-1.0 * consts::PI/4.0), BulletType::Normal));
                    self.bullets.push(self.enemy_ship.shoot(None, BulletType::Normal));

                    if let Network::Host = self.network_type {
                        let rand_angle = rand::thread_rng().gen_range(-1.0 * consts::PI/4.0, consts::PI/4.0);

                        let bullet = self.enemy_ship.shoot(Some(rand_angle), BulletType::Normal);
                        self.bullets.push(bullet);

                        let msg = Wrapper::BulletWrapper(bullet);
                        self.send_to_peers(msg);

                        if self.enemy_ship.health < BOSS_HEALTH/2.0 {
                            let rand_angle2 = rand::thread_rng().gen_range(-1.0 * consts::PI/4.0, consts::PI/4.0);

                            let bullet = self.enemy_ship.shoot(Some(rand_angle2), BulletType::Normal);
                            self.bullets.push(bullet);

                            let msg = Wrapper::BulletWrapper(bullet);
                            self.send_to_peers(msg);

                        }
                    }

                }

            }

            self.enemy_fire_delay = now + BOSS_FIRE_RATE;
        }


        self.check_collisions();


        if let State::Playing = self.state {
            if self.player_ship.health < 0.0 {
                self.state = State::Lost;
                let signal = Wrapper::DeathSignal(self.player_ship.id);
                self.send_to_peers(signal);
            }
            else if self.enemy_ship.health < 0.1 {
                self.state = State::Won;
                let msg  = Wrapper::WinSignal;
                self.send_to_peers(msg);

            }
        }

        // ==================================
        //         NETWORKING
        // ==================================

        match self.state {
            State::Loading => self.handle_connections(),
            _ => self.handle_updates()
        }.unwrap();

        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        self.player_ship.draw(&mut self.assets, ctx, Some(1))?;
        self.enemy_ship.draw(&mut self.assets, ctx, None)?;

        for ship in &self.other_players {
            ship.draw(&mut self.assets, ctx, Some(2))?;
        }

        for bullet in &self.bullets {
            bullet.draw(&mut self.assets, ctx)?;
        }

        self.draw_indicators(ctx)?;

        match self.state {
            State::Won => {
                self.draw_win_screen(ctx)
            },
            State::Lost => {
                self.draw_death_screen(ctx)
            },
            _ => Ok(()),
        }?;

        graphics::present(ctx)?;

        ggez::timer::yield_now();

        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, key: KeyCode, _mods: KeyMods, _: bool) {

            match key {
                KeyCode::Q => ggez::event::quit(ctx),
                KeyCode::R => {
                    let signal = Wrapper::RestartSignal;
                    self.send_to_peers(signal);
                    self.reset();
                },
                _ => {},
            }

            if self.player_ship.health > 0.0 {
                match key {
                    KeyCode::W => self.input_state.up = true,
                    KeyCode::S => self.input_state.down = true,
                    KeyCode::D => self.input_state.right = true,
                    KeyCode::A => self.input_state.left = true,
                    KeyCode::Space => self.input_state.fire = true,
                    KeyCode::J => self.input_state.special = true,
                    KeyCode::K => self.input_state.shield = true,

                    _ => {},
                }

                match key {
                    KeyCode::W | KeyCode::S | KeyCode::A | KeyCode::D | KeyCode::Space | KeyCode::J | KeyCode::K => {
                        match self.state {
                            State::Loading => {

                                let signal = Wrapper::StartSignal;
                                self.send_to_peers(signal);

                                self.state = State::Playing;
                            
                            },
                            _ => {},
                        }
                    }

                    _ => {},
                }
            }
    }
    fn key_up_event(&mut self, _ctx: &mut Context, key: KeyCode, _mods: KeyMods) {

        match key {
            KeyCode::W => self.input_state.up = false,
            KeyCode::S => self.input_state.down = false,
            KeyCode::D => self.input_state.right = false,
            KeyCode::A => self.input_state.left = false,
            KeyCode::Space => self.input_state.fire = false,
            KeyCode::J => self.input_state.special = false,
            KeyCode::K => self.input_state.shield = false,
            _ => {},
        }
    }
}
