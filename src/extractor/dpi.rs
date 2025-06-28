/// Pixel density in inches
#[derive(Debug)]
pub struct Dpi {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

impl Dpi {
    /// Create instance from single value in inches
    pub fn new<T: Copy + Into<u32>>(v: T) -> Dpi {
        Dpi {
            x: v.into(),
            y: v.into(),
        }
    }

    /// Create instance from x and y values in inches
    pub fn from<T: Copy + Into<u32>>(x: T, y: T) -> Dpi {
        Dpi {
            x: x.into(),
            y: y.into(),
        }
    }

    /// Create instance from x and y values in meters
    pub fn from_centimeter<T: Copy + Into<u32>>(x: T, y: T) -> Dpi {
        Dpi {
            x: (x.into() as f32 * 2.54) as u32,
            y: (y.into() as f32 * 2.54) as u32,
        }
    }

    /// Horizontal resultion in meters
    pub fn x_in_meters(&self) -> u32 {
        // 1 inch = 39.37 cm
        (self.x as f32 * 39.37) as u32
    }
    /// Vertical resultion in meters
    pub fn y_in_meters(&self) -> u32 {
        // 1 inch = 39.37 cm
        (self.y as f32 * 39.37) as u32
    }
}
