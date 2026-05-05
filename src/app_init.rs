use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, VecModel, ModelRc, SharedString};
use crate::config::{Config, get_workspaces_style, get_music_icon_style, parse_hex_color};
use crate::{StatusBarWindow, Theme};

pub fn init_window(app: &StatusBarWindow, width: u32, height: u32) {
    app.window().set_size(PhysicalSize::new(width, height));
    app.window().set_position(PhysicalPosition::new(0, 0));
}

pub fn init_workspaces(app: &StatusBarWindow, config: &Config) {
    app.set_workspace_total(config.workspaces.total_workspaces);
    if let Some(ref format) = config.workspaces.format {
        let shared_format: Vec<SharedString> = format.iter().map(|s| s.as_str().into()).collect();
        let model = VecModel::from(shared_format);
        app.set_workspace_format(ModelRc::new(model));
    }
}

pub fn init_styles(app: &StatusBarWindow, config: &Config) {
    let theme = app.global::<Theme>();
    
    let ws = get_workspaces_style(config);
    if let Some(color) = ws.bg_color {
        theme.set_workspaces_bg_color(parse_hex_color(&color));
    }
    if let Some(color) = ws.active_color {
        theme.set_workspaces_active_color(parse_hex_color(&color));
    }
    if let Some(color) = ws.occupied_bg {
        theme.set_workspaces_occupied_bg(parse_hex_color(&color));
    }
    if let Some(color) = ws.text_active {
        theme.set_workspaces_text_active(parse_hex_color(&color));
    }
    if let Some(color) = ws.text_occupied {
        theme.set_workspaces_text_occupied(parse_hex_color(&color));
    }
    if let Some(color) = ws.text_free {
        theme.set_workspaces_text_free(parse_hex_color(&color));
    }
    if let Some(radius) = ws.border_radius {
        theme.set_workspaces_border_radius(radius as i32);
    }
    
    let ms = get_music_icon_style(config);
    if let Some(color) = ms.artist_color {
        theme.set_music_artist_color(parse_hex_color(&color));
    }
    if let Some(radius) = ms.album_border_radius {
        theme.set_music_album_border_radius(radius as i32);
    }
}

pub fn init_app(app: &StatusBarWindow, config: &Config, display_width: u32, display_height: u32) {
    init_window(app, display_width, display_height);
    init_workspaces(app, config);
    init_styles(app, config);
}