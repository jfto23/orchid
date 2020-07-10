# Orchid

_orchid_ is a space shooter game written in Rust. You can play it
[here](https://jfto23.github.io/orchid/) (singleplayer only).

![](https://github.com/jfto23/orchid/blob/master/orchid_pic.png)

## Controls

- `WASD`: Move

- `Space`: Shoot

- `J`: Special

- `K`: Shield

- `Q`: Quit

- `R`: Restart

## Build

If you want to run the game as a native binary, you can clone the repository and do:

```
cargo run
```

## Multiplayer

The game also works on LAN. By default, `cargo run` will start a game where 
other players on the same network 
can join. For example, the first player can create a game:

```
cargo run
```

and other players can join by specifying the host's local IP and port when executing the
binary:

```
cargo build
cd target/debug
./orchid 192.168.0.8:7777
```

By default, the host's runs on port `7777` and all other players run on port
`7778`. Both of these are specified in `constants.rs`.

## WASM

The code in the master branch can be compiled as a native binary. The WASM compilable
version of the game is in the gh-pages branch and it's the one that runs on
the live website.
