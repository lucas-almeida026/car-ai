use std::f64::consts::PI;

use crate::car::Car;
use crate::fns::{get_intersectionf, lerpf64};
use crate::road::Border;
use sdl2::pixels::Color;
use sdl2::rect::FPoint;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub enum SensorPos {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
	CenterLeft,
	CenterRight
}

pub enum Facing {
    Forward,
    Backward,
    LeftSide,
    RightSide,
}

pub struct Sensor {
    pub rays: Vec<Ray>,
    pub pos: SensorPos,
}

impl Sensor {
    pub fn new(
        ray_count: u32,
        ray_length: f32,
        ray_spread: f64,
        pos: SensorPos,
        facing: Facing,
    ) -> Self {
        let mut rays = Vec::new();
        for i in 0..ray_count {
            let mut ray_angle = lerpf64(
                ray_spread / 2.0,
                -ray_spread / 2.0,
                if ray_count == 1 {
                    0.5
                } else {
                    i as f64 / (ray_count - 1) as f64
                },
            );
            match facing {
                Facing::LeftSide => ray_angle += PI / 2.0,
                Facing::RightSide => ray_angle -= PI / 2.0,
                Facing::Backward => ray_angle += PI,
                _ => {}
            }
            let ray = Ray::new(ray_length, ray_angle);
            rays.push(ray);
        }
        Self { rays, pos }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f64,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
        offset: f32,
        is_best: bool,
    ) -> Result<Vec<f32>, String> {
        let mut result = Vec::with_capacity(self.rays.len());
        for ray in self.rays.iter() {
            let reading = ray.render(
                canvas, x, y, w, h, angle, &borders, &traffic, offset, is_best, &self.pos
            )?;
            result.push(reading);
        }
        Ok(result)
    }
}

pub struct Ray {
    pub length: f32,
    pub angle: f64,
}

impl Ray {
    pub fn new(length: f32, angle: f64) -> Self {
        Self { length, angle }
    }
    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f64,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
        offset: f32,
        is_best: bool,
        pos: &SensorPos,
    ) -> Result<f32, String> {
        let (base_x, base_y) = Self::get_base_point(pos, x, y, w, h);
        let start = FPoint::new(base_x, base_y);
        let end = FPoint::new(
            base_x - self.length * (self.angle - angle.to_radians()).sin() as f32,
            base_y - self.length * (self.angle - angle.to_radians()).cos() as f32,
        );
        let mut touches: Vec<(FPoint, f32)> = Vec::new();
        for border in borders.iter() {
            let touch = get_intersectionf(
                start.x,
                start.y,
                end.x,
                end.y,
                border.start.x as f32,
                border.start.y as f32,
                border.end.x as f32,
                border.end.y as f32,
            );
            if let Some(t) = touch {
                touches.push(t);
            }
        }

        for car in traffic.iter() {
			let points = car.hitbox();
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let touch = get_intersectionf(
                    start.x, start.y, end.x, end.y, a.x as f32, a.y as f32, b.x as f32, b.y as f32,
                );

                if let Some(t) = touch {
                    touches.push(t);
                }
            }
        }

        let mut closest = end;
        let mut reading: Option<f32> = None;
        if touches.len() > 0 {
            touches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            closest = touches[0].0;

            reading = Some(1.0 - touches[0].1);
        }

        if is_best {
            canvas.set_draw_color(Color::RGB(32, 232, 32));
            canvas.draw_fline(start, closest)?;
        }

        if touches.len() > 0 && is_best {
            canvas.set_draw_color(Color::RGB(255, 32, 64));
            canvas.draw_fline(closest, end)?;
        }

        Ok(reading.unwrap_or(0.0))
    }

    fn get_base_point(pos: &SensorPos, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) { match pos {
			SensorPos::TopLeft => (x + w * 0.15, y + h * 0.1),
			SensorPos::TopRight => (x + w * 0.85, y + h * 0.1),
			SensorPos::BottomLeft => (x + w * 0.15, y + h * 0.9),
			SensorPos::BottomRight => (x + w * 0.85, y + h * 0.9),
			SensorPos::CenterLeft => (x + w * 0.15, y + h / 2.0),
			SensorPos::CenterRight => (x + w * 0.85, y + h / 2.0),
            _ => (x + w / 2.0, y + h / 2.0)
        }
    }
}
