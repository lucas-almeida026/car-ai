use std::f64::consts::PI;

use rand::{self, Rng};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{FPoint, FRect, Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture};
use sdl2::video::Window;

use crate::fns::get_intersectionf;
use crate::network::NeuralNetwork;
use crate::road::Border;
use crate::sensor::Sensor;
type Polygon = Vec<FPoint>;

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
    damaged: bool,
    texture: &'a Texture<'a>,
    dummy: bool,
    pub brain: Option<NeuralNetwork>,
    src_rect: Option<Rect>,
}

impl<'a> Car<'a> {
    pub fn new(texture: &'a Texture<'a>, width: u32, height: u32) -> Result<Self, String> {
        let amount_of_sensors = 11;
        let mut brain = NeuralNetwork::new(&[amount_of_sensors, 6, 4]);
        brain.randomize();

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
            damaged: false,
            texture,
            src_rect: None,
            dummy: false,
            brain: Some(brain),
            sensors: vec![Sensor::new(amount_of_sensors, 220.0, PI / 2.0)],
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

    pub fn is_passed_bottom_bound(&self, h: i32, offset: f32) -> bool {
        let (_, scaled_h) = self.get_scaled_size();
        let y = self.y - offset;
        y - scaled_h > (h as f32)
    }

    pub fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        offset: f32,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
        is_best: bool,
    ) -> Result<(), String> {
        // render texture
		if is_best {
			// self.texture.set_alpha_mod(255);
		}
        let (scaled_w, scaled_h) = self.get_scaled_size();

        if self.damaged {
			// drawing_texture.set_color_mod(64, 64, 64);
        }

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
        let rotated_points: Vec<Point> = self.get_rotated_hitbox_points(offset);

        for i in 0..rotated_points.len() {
            let a = rotated_points[i];
            let b = rotated_points[(i + 1) % rotated_points.len()];
            let mut touches: Vec<(Point, f32)> = Vec::new();
            canvas.draw_line(a, b)?;
            for border in borders.iter() {
                let touch = get_intersectionf(
                    a.x as f32,
                    a.y as f32,
                    b.x as f32,
                    b.y as f32,
                    border.start.x as f32,
                    border.start.y as f32,
                    border.end.x as f32,
                    border.end.y as f32,
                );
                if let Some((p, t)) = touch {
                    touches.push((Point::new(p.x as i32, p.y as i32), t));
                }
            }
            for car in traffic.iter() {
                let points = car.get_rotated_hitbox_points(offset);

                for i in 0..points.len() {
                    let c = points[i];
                    let d = points[(i + 1) % points.len()];
                    let touch = get_intersectionf(
                        a.x as f32, a.y as f32, b.x as f32, b.y as f32, c.x as f32, c.y as f32,
                        d.x as f32, d.y as f32,
                    );

                    if let Some((p, t)) = touch {
                        touches.push((Point::new(p.x as i32, p.y as i32), t));
                    }
                }
            }

            if touches.len() > 0 {
                self.damaged = true;
                canvas.set_draw_color(Color::RGB(255, 12, 255));
            } else {
                canvas.set_draw_color(Color::RGB(12, 0, 255));
            }

            canvas.draw_line(a, b)?;
        }

        let mut readings = Vec::new();
        for sensor in self.sensors.iter_mut() {
            readings = sensor
                .render(
                    canvas,
                    self.x,
                    self.y - offset,
                    scaled_w,
                    scaled_h,
                    self.angle,
                    &borders,
                    &traffic,
                    offset,
                    is_best,
                )
                .map_err(|e| e.to_string())?;
        }

        if self.brain.is_some() {
            let outputs = self.brain.as_mut().unwrap().feed_forward(readings);
            assert_eq!(outputs.len(), 4);
            self.forward = outputs[0] > 0.5;
            self.backward = outputs[1] > 0.5;
            self.left = outputs[2] > 0.5;
            self.right = outputs[3] > 0.5;
        }
        Ok(())
    }

    pub fn scaled_width(&self) -> f32 {
        self.width as f32 * self.scale
    }

    pub fn scaled_height(&self) -> f32 {
        self.height as f32 * self.scale
    }

    pub fn get_scaled_size(&self) -> (f32, f32) {
        let w = self.src_rect.map(|r| r.width()).unwrap_or(self.width) as f32;
        let h = self.src_rect.map(|r| r.height()).unwrap_or(self.height) as f32;

        let scaled_w = w * self.scale;
        let scaled_h = h * self.scale;

        (scaled_w, scaled_h)
    }

    pub fn get_hitbox_points(&self, w: f32, h: f32) -> [(f32, f32); 10] {
        let side_y = h * 0.6;
        let side_w = w * 0.1;

        let front_y = h * 0.2;
        let front_w = w * 0.2;

        let corner_y = h * 0.04;
        let corner_w = w * 0.6;

        let back_y = h * 0.3;
        let back_w = w * 0.55;

        let points = [
            (-(w - side_w) / 2.0, -(h - side_y) / 2.0),
            (-(w - front_w) / 2.0, -(h - front_y) / 2.0),
            (-(w - corner_w) / 2.0, -(h - corner_y) / 2.0),
            ((w - corner_w) / 2.0, -(h - corner_y) / 2.0),
            ((w - front_w) / 2.0, -(h - front_y) / 2.0),
            ((w - side_w) / 2.0, -(h - side_y) / 2.0),
            ((w - side_w) / 2.0, (h - back_y) / 2.0),
            ((w - back_w) / 2.0, (h - side_w) / 2.0),
            (-(w - back_w) / 2.0, (h - side_w) / 2.0),
            (-(w - side_w) / 2.0, (h - back_y) / 2.0),
        ];

        points
    }

    pub fn get_rotated_hitbox_points(&self, offset: f32) -> Vec<Point> {
        let (w, h) = self.get_scaled_size();
        let center_x = self.x + w / 2.0;
        let center_y = (self.y - offset) + h / 2.0;
        let angle_rad = self.angle.to_radians() as f32;
        self.get_hitbox_points(w, h)
            .iter()
            .map(|&(px, py)| {
                let rx = px * angle_rad.cos() - py * angle_rad.sin();
                let ry = px * angle_rad.sin() + py * angle_rad.cos();
                Point::new((rx + center_x) as i32, (ry + center_y) as i32)
            })
            .collect()
    }

    pub fn update_position(&mut self) {
        if self.damaged {
            self.velocity = 0.0;
            return;
        }
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
                self.angle -= 1.2 * if self.velocity > 0.0 { 1.0 } else { -1.0 };
            }
            if self.right {
                self.angle += 1.2 * if self.velocity > 0.0 { 1.0 } else { -1.0 };
            }
        }

        self.x += self.angle.to_radians().sin() as f32 * self.velocity;
        self.y -= self.angle.to_radians().cos() as f32 * self.velocity;
    }

    pub fn update_state(&mut self, event: &Event) {
        if self.damaged || self.brain.is_some() {
            return;
        }
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

    pub fn as_dummy(&mut self, max_velocity: f32) {
        self.forward = true;
        self.acceleration = 0.0;
        self.velocity = max_velocity;
        self.friction = 0.0;
        self.sensors = vec![];
        self.dummy = true;
        self.brain = None;
    }
}
