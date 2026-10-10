//! Entrada táctil ergonómica (Fase 5.1, propuesta
//! `2026-10-08_controles-tactiles-haptica`).
//!
//! Lógica pura: geometría de hitboxes, orientaciones y mapeo a los bits
//! KEYINPUT de la GBA (activo en bajo, ver `gba-core/src/mem.rs`:
//! `keys: u16` con 0x3FF = nada pulsado). Sin dependencias de Android,
//! testeable en host con `cargo test`.
//!
//! Geometría (validada en `docs/prototipos/layout_tactil_v2.html`):
//! - **Cruceta** con cuenca circular: 4 zonas direccionales dentro del
//!   anillo (arriba/abajo/izquierda/derecha), con zona muerta central.
//! - **Clúster diagonal** estilo GBA SP a la derecha: cápsula inclinada
//!   con B (abajo-izquierda) y A (arriba-derecha), y las pastillas L y R
//!   en el mismo eje por encima (L sobre B, R sobre A). Nunca en esquinas
//!   superiores (decisión de contexto de la propuesta).
//! - **Select/Start** centrados debajo.
//! - Portrait: controles agrandados con colores en la mitad inferior.
//!   Landscape: base inferior translúcida, opacidad regulable (la
//!   opacidad es presentación; aquí solo cambia la geometría).
//!
//! Precisión individual sobre multitáctil complejo: hitboxes holgadas,
//! descarte de toques ambiguos (primer botón que contiene el punto) y
//! cancelación total en `Cancel`.

/// Botón de GBA (nombres estándar de la consola).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    B,
    A,
    L,
    R,
    Select,
    Start,
}

impl Button {
    /// Bit de KEYINPUT (activo en bajo) que controla cada botón.
    /// Fuente: `gba-core/src/mem.rs` (`keys: u16`, 0x3FF = nada pulsado)
    /// y mapa estándar de KEYINPUT de la GBA.
    pub fn key_bit(self) -> u16 {
        match self {
            Button::A => 0x0001,
            Button::B => 0x0002,
            Button::Select => 0x0004,
            Button::Start => 0x0008,
            Button::Right => 0x0010,
            Button::Left => 0x0020,
            Button::Up => 0x0040,
            Button::Down => 0x0080,
            Button::R => 0x0100,
            Button::L => 0x0200,
        }
    }
}

/// Mapea un código de tecla estándar de Android (`android.view.KeyEvent.KEYCODE_*`) a un botón de GBA.
/// Soporta mandos Bluetooth, USB, teclados físicos y control remoto de Android TV.
pub fn button_from_android_keycode(keycode: i32) -> Option<Button> {
    match keycode {
        19 => Some(Button::Up),      // KEYCODE_DPAD_UP
        20 => Some(Button::Down),    // KEYCODE_DPAD_DOWN
        21 => Some(Button::Left),    // KEYCODE_DPAD_LEFT
        22 => Some(Button::Right),   // KEYCODE_DPAD_RIGHT
        23 | 66 | 96 | 160 => Some(Button::A), // KEYCODE_DPAD_CENTER, KEYCODE_ENTER, KEYCODE_BUTTON_A, KEYCODE_NUMPAD_ENTER
        4 | 97 | 111 => Some(Button::B),       // KEYCODE_BACK, KEYCODE_BUTTON_B, KEYCODE_ESCAPE
        102 => Some(Button::L),      // KEYCODE_BUTTON_L1
        103 => Some(Button::R),      // KEYCODE_BUTTON_R1
        108 | 85 => Some(Button::Start),       // KEYCODE_BUTTON_START, KEYCODE_MEDIA_PLAY_PAUSE
        109 => Some(Button::Select), // KEYCODE_BUTTON_SELECT
        _ => None,
    }
}

/// Orientación de la pantalla; determina la geometría de los controles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orientation {
    /// Vertical: controles agrandados y coloreados en la mitad inferior.
    Portrait,
    /// Horizontal: base inferior translúcida (opacidad regulable en la
    /// capa de presentación, no aquí).
    Landscape,
}

/// Caja táctil circular (hitbox holgada con radio > radio visual).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchArea {
    pub button: Button,
    /// Centro en píxeles de ventana (x, y).
    pub cx: f32,
    pub cy: f32,
    /// Radio en píxeles (incluye la zona muerta/folga del prototipo).
    pub radius: f32,
}

impl TouchArea {
    /// ¿Contiene este área el punto (x, y)?
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let dx = x - self.cx;
        let dy = y - self.cy;
        dx * dx + dy * dy <= self.radius * self.radius
    }
}

/// Calcula las hitboxes para una ventana de `w` x `h` píxeles y una
/// orientación. Determinista y sin estado: la misma entrada produce la
/// misma salida (regresión `test_touch_layout_orientation_and_toggle`).
///
/// Portrait (w < h): controles en la mitad inferior, agrandados y ergonómicos.
/// Landscape (w >= h): franja inferior flotante translúcida en esquinas.
pub fn layout(w: u32, h: u32, orientation: Orientation) -> Vec<TouchArea> {
    let (wf, hf) = (w as f32, h as f32);
    let portrait = orientation == Orientation::Portrait;

    // Escala y posicionamiento ergonómico según orientación
    // (Fase 5.2 / propuesta 2026-10-09_correccion-escala-y-renderizado-overlay)
    let (dpad_radius, dpad_cx, dpad_cy, right_cx, cluster_cy, ss_y) = if portrait {
        let r_dpad = wf * 0.215;
        let cy = hf * 0.72;
        (r_dpad, wf * 0.23, cy, wf * 0.77, cy, hf * 0.91)
    } else {
        let r_dpad = hf * 0.20;
        let cy = hf * 0.72;
        (r_dpad, hf * 0.20, cy, wf - hf * 0.20, cy, hf * 0.90)
    };

    // Unidad base para clúster y botones
    let unit = dpad_radius;

    // Radio de hitbox para botones B y A (~0.38 * unit, > 60 px en 1080p y landscape 1080p)
    let r = unit * 0.38;
    // Separación de centros del clúster diagonal (aumentada para mayor holgura entre B y A)
    let step = unit * 1.00;

    // Inclinación del eje B->A: 16° por encima de la horizontal, hacia
    // arriba a la derecha (prototipo: --ab-tilt: -16deg). En píxeles de
    // pantalla y crece hacia abajo, así que A queda con y menor que B.
    let tilt = 16.0f32.to_radians();
    let (dx, dy) = (tilt.cos(), -tilt.sin());

    // Vector ortogonal hacia arriba en el sistema del pozo rotado:
    let (ox, oy) = (dy, -dx);

    // Centro del par B/A; B en un extremo, A en el otro.
    let half = step * 0.5;
    let b = (right_cx - half * dx, cluster_cy - half * dy);
    let a = (right_cx + half * dx, cluster_cy + half * dy);

    // L sobre B y R sobre A, perfectamente alineados en el eje ortogonal del pozo inclinado
    let shoulder_dist = step * 0.80;
    let l = (b.0 + ox * shoulder_dist, b.1 + oy * shoulder_dist);
    let rr = (a.0 + ox * shoulder_dist, a.1 + oy * shoulder_dist);

    // Cruceta: 4 centros direccionales sobre anillo a distancia arm
    let arm = dpad_radius * 0.55;
    let up = (dpad_cx, dpad_cy - arm);
    let down = (dpad_cx, dpad_cy + arm);
    let left = (dpad_cx - arm, dpad_cy);
    let right = (dpad_cx + arm, dpad_cy);
    let dpad_btn_r = dpad_radius * 0.34;

    // Select/Start centrados
    let ss_r = unit * 0.28;
    let ss_gap = if portrait { wf * 0.12 } else { hf * 0.12 };
    let select = (wf * 0.5 - ss_gap, ss_y);
    let start = (wf * 0.5 + ss_gap, ss_y);

    let area = |button: Button, c: (f32, f32), rad: f32| TouchArea {
        button,
        cx: c.0,
        cy: c.1,
        radius: rad,
    };

    let r_shoulder = r * 1.02;

    vec![
        area(Button::Up, up, dpad_btn_r),
        area(Button::Down, down, dpad_btn_r),
        area(Button::Left, left, dpad_btn_r),
        area(Button::Right, right, dpad_btn_r),
        area(Button::B, b, r),
        area(Button::A, a, r),
        area(Button::L, l, r_shoulder),
        area(Button::R, rr, r_shoulder),
        area(Button::Select, select, ss_r),
        area(Button::Start, start, ss_r),
    ]
}

/// Controlador táctil: recibe puntos de la pantalla y mantiene el estado
/// KEYINPUT de la GBA.
///
/// Reglas (contexto de la propuesta):
/// - `touch_enabled == false` → ningún toque altera las teclas (uso de
///   mando físico).
/// - Precisión individual: un punto se mapea al primer botón que lo
///   contiene; si no contiene ninguno, no cambia nada (descarte de
///   toques ambiguos).
/// - `Cancel` libera todos los toques activos.
///
/// Extras de la Fase 5.2: `is_pressed`/`pressed_mask` exponen el estado
/// vivo para el resaltado visual del overlay.
#[derive(Debug)]
pub struct TouchController {
    areas: Vec<TouchArea>,
    orientation: Orientation,
    size: (u32, u32),
    pub touch_enabled: bool,
    /// IDs de punteros activos y el botón que cada uno mantiene pulsado.
    pressed: Vec<(i32, Button)>,
}

impl TouchController {
    /// Estado inicial: 1080x2400 portrait (típico), controles activos.
    pub fn new() -> TouchController {
        let (w, h) = (1080, 2400);
        TouchController {
            areas: layout(w, h, Orientation::Portrait),
            orientation: Orientation::Portrait,
            size: (w, h),
            touch_enabled: true,
            pressed: Vec::new(),
        }
    }

    /// Actualiza el tamaño de ventana y la orientación; recalcula
    /// hitboxes (los toques activos se liberan: la geometría cambió).
    pub fn resize(&mut self, w: u32, h: u32) {
        let orientation = if w >= h {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        };
        self.orientation = orientation;
        self.size = (w, h);
        self.areas = layout(w, h, orientation);
        self.pressed.clear();
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn areas(&self) -> &[TouchArea] {
        &self.areas
    }

    /// Botón bajo el punto, si hay alguno (primera coincidencia).
    fn hit_at(&self, x: f32, y: f32) -> Option<Button> {
        self.areas.iter().find(|a| a.contains(x, y)).map(|a| a.button)
    }

    /// ¿Está este botón pulsado ahora mismo (por algún puntero)?
    /// Fuente de verdad para el resaltado visual del overlay (Fase 5.2).
    pub fn is_pressed(&self, button: Button) -> bool {
        self.pressed.iter().any(|(_, b)| *b == button)
    }

    /// Máscara de bits de los botones pulsados (bit = `key_bit()` del
    /// botón). Útil para pasar al overlay en una sola palabra.
    pub fn pressed_mask(&self) -> u16 {
        self.pressed
            .iter()
            .map(|(_, b)| b.key_bit())
            .fold(0u16, |acc, b| acc | b)
    }

    /// Procesa un toque hacia abajo. Devuelve el botón si este toque lo
    /// activó (transición nueva, para háptica).
    pub fn touch_down(&mut self, pointer_id: i32, x: f32, y: f32) -> Option<Button> {
        if !self.touch_enabled {
            return None;
        }
        let Some(button) = self.hit_at(x, y) else { return None };
        // Precisión individual: si el puntero ya mantenía algo, no cambia.
        if self.pressed.iter().any(|(id, _)| *id == pointer_id) {
            return None;
        }
        self.pressed.push((pointer_id, button));
        Some(button)
    }

    /// Procesa un toque hacia arriba. Devuelve el botón si este toque lo
    /// liberó (transición, para háptica de liberación).
    pub fn touch_up(&mut self, pointer_id: i32) -> Option<Button> {
        let pos = self.pressed.iter().position(|(id, _)| *id == pointer_id);
        let button = pos.map(|i| self.pressed.remove(i).1);
        if self.pressed.is_empty() {
            // Nada activo: al soltar el último, el estado queda limpio.
        }
        button
    }

    /// Cancela todos los toques (evento `Cancel` de Android).
    pub fn cancel_all(&mut self) {
        self.pressed.clear();
    }

    /// Máscara KEYINPUT completa, lista para `MemMap.keys` (activo en
    /// bajo: bits de botones no pulsados en 1).
    pub fn key_mask(&self) -> u16 {
        let pressed_bits = self
            .pressed
            .iter()
            .map(|(_, b)| b.key_bit())
            .fold(0u16, |acc, b| acc | b);
        !pressed_bits & 0x03FF
    }
}

impl Default for TouchController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
