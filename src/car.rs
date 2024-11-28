use std::f64::consts::PI;

use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{FRect, Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::fns::get_intersectionf;
use crate::network::NeuralNetwork;
use crate::road::{Border, Road};
use crate::sensor::Sensor;
use crate::texture::{self, SizedTexture, TexturePool};

pub struct Car<'a> {
    dimentions: Dimentions,
    pub position: Position,
    sensors: Vec<Sensor>,
    motion: Motion,
    controls: Controls,
    pub damaged: bool,
    focused_texture: &'a Texture<'a>,
    unfocused_texture: &'a Texture<'a>,
    damaged_texture: &'a Texture<'a>,
    dummy: bool,
    pub brain: Option<NeuralNetwork>,
    src_rect: Option<Rect>,
    pub score: i64,
    changing_lane: bool,
    break_checking: bool,
    break_checking_frame_count: u32,
    target_lane: u32,
    current_lane: u32,
    hitbox: Vec<Point>,
    sensor_readings: Vec<f32>,
}

impl<'a> Car<'a> {
    pub fn new(
        current_lane: u32,
        focused: &'a SizedTexture<'a>,
        unfocused: &'a SizedTexture<'a>,
        damaged: &'a SizedTexture<'a>,
        ref_brain: Option<&NeuralNetwork>,
        t: f64,
    ) -> Self {
        let dimentions = Dimentions::new(focused.width, focused.height, 1.0);
        let position = Position::new(400.0, 600.0, 0.0);
        let motion = Motion::new(0.0, 10.0, 0.4, 0.08);
        let controls = Controls::new();

        let sensors = vec![
            Sensor::new(
                18,
                210.0,
                PI * 1.5,
                dimentions.w as u16,
                dimentions.h as u16,
            ),
            Sensor::new(
                11,
                380.0,
                PI * 0.3,
                dimentions.w as u16,
                dimentions.h as u16,
            ),
            Sensor::new(
                3,
                520.0,
                PI * 0.15,
                dimentions.w as u16,
                dimentions.h as u16,
            ),
        ];
        let total_sensors = sensors.iter().map(|s| s.rays.len() as u32).sum();
        let mut brain = NeuralNetwork::new(&[total_sensors, 64, 64, 64, 64, 4]);
        brain.randomize();

        if ref_brain.is_some() {
            brain.prune(ref_brain.unwrap(), t as f32);
        }

        Car {
            dimentions,
            position,
            motion,
            controls,
            damaged: false,
            focused_texture: &focused.texture,
            unfocused_texture: &unfocused.texture,
            damaged_texture: &damaged.texture,
            src_rect: None,
            dummy: false,
            brain: Some(brain),
            sensors,
            score: 0,
            target_lane: current_lane,
            current_lane,
            changing_lane: false,
            break_checking: false,
            break_checking_frame_count: 0,
            hitbox: vec![],
            sensor_readings: vec![0.0; total_sensors as usize],
        }
    }

    pub fn src_crop_center(&mut self, width: u32, height: u32, scale: f64) {
        if scale > 0.0 && scale < 1.0 {
            self.dimentions.scale = scale;
        }
        let x = (self.dimentions.w - width) / 2;
        let y = (self.dimentions.h - height) / 2;
        self.src_rect = Some(Rect::new(
            (x as i32).max(0),
            (y as i32).max(0),
            width.min(self.dimentions.w),
            height.min(self.dimentions.h),
        ));
        self.dimentions.w = (width as f64 * self.dimentions.scale) as u32;
        self.dimentions.h = (height as f64 * self.dimentions.scale) as u32;

        for sensor in self.sensors.iter_mut() {
            for ray in sensor.rays.iter_mut() {
                ray.w = (width as f64 * self.dimentions.scale) as u16;
                ray.h = (height as f64 * self.dimentions.scale) as u16;
            }
        }
    }

    pub fn set_in_lane(&mut self, road: &Road, idx: u32) -> Result<(), String> {
        let lane_center = road
            .lane_center(idx)
            .map(|x| x - (self.scaled_width() as f32 / 2.0));

        if lane_center.is_none() {
            return Err("Could not find lane center".to_string());
        }

        self.position.x = lane_center.unwrap();
        Ok(())
    }

    //TODO: inline this logic in the render method
    pub fn is_passed_bottom_bound(&self, h: i32, offset: f32) -> bool {
        let (_, scaled_h) = self.src_dimentions_scaled();
        let y = self.position.y - offset;
        y - scaled_h > (h as f32)
    }

    pub fn update(&mut self, offset: f32, road: &Road, traffic: &Vec<Car>, cars_alive: &mut i32) {
        if !self.damaged {
            self.score += 1;
        }
        self.hitbox = self.rotate_hitbox_points(offset);

        for i in 0..self.hitbox.len() {
            let a = self.hitbox[i];
            let b = self.hitbox[(i + 1) % self.hitbox.len()];
            let mut touches: Vec<(Point, f32)> = Vec::new();
            if !self.damaged {
                for border in road.borders.iter() {
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
                    let points = car.rotate_hitbox_points(offset);
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
            }

            if touches.len() > 0 {
                if !self.damaged {
                    *cars_alive -= 1;
                }
                self.damaged = true;
                self.changing_lane = false;
                self.position.angle = 0.0;
            }
        }

        if !self.damaged {
            self.sensor_readings.truncate(0);
            for sensor in self.sensors.iter_mut() {
                let r = sensor.update(
                    self.position.x,
                    self.position.y,
                    self.position.angle,
                    offset,
                    &road.borders,
                    &traffic,
                );
                self.sensor_readings.append(&mut r.clone());
            }
            self.score += 1;
        }

        if self.brain.is_some() && !self.damaged {
            let outputs = self
                .brain
                .as_mut()
                .unwrap()
                .feed_forward(&self.sensor_readings);
            assert_eq!(outputs.len(), 4);
            self.controls.forward = outputs[0] > 0.33;
            self.controls.backward = outputs[1] > 0.33;
            self.controls.left = outputs[2] > 0.33;
            self.controls.right = outputs[3] > 0.33;
            // println!("forward:  {}\nbackward: {}\nleft:     {}\nright:    {}\n\n", outputs[0], outputs[1], outputs[2], outputs[3]);
        }

        self.update_position(road);
    }

    pub fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        offset: f32,
        is_best: bool,
    ) -> Result<(), String> {
        // render texture
        let (scaled_w, scaled_h) = self.src_dimentions_scaled();
        let mut drawing_texture = if is_best {
            self.focused_texture
        } else {
            self.unfocused_texture
        };

        if self.damaged {
            drawing_texture = self.damaged_texture;
            canvas.set_draw_color(Color::RGB(255, 12, 255));
        } else {
            canvas.set_draw_color(Color::RGB(12, 0, 255));
        }

        let dst_rect = FRect::new(
            self.position.x,
            self.position.y as f32 - offset,
            scaled_w,
            scaled_h,
        );

        canvas.copy_ex_f(
            drawing_texture,
            self.src_rect,
            dst_rect,
            self.position.angle,
            None,
            false,
            false,
        )?;

        // render hitbox
        for i in 0..self.hitbox.len() {
            let a = self.hitbox[i];
            let b = self.hitbox[(i + 1) % self.hitbox.len()];
            canvas.draw_line(a, b)?;
        }

        if !self.damaged {
            for sensor in self.sensors.iter_mut() {
                if is_best {
                    sensor.render(canvas).map_err(|e| e.to_string())?;
                }
            }
        }
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

    pub fn hitbox(&self) -> &Vec<Point> {
        &self.hitbox
    }

    pub fn rotate_hitbox_points(&self, offset: f32) -> Vec<Point> {
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

    fn update_position(&mut self, road: &Road) {
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

        if self.motion.velocity > 0.1 || self.motion.velocity < -0.1 {
            if self.controls.left {
                self.turn_left_by(1.2);
            }
            if self.controls.right {
                self.turn_right_by(1.2);
            }
        }

        self.normalize_angle();
        self.normalize_velocity();
        self.apply_friction();

        if self.dummy {
            // let rand_num = rand::thread_rng().gen_range(1..60 * 6);
            // if rand_num == 1 && !self.changing_lane{
            //     self.motion.velocity /= 1.4;
            // } else if rand_num > 60 * 6 - 30 {
            //     self.motion.velocity = self.motion.max_velocity - 0.01;
            // }

            if !self.break_checking {
                let should_break_check = rand::thread_rng().gen_range(1..60 * 4) == 1;
                if should_break_check {
                    self.break_checking = true;
                    self.motion.velocity /= 1.3;
                    self.break_checking_frame_count = 40;
                }
            } else {
                if self.break_checking_frame_count == 0 {
                    self.break_checking = false;
                    self.motion.velocity = self.motion.max_velocity - 0.01;
                } else {
                    self.break_checking_frame_count -= 1;
                }
            }

            if !self.changing_lane {
                let should_change_lane = rand::thread_rng().gen_range(1..60 * 4) == 1;
                if should_change_lane {
                    self.changing_lane = true;
                    self.target_lane = road.random_lane_idx();
                    let aggressiveness = rand::thread_rng().gen_range(4..12);
                    if self.current_lane > self.target_lane {
                        self.turn_left_by(aggressiveness as f64);
                    } else if self.current_lane < self.target_lane {
                        self.turn_right_by(aggressiveness as f64);
                    } else {
                        self.changing_lane = false;
                    }
                }
            } else {
                let target_x =
                    road.lane_center(self.target_lane).unwrap() - (self.dimentions.w as f32 / 2.0);
                let x = self.position.x;
                let xmin = x - 1.0;
                let xmax = x + 1.0;
                if xmin < target_x && xmax > target_x {
                    self.position.angle = 0.0;
                    self.changing_lane = false;
                    self.current_lane = self.target_lane;
                }
            }
        }

        self.position.x += self.position.angle.to_radians().sin() as f32 * self.motion.velocity;
        self.position.y -= self.position.angle.to_radians().cos() as f32 * self.motion.velocity;
    }

    pub fn as_dummy(&mut self, max_velocity: f32) {
        self.controls.forward = true;
        self.motion.acceleration = 0.0;
        self.motion.velocity = max_velocity - 0.01;
        self.motion.max_velocity = max_velocity;
        self.motion.friction = 0.0;
        self.sensors = vec![];
        self.dummy = true;
        self.damaged = false;
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

    pub fn screen_offset(&self, target_y: f32) -> f32 {
        self.car.position.y - target_y
    }

    pub fn update(&mut self, offset: f32, road: &Road, traffic: &Vec<Car>, cars_alive: &mut i32) {
        self.car.update(offset, road, traffic, cars_alive);
    }

    pub fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        offset: f32,
        is_best: bool,
    ) -> Result<(), String> {
        self.car.render(canvas, offset, is_best)
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

pub fn create_main_texture<'a>(
    tc: &'a TextureCreator<WindowContext>,
) -> Result<SizedTexture<'a>, String> {
    let mut main = texture::from_file("assets/car.png", &tc)?;
    main.texture.set_blend_mode(BlendMode::Blend);
    Ok(main)
}

pub fn create_damaged_texture<'a>(
    tc: &'a TextureCreator<WindowContext>,
) -> Result<SizedTexture<'a>, String> {
    let mut damaged = texture::from_file("assets/car.png", &tc)?;
    damaged.texture.set_blend_mode(BlendMode::Blend);
    damaged.texture.set_alpha_mod(128);
    damaged.texture.set_color_mod(64, 64, 64);
    Ok(damaged)
}

pub fn create_unfocused_texture<'a>(
    tc: &'a TextureCreator<WindowContext>,
) -> Result<SizedTexture<'a>, String> {
    let mut unfocused = texture::from_file("assets/car.png", &tc)?;
    unfocused.texture.set_blend_mode(BlendMode::Blend);
    unfocused.texture.set_alpha_mod(128);
    Ok(unfocused)
}

pub fn create_traffic_texture_pool<'a>(
    tc: &'a TextureCreator<WindowContext>,
    size: u32,
) -> Result<TexturePool<'a>, String> {
    let mut pool = TexturePool::new(size, tc)?;
    let colors = [(255, 32, 64), (32, 255, 64)];
    for (i, t) in pool.pool.iter_mut().enumerate() {
        let color = colors[i % colors.len()];
        t.texture.set_blend_mode(BlendMode::Blend);
        t.texture.set_alpha_mod(192);
        t.texture.set_color_mod(color.0, color.1, color.2);
    }
    Ok(pool)
}
