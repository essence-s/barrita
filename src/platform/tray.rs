use std::io::Cursor;
use tray_icon::{
    menu::{Menu, MenuItem, MenuEvent},
    menu::accelerator::Accelerator,
    TrayIconBuilder,
    Icon,
};
use image::ImageDecoder;
use slint::quit_event_loop;

fn load_icon_from_png(bytes: &[u8]) -> Icon {
    let cursor = Cursor::new(bytes);
    let decoder = image::codecs::png::PngDecoder::new(cursor).expect("Failed to create PNG decoder s");
    let (width, height) = decoder.dimensions();
    let mut buffer = vec![0u8; width as usize * height as usize * 4];
    decoder.read_image(&mut buffer).expect("Failed to read PNG image v");

    Icon::from_rgba(buffer, width, height).expect("Failed to create icon from RGBA y")
}

pub fn init_tray() -> Option<tray_icon::TrayIcon> {
    let icon_bytes = include_bytes!("../../assets/icon.png");
    let icon = load_icon_from_png(icon_bytes);

    let quit_item = MenuItem::with_id("quit", "Salir", true, None::<Accelerator>);

    let menu = Menu::with_items(&[&quit_item]).expect("Failed to create menu t");

    let _ = MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id.as_ref() == "quit" {
            let _ = quit_event_loop();
        }
    }));

    let tray = TrayIconBuilder::new()
        .with_tooltip("Barrita")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create tray icon");

    Some(tray)
}