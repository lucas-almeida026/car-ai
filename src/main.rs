use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("AI Car", 1080, 800)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut car = Car::try_new("assets/car.png", &texture_creator)?;

    car.src_crop_center(200, 380);
    car.set_scale(0.5);

    let mut event_pump = sdl_context.event_pump()?;
    let target_frame_time = Duration::from_millis(1000 / 60);
    'running: loop {
        let frame_start = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    if car.velocity < car.max_velocity / 2.0 {
						car.acceleration = 0.33;
					} else {
						car.acceleration = 0.5;
					}
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    car.acceleration = 0.0;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
					if car.velocity < car.max_velocity / 2.0 {
						car.acceleration = -0.45;
					} else {
						car.acceleration = -0.33;
					}
                }
				Event::KeyUp {
					keycode: Some(Keycode::Down),
					..
				} => {
					car.acceleration = 0.0;
				}
                _ => {}
            }
        }
		car.update_position();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        car.render(&mut canvas)?;

        canvas.present();

        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
    }

    Ok(())
}

pub struct Car<'a> {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    velocity: f32,
    max_velocity: f32,
    acceleration: f32,
    texture: Texture<'a>,
    scale: f32,
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
            x: 420,
            y: 500,
            velocity: 0.0,
            max_velocity: 10.0,
            acceleration: 0.0,
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
        let dst_rect = Rect::new(self.x, self.y, scaled_w, scaled_h);
        canvas.copy(&self.texture, self.src_rect, dst_rect)
    }

    pub fn update_position(&mut self) {
        self.velocity += self.acceleration;

        if self.velocity > self.max_velocity {
            self.velocity = self.max_velocity;
        } else if self.velocity < -self.max_velocity {
            self.velocity = -self.max_velocity;
		} else if self.acceleration == 0.0 {
			if self.velocity > -1.9 && self.velocity < 1.9 {
				self.velocity = 0.0;
			} else {
				self.velocity *= 0.991;
			}
		}

        self.y -= (self.velocity * 0.5) as i32;

        // if self.y < 0 {

        // }
    }
}
