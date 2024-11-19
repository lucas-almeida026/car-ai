use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{FRect, Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

pub struct Car<'a> {
    width: u32,
    height: u32,
    x: f32,
    y: f32,
	angle: f32,
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
            src_rect: None,
            scale: 1.0,
            x: 420.0,
            y: 500.0,
			angle: 0.0,
            velocity: 0.0,
            max_velocity: 10.0,
            acceleration: 0.4,
			friction: 0.08,
			forward: false,
			backward: false,
			left: false,
			right: false,
            width,
            height,
            texture,
        })
    }

    pub fn src_crop_center(&mut self, width: u32, height: u32) {
        self.src_rect = Some(Rect::new(
            ((self.width as i32 - width as i32) / 2).max(0),
            ((self.height as i32 - height as i32) / 2).max(0),
            width.min(self.width),
            height.min(self.height),
        ));
    }

    pub fn set_scale(&mut self, scale: f32) {
        if scale > 0.0 && scale < 1.0 {
            self.scale = scale;
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let w = self.src_rect.map(|r| r.width()).unwrap_or(self.width);
        let h = self.src_rect.map(|r| r.height()).unwrap_or(self.height);
        let scaled_w = (w as f32 * self.scale) as u32;
        let scaled_h = (h as f32 * self.scale) as u32;
        let dst_rect = FRect::new(self.x as f32, self.y as f32, scaled_w, scaled_h);

		let center = Point::new((scaled_w / 2) as i32, (scaled_h / 2) as i32);

        canvas.copy_ex(&self.texture, self.src_rect, Some(dst_rect), self.angle as f64, Some(center), false, false)
		// canvas.copy(&self.texture, self.src_rect, dst_rect)
    }

    pub fn update_position(&mut self) {
		if self.forward {
			if self.velocity < self.max_velocity / 2.0 {
				self.velocity += self.acceleration / 1.6 ;
			} else {
				self.velocity += self.acceleration;
			}
		}
		if self.backward {
			if self.velocity < self.max_velocity / 2.0 {
				self.velocity -= self.acceleration / 1.6;
			} else {
				self.velocity -= self.acceleration;
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

		self.x += (self.angle.to_radians().sin() * self.velocity) as i32;
		self.y -= (self.angle.to_radians().cos() * self.velocity) as i32;
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
}
