use std::path::Path;

use ggez::event::{EventHandler, KeyCode, KeyMods};
use ggez::graphics::{self, Color, DrawParam};
use ggez::input::keyboard::is_key_pressed;
use ggez::{timer, Context, GameResult};
mod lighting;
mod map;
mod player;
mod screen;
mod sprite;
mod utilities;
use lighting::{Lighting, Torch};
use map::Map;
use num::clamp;
use player::Player;
use rayon::prelude::*;
use screen::Screen;
use sprite::Sprite;
use utilities::input::{mouse_grabbed_and_hidden, set_mouse_location};
use utilities::vector2::Vector2;
//https://mynoise.net/NoiseMachines/dungeonRPGSoundscapeGenerator.php?l=32343600005816020035&mt=1&tm=1
use crate::utilities::input::get_delta;

const PI: f32 = std::f32::consts::PI;
const RAYSPERPIXEL: usize = 2;
const FOV: f32 = 45.0;
pub struct MainState {
    player: Player,
    map_size: (usize, usize),
    cell_size: f32,
    map: Map,
    angles: Vec<f32>,
    buffer_floors: Vec<f32>,
    buffer_walking: Vec<f32>,
    sky: Sky,
    intersections: Intersections,
    screen: Screen,
    sprites: Vec<Sprite>,
    time: f32,
    lighting: Lighting,
    torch: Torch,
}

impl MainState {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let (w, h) = graphics::drawable_size(ctx);
        graphics::set_fullscreen(ctx, ggez::conf::FullscreenType::Desktop)?;
        let pos = Vector2::new(8.5, 12.5);
        let dir_norm = Vector2::new(0.0f32, -1.0); // Player direction
        let plane = Vector2::new((FOV.to_radians() * 0.5).tan(), 0.0); //Camera plane vector
        let map_size = (16, 25);
        let cell_size = 128.0;
        let minimap = graphics::Image::new(ctx, "/minimap.png")?;
        let minimap_sb =
            graphics::spritebatch::SpriteBatch::new(graphics::Image::new(ctx, "/sb.png")?);
        let map = Map::new(
            ctx,
            Path::new("/map.png"),
            Path::new("/floor.png"),
            minimap,
            minimap_sb,
            map_size,
        )?;

        let player = Player::new(
            ctx,
            pos,
            dir_norm,
            plane,
            (w * 0.5) / ((FOV.to_radians() * 0.5).tan()), //distance from the player to the projection plane
            0.0,
            0.0,
        )?;

        set_mouse_location(ctx, Vector2::new(w * 0.5, h * 0.5)).unwrap();

        let angles: Vec<f32> = (0..w as usize / RAYSPERPIXEL)
            .map(|r: usize| {
                ((r as f32 * w / (w / (RAYSPERPIXEL as f32) - 1.0) - w * 0.5) / player.planedist)
                    .atan()
            })
            .collect();

        let buffer_floors = (0..h as usize)
            .map(|y| player.planedist / (2.0 * y as f32 - h))
            .collect();

        let buffer_walking = (0..150)
            .map(|i| ((i as f32) / 150.0 * 2.0 * PI).sin())
            .collect();

        let mut skyimg = graphics::Image::new(ctx, "/sky2.png")?;
        skyimg.set_wrap(graphics::WrapMode::Tile, graphics::WrapMode::Mirror);
        skyimg.set_filter(graphics::FilterMode::Nearest);
        let mut sb = graphics::spritebatch::SpriteBatch::new(skyimg);
        let idx = sb.add(DrawParam::default());
        let sky = Sky { sb, idx };
        let intersections = Intersections::new(w as usize);

        let wall_textures = graphics::Image::new(ctx, "/wall128.png")?.to_rgba8(ctx)?;

        let sprite_textures = graphics::Image::new(ctx, "/sprite128.png")?.to_rgba8(ctx)?;

        let mut screen = Screen::new(h, w, 128, 128 * 8);
        screen.textures(wall_textures, sprite_textures);

        let sprites = vec![
            Sprite::new(sprite::SpriteType::Armor, Vector2::new(7.5, 7.5)),
            Sprite::new(sprite::SpriteType::Armor, Vector2::new(7.5, 9.5)),
            //Sprite::new(sprite::SpriteType::CandleHolder, Vector2::new(12.5, 12.5)),
            Sprite::new(sprite::SpriteType::Bat, Vector2::new(6.5, 12.5)),
            Sprite::new(sprite::SpriteType::Torch, Vector2::new(9.0, 15.0 - 0.048)),
            Sprite::new(sprite::SpriteType::Torch, Vector2::new(8.5, 24.0 - 0.048)),
            Sprite::new(sprite::SpriteType::Torch, Vector2::new(8.048, 12.0)),
            Sprite::new(sprite::SpriteType::Gore, Vector2::new(13.0, 3.0)),
        ];

        let lighting = lighting::Lighting::new(
            vec![1 + map_size.0 * 3, 14 + map_size.0 * 7, 8 + map_size.0 * 23],
            &map.solid,
            map_size,
        );

        let torch = lighting::Torch::default();

        Ok(Self {
            player,
            map_size,
            cell_size,
            map,
            angles,
            buffer_floors,
            buffer_walking,
            sky,
            intersections,
            screen,
            sprites,
            time: 0.0,
            lighting,
            torch,
        })
    }

    pub fn handle_input(&mut self, ctx: &mut Context, dir_norm: Vector2<f32>) {
        let (w, h) = graphics::drawable_size(ctx);
        let dt = ggez::timer::delta(ctx).as_secs_f32();
        mouse_grabbed_and_hidden(ctx, false, true).unwrap();

        let mut delta_mouse_loc_x = get_delta(ctx).x;
        let mut delta_mouse_loc_y = get_delta(ctx).y;

        set_mouse_location(ctx, Vector2::new(w * 0.5, h * 0.5)).unwrap();

        let mut angle_of_rot = 0.0f32;
        if delta_mouse_loc_x != 0.0 {
            delta_mouse_loc_x -= 240.0; // resolution width middle minus window width middle
        }

        if delta_mouse_loc_y != 0.0 {
            delta_mouse_loc_y -= 135.0;
        }
        self.player.pitch -= delta_mouse_loc_y * 0.75;

        self.player.pitch = clamp(self.player.pitch, -300.0, 300.0);

        angle_of_rot += 0.085 * delta_mouse_loc_x;
        self.player.plane = Vector2::rotate(self.player.plane, angle_of_rot.to_radians());
        self.player.dir_norm = Vector2::rotate(self.player.dir_norm, angle_of_rot.to_radians());

        let mut dir = Vector2::new(0.0, 0.0);
        let yoffset = 0.3125;
        let mut check_pos_y = self.player.pos;
        let mut check_pos_x = self.player.pos;
        self.player.walking = false;

        if is_key_pressed(ctx, KeyCode::W) {
            check_pos_y += Vector2::new(0.0, self.player.dir_norm.y.signum() * yoffset);
            check_pos_x += Vector2::new(self.player.dir_norm.x.signum() * yoffset, 0.0);
            dir += dir_norm;
            self.player.walking = true;
        }
        if is_key_pressed(ctx, KeyCode::S) {
            check_pos_y += Vector2::new(0.0, -self.player.dir_norm.y.signum() * yoffset);
            check_pos_x += Vector2::new(-self.player.dir_norm.x.signum() * yoffset, 0.0);
            dir -= dir_norm;
            self.player.walking = true;
        }

        if is_key_pressed(ctx, KeyCode::A) {
            check_pos_y += Vector2::new(0.0, -self.player.dir_norm.x.signum() * yoffset);
            check_pos_x += Vector2::new(self.player.dir_norm.y.signum() * yoffset, 0.0);
            dir += Vector2::new(dir_norm.y, -dir_norm.x);
            self.player.walking = true;
        }
        if is_key_pressed(ctx, KeyCode::D) {
            check_pos_y += Vector2::new(0.0, self.player.dir_norm.x.signum() * yoffset);
            check_pos_x += Vector2::new(-self.player.dir_norm.y.signum() * yoffset, 0.0);
            dir += Vector2::new(-dir_norm.y, dir_norm.x);
            self.player.walking = true;
        }

        if self.player.walking {
            let cell_check_y = self.map.solid
                [(check_pos_y.x) as usize + (check_pos_y.y) as usize * self.map_size.0];
            let cell_check_x = self.map.solid
                [(check_pos_x.x) as usize + (check_pos_x.y) as usize * self.map_size.0];

            if cell_check_y {
                dir.y = 0.0;
            }
            if cell_check_x {
                dir.x = 0.0;
            }
            dir.normalize();
            self.player.pos += dir * (2.5 * dt);
        }

        if is_key_pressed(ctx, KeyCode::Space) {
            let check_front = self.player.pos + self.player.dir_norm * 1.5;
            let pos_door = (check_front.x) as usize + (check_front.y) as usize * self.map_size.0;

            if self.map.walls[pos_door] == 6 {
                let door = self.map.doors.get_mut(&pos_door).expect("Cant find door");
                if !door.opening {
                    door.timer = self.time;
                    door.opening = true;
                }
            }
        }

        if is_key_pressed(ctx, KeyCode::LControl) {
            if self.player.height > -300.0 {
                self.player.height -= 30.0;
            }
        } else if self.player.height < 0.0 {
            self.player.height += 30.0;
        }

        if is_key_pressed(ctx, KeyCode::Q) {
            self.player.jump += 2.0;
        }

        if is_key_pressed(ctx, KeyCode::E) {
            self.player.jump -= 2.0;
        }
    }

    pub fn calculate_ray(
        &mut self,
        ray_dir_player: Vector2<f32>,
        theta: f32,
        j: usize,
    ) -> GameResult {
        let ray_dir_norm = Vector2::rotate(ray_dir_player, theta);
        let ray_unitstep_size = Vector2::new(
            (1.0 + (ray_dir_norm.y / ray_dir_norm.x) * (ray_dir_norm.y / ray_dir_norm.x)).sqrt(),
            (1.0 + (ray_dir_norm.x / ray_dir_norm.y) * (ray_dir_norm.x / ray_dir_norm.y)).sqrt(),
        );
        let startv = self.player.pos;

        let mut map_checkv = Vector2::new(startv.x.floor(), startv.y.floor());
        let mut ray_length1_d = Vector2::new(0.0f32, 0.0);
        let mut orientation;
        let mut stepv = Vector2::new(0.0f32, 0.0);
        let mut last_was_door = false;

        if ray_dir_norm.x < 0.0 {
            stepv.x = -1.0;
            ray_length1_d.x = (startv.x - map_checkv.x) * ray_unitstep_size.x;
        } else {
            stepv.x = 1.0;
            ray_length1_d.x = (map_checkv.x + 1.0 - startv.x) * ray_unitstep_size.x;
        }

        if ray_dir_norm.y < 0.0 {
            stepv.y = -1.0;
            ray_length1_d.y = (startv.y - map_checkv.y) * ray_unitstep_size.y;
        } else {
            stepv.y = 1.0;
            ray_length1_d.y = (map_checkv.y + 1.0 - startv.y) * ray_unitstep_size.y;
        }

        let mut tilefound = false;
        let mut distance = 0.0;

        while !tilefound && distance < 100.0 {
            //arbitrary max distance

            if ray_length1_d.x < ray_length1_d.y {
                map_checkv.x += stepv.x;
                distance = ray_length1_d.x;
                ray_length1_d.x += ray_unitstep_size.x;

                if ray_dir_norm.x < 0.0 {
                    orientation = Orientation::W;
                } else {
                    orientation = Orientation::E;
                }
            } else {
                map_checkv.y += stepv.y;
                distance = ray_length1_d.y;
                ray_length1_d.y += ray_unitstep_size.y;

                if ray_dir_norm.y < 0.0 {
                    orientation = Orientation::S;
                } else {
                    orientation = Orientation::N;
                }
            }
            if map_checkv.x >= 0.0
                && map_checkv.x < self.map_size.0 as f32
                && map_checkv.y >= 0.0
                && map_checkv.y < self.map_size.1 as f32
            {
                let mut wall_type =
                    self.map.walls[map_checkv.y as usize * self.map_size.0 + map_checkv.x as usize];

                if last_was_door && wall_type > 0 {
                    wall_type = 7;
                }
                last_was_door = false;
                if wall_type == 6 {
                    //door
                    let door_offset = self
                        .map
                        .doors
                        .get(&(map_checkv.y as usize * self.map_size.0 + map_checkv.x as usize))
                        .expect("error finding door")
                        .offset;

                    tilefound = true;
                    if orientation == Orientation::N || orientation == Orientation::S {
                        if ray_length1_d.y - 0.5 * ray_unitstep_size.y < ray_length1_d.x {
                            distance = ray_length1_d.y - ray_unitstep_size.y * 0.5;

                            if door_offset < 1.0 {
                                let pos_x = (startv.x + ray_dir_norm.x * distance) % 1.0;
                                if pos_x > door_offset * 0.5 && 1.0 - pos_x > door_offset * 0.5 {
                                    last_was_door = true;
                                    tilefound = false;
                                }
                            }
                        } else {
                            // side wall
                            if ray_dir_norm.x < 0.0 {
                                orientation = Orientation::W;
                                map_checkv.x -= 1.0;
                            } else {
                                orientation = Orientation::E;
                                map_checkv.x += 1.0;
                            }
                            wall_type = 7;
                            distance = ray_length1_d.x;
                        }
                    } else if orientation == Orientation::E || orientation == Orientation::W {
                        if ray_length1_d.x - 0.5 * ray_unitstep_size.x <= ray_length1_d.y {
                            distance = ray_length1_d.x - ray_unitstep_size.x * 0.5;
                            if door_offset < 1.0 {
                                let pos_y = (startv.y + ray_dir_norm.y * distance) % 1.0;
                                if pos_y > door_offset * 0.5 && 1.0 - pos_y > door_offset * 0.5 {
                                    last_was_door = true;
                                    tilefound = false;
                                }
                            }
                        } else {
                            if ray_dir_norm.y < 0.0 {
                                orientation = Orientation::S;
                                map_checkv.y -= 1.0;
                            } else {
                                orientation = Orientation::N;
                                map_checkv.y += 1.0;
                            }
                            wall_type = 7;
                            distance = ray_length1_d.y;
                        }
                    }
                } else if wall_type > 0 {
                    tilefound = true;
                }
                if ((orientation == Orientation::W || orientation == Orientation::E)
                    && self.map.walls
                        [startv.y as usize * self.map_size.0 + (map_checkv.x - stepv.x) as usize]
                        == 6)
                    || ((orientation == Orientation::N || orientation == Orientation::S)
                        && self.map.walls[(map_checkv.y - stepv.y) as usize * self.map_size.0
                            + startv.x as usize]
                            == 6)
                {
                    wall_type = 7;
                }
                if tilefound {
                    let intersection = startv + ray_dir_norm * distance;
                    self.intersections.points[j] = intersection.to_array();
                    self.intersections.distance_fisheye[j] = distance;
                    distance *= (theta).cos();

                    self.intersections.distances[j] = distance;
                    self.intersections.wall_type[j] = wall_type as usize;
                    self.intersections.map_checkv[j] =
                        map_checkv.y as usize * self.map_size.0 + map_checkv.x as usize;
                    self.intersections.orientation[j] = orientation;
                }
            }
        }
        Ok(())
    }

    fn draw_slice(&self, slice: &mut [u8], j: usize, h: f32) {
        let rect_h =
            (self.player.planedist / (self.intersections.distances[j]) * 100.0).round() / 100.0;
        let rect_top = (h - rect_h) * 0.5;
        let rect_bottom = (h + rect_h) * 0.5;
        let ty_step = (self.cell_size) / rect_h;
        let pos = self.intersections.points[j];

        let inter_x = pos[0] - pos[0].floor();
        let inter_y = pos[1] - pos[1].floor();

        let wall_type = self.intersections.wall_type[j];

        let pos_z = self.player.jump / (self.intersections.distances[j]);
        //draw walls
        let mut ty = {
            if rect_bottom - self.player.pitch - pos_z >= h {
                (-self.player.pitch - pos_z - rect_top) * ty_step
            } else if rect_top + self.player.pitch + pos_z < 0.0 {
                (-self.player.pitch - pos_z - rect_bottom) * ty_step
            } else {
                0.0
            }
        };

        let mut tx;
        match self.intersections.orientation[j] {
            Orientation::N => {
                tx = inter_x * self.cell_size;
                tx = self.cell_size - 1.0 - tx.floor();
            }
            Orientation::E => {
                tx = inter_y * self.cell_size;
            }
            Orientation::S => {
                tx = inter_x * self.cell_size;
            }
            Orientation::W => {
                tx = inter_y * self.cell_size;
                tx = self.cell_size - 1.0 - tx.floor();
            }
        }
        if wall_type == 6 {
            let offset = 1.0
                - self
                    .map
                    .doors
                    .get(&self.intersections.map_checkv[j])
                    .expect("error to draw door")
                    .offset;
            match self.intersections.orientation[j] {
                Orientation::N => {
                    if inter_x < 0.5 {
                        tx -= offset * 64.0;
                    } else {
                        tx += offset * 64.0;
                    }
                }
                Orientation::E => {
                    if inter_y > 0.5 {
                        tx -= offset * 64.0;
                    } else {
                        tx += offset * 64.0;
                    }
                }
                Orientation::S => {
                    if inter_x > 0.5 {
                        tx -= offset * 64.0;
                    } else {
                        tx += offset * 64.0;
                    }
                }
                Orientation::W => {
                    if inter_y < 0.5 {
                        tx -= offset * 64.0;
                    } else {
                        tx += offset * 64.0;
                    }
                }
            }
        }

        let rect_bottom_draw = {
            if self.player.pitch + pos_z + rect_bottom >= h {
                h - self.player.pitch - pos_z
            } else {
                rect_bottom
            }
        };

        for y in (self.player.pitch + pos_z + rect_top) as usize
            ..(self.player.pitch + pos_z + rect_bottom_draw) as usize
        {
            if ty >= 128.0 {
                dbg!(
                    rect_h,
                    rect_top,
                    rect_bottom,
                    ty,
                    self.player.pitch + pos_z + rect_top,
                    self.player.pitch + pos_z + rect_bottom_draw,
                    self.player.pitch
                );
                ty = 127.0;
            }

            self.screen.draw_texture(
                slice,
                [tx as usize, wall_type * 128 + ty as usize],
                y,
                RAYSPERPIXEL,
                self.torch.intensity
                    * self.lighting.get_lighting_wall(
                        tx / 128.0,
                        ty * 0.0234375, //*3.0/128.0
                        self.intersections.map_checkv[j],
                        &self.intersections.orientation[j],
                    ),
                (3.0 / (self.intersections.distance_fisheye[j]
                    * self.intersections.distance_fisheye[j]))
                    .min(1.5),
            );
            ty += ty_step;
        }

        //draw floor
        for y in (self.player.pitch + pos_z + rect_bottom) as usize..(h) as usize {
            if !(j > 24 / RAYSPERPIXEL && j < 308 / RAYSPERPIXEL && y > 805) {
                // Don't draw the floor behind the minimap image
                let current_dist = self.buffer_floors[y]; // Use a buffer since they're always the same values
                let weight = current_dist / (self.intersections.distances[j]);

                let rhs = self.player.pos * (1.0 - weight);
                let current_floor_x = weight * pos[0] + rhs.x;
                let current_floor_y = weight * pos[1] + rhs.y;

                let location =
                    current_floor_x as usize + current_floor_y as usize * self.map_size.0;
                let floor_type = self.map.floors[location];

                let ftx = (current_floor_x * self.cell_size) as usize % 128;
                let fty = (current_floor_y * self.cell_size) as usize % 128;
                let lighting = self.lighting.get_lighting_floor(
                    ftx as f32 / 128.0,
                    fty as f32 / 128.0,
                    location,
                );
                self.screen.draw_texture(
                    slice,
                    [ftx, (floor_type * 128) + fty],
                    y,
                    RAYSPERPIXEL,
                    self.torch.intensity * lighting,
                    (3.0 / (current_dist * current_dist)).min(1.5),
                )
            }
        }
        //draw ceiling
        let mut rect_top_draw = rect_top;
        if rect_top + self.player.pitch + pos_z > h {
            rect_top_draw = h - self.player.pitch - pos_z;
        }
        for y in 0..(rect_top_draw + self.player.pitch + pos_z) as usize {
            let current_dist = self.buffer_floors[y];
            let weight = current_dist / (self.intersections.distances[j]);

            let rhs = self.player.pos * (1.0 - weight);
            let current_floor_x = weight * pos[0] + rhs.x;
            let current_floor_y = weight * pos[1] + rhs.y;

            let ftx = (current_floor_x * self.cell_size) as usize % 128;
            let fty = (current_floor_y * self.cell_size) as usize % 128;

            self.screen.draw_texture(
                slice,
                [ftx, fty],
                y,
                RAYSPERPIXEL,
                self.torch.intensity
                    * self.lighting.get_lighting_floor(
                        ftx as f32 / 128.0,
                        fty as f32 / 128.0,
                        current_floor_x as usize + current_floor_y as usize * self.map_size.0,
                    ),
                (3.0 / (current_dist * current_dist)).min(1.5),
            );
            //self.screen.draw_pixel(slice, y as usize, &[0, 0, 0, 0]);
        }

        self.sprites.iter().for_each(|sprite| {
            sprite.draw(
                slice,
                &self.player,
                j,
                &self.screen,
                self.intersections.distances[j],
                RAYSPERPIXEL,
            )
        })
    }
}
impl EventHandler for MainState {
    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _: KeyMods, _: bool) {
        match keycode {
            KeyCode::L => self.lighting.switch = !self.lighting.switch,
            KeyCode::K => self.lighting.smooth_switch = !self.lighting.smooth_switch,
            KeyCode::Escape => ggez::event::quit(ctx),
            _ => (),
        }
    }
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.time += timer::delta(ctx).as_secs_f32();
        self.handle_input(ctx, self.player.dir_norm);

        self.player.walk_animation(&self.buffer_walking, self.time);

        for j in 0..self.angles.len() {
            self.calculate_ray(self.player.dir_norm, self.angles[j], j)?;
        }
        self.sprites
            .iter_mut()
            .for_each(|sprite| sprite.update(self.time));

        self.map.doors.iter_mut().for_each(|(_, d)| {
            if d.opening {
                d.update(self.time, 0.01, &mut self.map.solid)
            }
        });

        self.torch.update_intensity(self.time);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let (w, h) = graphics::drawable_size(ctx);
        graphics::clear(ctx, Color::BLACK);
        let mut corr_angle = self.player.dir_norm.angle();
        if corr_angle < 0.0 {
            corr_angle += 2.0 * PI;
        }
        let draw_param = graphics::DrawParam {
            src: graphics::Rect::new(
                360.0 / FOV * corr_angle / (2.0 * PI),
                0.4 - self.player.pitch / 864.0,
                1.0,
                1.0,
            ),
            ..Default::default()
        };
        self.sky.sb.set(self.sky.idx, draw_param)?;
        graphics::draw(ctx, &self.sky.sb, draw_param)?;

        (0..h as usize).for_each(|y| {
            if y < (self.player.pitch + h * 0.5) as usize {
                // Calculate ceiling y buffer
                self.buffer_floors[y] = (self.player.planedist - 2.0 * self.player.jump)
                    / (-2.0 * (y as f32 - self.player.pitch) + h);
            }
            if y > (h * 0.5 + self.player.pitch) as usize {
                // Calculate floor y buffer
                self.buffer_floors[y] = (self.player.planedist + 2.0 * self.player.jump)
                    / (2.0 * (-self.player.pitch + y as f32) - h);
            }
        });

        self.sprites
            .iter_mut()
            .for_each(|sprite| sprite.set_drawing_bounds(ctx, &self.player, RAYSPERPIXEL as f32));

        self.sprites.sort_by(|a: &Sprite, b: &Sprite| {
            b.calculate_distance_2(&self.player)
                .partial_cmp(&a.distance2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut img_arr = std::mem::take(&mut self.screen.img_arr);

        img_arr
            .par_chunks_mut(h as usize * 4 * RAYSPERPIXEL)
            .enumerate()
            .for_each(|(j, slice)| self.draw_slice(slice, w as usize / RAYSPERPIXEL - j - 1, h));

        self.screen.img_arr = img_arr;

        let img = self.screen.arr_to_rgba(ctx)?;

        graphics::draw(
            ctx,
            &img,
            DrawParam::default()
                .offset([0.5, 0.5])
                .rotation(std::f32::consts::FRAC_PI_2)
                .dest([w * 0.5, h * 0.5]),
        )?;

        draw_fps_counter(ctx)?;

        self.map.draw_minimap(ctx, self.map_size, &self.player)?;

        self.player.draw_circle(ctx)?;

        graphics::present(ctx)
    }
}

pub fn draw_fps_counter(ctx: &mut Context) -> GameResult<()> {
    let fps = timer::fps(ctx);
    let delta = timer::delta(ctx);
    let stats_display = graphics::Text::new(format!("FPS: {:.3}, delta: {:.3?}", fps, delta));

    graphics::draw(
        ctx,
        &stats_display,
        DrawParam::new()
            .dest([0.0, 0.0])
            .color(graphics::Color::WHITE),
    )
}

pub struct Intersections {
    points: Vec<[f32; 2]>,
    distances: Vec<f32>,
    distance_fisheye: Vec<f32>,
    map_checkv: Vec<usize>,
    orientation: Vec<Orientation>,
    wall_type: Vec<usize>,
}

impl Intersections {
    pub fn new(w:usize) -> Self {
        Self {
            points: vec![[0.0, 0.0]; w/RAYSPERPIXEL],
            distances: vec![0.0; w/RAYSPERPIXEL],
            distance_fisheye: vec![0.0; w/RAYSPERPIXEL],
            map_checkv: vec![0; w/RAYSPERPIXEL],
            orientation: vec![Orientation::N; w/RAYSPERPIXEL],
            wall_type: vec![0; w/RAYSPERPIXEL],
        }
    }
}
#[derive(PartialEq, Clone, Debug)]
pub enum Orientation {
    N = 1,
    E = 2,
    S = 3,
    W = 4,
}

pub struct Sky {
    sb: graphics::spritebatch::SpriteBatch,
    idx: graphics::spritebatch::SpriteIdx,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
