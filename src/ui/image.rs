use slint::{Image as SlintImage, Rgba8Pixel, SharedPixelBuffer};

#[allow(dead_code)]
pub fn bytes_to_slint_image(bytes: &[u8]) -> Option<SlintImage> {
    if bytes.len() < 500 {
        return None;
    }

    let img = if let Ok(decoder) = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(bytes)) {
        image::DynamicImage::from_decoder(decoder).ok()
    } else if let Ok(decoder) = image::codecs::png::PngDecoder::new(std::io::Cursor::new(bytes)) {
        image::DynamicImage::from_decoder(decoder).ok()
    } else {
        None
    }?;

    let (w, h) = (img.width() as u64, img.height() as u64);
    let new_width = ((35u64 * w) / h).max(1) as u32;
    let img = img.resize_exact(new_width, 35, image::imageops::FilterType::Triangle);

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return None;
    }

    let raw = rgba.into_raw();
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&raw, width, height);
    Some(SlintImage::from_rgba8_premultiplied(buffer))
}
