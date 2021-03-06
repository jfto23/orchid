mod constants;
mod states;
mod entities;
mod assets;
mod network;

use constants::{ PEER_PORT, HOST_PORT };
use network::{ Network, Wrapper };
use states::MainState;

use std::env;
use std::path;
use std::net::{UdpSocket, SocketAddrV4};

use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use rand::Rng;

use ggez::{ContextBuilder, GameResult};
use ggez::event;
use ggez::conf;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
// define own point to encode it
pub struct Point {
    x: f32,
    y: f32,
}

fn distance_2d(p1: Point, p2: Point) -> f32 {
    (((p1.x-p2.x).powf(2.0)) + ((p1.y-p2.y).powf(2.0))).sqrt()
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

    let (ctx, event_loop) = &mut cb.build()?;
 

    // networking
    let args: Vec<String> = env::args().collect();

    let (socket, network_type, rng) = match env::args().len() {
        1 => {
            let socket = UdpSocket::bind(format!("127.0.0.1:{}",HOST_PORT))?;
            socket.set_nonblocking(true)?;

            let mut rng_thread = rand::thread_rng();
            let rng = Xoshiro256Plus::seed_from_u64(rng_thread.gen::<u64>());
            (socket,Network::Host, Some(rng))
        }
        _ => {
            let socket = UdpSocket::bind(format!("127.0.0.1:{}",PEER_PORT))?;
            socket.set_nonblocking(true)?;

            let host_addr: SocketAddrV4 = args[1]
                .clone()
                .parse()
                .expect("invalid adress");

            // notify host that a connection occured

            let signal = bincode::serialize(&Wrapper::ConnectSignal).unwrap();
            socket.send_to(&signal, host_addr)?;


            (socket,Network::Peer,None)
        }
    };


    let mut my_game = MainState::new(ctx, network_type, socket, rng);

    event::run(ctx, event_loop, &mut my_game)
}


