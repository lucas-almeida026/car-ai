use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::{f32::consts::PI, time::{Duration, Instant}};

mod car;
mod fns;
mod road;
mod sensor;
use car::Car;
use road::Road;
// use sensor::Sensor;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let w_width = 1080;
    let w_height = 800;
    let window = video_subsystem
        .window("AI Car", w_width, w_height)
        .position(100, 100)
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let road = Road::new((w_width / 2) as i32, (w_width as f32 * 0.33) as i32, 3);
    let mut car = Car::try_new("assets/car.png", &texture_creator)?;

    car.src_crop_center(194, 380);
    car.set_scale(0.3);
    car.x = road
        .lane_center(1)
        .map(|x| x - (car.scaled_width() / 2.0))
        .or(Some(w_width as f32 / 2.0 - (car.scaled_width() / 2.0)))
        .unwrap();

	let mut traffic = vec![
		Car::try_new("assets/car.png", &texture_creator).unwrap(),
	];

	for car in &mut traffic {
		car.src_crop_center(194, 380);
		car.set_scale(0.3);
		car.y -= 100.0;
		car.x = road
			.lane_center(2)
			.map(|x| x - (car.scaled_width() / 2.0))
			.or(Some(w_width as f32 / 2.0 - (car.scaled_width() / 2.0)))
			.unwrap();
		car.random_filter();
		car.as_dummy(3.0);
	}

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
				Event::MouseButtonDown{x, y, ..} => {
					println!("x: {}, y: {}", x, y);
				}
                _ => {}
            }
            car.update_state(&event);
        }
        car.update_position();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        let camera_y_offset = car.y - (w_height as f32 * 0.7);

        road.render(&mut canvas, camera_y_offset)?;

		for car in &mut traffic {
			car.update_position();
			car.render(&mut canvas, camera_y_offset, &road.borders, &vec![])?;
		}
		car.render(&mut canvas, camera_y_offset, &road.borders, &traffic)?;

        canvas.present();

        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
    }

    Ok(())
}
