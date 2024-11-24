use std::f64::consts::PI;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{FRect, Point, Rect};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::fns::get_intersectionf;
use crate::network::NeuralNetwork;
use crate::road::Border;
use crate::sensor::{Facing, Sensor, SensorPos};

pub struct Car<'a> {
    dimentions: Dimentions,
    pub position: Position,
    sensors: Vec<Sensor>,
    motion: Motion,
    controls: Controls,
    pub damaged: bool,
    texture: &'a Texture<'a>,
    dummy: bool,
    pub brain: Option<NeuralNetwork>,
    src_rect: Option<Rect>,
    pub score: i64,
    last_y: f32,
    same_y_count: u32,
}

impl<'a> Car<'a> {
    pub fn new(
        texture: &'a Texture<'a>,
        width: u32,
        height: u32,
        ref_brain: Option<&NeuralNetwork>,
        t: f64,
    ) -> Result<Self, String> {
        let dimentions = Dimentions::new(width, height, 1.0);
        let position = Position::new(400.0, 600.0, 0.0);
        let motion = Motion::new(0.0, 10.0, 0.4, 0.08);
        let controls = Controls::new();

        let sensors = vec![
            Sensor::new(3, 320.0, PI / 7.0, SensorPos::TopLeft, Facing::Forward),
            Sensor::new(2, 550.0, PI / 5.0, SensorPos::Center, Facing::Forward),
            Sensor::new(3, 320.0, PI / 7.0, SensorPos::TopRight, Facing::Forward),
            Sensor::new(3, 120.0, PI / 2.0, SensorPos::CenterLeft, Facing::LeftSide),
            Sensor::new(
                3,
                120.0,
                PI / 2.0,
                SensorPos::CenterRight,
                Facing::RightSide,
            ),
            Sensor::new(1, 150.0, PI / 16.0, SensorPos::Center, Facing::Backward),
        ];
        let total_sensors = sensors.iter().map(|s| s.rays.len() as u32).sum();
        let mut brain = NeuralNetwork::new(&[total_sensors, 64, 64, 64, 4]);
        brain.randomize();

        if ref_brain.is_some() {
            brain.prune(ref_brain.unwrap(), t);
        }

        Ok(Car {
            dimentions,
            position,
            motion,
            controls,
            damaged: false,
            texture,
            src_rect: None,
            dummy: false,
            brain: Some(brain),
            sensors,
            score: 0,
            last_y: 600.0,
            same_y_count: 0,
        })
    }

    pub fn src_crop_center(&mut self, width: u32, height: u32) {
        let x = (self.dimentions.w - width) / 2;
        let y = (self.dimentions.h - height) / 2;
        self.src_rect = Some(Rect::new(
            (x as i32).max(0),
            (y as i32).max(0),
            width.min(self.dimentions.w),
            height.min(self.dimentions.h),
        ));
        self.dimentions.w = width;
        self.dimentions.h = height;
    }

    pub fn set_scale(&mut self, scale: f64) -> Result<(), String> {
        if scale > 0.0 && scale < 1.0 {
            self.dimentions.scale = scale;
            Ok(())
        } else {
            Err("Scale must be between 0 and 1".to_string())
        }
    }

    pub fn is_passed_bottom_bound(&self, h: i32, offset: f32) -> bool {
        let (_, scaled_h) = self.src_dimentions_scaled();
        let y = self.position.y - offset;
        y - scaled_h > (h as f32)
    }

    pub fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        offset: f32,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
        is_best: bool,
        cars_alive: &mut i32,
    ) -> Result<(), String> {
        // render texture
        let (scaled_w, scaled_h) = self.src_dimentions_scaled();

        if !self.damaged {
            self.score += 1;
            // drawing_texture.set_color_mod(64, 64, 64);
        }

        let dst_rect = FRect::new(
            self.position.x,
            self.position.y as f32 - offset,
            scaled_w,
            scaled_h,
        );

        canvas.copy_ex_f(
            &self.texture,
            self.src_rect,
            dst_rect,
            self.position.angle,
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
                if !self.damaged {
                    *cars_alive -= 1;
                }
                self.damaged = true;
                canvas.set_draw_color(Color::RGB(255, 12, 255));
            } else {
                canvas.set_draw_color(Color::RGB(12, 0, 255));
            }

            canvas.draw_line(a, b)?;
        }

        let mut readings = vec![];
        for sensor in self.sensors.iter_mut() {
            let mut buffer = sensor
                .render(
                    canvas,
                    self.position.x,
                    self.position.y - offset,
                    scaled_w,
                    scaled_h,
                    self.position.angle,
                    &borders,
                    &traffic,
                    offset,
                    is_best,
                )
                .map_err(|e| e.to_string())?;
            readings.append(&mut buffer);
        }

        if self.brain.is_some() {
            let outputs = self.brain.as_mut().unwrap().feed_forward(readings);
            assert_eq!(outputs.len(), 4);
            self.controls.forward = outputs[0] > 0.5;
            self.controls.backward = outputs[1] > 0.5;
            self.controls.left = outputs[2] > 0.5;
            self.controls.right = outputs[3] > 0.5;
        }

        if offset == self.last_y {
            self.same_y_count += 1;
            self.score -= 100;
            if self.same_y_count > 60 {
                *cars_alive -= 1;
                self.damaged = true;
            }
        } else if self.position.y < self.last_y {
            self.score += 2;
        }
        if self.motion.velocity > self.motion.max_velocity * 0.7 {
            self.score += 1;
        }
        self.last_y = offset;
        Ok(())
    }

    pub fn scaled_width(&self) -> f64 {
        self.dimentions.w as f64 * self.dimentions.scale
    }

    pub fn scaled_height(&self) -> f64 {
        self.dimentions.h as f64 * self.dimentions.scale
    }

    pub fn src_dimentions_scaled(&self) -> (f32, f32) {
        let w = self
            .src_rect
            .map(|r| r.width())
            .unwrap_or(self.dimentions.w) as f32;
        let h = self
            .src_rect
            .map(|r| r.height())
            .unwrap_or(self.dimentions.h) as f32;

        let scaled_w = w * self.dimentions.scale as f32;
        let scaled_h = h * self.dimentions.scale as f32;

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
        let (w, h) = self.src_dimentions_scaled();
        let center_x = self.position.x + w / 2.0;
        let center_y = (self.position.y - offset) + h / 2.0;
        let angle_rad = self.position.angle.to_radians() as f32;
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
            self.motion.velocity = 0.0;
            return;
        }
        if self.controls.forward {
            if self.motion.velocity < self.motion.max_velocity / 2.0 {
                self.motion.velocity += self.motion.acceleration / 1.6;
            } else {
                self.motion.velocity += self.motion.acceleration;
            }
        }
        if self.controls.backward {
            if self.motion.velocity < self.motion.max_velocity / 2.0 {
                self.motion.velocity -= self.motion.acceleration / 2.0;
            } else {
                self.motion.velocity -= self.motion.acceleration / 1.4;
            }
        }

        self.normalize_angle();
        self.normalize_velocity();
        self.apply_friction();

        if self.motion.velocity != 0.0 && self.motion.velocity > 0.0 {
            if self.controls.left {
                self.turn_left_by(1.2);
            }
            if self.controls.right && self.motion.velocity > 0.0 {
                self.turn_right_by(1.2);
            }
        }

        self.position.x += self.position.angle.to_radians().sin() as f32 * self.motion.velocity;
        self.position.y -= self.position.angle.to_radians().cos() as f32 * self.motion.velocity;
    }

    pub fn as_dummy(&mut self, max_velocity: f32) {
        self.controls.forward = true;
        self.motion.acceleration = 0.0;
        self.motion.velocity = max_velocity;
        self.motion.friction = 0.0;
        self.sensors = vec![];
        self.dummy = true;
        self.brain = None;
    }

    fn turn_left_by(&mut self, amount: f64) {
        self.position.angle -= amount
    }

    fn turn_right_by(&mut self, amount: f64) {
        self.position.angle += amount
    }

    fn normalize_angle(&mut self) {
        self.position.angle %= 360.0;
        if self.position.angle < 0.0 {
            self.position.angle += 360.0
        }
    }

    fn normalize_velocity(&mut self) {
        if self.motion.velocity > self.motion.max_velocity {
            self.motion.velocity = self.motion.max_velocity;
        } else if self.motion.velocity < -self.motion.max_velocity / 2.0 {
            self.motion.velocity = -self.motion.max_velocity / 2.0;
        }
        if self.motion.velocity.abs() < self.motion.friction {
            self.motion.velocity = 0.0;
        }
    }

    fn apply_friction(&mut self) {
        if self.motion.velocity > 0.0 {
            self.motion.velocity -= self.motion.friction;
        } else if self.motion.velocity < 0.0 {
            self.motion.velocity += self.motion.friction;
        }
    }
}

pub struct Dimentions {
    pub w: u32,
    pub h: u32,
    pub scale: f64,
}
impl Dimentions {
    pub fn new(w: u32, h: u32, scale: f64) -> Self {
        Self { w, h, scale }
    }
}

pub struct Position {
    pub x: f32,
    pub y: f32,
    pub angle: f64,
}
impl Position {
    pub fn new(x: f32, y: f32, angle: f64) -> Self {
        Self { x, y, angle }
    }
}

pub struct Motion {
    pub velocity: f32,
    pub max_velocity: f32,
    pub acceleration: f32,
    pub friction: f32,
}
impl Motion {
    pub fn new(velocity: f32, max_velocity: f32, acceleration: f32, friction: f32) -> Self {
        Self {
            velocity,
            max_velocity,
            acceleration,
            friction,
        }
    }
}

pub struct Controls {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}
impl Controls {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
        }
    }
}

pub struct ControlledCar<'a> {
    car: Car<'a>,
}
impl<'a> ControlledCar<'a> {
    pub fn new(mut car: Car<'a>) -> Self {
        car.brain = None;
        Self { car }
    }

    pub fn update_position(&mut self) {
        self.car.update_position();
    }

    pub fn screen_offset(&self, target_y: f32) -> f32 {
        self.car.position.y - target_y
    }

    pub fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        offset: f32,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
        is_best: bool,
        cars_alive: &mut i32,
    ) -> Result<(), String> {
        self.car
            .render(canvas, offset, borders, traffic, is_best, cars_alive)
    }

    pub fn process_event(&mut self, event: &Event) {
        if self.car.damaged {
            return;
        }
        match event {
            Event::KeyDown {
                keycode: Some(Keycode::Left),
                ..
            } => {
                self.car.controls.left = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Left),
                ..
            } => {
                self.car.controls.left = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Right),
                ..
            } => {
                self.car.controls.right = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Right),
                ..
            } => {
                self.car.controls.right = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Up),
                ..
            } => {
                self.car.controls.forward = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Up),
                ..
            } => {
                self.car.controls.forward = false;
            }
            Event::KeyDown {
                keycode: Some(Keycode::Down),
                ..
            } => {
                self.car.controls.backward = true;
            }
            Event::KeyUp {
                keycode: Some(Keycode::Down),
                ..
            } => {
                self.car.controls.backward = false;
            }
            _ => {}
        }
    }
}
