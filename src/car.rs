use std::f64::consts::PI;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{FPoint, FRect, Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::road::Border;
use crate::sensor::Sensor;

pub struct Car<'a> {
    width: u32,
    height: u32,
    pub x: f32,
    pub y: f32,
    sensors: Vec<Sensor>,
    angle: f64,
    scale: f32,
    velocity: f32,
    max_velocity: f32,
    acceleration: f32,
    friction: f32,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    texture: Texture<'a>,
    src_rect: Option<Rect>,
}

impl<'a> Car<'a> {
    pub fn try_new(
        path: &'static str,
        tc: &'a TextureCreator<WindowContext>,
    ) -> Result<Self, String> {
        let mut image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();

        let (width, height) = (image.width(), image.height());

        let surface = Surface::from_data(
            &mut image,
            width,
            height,
            width * 4,
            PixelFormatEnum::ABGR8888,
        )
        .map_err(|e| e.to_string())?;

        let mut texture = tc
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;

        texture.set_blend_mode(BlendMode::Blend);

        Ok(Car {
            width,
            height,
            x: 400.0,
            y: 600.0,
            angle: 0.0,
            scale: 1.0,
            velocity: 0.0,
            max_velocity: 10.0,
            acceleration: 0.4,
            friction: 0.08,
            forward: false,
            backward: false,
            left: false,
            right: false,
            texture,
            src_rect: None,
            sensors: vec![Sensor::new(3, 220.0, PI / 4.0)],
        })
    }

    pub fn src_crop_center(&mut self, width: u32, height: u32) {
        let x = (self.width - width) / 2;
        let y = (self.height - height) / 2;
        self.src_rect = Some(Rect::new(
            (x as i32).max(0),
            (y as i32).max(0),
            width.min(self.width),
            height.min(self.height),
        ));
        self.width = width;
        self.height = height;
    }

    pub fn set_scale(&mut self, scale: f32) {
        if scale > 0.0 && scale < 1.0 {
            self.scale = scale;
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas<Window>, offset: f32, borders: &Vec<Border>) -> Result<(), String> {
        // render texture
        let w = self.src_rect.map(|r| r.width()).unwrap_or(self.width) as f32;
        let h = self.src_rect.map(|r| r.height()).unwrap_or(self.height) as f32;

        let scaled_w = w * self.scale;
        let scaled_h = h * self.scale;

        let dst_rect = FRect::new(self.x, self.y as f32 - offset, scaled_w, scaled_h);

        canvas.copy_ex_f(
            &self.texture,
            self.src_rect,
            dst_rect,
            self.angle,
            None,
            false,
            false,
        )?;

        // render hitbox
        let points = [
            (-scaled_w / 2.0, -scaled_h / 2.0),
            (scaled_w / 2.0, -scaled_h / 2.0),
            (scaled_w / 2.0, scaled_h / 2.0),
            (-scaled_w / 2.0, scaled_h / 2.0),
        ];

        let center_x = self.x + scaled_w / 2.0;
        let center_y = (self.y - offset) + scaled_h / 2.0;

        let angle_rad = self.angle.to_radians() as f32;
        let rotated_points: Vec<Point> = points
            .iter()
            .map(|&(px, py)| {
                let rx = px * angle_rad.cos() - py * angle_rad.sin();
                let ry = px * angle_rad.sin() + py * angle_rad.cos();
                Point::new((rx + center_x) as i32, (ry + center_y) as i32)
            })
            .collect();

        canvas.set_draw_color(Color::RGB(250, 240, 90));
        for (i, point) in rotated_points.iter().enumerate() {
            let end = rotated_points[(i + 1) % rotated_points.len()];
            canvas.draw_line(*point, end).map_err(|e| e.to_string())?
        }

        for sensor in self.sensors.iter_mut() {
            sensor
                .render(
                    canvas,
                    self.x,
                    self.y - offset,
                    scaled_w,
                    scaled_h,
                    self.angle,
					&borders
                )
                .map_err(|e| e.to_string())?
        }
        Ok(())
    }

    pub fn scaled_width(&self) -> f32 {
        self.width as f32 * self.scale
    }

    pub fn scaled_height(&self) -> f32 {
        self.height as f32 * self.scale
    }

    pub fn update_position(&mut self) {
        if self.forward {
            if self.velocity < self.max_velocity / 2.0 {
                self.velocity += self.acceleration / 1.6;
            } else {
                self.velocity += self.acceleration;
            }
        }
        if self.backward {
            if self.velocity < self.max_velocity / 2.0 {
                self.velocity -= self.acceleration / 2.0;
            } else {
                self.velocity -= self.acceleration / 1.4;
            }
        }

        self.angle %= 360.0;

        if self.angle < 0.0 {
            self.angle += 360.0;
        }

        if self.velocity > self.max_velocity {
            self.velocity = self.max_velocity;
        } else if self.velocity < -self.max_velocity / 2.0 {
            self.velocity = -self.max_velocity / 2.0;
        }

        if self.velocity > 0.0 {
            self.velocity -= self.friction;
        } else if self.velocity < 0.0 {
            self.velocity += self.friction;
        }
        if self.velocity.abs() < self.friction {
            self.velocity = 0.0;
        }

        if self.velocity != 0.0 {
            if self.left {
                self.angle -= 1.5 * if self.velocity > 0.0 { 1.0 } else { -1.0 };
            }
            if self.right {
                self.angle += 1.5 * if self.velocity > 0.0 { 1.0 } else { -1.0 };
            }
        }

        self.x += self.angle.to_radians().sin() as f32 * self.velocity;
        self.y -= self.angle.to_radians().cos() as f32 * self.velocity;
    }

    pub fn update_state(&mut self, event: &Event) {
        match event {
            Event::KeyDown {
                keycode: Some(Keycode::Left),
                ..
            } => {
                self.left = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Left),
                ..
            } => {
                self.left = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Right),
                ..
            } => {
                self.right = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Right),
                ..
            } => {
                self.right = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Up),
                ..
            } => {
                self.forward = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Up),
                ..
            } => {
                self.forward = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Down),
                ..
            } => {
                self.backward = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Down),
                ..
            } => {
                self.backward = false;
            }
            _ => {}
        }
    }

    pub fn center_point(&self) -> FPoint {
        FPoint::new(self.x + self.scaled_width() / 2.0, self.y)
    }
}
