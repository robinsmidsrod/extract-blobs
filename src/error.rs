use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    // -- Externals
    #[from]
    Utf8(std::str::Utf8Error),
    #[from]
    Io(std::io::Error),
    #[from]
    Image(image::error::ImageError),
    #[from]
    Png(png::EncodingError),
    #[from]
    LepPix(leptess::leptonica::PixError),
    #[from]
    TessInit(leptess::tesseract::TessInitError),
    #[from]
    TessVar(leptess::tesseract::TessSetVariableError),
}

impl std::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}
