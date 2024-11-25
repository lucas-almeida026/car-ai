use std::f64::consts::PI;

use crate::car::Car;
use crate::fns::{get_intersectionf, lerpf64};
use crate::road::Border;
use sdl2::pixels::Color;
use sdl2::rect::FPoint;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct Sensor {
    pub rays: Vec<Ray>,
    pub readings: Vec<f32>,
}

impl Sensor {
    pub fn new(ray_count: u32, ray_length: f32, ray_spread: f64, w: u16, h: u16) -> Self {
        let mut rays = Vec::new();
        let start = FPoint::new(0.0, 0.0);
        for i in 0..ray_count {
            let ray_angle = lerpf64(
                ray_spread / 2.0,
                -ray_spread / 2.0,
                if ray_count == 1 {
                    0.5
                } else {
                    i as f64 / (ray_count - 1) as f64
                },
            );
            let ray = Ray::new(ray_length, ray_angle, start, w, h);
            rays.push(ray);
        }
        Self {
            rays,
            readings: vec![0.0; ray_count as usize],
        }
    }

    pub fn update<'a>(
		&'a mut self,
        x: f32,
        y: f32,
        angle: f64,
		offset: f32,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
    ) -> &'a Vec<f32> {
		for (i, ray) in self.rays.iter_mut().enumerate() {
			ray.update(x, y, angle, offset, borders, traffic);
			self.readings[i] = ray.value.unwrap_or(0.0);
		}
		// println!("readings: {:#?}", &self.readings);
		&self.readings
    }

    pub fn render(&self, canvas: &mut Canvas<Window>, is_best: bool) -> Result<(), String> {
        for ray in self.rays.iter() {
            ray.render(canvas, is_best)?;
        }
		Ok(())
    }
}

pub struct Ray {
    pub length: f32,
    pub angle: f64,
    start: FPoint,
    end: FPoint,
    mid: FPoint,
    pub value: Option<f32>,
    pub w: u16,
    pub h: u16,
}

impl Ray {
    pub fn new(length: f32, angle: f64, start: FPoint, w: u16, h: u16) -> Self {
        Self {
            w,
            h,
            length,
            angle,
            start,
            mid: start.clone(),
            end: start.clone(),
            value: None,
        }
    }

    pub fn update(
        &mut self,
        x: f32,
        y: f32,
        angle: f64,
		offset: f32,
        borders: &Vec<Border>,
        traffic: &Vec<Car>,
    ) {
        let (base_x, base_y) = (
			x + self.w as f32 / 2.0,
			(y + self.h as f32 / 2.0) - offset
		);

        self.start.x = base_x;
        self.start.y = base_y;

        self.end.x = base_x - self.length * (self.angle - angle.to_radians()).sin() as f32;
        self.end.y = base_y - self.length * (self.angle - angle.to_radians()).cos() as f32;

        let mut touches: Vec<(FPoint, f32)> = Vec::new();
        for border in borders.iter() {
            let touch = get_intersectionf(
                self.start.x,
                self.start.y,
                self.end.x,
                self.end.y,
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
                    self.start.x,
                    self.start.y,
                    self.end.x,
                    self.end.y,
                    a.x as f32,
                    a.y as f32,
                    b.x as f32,
                    b.y as f32,
                );

                if let Some(t) = touch {
                    touches.push(t);
                }
            }
        }

        self.mid = self.end.clone();
        self.value = None;
        if touches.len() > 0 {
            touches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            self.mid = touches[0].0;

            self.value = Some(1.0 - touches[0].1);
        }
    }

    pub fn render(&self, canvas: &mut Canvas<Window>, is_best: bool) -> Result<(), String> {
        if !is_best {
            return Ok(());
        }
        canvas.set_draw_color(Color::RGB(32, 232, 32));
        canvas.draw_fline(self.start, self.mid)?;

        let val = self.value.unwrap_or(0.0);

        if val > 0.1 && val < 0.99 {
            canvas.set_draw_color(Color::RGB(255, 32, 64));
            canvas.draw_fline(self.mid, self.end)?;
        };
		Ok(())
    }
}
