slint::include_modules!();
spell_framework::generate_widgets![StatusBarWindow, MediaPopupWindow];

mod app;
mod config;
mod ui;

use spell_framework::{
    cast_spell,
    layer_properties::{LayerAnchor, LayerType, WindowConf},
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _cfg = config::load_or_create_config();

    let bar_conf = WindowConf::builder()
        .width(1366_u32)
        .height(38_u32)
        .anchor_1(LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT)
        .exclusive_zone(38)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let popup_conf = WindowConf::builder()
        .width(300_u32)
        .height(200_u32)
        .anchor_1(LayerAnchor::TOP)
        .margins(34, 0, 0, 0)
        .exclusive_zone(-1)
        .build()
        .unwrap();

    let bar = StatusBarWindowSpell::invoke_spell("barrita", bar_conf);
    let popup = MediaPopupWindowSpell::invoke_spell("media-popup", popup_conf);
    popup.hide();

    let popup_handler = popup.get_handler();
    ui::adapters::connect_all(&bar, popup_handler);

    // spell's first render happens before the Slint component exists,
    // so force a redraw now to ensure content appears on the first frame.
    bar.window().request_redraw();
    popup.window().request_redraw();

    cast_spell!(windows: [bar, popup])
}
