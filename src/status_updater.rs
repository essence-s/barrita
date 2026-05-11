use crate::StatusBarWindow;

use crate::core::data::StatusBarData;

pub fn update(window: &StatusBarWindow) {
    let mut data = StatusBarData::new();
    data.refresh();

    window.set_time(data.time.into());
    window.set_date(data.date.into());
    window.set_network_status(data.network_status.into());
    window.set_network_connected(data.network_connected);
    window.set_network_icon(data.network_icon.into());

    // Media info is now updated in main.rs via get_current_media_info()
    // These lines are commented out to avoid being overwritten:
    // window.set_media_status(data.media_status.into());
    // window.set_media_has_player(data.media_has_player);
    // window.set_media_title(data.media_title.into());
    // window.set_media_artist(data.media_artist.into());
    // window.set_media_progress(data.media_progress);
    // window.set_media_progress_time(data.media_progress_time.into());
    // window.set_media_total_time(data.media_total_time.into());

    // update_thumbnail(window, &data.media_album_art);
}

// Album art is now handled in main.rs via get_current_media_info()
// This function is commented out to avoid double updates.
// fn update_thumbnail(window: &StatusBarWindow, album_art: &[u8]) {
//     let mut counter_guard = THUMBNAIL_COUNTER.lock().unwrap();
//     *counter_guard += 1;
//     let should_update = *counter_guard % 2 == 0;
//     drop(counter_guard);

//     if !should_update || album_art.len() < 100 {
//         return;
//     }

//     let mut last_bytes = LAST_ALBUM_BYTES.lock().unwrap();
//     if album_art == *last_bytes {
//         return;
//     }

//     if let Ok(img) = load_from_memory(album_art) {
//         let rgba = img.to_rgba8();
//         let (width, height) = rgba.dimensions();

//         if width > 0 && height > 0 && width < 500 && height < 500 {
//             let raw = rgba.into_raw();
//             let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&raw, width, height);
//             let slint_img = SlintImage::from_rgba8_premultiplied(buffer);
//             window.set_media_album_art(slint_img);
//             *last_bytes = album_art.to_vec();
//         }
//     }
// }