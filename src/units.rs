//UNIT REFERENCE: 109 pixels = 4.12 meters (based on size of car)
//there for 1 pixel = 37.7981 millimeters

const MM_PER_PIXEL: f64 = 37.7981;

pub enum UnitType {
	Pixels,
	Meters,
	Millimeters,
	Centimeters,
}

pub struct Unit {
	pub value: f64,
	pub unit: UnitType
}

impl Unit {
	pub fn px(value: f64) -> Self {
		Self {
			value,
			unit: UnitType::Pixels
		}
	}

	pub fn mm(value: f64) -> Self {
		Self {
			value,
			unit: UnitType::Millimeters
		}
	}

	pub fn cm(value: f64) -> Self {
		Self {
			value,
			unit: UnitType::Centimeters
		}
	}

	pub fn m(value: f64) -> Self {
		Self {
			value,
			unit: UnitType::Meters
		}
	}

	pub fn as_px(&self) -> f64 {
		match self.unit {
			UnitType::Pixels => self.value,
			UnitType::Millimeters => self.value / MM_PER_PIXEL,
			UnitType::Centimeters => self.value / (MM_PER_PIXEL / 10.0),
			UnitType::Meters => self.value / (MM_PER_PIXEL / 1000.0)
		}
	}
	pub fn as_mm(&self) -> f64 {
		match self.unit {
			UnitType::Pixels => self.value * MM_PER_PIXEL,
			UnitType::Millimeters => self.value,
			UnitType::Centimeters => self.value * 10.0,
			UnitType::Meters => self.value * 1000.0
		}
	}
	pub fn as_cm(&self) -> f64 {
		match self.unit {
			UnitType::Centimeters => self.value,
			UnitType::Millimeters => self.value / 10.0,
			UnitType::Meters => self.value * 100.0,
			UnitType::Pixels => self.value * MM_PER_PIXEL / 10.0
		}
	}
	pub fn as_m(&self) -> f64 {
		match self.unit {
			UnitType::Meters => self.value,
			UnitType::Millimeters => self.value / 1000.0,
			UnitType::Centimeters => self.value / 100.0,
			UnitType::Pixels => self.value * MM_PER_PIXEL / 1000.0
		}
	}
}

pub fn m_to_px(m: f32) -> f32 {
	m / (MM_PER_PIXEL as f32 / 1000.0)
}

pub fn px_to_m(px: f32) -> f32 {
	px * (MM_PER_PIXEL as f32 / 1000.0)
}