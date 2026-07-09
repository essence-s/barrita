slint::include_modules!();
spell_framework::generate_widgets![StatusBarWindow, ControlCenter, TrayPopup];

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
        .height(36_u32)
        .anchor_1(LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT)
        .exclusive_zone(36)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let ctrl_conf = WindowConf::builder()
        .width(800_u32)
        .height(450_u32)
        .anchor_1(LayerAnchor::TOP)
        .margins(0, 0, 0, 0)
        .exclusive_zone(-1)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let popup_conf = WindowConf::builder()
        .width(200_u32)
        .height(200_u32)
        .anchor_1(LayerAnchor::TOP | LayerAnchor::RIGHT)
        .margins(36, 0, 0, 4)
        .exclusive_zone(-1)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let bar = StatusBarWindowSpell::invoke_spell("barrita", bar_conf);
    let ctrl = ControlCenterSpell::invoke_spell("control-center", ctrl_conf);
    let popup = TrayPopupSpell::invoke_spell("tray-popup", popup_conf);
    ctrl.hide();
    popup.hide();

    let ctrl_handler = ctrl.get_handler();
    let tray_popup_handler = popup.get_handler();
    ui::adapters::connect_all(&bar, ctrl_handler, tray_popup_handler, popup.as_weak());

    // spell's first render happens before the Slint component exists,
    // so force a redraw now to ensure content appears on the first frame.
    bar.window().request_redraw();
    ctrl.window().request_redraw();
    // popup.window().request_redraw();

    cast_spell!(windows: [bar, ctrl, popup])
}
