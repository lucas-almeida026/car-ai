use rand::Rng;
use sdl2::{
    pixels::PixelFormatEnum,
    render::{Texture, TextureCreator},
    surface::Surface,
    video::WindowContext,
};

pub fn from_file<'a>(
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

pub struct SizedTexture<'a> {
    pub texture: Texture<'a>,
    pub width: u32,
    pub height: u32,
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

pub struct TexturePool<'a> {
	pub pool: Vec<SizedTexture<'a>>,
	pub size: u32,
}
impl<'a> TexturePool<'a> {
	pub fn new(size: u32, tc: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
		let mut pool = Vec::with_capacity(size as usize);
		for _ in 0..size {
			let texture = from_file("assets/car.png", &tc)?;
			pool.push(texture);
		}
		Ok(Self {
			pool,
			size,
		})
	}

	pub fn get<'b>(&'a self) -> &'b SizedTexture<'a> {
		let idx = rand::thread_rng().gen_range(0..self.size);
		&self.pool[idx as usize]
	}
}