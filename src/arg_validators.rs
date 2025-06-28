use color::{AlphaColor, ParseError};
use image::Rgba;

pub(crate) fn validate_blur_edge_factor(value: &str) -> Result<f32, String> {
    let num = value
        .parse::<f32>()
        .map_err(|_| "Not a valid floating point number".to_string())?;
    if num <= 0.0 {
        return Err("Number must be greater than 0".to_string());
    }
    Ok(num)
}

pub(crate) fn validate_chroma_key_color(value: &str) -> Result<Rgba<u8>, String> {
    match parse_color(value) {
        Ok(color) => Ok(color),
        Err(e) => Err(e.to_string()),
    }
}

/// Parse a string into a color, with format like this #RRGGBB
fn parse_color(color: &str) -> Result<Rgba<u8>, ParseError> {
    let color = color::parse_color(color)?;
    let color: AlphaColor<color::Srgb> = color.to_alpha_color();
    let color = color.to_rgba8();
    let color = Rgba(color.to_u8_array());
    Ok(color)
}
