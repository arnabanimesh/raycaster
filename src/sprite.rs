use ggez::{
    graphics::{self},
    Context,
};

use crate::{player::Player, screen::Screen, utilities::vector2::Vector2};
const PI: f32 = std::f32::consts::PI;
const SIZE: usize = 64;
pub struct Sprite {
    pub stype: usize,
    pub pos: Vector2<f32>,
    pub sprite_rotation: usize,
    pub time: f32,
}

impl Sprite {
    pub fn new(stype: SpriteType, pos: Vector2<f32>) -> Self {
        Self {
            stype: stype as usize,
            pos,
            sprite_rotation: 0,
            time: 0.0,
        }
    }
    pub fn calculate_distance_2(&self, player: &Player) -> f32 {
        (player.pos.x - self.pos.x) * (player.pos.x - self.pos.x)
            + (player.pos.y - self.pos.y) * (player.pos.y - self.pos.y) //square root not necessary
    }

    pub fn update(&mut self, time: f32) {
        self.time = time;
    }

    pub fn draw(
        &mut self,
        ctx: &mut Context,
        player: &Player,
        screen: &mut Screen,
        distances: &[f32],
        rect_w: usize,
    ) {
        let (w, h) = graphics::drawable_size(ctx);
        let sprite_delpos = self.pos - player.pos;
        let mut angle = (sprite_delpos.y).atan2(sprite_delpos.x);
        let angle_player = player.dir_norm.angle();
        let delta_angle = angle - angle_player;
        let cos = delta_angle.cos();
        let inv_det =
            1.0 / (player.plane.x * player.dir_norm.y - player.dir_norm.x * player.plane.y);
        let sprite_distance = self.calculate_distance_2(player);

        let transform_x =
            inv_det * (player.dir_norm.y * sprite_delpos.x - player.dir_norm.x * sprite_delpos.y);
        let transform_y =
            inv_det * (-player.plane.y * sprite_delpos.x + player.plane.x * sprite_delpos.y);

        let sprite_screen_x = (w * 0.5) * (1.0 + transform_x / transform_y);
        let sprite_height = (player.planedist / transform_y).abs();
        let mut start_y = -sprite_height * 0.5 + h * 0.5 + player.pitch + player.jump / transform_y;
        if start_y < 0.0 {
            start_y = 0.0;
        }
        let mut end_y = sprite_height * 0.5 + h * 0.5 + player.pitch + player.jump / transform_y;
        if end_y > h - 1.0 {
            end_y = h - 1.0;
        }

        let sprite_width = (player.planedist / transform_y).abs();
        let mut start_x = -sprite_width * 0.5 + sprite_screen_x;
        if start_x < 0.0 {
            start_x = 0.0;
        }
        let mut end_x = sprite_width * 0.5 + sprite_screen_x;
        if end_x > w - 1.0 {
            end_x = w - 1.0;
        }

        if transform_y > 0.0 && start_x < w && end_x > 0.0 && end_y > 0.0 && start_y < h {
            if self.stype == SpriteType::Bat as usize {
                if angle < 0.0 {
                    angle += 2.0 * PI;
                }
                self.sprite_rotation = (angle / (2.0 * PI) * 8.0).round() as usize;

                self.sprite_rotation = 7 - self.sprite_rotation;
            } else if self.stype == SpriteType::Torch as usize {
                self.sprite_rotation = (self.time % 1.0 * 8.0) as usize;
            }
            if self.sprite_rotation > 7 {
                self.sprite_rotation = 0;
            }

            let denominator = SIZE as f32 / sprite_height;
            let mut sty = Vec::new();
            let shade = {
                if self.stype == SpriteType::Torch as usize {
                    1.0
                } else {
                    num::clamp(19.0 / sprite_distance, 0.2, 1.0)
                }
            };

            for y in start_y as usize..1 + end_y as usize {
                //for every pixel of the current stripe
                let d = (y as f32) - h * 0.5 + sprite_height * 0.5
                    - player.pitch
                    - player.jump / transform_y;
                sty.push((d * denominator) as usize);
            }

            for stripe in start_x as usize..1 + end_x as usize {
                let stx = ((stripe as f32 - (-sprite_width * 0.5 + sprite_screen_x)) * SIZE as f32
                    / sprite_width) as usize;
                if stripe > 0
                    && stripe < w as usize
                    && distances[stripe / rect_w] * distances[stripe / rect_w] / (cos * cos)
                        > sprite_distance
                {
                    for y in start_y as usize..1 + end_y as usize {
                        screen.draw_sprite(
                            [
                                self.sprite_rotation * SIZE + stx,
                                self.stype * SIZE + sty[y - start_y as usize],
                            ],
                            [y, w as usize - stripe],
                            1,
                            shade,
                            SIZE*8,
                        );
                    }
                }
            }
        }
    }
}

pub enum SpriteType {
    Armor = 0,
    CandleHolder = 1,
    Bat = 2,
    Torch = 3,
    Gore = 4,
}
