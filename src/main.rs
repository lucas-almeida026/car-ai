use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::{Duration, Instant};

mod car;
mod road;
use car::Car;
use road::Road;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
	let w_width = 1080;
	let w_height = 800;
    let window = video_subsystem
        .window("AI Car", w_width, w_height)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut car = Car::try_new("assets/car.png", &texture_creator)?;
	let road = Road::new((w_width / 2) as i32, (w_width as f32 * 0.75) as i32, 3);

    car.src_crop_center(200, 380);
    car.set_scale(0.35);

    let mut event_pump = sdl_context.event_pump()?;
	let target_fps = 60;
    let target_frame_time = Duration::from_millis(1000 / target_fps);
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
                _ => {}
            }
			car.update_state(&event);
        }
		car.update_position();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        car.render(&mut canvas)?;
		road.render(&mut canvas)?;

        canvas.present();

        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
    }

    Ok(())
}
