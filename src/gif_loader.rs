use image::{AnimationDecoder, ImageDecoder};
use slint::Image as SlintImage;

pub struct GifLoader {
    frames: Vec<SlintImage>,
}

impl GifLoader {
    pub fn new() -> Option<Self> {
        let gif_data = include_bytes!("./assets/bongocat.gif");
        Self::from_bytes(gif_data)
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        use std::io::Cursor;
        use slint::SharedPixelBuffer;

        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data)).ok()?;
        let (width, height) = decoder.dimensions();

        let frames = decoder
            .into_frames()
            .collect::<Result<Vec<_>, _>>()
            .ok()?;

        let mut result = Vec::new();
        for frame in frames {
            let buffer = frame.into_buffer();
            let (w, h) = buffer.dimensions();

            if w != width || h != height {
                continue;
            }

            let pixel_data = buffer.into_raw();
            let pixel_buffer =
                SharedPixelBuffer::clone_from_slice(&pixel_data, width, height);
            let slint_image = SlintImage::from_rgba8(pixel_buffer);
            result.push(slint_image);
        }

        Some(Self { frames: result })
    }

    pub fn into_frames(self) -> Vec<SlintImage> {
        self.frames
    }
}

impl Default for GifLoader {
    fn default() -> Self {
        Self::new().unwrap_or(Self { frames: Vec::new() })
    }
}