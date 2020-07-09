use ggez::{graphics, Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::conf;

use serde::{Serialize, Deserialize};

use mint;

use rand::Rng;

use uuid::Uuid;

use std::env;
use std::path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::f32::consts;
use std::net::{UdpSocket, SocketAddrV4, SocketAddr};

const SHIP_SPEED: f32 = 350.0;
const BOSS_SPEED: f32 = 125.0;
const BULLET_SPEED: f32 = 500.0;
const PLAYER_FIRE_RATE: f64 = 200.0;
const BOSS_FIRE_RATE: f64 = 250.0;
const SPECIAL_BULLET_SPEED: f32 = 250.0;
const SPECIAL_BULLET_COOLDOWN: f32 = 5.0;
const SPECIAL_BULLET_DAMAGE: f32 = 5.0;
const GREEN: graphics::Color = graphics::Color::new(0.0, 255.0, 0.0, 1.0);
const RED: graphics::Color = graphics::Color::new(255.0, 0.0, 0.0, 1.0);
const SCREEN_BORDER: f32 = 20.0;
const SHIELD_COOLDOWN: f32 = 15.0;
const SHIELD_DURATION: f32 = 2.0;
const BOSS_HEALTH: f32 = 1.0;
const BROADCAST_TICK: f32 = 1.0/30.0;
const PLAYER_SPAWN: Point = Point{ x: 400.0, y: 500.0};

// defines a wrapper that can be sent through UDP
// Contains data that is shared between peers
// For simplicity, the bullets and ship are sent as a whole
#[derive(Serialize, Deserialize, Debug)]
enum Wrapper {
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
struct ShipUpdate {
    id: Uuid,
    x: f32,
    y: f32,
    shield: bool,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
// define own point to encode it
struct Point {
    x: f32,
    y: f32,
}


enum Network {
    Host,
    Peer,
}

struct Assets {
    player_ship: graphics::Image,
    enemy_ship: graphics::Image,
    player_bullet: graphics::Image,
    other_players: graphics::Image,
    enemy_bullet: graphics::Image,
    player_dead: graphics::Image,
    special_bullet: graphics::Image,
    shield: graphics::Image,
    font: graphics::Font,
}

impl Assets {
    fn new(ctx: &mut Context) -> Assets {
        Assets {
            player_ship: graphics::Image::new(ctx, "/player_shipv1.png").unwrap(),
            enemy_ship: graphics::Image::new(ctx, "/enemy_ship.png").unwrap(),
            other_players: graphics::Image::new(ctx, "/player_shipv2.png").unwrap(),
            player_bullet: graphics::Image::new(ctx, "/player_bullet2.png").unwrap(),
            enemy_bullet: graphics::Image::new(ctx, "/enemy_bullet.png").unwrap(),
            player_dead: graphics::Image::new(ctx, "/player_ship_dead.png").unwrap(),
            special_bullet: graphics::Image::new(ctx, "/special_bullet.png").unwrap(),
            font: graphics::Font::new(ctx, "/ARCADE_N.TTF").unwrap(),
            shield: graphics::Image::new(ctx, "/shieldv2.png").unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct Bullet {
    possession: Possession,
    angle: f32,
    pos: Point,
    hit: bool,
    bullet_type: BulletType

}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum BulletType {
    Normal,
    Special,
}


impl Bullet {
    fn new(possession: Possession, angle: f32, pos: Point, bullet_type: BulletType) -> Bullet {
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

    fn draw(&self, assets: &mut Assets, ctx: &mut Context) -> GameResult {

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

    fn update_pos(&mut self, dt: f32) {
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

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
struct Ship {
    health: f32,
    ship_type: Possession,
    pos: Point,
    angle: f32,
    direction: Option<f32>,
    shield: bool,
    id: Uuid,
}

impl Ship {
    fn new(ship_type: Possession) -> Ship {
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

    fn reset(&mut self) {
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

    fn move_to_point(&mut self, p: Point) {
        self.pos.x = p.x;
        self.pos.y = p.y;
    }

    // the optionnal argument lets ships shoot in more directions
    // i.e bosses can shoot in diagonals
    fn shoot(&self, curve: Option<f32>, bullet_type: BulletType) -> Bullet {
        match curve {
            Some(angle) => Bullet::new(self.ship_type, self.angle+angle, self.pos, BulletType::Normal),
            _ => Bullet::new(self.ship_type, self.angle, self.pos, bullet_type),
        }
    }

    fn draw(&self, assets: &mut Assets, ctx: &mut Context, version: Option<i32>) -> GameResult {

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
    fn update_pos(&mut self, dt: f32, input_state: &InputState, width: f32, height: f32) -> bool {
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
    fn oscillate(&mut self, dt: f32, width: f32) {
        if self.pos.x <= 0.0 + SCREEN_BORDER {
            self.direction = Some(1.0);
        }
        else if self.pos.x >= width - SCREEN_BORDER {
            self.direction = Some(-1.0);
        }

        self.pos.x += dt * BOSS_SPEED * self.direction.unwrap();

    }
}

#[derive(Serialize, Deserialize)]
struct InputState {
    up: bool,
    down: bool,
    right: bool,
    left: bool,
    fire: bool,
    special: bool,
    shield: bool,
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


#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
enum Possession {
    Player,
    Enemy,
}


fn main() -> GameResult {
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    }
    // running from /target/debug
    else {
        path::PathBuf::from("../../resources")
    };

    let cb = ContextBuilder::new("Orchid", "jfto23")
        .window_setup(conf::WindowSetup::default().title("Orchid"))
        .add_resource_path(resource_dir);

    let (ctx, event_loop) = &mut cb.build().expect("failed to build");
 

    // networking
    let args: Vec<String> = env::args().collect();

    let (socket, network_type) = match env::args().len() {
        1 => {
            let socket = UdpSocket::bind("127.0.0.1:7777").expect("can't bind to local comptuter");
            socket.set_nonblocking(true).expect("couldn't set to non-blocking");

            (socket,Network::Host)
        }
        _ => {
            let socket = UdpSocket::bind("127.0.0.1:7778").expect("can't bind to local comptuter");
            socket.set_nonblocking(true).expect("couldn't set to non-blocking");

            let host_addr: SocketAddrV4 = args[1].clone().parse().expect("Invalid IPV4 adress");

            // notify host that a connection occured

            let signal = bincode::serialize(&Wrapper::ConnectSignal).unwrap();
            socket.send_to(&signal, host_addr).expect("couldn't connect to host");


            (socket,Network::Peer)
        }
    };


    let mut my_game = MainState::new(ctx, network_type, socket);

    event::run(ctx, event_loop, &mut my_game)
}

fn distance_2d(p1: Point, p2: Point) -> f32{
    (((p1.x-p2.x).powf(2.0)) + ((p1.y-p2.y).powf(2.0))).sqrt()
}

#[derive(Serialize, Deserialize, Debug)]
enum State {
    Playing,
    Won,
    Lost,
    Loading,
}

struct MainState {
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
            assets: Assets::new(ctx),
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
                        if player_distance < 24.0 && self.enemy_ship.health > 0.0 {
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


    fn reset(&mut self, ctx: &mut Context) {
            self.player_ship.reset();
            self.enemy_ship.reset();
            self.bullets = Vec::<Bullet>::new();
            self.assets = Assets::new(ctx);
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


    fn draw_indicators(&mut self, ctx: &mut Context) {

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

        graphics::draw(ctx, &special_text, (mint::Point2{x:5.0,y:5.0}, 0.0, special_color)).unwrap();
        graphics::draw(ctx, &shield_text, (mint::Point2{x:180.0,y:5.0}, 0.0, shield_color)).unwrap();
    }

    fn draw_death_screen(&mut self, ctx: &mut Context) {
        let text = graphics::Text::new(("YOU DIED",self.assets.font,16.0));
        graphics::draw(ctx, &text, (mint::Point2{x:350.0,y:100.0}, 0.0, graphics::WHITE)).unwrap();
    }

    fn draw_win_screen(&mut self, ctx: &mut Context) {
        let text = graphics::Text::new(("YOU WON",self.assets.font,16.0));
        graphics::draw(ctx, &text, (mint::Point2{x:350.0,y:100.0}, 0.0, graphics::WHITE)).unwrap();
    }

    fn send_to_peers(&self, msg: Wrapper) {
        let encoded = bincode::serialize(&msg).unwrap();
        for peer in self.peers.iter() {
            self.socket.send_to(&encoded, peer).unwrap();
        }
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let dt = ggez::timer::duration_to_f64(ggez::timer::delta(ctx)) as f32;

        self.broadcast_timer -= dt;

        let (width, height) = graphics::drawable_size(ctx);

        let moved = self.player_ship.update_pos(dt, &self.input_state, width, height);

        // broadcast_timer limits the amount of time the position of the ship gets broadcasted 
        // to all peers. Without it, lags accumulates over time.
        if moved && self.broadcast_timer < 0.0 {
            let movement = Wrapper::ShipUpdateWrapper(ShipUpdate {id: self.player_ship.id, x:self.player_ship.pos.x, y:self.player_ship.pos.y, shield: self.player_ship.shield });
            self.send_to_peers(movement);
            self.broadcast_timer = BROADCAST_TICK;
        }

        if let State::Playing | State::Lost = self.state {
            self.enemy_ship.oscillate(dt, width);
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

            let update = Wrapper::ShipUpdateWrapper(ShipUpdate{id: self.player_ship.id, x:self.player_ship.pos.x, y:self.player_ship.pos.y, shield: true });
            self.send_to_peers(update);
        }

        if self.shield_active > 0.0 {
            self.shield_active -= dt;
        }
        else if self.player_ship.shield {
            self.player_ship.shield = false;
            let update = Wrapper::ShipUpdateWrapper(ShipUpdate {id: self.player_ship.id, x:self.player_ship.pos.x, y:self.player_ship.pos.y, shield: self.player_ship.shield });
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

                    let rand_angle = rand::thread_rng().gen_range(-1.0 * consts::PI/4.0, consts::PI/4.0);
                    self.bullets.push(self.enemy_ship.shoot(Some(rand_angle), BulletType::Normal));

                    if self.enemy_ship.health < BOSS_HEALTH/2.0 {
                        let rand_angle2 = rand::thread_rng().gen_range(-1.0 * consts::PI/4.0, consts::PI/4.0);
                        self.bullets.push(self.enemy_ship.shoot(Some(rand_angle2), BulletType::Normal));

                    }
                }

            }

            self.enemy_fire_delay = now + BOSS_FIRE_RATE;
        }


        self.check_collisions();


        if let State::Playing = self.state {
            if self.player_ship.health < 0.0 {
                self.state = State::Lost;
            }
            else if self.enemy_ship.health < 0.1 {
                self.state = State::Won;
                let msg  = Wrapper::WinSignal;
                self.send_to_peers(msg);

            }
        }

        // ==================================
        //         NETWORKING (MESS)
        // ==================================

        match self.state {

            // ===========================================
            //     HANDLING CONNECTION DURING LOADING
            // ===========================================

            State::Loading => {

                for ship in &mut self.other_players {
                    ship.move_to_point(PLAYER_SPAWN);
                }

                // first make all ship appear
                if self.peers.len() != self.other_players.len() {
                    
                    println!("loading all ships...");
                    let mut buf = [0u8; 512];
                    let result = self.socket.recv_from(&mut buf);

                    match result {
                        Ok(_) => {
                                let decoded: Wrapper = bincode::deserialize(&buf).unwrap();
                                if let Wrapper::ShipWrapper(ship) = decoded {
                                    let index = self.other_players
                                        .iter()
                                        .position(|&x| x.id == ship.id);
                                    match index {
                                        Some(_) => {},
                                        None => self.other_players.push(ship),
                                    }

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
                            // some "client" connected to host
                            Ok((_amt, src)) => {
                                let decoded: Wrapper = bincode::deserialize(&buf).unwrap();
                                match decoded {
                                    Wrapper::ConnectSignal => {
                                        self.peers.push(src);
                                        // send new peer address to all peers except the new one
                                        let encoded_address = bincode::serialize(&Wrapper::AddressWrapper(src)).unwrap();
                                        for peer in self.peers.iter() {
                                            if *peer == src {
                                                for e in self.peers.clone().iter() {
                                                    if *e == src {
                                                        continue;
                                                    }
                                                    let encoded_e = bincode::serialize(&Wrapper::AddressWrapper(*e)).unwrap();
                                                    self.socket.send_to(&encoded_e, src)?;
                                                }
                                                let encoded_host = bincode::serialize(&Wrapper::AddressWrapper(self.socket.local_addr().unwrap())).unwrap();
                                                self.socket.send_to(&encoded_host, src)?;
                                                continue;
                                            }
                                            self.socket.send_to(&encoded_address, peer)?;
                                        }
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
                                let decoded: Wrapper = bincode::deserialize(&buf).unwrap();
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

            },

            _ => {

                // =======================
                // RECEIVE FROM PEERS
                // =======================
                
                let mut buf = [0u8; 128];
                let result = self.socket.recv_from(&mut buf);
                match result {
                    Ok((_amt, _src)) => {
                        let decoded: Wrapper = bincode::deserialize(&buf).unwrap();

                        match decoded {
                            Wrapper::ShipUpdateWrapper(ship_update) => {
                                //println!("got ship");
                                let index = self.other_players.iter().position(|&x| x.id == ship_update.id);
                                match index {
                                    Some(i) => {
                                        self.other_players[i].pos.x = ship_update.x;
                                        self.other_players[i].pos.y = ship_update.y;
                                        self.other_players[i].shield = ship_update.shield;
                                    },
                                    None => {},
                                }
                            },
                            Wrapper::BulletWrapper(bullet) => {
                                //println!("got bullet");
                                self.bullets.push(bullet);
                            },
                            Wrapper::RestartSignal => self.reset(ctx),
                            Wrapper::WinSignal => self.state = State::Won,
                            _ => {}
                        }
                    
                    },

                    Err(_) => {},

                }

            
            },

        }

        
        
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

        self.draw_indicators(ctx);

        match self.state {
            State::Won => {
                self.draw_win_screen(ctx);
            },
            State::Lost => self.draw_death_screen(ctx),
            _ => {},
        };

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

                    self.reset(ctx);

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
