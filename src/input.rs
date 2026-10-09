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
        (r_dpad, wf * 0.25, cy, wf * 0.75, cy, hf * 0.91)
    } else {
        let r_dpad = hf * 0.20;
        let cy = hf * 0.72;
        (r_dpad, hf * 0.32, cy, wf - hf * 0.32, cy, hf * 0.90)
    };

    // Unidad base para clúster y botones
    let unit = dpad_radius;

    // Radio de hitbox para botones B y A (~0.38 * unit, > 60 px en 1080p y landscape 1080p)
    let r = unit * 0.38;
    // Separación de centros del clúster diagonal
    let step = unit * 0.86;

    // Inclinación del eje B->A: 16° por encima de la horizontal, hacia
    // arriba a la derecha (prototipo: --ab-tilt: -16deg). En píxeles de
    // pantalla y crece hacia abajo, así que A queda con y menor que B.
    let tilt = 16.0f32.to_radians();
    let (dx, dy) = (tilt.cos(), -tilt.sin());

    // Centro del par B/A; B en un extremo, A en el otro.
    let half = step * 0.5;
    let b = (right_cx - half * dx, cluster_cy - half * dy);
    let a = (right_cx + half * dx, cluster_cy + half * dy);

    // L sobre B y R sobre A, en el mismo eje paralelo
    let shoulder_dy = -step * 0.85;
    let l = (b.0, b.1 + shoulder_dy);
    let rr = (a.0, a.1 + shoulder_dy);

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

    vec![
        area(Button::Up, up, dpad_btn_r),
        area(Button::Down, down, dpad_btn_r),
        area(Button::Left, left, dpad_btn_r),
        area(Button::Right, right, dpad_btn_r),
        area(Button::B, b, r),
        area(Button::A, a, r),
        area(Button::L, l, r * 0.85),
        area(Button::R, rr, r * 0.85),
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
mod tests {
    use super::*;

    /// Criterio 1 (normal): pulsar A en landscape 2400x1080 activa el bit
    /// 0 en bajo; soltar lo restaura a alto.
    #[test]
    fn test_touch_hitbox_mapping() {
        let mut tc = TouchController::new();
        tc.resize(2400, 1080); // landscape
        assert_eq!(tc.orientation(), Orientation::Landscape);

        let (ax, ay) = {
            let a = tc
                .areas()
                .iter()
                .find(|a| a.button == Button::A)
                .expect("hitbox A existe");
            (a.cx, a.cy)
        };
        // Pulsar en el centro exacto del botón A.
        let pressed = tc.touch_down(7, ax, ay);
        assert_eq!(pressed, Some(Button::A), "el toque en A debe activar A");
        assert_eq!(tc.key_mask() & 0x0001, 0x0000, "bit 0 de A en bajo");

        // Soltar: bit restaurado a alto.
        let released = tc.touch_up(7);
        assert_eq!(released, Some(Button::A));
        assert_eq!(tc.key_mask() & 0x0001, 0x0001, "bit 0 de A en alto");
        assert_eq!(tc.key_mask(), 0x03FF, "sin nada pulsado = 0x3FF");
    }

    /// Criterio 2 (límite): portrait reubica el clúster en la mitad
    /// inferior, y con touch_enabled=false ningún toque altera teclas.
    #[test]
    fn test_touch_layout_orientation_and_toggle() {
        // (a) Portrait 1080x2400: el clúster B/A cae en la mitad inferior.
        let mut tc = TouchController::new();
        tc.resize(1080, 2400);
        assert_eq!(tc.orientation(), Orientation::Portrait);
        let a = {
            let a = tc
                .areas()
                .iter()
                .find(|a| a.button == Button::A)
                .expect("hitbox A existe");
            (a.cx, a.cy)
        };
        assert!(
            a.1 > 2400.0 * 0.5,
            "el clúster debe estar en la mitad inferior (cy={}, mitad=1200)",
            a.1
        );
        // Y el par diagonal: A arriba a la derecha de B (eje inclinado).
        let b = {
            let b = tc
                .areas()
                .iter()
                .find(|x| x.button == Button::B)
                .expect("hitbox B existe");
            (b.cx, b.cy)
        };
        assert!(a.0 > b.0 && a.1 < b.1, "A arriba-derecha de B (16°)");

        // L sobre B en el mismo eje (nunca esquinas superiores).
        let l = {
            let l = tc
                .areas()
                .iter()
                .find(|x| x.button == Button::L)
                .expect("hitbox L existe");
            (l.cx, l.cy)
        };
        assert!(
            (l.0 - b.0).abs() < 1.0 && l.1 < b.1,
            "L directamente encima de B"
        );

        // (b) touch_enabled = false: ningún toque altera las teclas.
        tc.touch_enabled = false;
        let pressed = tc.touch_down(1, a.0, a.1);
        assert_eq!(pressed, None, "con controles apagados no hay activación");
        assert_eq!(tc.key_mask(), 0x03FF, "estado de teclas intacto");
    }

    /// Reglas de precisión individual y cancelación (contexto de la
    /// propuesta): toques ambiguos se descartan y Cancel libera todo.
    #[test]
    fn test_touch_ambiguity_and_cancel() {
        let mut tc = TouchController::new();
        tc.resize(2400, 1080);

        // Toque fuera de cualquier hitbox: descartado.
        assert_eq!(tc.touch_down(1, 5.0, 5.0), None);
        assert_eq!(tc.key_mask(), 0x03FF);

        // Pulsar B y A a la vez (dos punteros): precisión individual.
        let (b, a) = {
            let b = tc.areas().iter().find(|a| a.button == Button::B).unwrap();
            let a = tc.areas().iter().find(|x| x.button == Button::A).unwrap();
            ((b.cx, b.cy), (a.cx, a.cy))
        };
        assert_eq!(tc.touch_down(1, b.0, b.1), Some(Button::B));
        assert_eq!(tc.touch_down(2, a.0, a.1), Some(Button::A));
        assert_eq!(tc.key_mask(), !0x0003 & 0x03FF, "B y A en bajo");

        // Un puntero que ya mantiene no puede reactivar: descartado.
        assert_eq!(tc.touch_down(1, a.0, a.1), None);

        // Cancel libera todo.
        tc.cancel_all();
        assert_eq!(tc.key_mask(), 0x03FF);
    }

    /// Determinismo geométrico: misma entrada → mismas hitboxes.
    #[test]
    fn test_layout_deterministic() {
        let l1 = layout(2400, 1080, Orientation::Landscape);
        let l2 = layout(2400, 1080, Orientation::Landscape);
        assert_eq!(l1, l2);
    }

    /// Criterio de aceptación 1: proporciones físicas ergonómicas en Portrait.
    /// Radio de B y A > 60 px y centro vertical entre 65% y 80% de la pantalla.
    #[test]
    fn test_touch_layout_physical_proportions() {
        let areas = layout(1080, 2400, Orientation::Portrait);
        let a = areas.iter().find(|x| x.button == Button::A).expect("A existe");
        let b = areas.iter().find(|x| x.button == Button::B).expect("B existe");
        assert!(a.radius > 60.0, "radio de A ({}) debe ser > 60 px", a.radius);
        assert!(b.radius > 60.0, "radio de B ({}) debe ser > 60 px", b.radius);

        let cy = (a.cy + b.cy) * 0.5;
        let frac = cy / 2400.0;
        assert!(
            (0.65..=0.80).contains(&frac),
            "centro vertical ({:.2}%) debe estar entre 65% y 80%",
            frac * 100.0
        );
    }

    /// Criterio de aceptación 2: sincronización reactiva al cambiar tamaño/orientación.
    /// Al cambiar a Landscape (2400x1080), todos los botones quedan contenidos en 0..1080.
    #[test]
    fn test_touch_controller_resizes_dynamically() {
        let mut tc = TouchController::new();
        assert_eq!(tc.orientation(), Orientation::Portrait);
        assert_eq!(tc.size(), (1080, 2400));

        tc.resize(2400, 1080);
        assert_eq!(tc.orientation(), Orientation::Landscape);
        assert_eq!(tc.size(), (2400, 1080));

        for area in tc.areas() {
            assert!(
                area.cy - area.radius >= 0.0 && area.cy + area.radius <= 1080.0,
                "botón {:?} en y={} r={} fuera de límites verticales 0..1080",
                area.button, area.cy, area.radius
            );
            assert!(
                area.cx - area.radius >= 0.0 && area.cx + area.radius <= 2400.0,
                "botón {:?} en x={} r={} fuera de límites horizontales 0..2400",
                area.button, area.cx, area.radius
            );
        }
    }

    /// Valida que múltiples resoluciones estándar (16:9, HD, 2K) mantengan todos
    /// los controles dentro de los límites de la pantalla y con radios positivos.
    #[test]
    fn test_touch_controller_various_resolutions() {
        let resolutions = [
            (1080, 1920), // 16:9 portrait
            (720, 1280),  // HD portrait
            (1920, 1080), // 16:9 landscape
            (1280, 720),  // HD landscape
            (2560, 1440), // 2K landscape
        ];

        for (w, h) in resolutions {
            let mut tc = TouchController::new();
            tc.resize(w, h);
            assert_eq!(tc.size(), (w, h));

            let (wf, hf) = (w as f32, h as f32);
            for area in tc.areas() {
                assert!(area.radius > 0.0, "radio positivo para {:?}", area.button);
                assert!(
                    area.cx - area.radius >= 0.0 && area.cx + area.radius <= wf,
                    "botón {:?} en ({}, {}) r={} fuera de límites horizontales 0..{}",
                    area.button, area.cx, area.cy, area.radius, w
                );
                assert!(
                    area.cy - area.radius >= 0.0 && area.cy + area.radius <= hf,
                    "botón {:?} en ({}, {}) r={} fuera de límites verticales 0..{}",
                    area.button, area.cx, area.cy, area.radius, h
                );
            }
        }
    }
}
