use slint::{Image as SlintImage, Rgba8Pixel, SharedPixelBuffer};
use std::io::Read;

const COVER_MAX_BYTES: u64 = 8 * 1024 * 1024;
const COVER_THUMB: u32 = 192;

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decodifica `%XX` sin añadir dependencias (rutas `file://` con espacios, etc).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex_val(b[i + 1]), hex_val(b[i + 2]))
        {
            out.push(h << 4 | l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Lee los bytes de un `mpris:artUrl`: `http(s)://`, `file://` o ruta plana.
/// `data:` se rechaza (raro y suele ser placeholder 1x1).
fn cover_bytes(art_url: &str) -> Option<Vec<u8>> {
    let url = art_url.trim();
    if url.is_empty() || url.starts_with("data:") {
        return None;
    }

    if url.starts_with("http://") || url.starts_with("https://") {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(6)))
            .build();
        let agent: ureq::Agent = config.into();
        let mut resp = agent.get(url).call().ok()?;
        let mut limited = resp
            .body_mut()
            .as_reader()
            .take(COVER_MAX_BYTES);
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut limited, &mut buf).ok()?;
        if buf.is_empty() {
            return None;
        }
        return Some(buf);
    }

    let mut path = url;
    if let Some(stripped) = path.strip_prefix("file://") {
        path = stripped.strip_prefix("localhost").unwrap_or(stripped);
    }
    let decoded = percent_decode(path);
    let bytes = std::fs::read(&decoded).ok()?;
    if bytes.is_empty() || bytes.len() as u64 > COVER_MAX_BYTES {
        return None;
    }
    Some(bytes)
}

/// Carga la carátula MPRIS, la reduce a thumb y la difumina en el hilo de
/// fondo (Slint no tiene backdrop-blur). Retorna píxeles RGBA crudos porque
/// `slint::Image` no es `Send`: la imagen se construye con
/// [`rgba_to_slint_image`] ya dentro del event-loop. Retorna `None` si no
/// hay arte útil. Llamar solo cuando cambia la URL: lleva red + decode +
/// blur (~ms).
pub fn load_cover(art_url: &str) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = cover_bytes(art_url)?;
    if bytes.len() < 500 {
        return None;
    }
    let img = image::load_from_memory(&bytes).ok()?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }

    let scale = COVER_THUMB as f32 / w.max(h) as f32;
    let nw = ((w as f32 * scale) as u32).max(1);
    let nh = ((h as f32 * scale) as u32).max(1);
    let thumb = img.thumbnail(nw, nh);
    let blurred = image::imageops::blur(&thumb, 1.5);
    let (bw, bh) = (blurred.width(), blurred.height());
    if bw == 0 || bh == 0 {
        return None;
    }

    Some((blurred.into_raw(), bw, bh))
}

/// Construye el `slint::Image` desde píxeles RGBA. Llamar dentro del
/// event-loop de Slint (el tipo no cruza threads).
pub fn rgba_to_slint_image(raw: Vec<u8>, width: u32, height: u32) -> SlintImage {
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&raw, width, height);
    SlintImage::from_rgba8(buffer)
}

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
