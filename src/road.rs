use crate::fns;

use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;

const INFINITY: i32 = 1000000;
pub struct Road {
	x: i32,
	width: i32,
	lanes: i32,
	left: f32,
	right: f32,
	top: i32,
	bottom: i32,
}

impl Road {
	pub fn new(x: i32, width: i32, lanes: i32) -> Self {
		Self {
			x,
			width,
			lanes,
			left: (x - width / 2) as f32,
			right: (x + width / 2) as f32,
			top: -INFINITY,
			bottom: INFINITY,
		}
	}

	pub fn render(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
		canvas.set_draw_color(Color::RGB(255, 255, 255));
		let (left, right) = (
			self.x - (self.width / 2) as i32,
			self.x + (self.width / 2) as i32,
		);
		canvas.fill_rect(Rect::new(left, -INFINITY / 2, 5, INFINITY as u32)).map_err(|e| e.to_string())?;
		canvas.fill_rect(Rect::new(right, -INFINITY / 2, 5, INFINITY as u32)).map_err(|e| e.to_string())?;

		for i in 0..self.lanes {
			let x = fns::lerp(self.left, self.right, i as f32 / self.lanes as f32);
			let dashes = Road::dashed_line_vertical(x as i32, -INFINITY / 2, 5, INFINITY as u32, 40, 40);
			for dash in dashes {
				canvas.fill_rect(dash).map_err(|e| e.to_string())?;
			}
			// canvas.fill_rect(Rect::new(x as i32, -INFINITY / 2, 5, INFINITY as u32)).map_err(|e| e.to_string())?;
		}
		Ok(())
	}

	fn dashed_line_vertical(x: i32, y: i32, width: u32, height: u32, length: u32, gap: u32) -> Vec<Rect> {
		let mut rects = Vec::new();
		let size = length + gap;
		for i in 0..height / size {
			rects.push(
				Rect::new(
					x,
					y + i as i32 * size as i32,
					width,
					length,
				),
			)
		}
		rects
	}
}