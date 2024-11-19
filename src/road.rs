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
			left: (width as f32) / 2.0,
			right: (width as f32) / 2.0,
			top: -INFINITY,
			bottom: INFINITY,
		}
	}

	pub fn render(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
		canvas.set_draw_color(Color::RGB(255, 255, 255));
		// canvas.draw_line(
		// 	Point::new(self.x, self.top),
		// 	Point::new(self.x + self.width, self.top),
		// ).map_err(|e| e.to_string())?;
		// canvas.draw_line(
		// 	Point::new(self.x, self.bottom),
		// 	Point::new(self.x + self.width, self.bottom),
		// ).map_err(|e| e.to_string())?;
		let (left, right) = (
			self.x - (self.width / 2) as i32,
			self.x + (self.width / 2) as i32,
		);
		canvas.fill_rect(Rect::new(left, -INFINITY / 2, 5, INFINITY as u32)).map_err(|e| e.to_string())?;
		canvas.fill_rect(Rect::new(right, -INFINITY / 2, 5, INFINITY as u32)).map_err(|e| e.to_string())?;
		Ok(())
	}
}