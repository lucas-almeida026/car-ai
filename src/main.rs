use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::rect::Rect;
use sdl2::render::BlendMode;

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

    let mut car_image_buf = image::open("assets/car.png")
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let car_w = car_image_buf.width();
    let car_h = car_image_buf.height();

    let car_surface = Surface::from_data(
        &mut car_image_buf,
        car_w,
        car_h,
        car_w * 4,
        PixelFormatEnum::ABGR8888
    )
    .map_err(|e| e.to_string())?;

    let mut car_texture = texture_creator
        .create_texture_from_surface(&car_surface)
        .map_err(|e| e.to_string())?;

	car_texture.set_blend_mode(BlendMode::Blend);

	let twidth = 200;
	let theight = 380;

	let car_rect_src = Rect::new(
		((car_w as i32 - twidth) / 2).max(0),
		((car_h as i32 - theight) / 2).max(0),
		twidth.min(car_w as i32) as u32,
		theight.min(car_h as i32) as u32,
	);

	let car_rect_dst = Rect::new(0, 0, twidth as u32, theight as u32);

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
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
        }

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        canvas.copy(&car_texture, Some(car_rect_src), Some(car_rect_dst))?;

        canvas.present();
    }

    Ok(())
}

struct Car {
	
}