pub fn lerpf32(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

pub fn lerpf64(a: f64, b: f64, t: f64) -> f64 {
	a + (b - a) * t
}