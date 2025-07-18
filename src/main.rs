use network::NeuralNetwork;
use rand::Rng;
use rayon::{prelude::*, ThreadPoolBuilder};
use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};
use std::time::{Duration, Instant};

mod car;
mod fns;
mod network;
mod road;
mod sensor;
mod texture;
mod units;

use car::{Car, ControlledCar};
use road::Road;
use texture::{SizedTexture, TexturePool};

fn main() -> Result<(), String> {
    ThreadPoolBuilder::new()
        .num_threads(16)
        .build_global()
        .unwrap();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let w_width = 1080;
    let w_height = 800;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let window = video_subsystem
        .window("AI Car", w_width, w_height)
        .position(100, 100)
        .build()
        .map_err(|e| e.to_string())?;

    let use_controlled_car = false;
    let amount_cars = 200;
    let traffic_size = 4;
    let traffic_min_velocity = 27.33; // ~98 km/h
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let focused_texture = car::create_main_texture(&texture_creator)?;
    let unfocused_texture = car::create_unfocused_texture(&texture_creator)?;
    let damaged_texture = car::create_damaged_texture(&texture_creator)?;
    let texture_pool = car::create_traffic_texture_pool(&texture_creator, traffic_size)?;

    let road = Road::new((w_width / 2) as i32, (w_width as f32 * 0.3) as i32, 3);
    let mut car = Car::new(
        1,
        focused_texture.width,
        unfocused_texture.height,
        None,
        0.0,
    );

    car.src_crop_center(194, 380, 0.3);
    car.set_in_lane(&road, 1)?;
    let mut controlled_car = ControlledCar::new(car);

    let mut ai_cars = generate_ai_cars(
        amount_cars,
        &road,
        NeuralNetwork::load_from_file("./brains/best.json").ok(),
        NeuralNetwork::load_from_file("./brains/sec_best.json").ok(),
        &focused_texture,
    );
    let mut min_y_idx: usize = 0;
    let mut max_score_idx: usize = 1;
    let mut best_brain = ai_cars.get(min_y_idx).and_then(|c| c.brain.clone());
    let mut sec_best_brain = ai_cars.get(max_score_idx).and_then(|c| c.brain.clone());
    // let mut cars_alive = ai_cars.len() as i32;

    let mut textures = Vec::new();
    let mut traffic = generate_traffic(
        traffic_size,
        traffic_min_velocity,
        w_height as i32,
        &road,
        &texture_pool,
        &mut textures,
    );

    let mut event_pump = sdl_context.event_pump()?;
    let target_fps = 60;
    let target_frame_time = Duration::from_millis(1000 / target_fps);

    let mut previous_time = Instant::now();
    let mut current_second = 0.0;
    let mut frame_count = 0;
    let font = ttf_context.load_font("./assets/fonts/RedHatDisplay-Regular.ttf", 28)?;

    'running: loop {
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(previous_time);
        previous_time = current_time;
        let delta_t_s = delta_time.as_secs_f32();
        // println!("delta_t_s: {}", delta_t_s);
        current_second += delta_t_s;
        if current_second >= 1.0 {
            current_second = 0.0;
            // println!("FPS: {}, car_alive: {}", frame_count, cars_alive);
            frame_count = 0;
        }

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
                    keycode: Some(Keycode::X),
                    ..
                } => {
                    if !use_controlled_car {
                        let focused_car = &mut ai_cars[min_y_idx];
                        focused_car.damaged = true;
                        focused_car.score = -1000;
                        // cars_alive -= 1;
                    }
                }
                _ => {}
            }
            if use_controlled_car {
                controlled_car.process_event(&event);
            }
        }

        let max_score = ai_cars
            .iter()
            .filter(|&c| !c.damaged)
            .map(|c| c.score)
            .max()
            .unwrap_or(0);
        let min_y = ai_cars
            .iter()
            .filter(|&c| !c.damaged)
            .map(|c| c.position.y)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        let n_min_y_idx = ai_cars
            .iter()
            .position(|c| c.position.y == min_y)
            .unwrap_or(0);
        let n_max_score_idx = ai_cars
            .iter()
            .position(|c| c.score == max_score)
            .unwrap_or(0);

        if ai_cars.get(n_min_y_idx).map(|c| c.position.y)
            < ai_cars.get(min_y_idx).map(|c| c.position.y)
        {
            min_y_idx = n_min_y_idx;
            // println!("changed min_y: {min_y_idx}");
            // println!("score: {}", ai_cars[min_y_idx].score);
            best_brain = ai_cars[min_y_idx].brain.clone();
        }
        if ai_cars.get(n_max_score_idx).map(|c| c.position.y)
            < ai_cars.get(max_score_idx).map(|c| c.position.y)
            && ai_cars[n_max_score_idx].score > ai_cars[max_score_idx].score
        {
            max_score_idx = n_max_score_idx;
            // println!("changed max_score: {max_score_idx}");
            // println!("score: {}", ai_cars[max_score_idx].score);
            sec_best_brain = ai_cars[n_max_score_idx].brain.clone();
        }

        canvas.set_draw_color(Color::RGB(12, 12, 16));
        canvas.clear();

        let camera_y_offset = if use_controlled_car {
            controlled_car.screen_offset(w_height as f32 * 0.7)
        } else {
            ai_cars
                .get(min_y_idx)
                .map(|c| c.position.y - (w_height as f32 * 0.7))
                .unwrap_or(traffic[0].position.y - (w_height as f32 * 0.7))
            // controlled_car.screen_offset(w_height as f32 * 0.7)
        };

        road.render(&mut canvas, camera_y_offset)?;

        for (i, car) in &mut traffic.iter_mut().enumerate() {
            let st = textures.get(i).unwrap();
            car.render(
                &mut canvas,
                camera_y_offset,
                false,
                &st.texture,
                &st.texture,
                &damaged_texture.texture,
            )?;
            car.update(delta_t_s, camera_y_offset, &road, &vec![]);
            let passed = car.is_passed_bottom_bound(w_height as i32, camera_y_offset);
            if passed {
                reset_passed_car(
                    car,
                    traffic_min_velocity,
                    w_width as f32,
                    w_height as f32,
                    &road,
                );
            }
        }
        // let best_car_y = ai_cars.get(best_car_index).unwrap().position.y;
        for (i, car) in ai_cars.iter_mut().enumerate() {
            let is_best = i == min_y_idx || i == max_score_idx;
            car.render(
                &mut canvas,
                camera_y_offset,
                is_best,
                &focused_texture.texture,
                &unfocused_texture.texture,
                &damaged_texture.texture,
            )?;
        }

        // update_cars_parallel(&mut ai_cars, 8);
        ai_cars.par_iter_mut().for_each(|car| {
            car.update(delta_t_s, camera_y_offset, &road, &traffic);
        });

        for car in ai_cars.iter_mut() {
            let over_bottom_bound = car.is_passed_bottom_bound(w_height as i32, camera_y_offset);
            if car.did_just_crashed || over_bottom_bound {
                let rand = rand::thread_rng().gen_range(0.0..1.0);
                let ref_brain = if rand < 0.5 {
                    best_brain.as_ref()
                } else {
                    sec_best_brain.as_ref()
                };
                car.reset(min_y as f32 + w_height as f32 * 0.22, &road, ref_brain);
            }
        }
        if use_controlled_car {
            controlled_car.render(
                &mut canvas,
                camera_y_offset,
                true,
                &focused_texture.texture,
                &unfocused_texture.texture,
                &damaged_texture.texture,
            )?;
            controlled_car.update(delta_t_s, camera_y_offset, &road, &vec![], &mut 1);
        }

		let txt_content = format!("#1 score = {}", ai_cars[min_y_idx].score);
        let txt_surface = font
            .render(&txt_content)
            .blended(Color::RGBA(255, 0, 0, 255))
            .map_err(|e| e.to_string())?;
        let txt_texture = texture_creator
            .create_texture_from_surface(&txt_surface)
            .map_err(|e| e.to_string())?;

        let (txt_width, txt_height) = txt_surface.size();
        let txt_target = Rect::new(64, 64, txt_width, txt_height);

        canvas.copy(&txt_texture, None, Some(txt_target))?;

		let txt_content2 = format!("#2 score = {}", ai_cars[max_score_idx].score);
        let txt_surface2 = font
            .render(&txt_content2)
            .blended(Color::RGBA(255, 0, 0, 255))
            .map_err(|e| e.to_string())?;
        let txt_texture2 = texture_creator
            .create_texture_from_surface(&txt_surface2)
            .map_err(|e| e.to_string())?;

        let (txt_width, txt_height) = txt_surface2.size();
        let txt_target = Rect::new(64, 64 + txt_height as i32 + 12, txt_width, txt_height);

        canvas.copy(&txt_texture2, None, Some(txt_target))?;

        canvas.present();
        frame_count += 1;

        let frame_duration = current_time.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
    }
    best_brain.map(|b| {
        b.save_as_file("brains/best.json")
            .expect("failed to save network");
    });
    sec_best_brain.map(|b| {
        b.save_as_file("brains/second_best.json")
            .expect("failed to save network");
    });
    println!("out of loop: saved networks");

    Ok(())
}

fn generate_ai_cars<'a>(
    amount: u32,
    road: &'a Road,
    ref_brain: Option<NeuralNetwork>,
    ref_brain2: Option<NeuralNetwork>,
    fc: &'a SizedTexture,
) -> Vec<Car> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;

    for i in 0..amount {
        let brain = if i % 2 == 0 {
            ref_brain.as_ref()
        } else {
            ref_brain2.as_ref()
        };
        let t = if i % 5 == 0 { 0.33 } else { 0.92 };
        let lane_idx = road.random_lane_idx();
        car = Car::new(lane_idx, fc.width, fc.height, brain, t);
        car.src_crop_center(194, 380, 0.3);
        let _ = car.set_in_lane(&road, lane_idx);
        cars.push(car);
    }
    cars
}

fn generate_traffic<'a>(
    amount: u32,
    min_velocity: f32,
    h: i32,
    road: &'a Road,
    pool: &'a TexturePool,
    textures: &mut Vec<&'a SizedTexture<'a>>,
) -> Vec<Car> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;
    for _ in 0..amount {
        let fc = pool.get();
        let lane_idx = road.random_lane_idx();
        car = Car::new(lane_idx, fc.width, fc.height, None, 0.0);
        let max_velocity = rand::thread_rng().gen_range(min_velocity..min_velocity + 4.0); // 4.0 m/s ≃ 15 km/h
                                                                                           // let start_y = rand::thread_rng().gen_range((h as f32 * 0.5)..(h as f32 * 1.5));
        let y_step = rand::thread_rng().gen_range(1..6);
        let start_y = h as f32 + y_step as f32 * 3.0;
        car.src_crop_center(194, 380, 0.3);
        car.position.y -= start_y;
        let _ = car.set_in_lane(&road, lane_idx);
        car.as_dummy(max_velocity);

        cars.push(car);
        textures.push(fc);
    }
    cars
}

fn reset_passed_car<'a>(car: &'a mut Car, min_velocity: f32, w: f32, h: f32, road: &'a Road) {
    let lane_idx = road.random_lane_idx();
    let max_velocity = rand::thread_rng().gen_range(min_velocity..min_velocity + 4.0);
    let y_step = rand::thread_rng().gen_range(1..6);
    let start_y = h as f32 + (y_step as f32 * (h as f32 * 0.15));
    car.position.y -= start_y;
    let _ = car.set_in_lane(&road, lane_idx);
    car.as_dummy(max_velocity);
}

macro_rules! vec4_4096 {
	($($x:expr),*) => {{
		let elements = vec![$($x),*];
		if elements.len() != 4 {
			panic!("vec4_4096! macro requires exactly 4 elements.");
		}
		elements.into_iter().flat_map(|e| vec![e; 1024]).collect::<Vec<_>>()
	}};
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::future;
    use network::*;
    use std::time::Instant;
    use tokio::time;
    use wgpu::Instance;

    #[test]
    fn feed_forward_cpu() {
        let start_time = Instant::now();
        let neuron_count = &[4096, 4096, 4096, 4096, 4096];
        let mut net = NeuralNetwork::new(neuron_count);
        for level in net.levels.iter_mut() {
            level.biases = vec4_4096![0.3, -0.1, 0.7, 0.1];
            level.weights = vec4_4096![
                vec4_4096![0.3, -0.1, 0.7, 0.4],
                vec4_4096![0.4, -0.2, 0.6, 0.3],
                vec4_4096![0.5, -0.3, 0.5, 0.2],
                vec4_4096![0.6, -0.1, 0.4, 0.1]
            ]
        }
        let input = &vec4_4096![0.11, -0.7, 0.5, 0.4];
        let output = net.feed_forward(input);
        let duration = start_time.elapsed();
        println!("Time CPU: {} ms", duration.as_millis());
        assert_eq!(output.len(), 4096);
    }

    #[tokio::test]
    async fn feed_forward_gpu() {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to find a GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Failed to create device");

        let mut gpu_handler_factory = GpuHandlerFactory::new(&device, &queue);

        let start_time = Instant::now();
        let neuron_count = &[4096, 4096, 4096, 4096, 4096];
        let mut net = NeuralNetwork::new(neuron_count);
        for level in net.levels.iter_mut() {
            level.biases = vec4_4096![0.3, -0.1, 0.7, 0.1];
            level.weights = vec4_4096![
                vec4_4096![0.3, -0.1, 0.7, 0.4],
                vec4_4096![0.4, -0.2, 0.6, 0.3],
                vec4_4096![0.5, -0.3, 0.5, 0.2],
                vec4_4096![0.6, -0.1, 0.4, 0.1]
            ]
        }
        let input = &vec4_4096![0.11, -0.7, 0.5, 0.4];
        let output = net.gpu_feed_forward(input, &mut gpu_handler_factory).await;
        let duration = start_time.elapsed();
        println!("Time GPU: {} ms", duration.as_millis());
        assert_eq!(output.len(), 4096);
    }
}
