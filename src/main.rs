use image::{ImageBuffer, Rgba};
use network::NeuralNetwork;
use rand::Rng;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{BlendMode, Texture};
use sdl2::surface::Surface;
use sdl2::video::WindowContext;
use sdl2::{event::Event, render::TextureCreator};
use std::time::{Duration, Instant};

mod car;
mod fns;
mod network;
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

    let amount_cars = 200;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let mut car_texture = load_texture("assets/car.png", &texture_creator)?;
    car_texture.texture.set_blend_mode(BlendMode::Blend);
    car_texture.texture.set_alpha_mod(128);

    let mut traffic_car_texture = load_texture("assets/car.png", &texture_creator)?;
    traffic_car_texture.texture.set_blend_mode(BlendMode::Blend);
    traffic_car_texture.texture.set_alpha_mod(128);
    traffic_car_texture.texture.set_color_mod(16, 255, 64);

    let road = Road::new((w_width / 2) as i32, (w_width as f32 * 0.33) as i32, 3);
    // let mut car = Car::new(&car_texture.texture, car_texture.width, car_texture.height, None, 0.0)?;

    // car.src_crop_center(194, 380);
    // car.set_scale(0.3);
    // car.x = road
    //     .lane_center(1)
    //     .map(|x| x - (car.scaled_width() / 2.0))
    //     .or(Some(w_width as f32 / 2.0 - (car.scaled_width() / 2.0)))
    //     .unwrap();
    // car.brain = None;

    let mut ai_cars = generate_ai_cars(amount_cars, w_width as f32, &road, &car_texture);
    let mut best_car_index: usize;
    let mut cars_alive = ai_cars.len() as i32;

    let mut traffic = generate_traffic(
        5,
        w_width as f32,
        w_height as i32,
        &road,
        &traffic_car_texture,
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
                Event::MouseButtonDown { x, y, .. } => {
                    println!("x: {}, y: {}", x, y);
                }
                _ => {}
            }
            // car.update_state(&event);
        }
        // car.update_position();
        for car in ai_cars.iter_mut() {
            car.update_position();
        }

        let max_score = ai_cars
            .iter()
            .map(|c| c.score)
            .max()
            .unwrap_or(0);
        best_car_index = ai_cars.iter().position(|c| c.score == max_score).unwrap();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        let camera_y_offset = ai_cars[best_car_index].y - (w_height as f32 * 0.7);
        // let camera_y_offset = car.y - (w_height as f32 * 0.7);

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
        // let best_car_y = ai_cars.get(best_car_index).unwrap().y;
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
            // if car.is_passed_bottom_bound(w_height as i32, camera_y_offset) {
            //     if !car.damaged {
            //         car.damaged = true;
            //         cars_alive -= 1;
            //     }
            // }
            // println!("past bounds: {}", car.is_passed_bottom_bound(w_height as i32, camera_y_offset))
        }
        if cars_alive < 4 {
            let best = &ai_cars[best_car_index];
			println!("top score: {}", best.score);
            if best.brain.is_some() {
                let net = best.brain.as_ref().unwrap();
                net.save_as_file("networks/best.json")
                    .expect("failed to save network");
                ai_cars = generate_ai_cars(amount_cars, w_width as f32, &road, &car_texture);
                cars_alive = ai_cars.len() as i32;
                traffic = generate_traffic(
                    5,
                    w_width as f32,
                    w_height as i32,
                    &road,
                    &traffic_car_texture,
                );
            }
        }
        // car.render(&mut canvas, camera_y_offset, &road.borders, &traffic, true, &mut 0)?;

        canvas.present();

        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }

        // println!("cars alive: {}", cars_alive);
    }

    Ok(())
}

fn generate_ai_cars<'a>(amount: u32, w: f32, road: &'a Road, st: &'a SizedTexture) -> Vec<Car<'a>> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;

    let mut ref_brain: Option<NeuralNetwork> = None;
    let net = NeuralNetwork::load_from_file("networks/best.json");
    if let Ok(net) = net {
        ref_brain = Some(net);
    }

    for i in 0..amount {
		let t = if i < 30 {
			0.666
		} else {
			0.98
		};
        car = Car::new(&st.texture, st.width, st.height, ref_brain.as_ref(), t);
        if car.is_err() {
            continue;
        }
        let mut car = car.unwrap();
        car.src_crop_center(194, 380);
        car.set_scale(0.3);
        car.x = road
            .lane_center(1)
            .map(|x| x - (car.scaled_width() / 2.0))
            .or(Some(w / 2.0 - (car.scaled_width() / 2.0)))
            .unwrap();

        cars.push(car);
    }
    cars
}

fn generate_traffic<'a>(
    amount: u32,
    w: f32,
    h: i32,
    road: &'a Road,
    st: &'a SizedTexture,
) -> Vec<Car<'a>> {
    let mut cars = Vec::with_capacity(amount as usize);
    let mut car;
    for i in 0..amount {
        car = Car::new(&st.texture, st.width, st.height, None, 0.0);
        if car.is_err() {
            continue;
        }
        let max_velocity = rand::thread_rng().gen_range(6.0..8.5);
        let start_y = rand::thread_rng().gen_range((h as f32 * 0.75)..(h as f32 * 2.0));
        let lane = i as i32 % road.lanes;

        let mut car = car.unwrap();
        car.src_crop_center(194, 380);
        car.set_scale(0.3);
        car.y = -start_y as f32;
        car.x = road
            .lane_center(lane as u32)
            .map(|x| x - (car.scaled_width() / 2.0))
            .or(Some(w / 2.0 - (car.scaled_width() / 2.0)))
            .unwrap();
        // car.random_filter();
        car.as_dummy(max_velocity);

        cars.push(car);
    }
    cars
}

fn reset_passed_car<'a>(car: &'a mut Car, w: f32, h: f32, road: &'a Road) {
    let max_velocity = rand::thread_rng().gen_range(6.0..8.5);
    let jump_y = rand::thread_rng().gen_range((h * 1.5)..(h * 3.5));
    let lane = rand::thread_rng().gen_range(0..road.lanes);

    car.y -= jump_y as f32;
    car.x = road
        .lane_center(lane as u32)
        .map(|x| x - (car.scaled_width() / 2.0))
        .or(Some(w / 2.0 - (car.scaled_width() / 2.0)))
        .unwrap();
    car.as_dummy(max_velocity);
}

fn load_texture<'a>(
    path: &str,
    tc: &'a TextureCreator<WindowContext>,
) -> Result<SizedTexture<'a>, String> {
    let mut img_buf = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let (width, height) = (img_buf.width(), img_buf.height());

    let surface = Surface::from_data(
        &mut img_buf,
        width,
        height,
        width * 4,
        PixelFormatEnum::ABGR8888,
    )
    .map_err(|e| e.to_string())?;

    let texture = tc
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;
    Ok(SizedTexture::new(texture, width, height))
}

struct SizedTexture<'a> {
    texture: Texture<'a>,
    width: u32,
    height: u32,
}
impl<'a> SizedTexture<'a> {
    fn new(texture: Texture<'a>, width: u32, height: u32) -> SizedTexture<'a> {
        SizedTexture {
            texture,
            width,
            height,
        }
    }
}
