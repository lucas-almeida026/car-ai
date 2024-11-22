use crate::fns::lerpf64;
use sdl2::pixels::Color;
use sdl2::rect::FPoint;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct Sensor {
    pub rays: Vec<Ray>,
}

impl Sensor {
    pub fn new(ray_count: u32, ray_length: f32, ray_spread: f64) -> Self {
        let mut rays = Vec::new();
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
            let ray = Ray::new(ray_length, ray_angle);
            rays.push(ray);
        }
        Self { rays }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f64,
    ) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 50, 16));
        for ray in self.rays.iter() {
            ray.render(canvas, x, y, w, h, angle)?;
        }
        Ok(())
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
    ) -> Result<(), String> {
        let center_x = x + w / 2.0;
        let center_y = y + h / 2.0;
        let start = FPoint::new(center_x, center_y);
        let end = FPoint::new(
            center_x - self.length * (self.angle - angle.to_radians()).sin() as f32,
            center_y - self.length * (self.angle - angle.to_radians()).cos() as f32,
        );
        canvas.draw_fline(start, end)
    }
}