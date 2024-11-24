use sdl2::rect::FPoint;

pub fn lerpf32(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

pub fn lerpf64(a: f64, b: f64, t: f64) -> f64 {
	a + (b - a) * t
}

pub fn get_intersectionf(
	start_a_x: f32, //A
	start_a_y: f32, //A
	end_a_x: f32, //B
	end_a_y: f32, //B
	start_b_x: f32, //C
	start_b_y: f32, //C
	end_b_x: f32, //D
	end_b_y: f32 //D
) -> Option<(FPoint, f32)> {
	let t_top = (end_b_x - start_b_x) * (start_a_y - start_b_y) - (end_b_y - start_b_y) * (start_a_x - start_b_x);
	let u_top = (end_a_x - start_a_x) * (start_a_y - start_b_y) - (end_a_y - start_a_y) * (start_a_x - start_b_x);
	let bottom = (end_b_y - start_b_y) * (end_a_x - start_a_x) - (end_b_x - start_b_x) * (end_a_y - start_a_y);

	if bottom != 0.0 {
		let t = t_top / bottom;
		let u = u_top / bottom;

		if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
			return Some((
				FPoint::new(lerpf32(start_a_x, end_a_x, t), lerpf32(start_a_y, end_a_y, t)),
				t,
			));
		}
	}
	None
}

pub fn sigmoid(f: f64) -> f64 {
	1.0 / (1.0 + (-f).exp())
}