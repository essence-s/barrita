# Barrita — Status bar (Slint + Rust + Wayland)

## Backend crate (`backend/`)

Crate reutilizable que maneja la conexión Wayland y el renderizado Slint+Skia.

### API pública

```rust
backend::windows![StatusBarWindow, ControlCenter];
// Genera: StatusBarWindowWl, ControlCenterWl
//          (wrappers Slint + WaylandWindow)

let w = StatusBarWindowWl::spawn("name", window_conf);
w.toggle();
w.hide();
let handle: WinHandle = w.get_handler();
let (ui, way): (StatusBarWindow, WaylandWindow) = w.parts();

run_windows!(windows: [w1, w2]);  // event loop
```

### Tipos principales

| Tipo | Rol |
|---|---|
| `WaylandWindow` | Ventana Wayland (layer shell, compositor, seat) |
| `SkiaWindowAdapter` | Implementa `slint::platform::WindowAdapter` con Skia |
| `SlintPlatform` | Implementa `slint::platform::Platform` |
| `WindowHandler` | Trait para ventanas que participan en el event loop |
| `WinHandle` | Handle para controlar ventanas desde cualquier thread |
| `WindowConf` / `WindowConfBuilder` | Configuración de layer surface |

### Dependencias clave

- `sctk = "0.20"` (smithay-client-toolkit)
- `slint = "1.17"` con renderer-skia
- `calloop` event loop
- `wayland-protocols` (wlr-layer, fractional-scale, viewporter, cursor-shape)

### Parche local (FIFO fix)

`backend/src/wayland_adapter/way_helper.rs` procesa eventos de
`slint::invoke_from_event_loop` con `drain(..)` en vez de `pop()` (FIFO, no LIFO).
Esto evita que eventos encolados rápido (ej: tray icons) se ejecuten en orden
incorrecto.

### Build & Run

```sh
cargo run
```

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
use crate::app::screenshot::ScreenshotController;
use crate::app::clock::ClockController;

pub fn connect_all(window: &StatusBarWindow) {
    ScreenshotController::connect(window);
    ClockController::connect(window);
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
backend::windows![StatusBarWindow, ControlCenter, TrayPopup];

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let bar = StatusBarWindowWl::spawn("barrita", bar_conf);
    let ctrl = ControlCenterWl::spawn("control-center", ctrl_conf);
    run_windows!(windows: [bar, ctrl])
}
```

## Flujo completo

```
Usuario interactúa con widget
  → widget.slint: TouchArea.clicked
    → WidgetAdapter.callback()
      → Controller::connect() (Rust)
        → lógica, timers, servicios
          → adapter.set_property(...) → UI se actualiza sola
```

## Ventajas

- **Widget autónomo**: su `.slint` + `mod.rs` están en la misma carpeta
- **Sin forwarding**: no hay callbacks que burbujean hasta StatusBarWindow
- **Un adaptador central**: `ui/adapters.rs` es el único punto de cableado
- **main.rs minimal**: no toca nada del dominio
- **Backend reutilizable**: Wayland + Slint en un crate separado

## Comandos útiles

```sh
cargo build
cargo run
```

## Estructura del proyecto

```
barrita/
├── backend/
│   └── src/
│       ├── lib.rs                  # WindowHandler trait, run_event_loop
│       ├── configure.rs            # WindowConf builder
│       ├── event_macros.rs         # windows!, run_windows!
│       ├── skia_non_docs.rs        # SkiaWindowAdapter
│       ├── slint_adapter.rs        # SlintPlatform
│       └── wayland_adapter/
│           ├── mod.rs              # WaylandWindow, WinHandle
│           ├── win_impl.rs         # Seat, Keyboard, Pointer, Touch handlers
│           ├── way_helper.rs       # PointerState, event draining
│           ├── fractional_scaling.rs
│           ├── viewporter.rs
│           └── slint_to_wl_cursor_mapping.rs
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
│       └── workspaces/
└── assets/icons/
```
