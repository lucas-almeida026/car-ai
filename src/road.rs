use crate::fns;

use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::{FPoint, Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;

const INFINITY: i32 = 1000000;
pub struct Road {
    x: i32,
    width: i32,
    pub lanes: i32,
    left: f32,
    right: f32,
    top: i32,
    bottom: i32,
    pub borders: Vec<Border>,
}

impl Road {
    pub fn new(x: i32, width: i32, lanes: i32) -> Self {
        let left = (x - width / 2) as f32;
        let right = (x + width / 2) as f32;
        Self {
            x,
            width,
            lanes,
            left,
            right,
            top: -INFINITY,
            bottom: INFINITY,
            borders: vec![
                Border::new(
                    Point::new(left as i32, -INFINITY / 2),
                    Point::new(left as i32, INFINITY / 2),
                ),
                Border::new(
                    Point::new(right as i32, -INFINITY / 2),
                    Point::new(right as i32, INFINITY / 2),
                ),
            ],
        }
    }

    pub fn render(&self, canvas: &mut Canvas<Window>, offset: f32) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        for border in self.borders.iter() {
            let rect = Rect::new(
                border.start.x,
                border.start.y - offset as i32,
                5,
                ((border.end.y - border.start.y) - offset as i32) as u32,
            );
            canvas.fill_rect(rect).map_err(|e| e.to_string())?;
        }

        for i in 1..=self.lanes - 1 {
            let x = fns::lerpf32(self.left, self.right, i as f32 / self.lanes as f32);
            let dashes = Road::dashed_line_vertical(
                x as i32,
                self.top / 2 - (offset as i32),
                4,
                self.bottom as u32,
                30,
                60,
            );
            for dash in dashes {
                canvas.fill_rect(dash).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    // pub fn render_with_offset(&self, canvas: &mut Canvas<Window>, offset: f32) -> Result<(), String> {

    // }

    fn dashed_line_vertical(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        length: u32,
        gap: u32,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();
        let size = length + gap;
        for i in 0..height / size {
            rects.push(Rect::new(x, y + i as i32 * size as i32, width, length))
        }
        rects
    }

    pub fn lane_center(&self, lane: u32) -> Option<f32> {
        let line_width = self.width / self.lanes;
        if (lane as i32) < self.lanes {
            Some(self.left + line_width as f32 / 2.0 + lane as f32 * line_width as f32)
        } else {
            None
        }
    }

	pub fn random_lane_idx(&self) -> u32 {
		rand::thread_rng().gen_range(0..(self.lanes as u32))
	}
}

pub struct Border {
    pub start: Point,
    pub end: Point,
}
impl Border {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }
}