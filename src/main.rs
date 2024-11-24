use network::NeuralNetwork;
use rand::Rng;
use sdl2::{event::Event, keyboard::Keycode, pixels::Color};
use std::time::{Duration, Instant};

mod car;
mod fns;
mod network;
mod road;
mod sensor;
mod texture;
mod gpu;

use car::{Car, ControlledCar};
use road::Road;
use texture::{SizedTexture, TexturePool};
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

    let use_controlled_car = false;
    let amount_cars = 12;
    let traffic_size = 4;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let focused_texture = car::create_main_texture(&texture_creator)?;
    let unfocused_texture = car::create_unfocused_texture(&texture_creator)?;
    let damaged_texture = car::create_damaged_texture(&texture_creator)?;
    let texture_pool = car::create_traffic_texture_pool(&texture_creator, traffic_size)?;

    let road = Road::new((w_width / 2) as i32, (w_width as f32 * 0.33) as i32, 3);
    let mut car = Car::new(
        &focused_texture,
        &unfocused_texture,
        &damaged_texture,
        None,
        0.0,
    )?;

    car.src_crop_center(194, 380);
    car.set_scale(0.3)?;
    car.set_in_lane(&road, 1)?;
    let mut controlled_car = ControlledCar::new(car);

    let mut ai_cars = generate_ai_cars(
        amount_cars,
        &road,
        &focused_texture,
        &unfocused_texture,
        &damaged_texture,
    );
    let mut best_car_index: usize;
    let mut cars_alive = ai_cars.len() as i32;

    let mut traffic = generate_traffic(
        traffic_size,
        w_height as i32,
        &road,
        &texture_pool,
        &damaged_texture,
    );

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
            if use_controlled_car {
                controlled_car.process_event(&event);
            }
        }
        if use_controlled_car {
            controlled_car.update_position();
        } else {
            for car in ai_cars.iter_mut() {
                car.update_position();
            }
        }

        // let max_score = ai_cars.iter().filter(|&c| !c.damaged).map(|c| c.score).max().unwrap_or(0);
		let min_y = ai_cars.iter().filter(|&c| !c.damaged).map(|c| c.position.y).min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0.0);
        best_car_index = ai_cars.iter().position(|c| c.position.y == min_y).unwrap();

        canvas.set_draw_color(Color::RGB(12, 12, 16));
        canvas.clear();

        let camera_y_offset = if use_controlled_car {
            controlled_car.screen_offset(w_height as f32 * 0.7)
        } else {
            ai_cars[best_car_index].position.y - (w_height as f32 * 0.7)
        };

        road.render(&mut canvas, camera_y_offset)?;

        for car in &mut traffic.iter_mut() {
            car.update_position();
            car.render(
                &mut canvas,
                camera_y_offset,
                &road.borders,
                &vec![],
                false,
                &mut 1,
            )?;
            let passed = car.is_passed_bottom_bound(w_height as i32, camera_y_offset);
            if passed {
                reset_passed_car(car, w_width as f32, w_height as f32, &road);
            }
        }

        // let best_car_y = ai_cars.get(best_car_index).unwrap().position.y;
        for (i, car) in ai_cars.iter_mut().enumerate() {
            let is_best = i == best_car_index;
            car.render(
                &mut canvas,
                camera_y_offset,
                &road.borders,
                &traffic,
                is_best,
                &mut cars_alive,
            )?;
            if car.is_passed_bottom_bound(w_height as i32, camera_y_offset) {
                if !car.damaged {
                    car.damaged = true;
                    cars_alive -= 1;
                }
            }
            // println!("past bounds: {}", car.is_passed_bottom_bound(w_height as i32, camera_y_offset))
        }
        if cars_alive < 3 {
            let best = &ai_cars[best_car_index];
            println!("top score: {}", best.score);
            if best.brain.is_some() {
                let net = best.brain.as_ref().unwrap();
                net.save_as_file("networks/best.json")
                    .expect("failed to save network");
                ai_cars = generate_ai_cars(
                    amount_cars,
                    &road,
                    &focused_texture,
                    &unfocused_texture,
                    &damaged_texture,
                );
                cars_alive = ai_cars.len() as i32;
                traffic = generate_traffic(
                    traffic_size,
                    w_height as i32,
                    &road,
                    &texture_pool,
                    &damaged_texture,
                );
            }
        }
        if use_controlled_car {
            controlled_car.render(
                &mut canvas,
                camera_y_offset,
                &road.borders,
                &traffic,
                true,
                &mut 0,
            )?;
        }

        canvas.present();

        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }

        // println!("cars alive: {}", cars_alive);
    }

    Ok(())
}

fn generate_ai_cars<'a>(
    amount: u32,
    road: &'a Road,
    fc: &'a SizedTexture,
    uf: &'a SizedTexture,
    dm: &'a SizedTexture,
) -> Vec<Car<'a>> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;

    let mut ref_brain: Option<NeuralNetwork> = None;
    let net = NeuralNetwork::load_from_file("networks/best.json");
    if let Ok(net) = net {
        ref_brain = Some(net);
    }

    for i in 0..amount {
        let t = if i < 30 { 0.666 } else { 0.98 };
        car = Car::new(fc, uf, dm, ref_brain.as_ref(), t);
        if car.is_err() {
            continue;
        }
        let mut car = car.unwrap();
        car.src_crop_center(194, 380);
        let _ = car.set_scale(0.3);
		let lane_idx = road.random_lane_idx();
		let _ = car.set_in_lane(&road, lane_idx);
        cars.push(car);
    }
    cars
}

fn generate_traffic<'a>(
    amount: u32,
    h: i32,
    road: &'a Road,
    pool: &'a TexturePool,
    dm: &'a SizedTexture,
) -> Vec<Car<'a>> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;
    for _ in 0..amount {
        let fc = pool.get();
        car = Car::new(fc, fc, dm, None, 0.0);
        if car.is_err() {
            continue;
        }
        let max_velocity = rand::thread_rng().gen_range(6.0..8.5);
        let start_y = rand::thread_rng().gen_range((h as f32 * 0.5)..(h as f32 * 1.5));

        let mut car = car.unwrap();
        car.src_crop_center(194, 380);
        let _ = car.set_scale(0.3);
        car.position.y -= start_y as f32;
		let lane_idx = road.random_lane_idx();
		let _ = car.set_in_lane(&road, lane_idx);
        car.as_dummy(max_velocity);

        cars.push(car);
    }
    cars
}

fn reset_passed_car<'a>(car: &'a mut Car, w: f32, h: f32, road: &'a Road) {
    let max_velocity = rand::thread_rng().gen_range(6.0..8.5);
    let jump_y = rand::thread_rng().gen_range((h * 1.5)..(h * 3.5));
    let lane = rand::thread_rng().gen_range(0..road.lanes);

    car.position.y -= jump_y as f32;
    car.position.x = road
        .lane_center(lane as u32)
        .map(|x| x - (car.scaled_width() as f32 / 2.0))
        .or(Some(w / 2.0 - (car.scaled_width() as f32 / 2.0)))
        .unwrap();
    car.as_dummy(max_velocity);
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn feed_forward_cpu() {
		let neuron_count = &[2, 2, 2];
		let mut net = NeuralNetwork::new(neuron_count);
		for level in net.levels.iter_mut() {
			level.biases = vec![0.3, -0.1];
			level.weights = vec![
				vec![-0.1, 0.2],
				vec![0.3, -0.4],
			]
		}
		let output = net.feed_forward(&vec![0.1, 0.42]);
		let expected = [
			0.5814949520460782,
			0.47537390107617306,
		];
		assert_eq!(output, expected);
	}
}