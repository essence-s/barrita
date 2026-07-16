use slint::ComponentHandle;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use system_tray::client::{ActivateRequest, Client};
use system_tray::item::IconPixmap;
use slint_layer_shell::wayland_adapter::WinHandle;
use system_tray::menu::{MenuType, ToggleState, ToggleType, TrayMenu};

pub mod popup;

#[derive(Clone, Default)]
struct RawItem {
    icon_pixels: Vec<u8>,
    icon_width: i32,
    icon_height: i32,
    tooltip: String,
}

#[derive(Clone, Default)]
struct FlatMenuItem {
    id: i32,
    label: String,
    enabled: bool,
    is_separator: bool,
    is_checked: bool,
}

#[derive(Debug)]
enum TrayCommand {
    Activate(ActivateRequest),
    ShowMenu { index: i32 },
}

fn best_icon<'a>(pixmaps: &'a [IconPixmap], target: u32) -> Option<&'a IconPixmap> {
    pixmaps
        .iter()
        .filter(|p| p.width > 0 && p.height > 0 && !p.pixels.is_empty())
        .min_by_key(|p| (p.width.max(p.height) as u32 as i32 - target as i32).abs())
}

fn load_icon_from_path(
    icon_name: &str,
    icon_theme_path: Option<&str>,
) -> Option<(Vec<u8>, i32, i32)> {
    let path_str = format!("{}/{}.png", icon_theme_path?, icon_name);
    let img = image::open(&path_str).ok()?.into_rgba8();
    let (orig_w, orig_h) = (img.width(), img.height());
    const TARGET: u32 = 18;

    let resized = if orig_w != TARGET || orig_h != TARGET {
        image::imageops::resize(&img, TARGET, TARGET, image::imageops::CatmullRom)
    } else {
        img
    };

    let raw = resized.into_raw();
    let mut argb = Vec::with_capacity(raw.len());
    for rgba in raw.chunks(4) {
        argb.push(rgba[3]);
        argb.push(rgba[0]);
        argb.push(rgba[1]);
        argb.push(rgba[2]);
    }
    Some((argb, TARGET as i32, TARGET as i32))
}

fn raw_to_image(pixels: &[u8], width: i32, height: i32) -> slint::Image {
    if width <= 0 || height <= 0 || pixels.is_empty() {
        return slint::Image::default();
    }
    let w = width as u32;
    let h = height as u32;
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let slice = buffer.make_mut_slice();
    for (i, chunk) in pixels.chunks(4).enumerate() {
        if i < slice.len() {
            slice[i] = slint::Rgba8Pixel::new(chunk[1], chunk[2], chunk[3], chunk[0]);
        }
    }
    slint::Image::from_rgba8(buffer)
}

fn build_raw_item(
    _addr: &str,
    item: &system_tray::item::StatusNotifierItem,
) -> RawItem {
    let (icon_pixels, icon_width, icon_height) = item
        .icon_pixmap
        .as_ref()
        .and_then(|pixmaps| best_icon(pixmaps, 22))
        .map(|p| (p.pixels.clone(), p.width, p.height))
        .unwrap_or_default();
    let (icon_pixels, icon_width, icon_height) = if icon_pixels.is_empty() {
        item.icon_name
            .as_deref()
            .and_then(|name| load_icon_from_path(name, item.icon_theme_path.as_deref()))
            .unwrap_or_default()
    } else {
        (icon_pixels, icon_width, icon_height)
    };
    let tooltip = item
        .title
        .as_deref()
        .or(item.tool_tip.as_ref().map(|t| t.title.as_str()))
        .unwrap_or("")
        .to_owned();
    RawItem {
        icon_pixels,
        icon_width,
        icon_height,
        tooltip,
    }
}

fn build_flat_menu_items(menu: &TrayMenu) -> Vec<FlatMenuItem> {
    menu.submenus
        .iter()
        .filter(|item| item.visible)
        .map(|item| FlatMenuItem {
            id: item.id,
            label: item.label.clone().unwrap_or_default(),
            enabled: item.enabled,
            is_separator: matches!(item.menu_type, MenuType::Separator),
            is_checked: !matches!(item.toggle_type, ToggleType::CannotBeToggled)
                && matches!(item.toggle_state, ToggleState::On),
        })
        .collect()
}

fn update_ui(weak: &slint::Weak<crate::StatusBarWindow>, items: Vec<RawItem>) {
    let w = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(window) = w.upgrade() {
            let adapter = window.global::<crate::TrayAdapter>();
            let images: Vec<slint::Image> = items
                .iter()
                .map(|ri| raw_to_image(&ri.icon_pixels, ri.icon_width, ri.icon_height))
                .collect();
            let tooltips: Vec<slint::SharedString> =
                items.iter().map(|ri| ri.tooltip.as_str().into()).collect();
            let icons_model = Rc::new(slint::VecModel::from(images));
            let tips_model = Rc::new(slint::VecModel::from(tooltips));
            adapter.set_item_icons(icons_model.into());
            adapter.set_item_tooltips(tips_model.into());
            adapter.set_item_count(items.len() as i32);
        }
    });
}

pub struct TrayController;

impl TrayController {
    pub fn connect(window: &crate::StatusBarWindow, popup_handler: WinHandle, popup_weak: slint::Weak<crate::TrayPopup>) {
        let weak = window.as_weak();
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<TrayCommand>(16);
        let addresses: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let active_menu: Arc<Mutex<Option<(String, String)>>> = Arc::new(Mutex::new(None));

        let adapter = window.global::<crate::TrayAdapter>();
        adapter.on_item_clicked({
            let addresses = addresses.clone();
            let cmd_tx = cmd_tx.clone();
            move |index| {
                let addr = {
                    let list = addresses.lock().unwrap();
                    list.get(index as usize).cloned()
                };
                if let Some(address) = addr {
                    let _ = cmd_tx.try_send(TrayCommand::Activate(ActivateRequest::Default {
                        address,
                        x: 0,
                        y: 0,
                    }));
                }
            }
        });

        adapter.on_right_clicked({
            let cmd_tx = cmd_tx.clone();
            let w = weak.clone();
            move |index| {
                if let Some(win) = w.upgrade() {
                    let a = win.global::<crate::TrayAdapter>();
                    a.invoke_show_popup();
                }
                let _ = cmd_tx.try_send(TrayCommand::ShowMenu { index });
            }
        });

        let ph = popup_handler.clone();
        adapter.on_menu_item_selected({
            let active_menu = active_menu.clone();
            let cmd_tx = cmd_tx.clone();
            let ph = ph.clone();
            move |item_id| {
                let (address, menu_path) = match active_menu.lock().unwrap().clone() {
                    Some(a) => a,
                    None => return,
                };
                let _ = cmd_tx.try_send(TrayCommand::Activate(
                    ActivateRequest::MenuItem {
                        address,
                        menu_path,
                        submenu_id: item_id,
                    },
                ));
                ph.hide();
            }
        });

        adapter.on_show_popup({
            let ph = popup_handler.clone();
            let pw = popup_weak.clone();
            move || {
                ph.toggle();
                // ph.show_again();
                if let Some(p) = pw.upgrade() {
                    p.set_render_trigger(p.get_render_trigger() + 1);
                }
            }
        });

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("[tray] failed to create tokio runtime: {e}");
                    return;
                }
            };

            rt.block_on(async move {
                let client = match Client::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("[tray] client creation failed: {e}");
                        return;
                    }
                };

                let mut rx = client.subscribe();
                let items_map = client.items();

                {
                    let map = items_map.lock().unwrap();
                    if !map.is_empty() {
                        let mut addrs = Vec::with_capacity(map.len());
                        let items: Vec<RawItem> = map
                            .iter()
                            .map(|(addr, (item, _))| {
                                addrs.push(addr.clone());
                                build_raw_item(addr, item)
                            })
                            .collect();
                        *addresses.lock().unwrap() = addrs;
                        update_ui(&weak, items);
                    }
                }

                loop {
                    tokio::select! {
                        event = rx.recv() => {
                            match event {
                                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    log::info!("[tray] event received");
                                    let map = items_map.lock().unwrap();
                                    let mut addrs = Vec::with_capacity(map.len());
                                    let items: Vec<RawItem> = map
                                        .iter()
                                        .map(|(addr, (item, _))| {
                                            addrs.push(addr.clone());
                                            build_raw_item(addr, item)
                                        })
                                        .collect();
                                    *addresses.lock().unwrap() = addrs;
                                    update_ui(&weak, items);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    log::error!("[tray] event channel closed");
                                    break;
                                }
                            }
                        }
                        cmd = cmd_rx.recv() => {
                            match cmd {
                                Some(TrayCommand::Activate(req)) => {
                                    log::info!("[tray] sending activation: {req:?}");
                                    if let Err(e) = client.activate(req).await {
                                        log::error!("[tray] activation failed: {e}");
                                    }
                                }
                                Some(TrayCommand::ShowMenu { index }) => {
                                    log::info!("[tray] showing menu for index {index}");
                                    let (address, menu_path, flat_items) = {
                                        let map = items_map.lock().unwrap();
                                        let addr_list = addresses.lock().unwrap();
                                        let addr = match addr_list.get(index as usize) {
                                            Some(a) => a.clone(),
                                            None => {
                                                log::warn!("[tray] no address for index {index}");
                                                continue;
                                            }
                                        };
                                        let entry = match map.get(&addr) {
                                            Some(e) => e,
                                            None => {
                                                log::warn!("[tray] no item for address {addr}");
                                                continue;
                                            }
                                        };
                                        let (item, menu_opt) = entry;
                                        let menu_path = item.menu.clone().unwrap_or_default();
                                        let flat = menu_opt
                                            .as_ref()
                                            .map(build_flat_menu_items)
                                            .unwrap_or_default();
                                        (addr, menu_path, flat)
                                    };

                                    let popup_items: Vec<crate::PopupMenuItem> = flat_items
                                        .iter()
                                        .map(|fi| crate::PopupMenuItem {
                                            id: fi.id,
                                            label: fi.label.clone().into(),
                                            enabled: fi.enabled,
                                            is_separator: fi.is_separator,
                                            is_checked: fi.is_checked,
                                        })
                                        .collect();

                                    let w = weak.clone();
                                    let am = active_menu.clone();
                                    let addr_set = address.clone();
                                    let path_set = menu_path.clone();
                                    let pw = popup_weak.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        *am.lock().unwrap() = Some((addr_set, path_set));
                                        let model = Rc::new(slint::VecModel::from(popup_items));
                                        if let Some(window) = w.upgrade() {
                                            let adapter = window.global::<crate::TrayAdapter>();
                                            adapter.set_popup_items(model.clone().into());
                                        }
                                        if let Some(popup) = pw.upgrade() {
                                            let pa = popup.global::<crate::TrayAdapter>();
                                            pa.set_popup_items(model.into());
                                            popup.set_render_trigger(popup.get_render_trigger() + 1);
                                        }
                                    });

                                    if !menu_path.is_empty() {
                                        let _ = client.about_to_show_menuitem(address, menu_path, 0).await;
                                    }
                                }
                                None => {
                                    log::info!("[tray] command channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        });
    }
}
