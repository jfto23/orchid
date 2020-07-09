use ggez::{graphics, Context, GameResult};

pub struct Assets {
    pub player_ship: graphics::Image,
    pub enemy_ship: graphics::Image,
    pub player_bullet: graphics::Image,
    pub other_players: graphics::Image,
    pub enemy_bullet: graphics::Image,
    pub player_dead: graphics::Image,
    pub special_bullet: graphics::Image,
    pub shield: graphics::Image,
    pub font: graphics::Font,
}

impl Assets {
    pub fn new(ctx: &mut Context) -> GameResult<Assets> {
        Ok(
            Assets {
                player_ship: graphics::Image::new(ctx, "/player_shipv1.png")?,
                enemy_ship: graphics::Image::new(ctx, "/enemy_ship.png")?,
                other_players: graphics::Image::new(ctx, "/player_shipv2.png")?,
                player_bullet: graphics::Image::new(ctx, "/player_bullet2.png")?,
                enemy_bullet: graphics::Image::new(ctx, "/enemy_bullet.png")?,
                player_dead: graphics::Image::new(ctx, "/player_ship_dead.png")?,
                special_bullet: graphics::Image::new(ctx, "/special_bullet.png")?,
                font: graphics::Font::new(ctx, "/ARCADE_N.TTF")?,
                shield: graphics::Image::new(ctx, "/shieldv2.png")?,
            }
          )
    }
}
