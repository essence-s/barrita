# Barrita — Status bar (Slint + Rust + Wayland)

## Integración Wayland (`slint-layer-shell`)

Crate externo que maneja la conexión Wayland, layer surfaces y el event loop.
Se usa como dependencia git:

```toml
slint-layer-shell = { git = "https://github.com/essence-s/slint-layer-shell" }
```

### Dependencias clave

- `slint = "1.17"` con renderer-skia
- `slint-layer-shell` (git) — Wayland layer-shell, compositor, seat, event loop
- `mpris = "2.1"` — MPRIS media player D-Bus
- `dbus = "0.9"` — D-Bus (battery/UPower, media)
- `hyprland = "0.4.0-beta.3"` — Hyprland IPC (workspaces)
- `system-tray = "0.8"` — StatusNotifierItem (system tray)
- `tokio = "1"` — Async runtime para tray client
- `chrono = "0.4"` — Fecha/hora para clock widget
- `image = "0.25"` — Decodificación de imágenes (tray icons)

## Widget Architecture Pattern

Cada widget sigue este patrón (`src/app/<widget>/`):

### 1. UI (`<widget>.slint`) — Solo declarativa

Define un **`global`** propio para el estado y callbacks, y el componente visual que lo consume directamente:

```slint
export global ClockAdapter {
    in-out property <string> current-time;
    callback show-time;
}

export component ClockWidget inherits Rectangle {
    Text { text: ClockAdapter.current-time; }
    TouchArea {
        clicked => { ClockAdapter.show-time(); }
    }
}
```

Reglas:

- El `global` **se exporta** (para que Rust pueda accederlo)
- El componente **no tiene** `callback` propio — usa el global directo
- El componente **no recibe** `in property` de state — lee del global

### 2. Controller (`mod.rs`) — Lógica Rust

```rust
use slint::ComponentHandle;

pub struct ClockController;

impl ClockController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::ClockAdapter>();
        adapter.on_show_time(|| {
            log::info!("[clock] show time clicked");
        });
    }
}
```

### 3. Adapter central (`src/ui/adapters.rs`)

Orquesta todos los controllers:

```rust
use crate::StatusBarWindow;
use crate::app::clock::ClockController;
use crate::app::media::MediaController;
use crate::app::tray::TrayController;
use slint_layer_shell::wayland_adapter::WinHandle;

pub fn connect_all(
    window: &StatusBarWindow,
    ctrl_handler: WinHandle,
    tray_popup_handler: WinHandle,
    popup_weak: slint::Weak<crate::TrayPopup>,
) {
    ClockController::connect(window);
    MediaController::connect(window, ctrl_handler);
    TrayController::connect(window, tray_popup_handler, popup_weak);
    // ... resto de controllers
}
```

### 4. Entry point (`src/ui/app.slint`)

Importa y re-exporta los globals para que Slint genere el Rust code:

```slint
import { ClockAdapter } from "../app/clock/clock.slint";
import { StatusBarWindow } from "status_bar.slint";
// ...

export { StatusBarWindow, ClockAdapter, ... }
```

### 5. Main (`src/main.rs`) — solo init

```rust
fn main() {
    env_logger::init();
    barrita::run();
}
```

### 6. Lib (`src/lib.rs`) — setup

```rust
slint::include_modules!();
slint_layer_shell::windows![StatusBarWindow, ControlCenter, TrayPopup];

use slint_layer_shell::{
    run_windows,
    layer_properties::{LayerAnchor, LayerType, WindowConf},
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let bar_conf = WindowConf::builder()
        .width(1366_u32).height(36_u32)
        .anchor_1(LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT)
        .exclusive_zone(36).layer_type(LayerType::Top)
        .build().unwrap();

    let bar = StatusBarWindowWl::spawn("barrita", bar_conf);
    let ctrl = ControlCenterWl::spawn("control-center", ctrl_conf);
    let popup = TrayPopupWl::spawn("tray-popup", popup_conf);
    ctrl.hide(); popup.hide();

    let ctrl_handler = ctrl.get_handler();
    ui::adapters::connect_all(&bar, ctrl_handler, ...);

    run_windows!(windows: [bar, ctrl, popup])
}
```

## Estructura del proyecto

```
barrita/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── ui/
│   │   ├── app.slint
│   │   ├── bar.slint
│   │   ├── styles/theme.slint
│   │   ├── adapters.rs
│   │   └── image.rs
│   └── app/
│       ├── screenshot/
│       ├── clock/
│       ├── media/
│       ├── battery/
│       ├── network/
│       ├── bluetooth/
│       ├── colorize/
│       ├── article/
│       ├── logo/
│       ├── tray/
│       ├── control_center/
│       └── workspaces/
└── assets/icons/
```
