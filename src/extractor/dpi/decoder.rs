use std::io::BufReader;
use std::io::Cursor;

use exif::In;
use exif::Tag;
use exif::Value;
use jfifdump::SegmentKind;

use super::Dpi;

/// Read pixel density from file metadata
pub fn read_from_bytes(file_contents: &[u8], exif: &[u8]) -> Option<Dpi> {
    // Use already decoded EXIF data if we have it, or default to using the entire file contents
    let exif = if exif.is_empty() { file_contents } else { exif };
    // Define functions that can decode DPI information, in priority order
    let funcs: Vec<Box<dyn FnOnce() -> Option<Dpi>>> = vec![
        Box::new(|| read_dpi_from_exif(exif)),
        Box::new(|| read_dpi_from_jfif(file_contents)),
        // TODO: Support reading PNG pixel density
    ];
    // Try to decode pixel density, take the first one that has something
    for func in funcs {
        let dpi = func();
        if dpi.is_some() {
            return dpi;
        }
    }
    None
}

/// Read pixel density from EXIF header
pub(crate) fn read_dpi_from_exif(exif_raw: &[u8]) -> Option<Dpi> {
    let reader = exif::Reader::new();
    let exif = reader.read_raw(exif_raw.to_vec()).ok()?;
    let unit = exif
        .get_field(Tag::ResolutionUnit, In::PRIMARY)
        .and_then(|unit| unit.value.get_uint(0))?;
    let x_res = match &exif.get_field(Tag::XResolution, In::PRIMARY)?.value {
        Value::Rational(vec) => vec.first().map(|value| value.to_f32() as u32),
        _ => return None,
    }?;
    let y_res = match &exif.get_field(Tag::YResolution, In::PRIMARY)?.value {
        Value::Rational(vec) => vec.first().map(|value| value.to_f32() as u32),
        _ => return None,
    }?;
    // https://www.media.mit.edu/pia/Research/deepview/exif.html#ExifTags
    match unit {
        // 1 means no-unit (aspect ratio)
        1 => None,
        // 2 means inch
        2 => Some(Dpi::from(x_res, y_res)),
        // 3 means centimeter
        3 => Some(Dpi::from_centimeter(x_res, y_res)),
        _ => None,
    }
}

/// Read pixel density from JPEG JFIF header
pub(crate) fn read_dpi_from_jfif(file_contents: &[u8]) -> Option<Dpi> {
    let c = Cursor::new(file_contents);
    let r = BufReader::new(c);
    let mut reader = jfifdump::Reader::new(r).ok()?;
    loop {
        match reader.next_segment().ok()?.kind {
            // https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#File_format_structure
            SegmentKind::Eoi => break,
            // https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format#JFIF_APP0_marker_segment
            SegmentKind::App0Jfif(jfif) => {
                match jfif.unit {
                    // unit=0 means pixel aspect ratio (y:x)
                    0 => return None,
                    // unit=1 means pixels per inch (2.54cm)
                    1 => {
                        return Some(Dpi::from(jfif.x_density, jfif.y_density));
                    }
                    // unit=2 means pixels per centimeter
                    2 => {
                        return Some(Dpi::from_centimeter(jfif.x_density, jfif.y_density));
                    }
                    _ => return None,
                }
            }
            _ => {}
        }
    }
    None
}
