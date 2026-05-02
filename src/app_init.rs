use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, VecModel, ModelRc, SharedString};
use crate::config::{Config, get_workspaces_style, get_music_icon_style, workspaces_style_to_slint, music_icon_style_to_slint};
use crate::StatusBarWindow;

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
    let workspaces_style = get_workspaces_style(config);
    let (bg, active, occupied, text_active, text_occupied, text_free, border) = workspaces_style_to_slint(&workspaces_style);

    app.set_workspaces_bg_color(bg);
    app.set_workspaces_active_color(active);
    app.set_workspaces_occupied_bg(occupied);
    app.set_workspaces_text_active(text_active);
    app.set_workspaces_text_occupied(text_occupied);
    app.set_workspaces_text_free(text_free);
    app.set_workspaces_border_radius(border);

    let music_icon_style = get_music_icon_style(config);
    let (artist_color, album_border) = music_icon_style_to_slint(&music_icon_style);
    app.set_music_artist_color(artist_color);
    app.set_music_album_border_radius(album_border);
}

pub fn init_app(app: &StatusBarWindow, config: &Config, display_width: u32, display_height: u32) {
    init_window(app, display_width, display_height);
    init_workspaces(app, config);
    init_styles(app, config);
}