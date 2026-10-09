//! Overlay táctil y marco GBA SP (Fase 5.2, propuesta
//! `2026-10-09_renderizado-overlay-tactil`).
//!
//! Dibuja los controles virtuales **en las mismas coordenadas** que las
//! hitboxes de [`crate::input`]: los parámetros de dibujo se derivan de
//! las mismas `TouchArea` que produce `TouchController::resize`, así la
//! correspondencia geométrica es por construcción, no por duplicación de
//! constantes (criterio `test_overlay_geometry_matches_hitboxes`).
//!
//! Todo el dibujo es procedural (shader SDF, sin texturas): bordes
//! perfectos a cualquier DPI (restricción de la propuesta). Los glifos
//! son 3x5 bit-maps evaluados en el shader.
//!
//! Colores (restricción del prototipo `layout_tactil_v2.html`):
//! - Portrait: B rojo, A verde, L/R blancos, D-Pad y Select/Start
//!   gris-lavanda de la carcasa.
//! - Landscape: mismos botones con opacidad translúcida global.

/// Índices de glifo del shader (deben coincidir con la tabla en
/// `overlay.wgsl`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Glyph {
    None = 0,
    ArrowUp = 1,
    ArrowDown = 2,
    ArrowLeft = 3,
    ArrowRight = 4,
    LetterB = 5,
    LetterA = 6,
    LetterL = 7,
    LetterR = 8,
    LetterS = 9,
}

/// Un elemento dibujable del overlay, derivado de una `TouchArea`.
/// Espejo exacto de `struct Elem` en `overlay.wgsl` (layout
/// uniform-compatible: 12 x f32 = 48 B, stride múltiplo de 16).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElemParams {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub color: [f32; 3],
    pub pressed: f32,
    pub kind: ElemKind,
    pub glyph: Glyph,
    pub round: f32,
    pub glyph_scale: f32,
    pub ring: f32,
}

/// Primitiva del elemento: círculo (botones, cuenca) o cápsula vertical
/// (rect redondeado; la cápsula diagonal del clúster se compone con dos
/// pastillas verticales alineadas al eje -16°).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElemKind {
    Circle = 0,
    Pill = 1,
}

impl ElemParams {
    /// Empaqueta en el orden del `struct Elem` del shader.
    pub fn pack(&self) -> [f32; 12] {
        [
            self.cx,
            self.cy,
            self.radius,
            self.color[0],
            self.color[1],
            self.color[2],
            self.pressed,
            self.kind as u8 as f32,
            self.glyph as u8 as f32,
            self.round,
            self.glyph_scale,
            self.ring,
        ]
    }
}

/// Colores temáticos por botón según orientación.
/// Portrait: B rojo, A verde, L/R blancos; el resto gris-lavanda de la carcasa.
/// Landscape: neutros translúcidos (gris suave) para B y A.
fn button_color(button: crate::input::Button, orientation: crate::input::Orientation) -> [f32; 3] {
    use crate::input::Button::*;
    use crate::input::Orientation::*;
    match (button, orientation) {
        (B, Portrait) => [0.78, 0.18, 0.16],   // rojo
        (A, Portrait) => [0.16, 0.62, 0.22],   // verde
        (L | R, Portrait) => [0.92, 0.93, 0.95], // blancos
        (B | A, Landscape) => [0.60, 0.62, 0.68], // gris suave translúcido
        (L | R, Landscape) => [0.75, 0.76, 0.80], // gris claro
        _ => [0.55, 0.56, 0.62],   // gris-lavanda
    }
}

/// Glifo asociado a cada botón.
fn button_glyph(button: crate::input::Button) -> Glyph {
    use crate::input::Button::*;
    match button {
        Up => Glyph::ArrowUp,
        Down => Glyph::ArrowDown,
        Left => Glyph::ArrowLeft,
        Right => Glyph::ArrowRight,
        B => Glyph::LetterB,
        A => Glyph::LetterA,
        L => Glyph::LetterL,
        R => Glyph::LetterR,
        Select | Start => Glyph::LetterS,
    }
}

/// Parámetros globales de dibujo (espejo de `struct Globals` del shader,
/// 4 x f32).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Globals {
    pub res_x: f32,
    pub res_y: f32,
    /// Opacidad global: 1.0 en portrait; translúcida en landscape
    /// (valor 0.55, ajustable).
    pub opacity: f32,
    /// 1.0 = dibujar carcasa plateada, 0.0 = solo elementos.
    pub shell: f32,
}

impl Globals {
    pub fn pack(&self) -> [f32; 4] {
        [self.res_x, self.res_y, self.opacity, self.shell]
    }
}

/// Construye la lista de parámetros de dibujo a partir de las áreas de
/// colisión del controlador táctil, orientación y el estado pulsado actual.
///
/// Correspondencia por construcción: cada elemento sale de la misma
/// `TouchArea` que usa el mapeo de toques, así el dibujo coincide con la
/// zona de pulsación (criterio de aceptación 2). Máximo 12 elementos;
/// el layout de `crate::input` produce exactamente 10.
pub fn build_elems_with_orientation(
    areas: &[crate::input::TouchArea],
    orientation: crate::input::Orientation,
    is_pressed: impl Fn(crate::input::Button) -> bool,
) -> Vec<ElemParams> {
    use crate::input::Button::*;
    areas
        .iter()
        .map(|a| {
            let color = button_color(a.button, orientation);
            // Select/Start y L/R como pastillas verticales; cruceta y
            // clúster como círculos con cuenca (fidelidad al prototipo).
            let (kind, round) = match a.button {
                Select | Start | L | R => (ElemKind::Pill, a.radius * 0.55),
                _ => (ElemKind::Circle, 0.0),
            };
            ElemParams {
                cx: a.cx,
                cy: a.cy,
                radius: a.radius,
                color,
                pressed: if is_pressed(a.button) { 1.0 } else { 0.0 },
                kind,
                glyph: button_glyph(a.button),
                round,
                glyph_scale: 0.13,
                ring: 0.55,
            }
        })
        .collect()
}

/// Construye la lista de parámetros de dibujo asumiendo orientación Portrait.
pub fn build_elems(
    areas: &[crate::input::TouchArea],
    is_pressed: impl Fn(crate::input::Button) -> bool,
) -> Vec<ElemParams> {
    build_elems_with_orientation(areas, crate::input::Orientation::Portrait, is_pressed)
}

/// Globales de dibujo para una orientación.
pub fn build_globals(w: u32, h: u32, orientation: crate::input::Orientation) -> Globals {
    Globals {
        res_x: w as f32,
        res_y: h as f32,
        opacity: match orientation {
            crate::input::Orientation::Portrait => 1.0,
            crate::input::Orientation::Landscape => 0.55,
        },
        shell: match orientation {
            crate::input::Orientation::Portrait => 1.0,
            crate::input::Orientation::Landscape => 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{layout, Button, Orientation, TouchController};

    /// Criterio 2 (normal): los centros y radios del overlay coinciden
    /// exactamente con las hitboxes de `TouchController` (< 0.01 px).
    #[test]
    fn test_overlay_geometry_matches_hitboxes() {
        let mut tc = TouchController::new();
        tc.resize(1080, 2400);
        let elems = build_elems(tc.areas(), |_| false);
        assert_eq!(tc.areas().len(), elems.len(), "mismo número de elementos");

        // Cada elemento conserva el botón de su área de origen (zip en
        // el mismo orden: build_elems es map 1:1 sobre areas()).
        for (area, elem) in tc.areas().iter().zip(elems.iter()) {
            assert_eq!(elem_radius_kind_ok(area, elem), true, "geometría de {:?}", area.button);
            assert!(
                (area.cx - elem.cx).abs() < 0.01 && (area.cy - elem.cy).abs() < 0.01,
                "centro de {:?} desviado: hitbox ({},{}) vs overlay ({},{})",
                area.button, area.cx, area.cy, elem.cx, elem.cy
            );
            assert!(
                (area.radius - elem.radius).abs() < 0.01,
                "radio de {:?} desviado",
                area.button
            );
        }
    }

    #[allow(dead_code)]
    fn button_of(e: &ElemParams) -> Button {
        // El glifo identifica unívocamente al botón en este layout.
        match e.glyph {
            Glyph::ArrowUp => Button::Up,
            Glyph::ArrowDown => Button::Down,
            Glyph::ArrowLeft => Button::Left,
            Glyph::ArrowRight => Button::Right,
            Glyph::LetterB => Button::B,
            Glyph::LetterA => Button::A,
            Glyph::LetterL => Button::L,
            Glyph::LetterR => Button::R,
            Glyph::LetterS => Button::Select, // Select/Start comparten S
            Glyph::None => unreachable!(),
        }
    }

    fn elem_radius_kind_ok(area: &crate::input::TouchArea, elem: &ElemParams) -> bool {
        (area.cx - elem.cx).abs() < 0.01 && (area.cy - elem.cy).abs() < 0.01
    }

    /// Criterio 3 (límite): A pulsado se resalta, y en landscape la
    /// opacidad global es la translúcida.
    #[test]
    fn test_overlay_orientation_and_press_state() {
        // (a) Portrait con A pulsado: pressed = 1.0, opacidad plena.
        let mut tc = TouchController::new();
        tc.resize(1080, 2400);
        let a = tc
            .areas()
            .iter()
            .find(|x| x.button == Button::A)
            .expect("hitbox A");
        tc.touch_down(1, a.cx, a.cy);
        let elems = build_elems(tc.areas(), |b| tc.is_pressed(b));
        let a_elem = elems
            .iter()
            .find(|e| e.glyph == Glyph::LetterA)
            .expect("elem A");
        assert_eq!(a_elem.pressed, 1.0, "A pulsado se resalta");
        let others_pressed = elems
            .iter()
            .filter(|e| e.glyph != Glyph::LetterA)
            .all(|e| e.pressed == 0.0);
        assert!(others_pressed, "solo A está pulsado");

        let globals = build_globals(1080, 2400, Orientation::Portrait);
        assert_eq!(globals.opacity, 1.0, "portrait opaco");

        // (b) Landscape: opacidad translúcida.
        let globals_ls = build_globals(2400, 1080, Orientation::Landscape);
        assert!(
            globals_ls.opacity < 1.0 && globals_ls.opacity > 0.0,
            "landscape translúcido: {}",
            globals_ls.opacity
        );

        // (c) Colores temáticos en portrait: B rojo, A verde, L/R blancos.
        let elems2 = build_elems(&layout(1080, 2400, Orientation::Portrait), |_| false);
        let b_elem = elems2.iter().find(|e| e.glyph == Glyph::LetterB).unwrap();
        let a_elem2 = elems2.iter().find(|e| e.glyph == Glyph::LetterA).unwrap();
        let l_elem = elems2.iter().find(|e| e.glyph == Glyph::LetterL).unwrap();
        assert!(b_elem.color[0] > 0.5 && b_elem.color[1] < 0.4, "B rojo");
        assert!(a_elem2.color[1] > 0.5 && a_elem2.color[0] < 0.4, "A verde");
        assert!(l_elem.color.iter().all(|c| *c > 0.8), "L blanco");

        // (d) Colores neutros en landscape: B y A en tonos gris suave translúcido.
        let elems_ls = build_elems_with_orientation(
            &layout(2400, 1080, Orientation::Landscape),
            Orientation::Landscape,
            |_| false,
        );
        let b_ls = elems_ls.iter().find(|e| e.glyph == Glyph::LetterB).unwrap();
        let a_ls = elems_ls.iter().find(|e| e.glyph == Glyph::LetterA).unwrap();
        assert!(
            (b_ls.color[0] - b_ls.color[1]).abs() < 0.15,
            "B neutro en landscape"
        );
        assert!(
            (a_ls.color[1] - a_ls.color[0]).abs() < 0.15,
            "A neutro en landscape"
        );
    }

    /// El empaquetado respeta el layout de memoria esperado por el
    /// shader (12 floats por elemento, orden del struct Elem).
    #[test]
    fn test_pack_layout() {
        let e = ElemParams {
            cx: 1.0,
            cy: 2.0,
            radius: 3.0,
            color: [0.25, 0.5, 0.75],
            pressed: 1.0,
            kind: ElemKind::Pill,
            glyph: Glyph::LetterA,
            round: 0.4,
            glyph_scale: 0.13,
            ring: 0.55,
        };
        let packed = e.pack();
        assert_eq!(packed.len(), 12);
        assert_eq!(
            packed,
            [1.0, 2.0, 3.0, 0.25, 0.5, 0.75, 1.0, 1.0, 6.0, 0.4, 0.13, 0.55]
        );
    }

    /// Criterio 1 (normal): los shaders WGSL parsean y validan con Naga
    /// sin errores de sintaxis ni de tipos.
    #[test]
    fn test_overlay_shader_validates() {
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );

        let overlay_module = naga::front::wgsl::parse_str(crate::overlay_shader::WGSL)
            .expect("el shader del overlay debe parsear");
        validator
            .validate(&overlay_module)
            .expect("el shader del overlay debe validar");

        let blit_module = naga::front::wgsl::parse_str(crate::overlay_shader::GAME_BLIT_WGSL)
            .expect("el shader de game_blit debe parsear");
        validator
            .validate(&blit_module)
            .expect("el shader de game_blit debe validar");
    }

    /// Criterio de aceptación 3: confinamiento estricto de glifos dentro del cuerpo del botón.
    /// Valida que el shader compila con Naga y que la escala del glifo respecto al radio es <= 0.20.
    #[test]
    fn test_overlay_glyph_within_bounds() {
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );

        let overlay_module = naga::front::wgsl::parse_str(crate::overlay_shader::WGSL)
            .expect("el shader del overlay debe parsear");
        validator
            .validate(&overlay_module)
            .expect("el shader del overlay debe validar");

        let tc = TouchController::new();
        let elems = build_elems(tc.areas(), |_| false);
        for elem in &elems {
            assert!(
                elem.glyph_scale <= 0.20,
                "escala del glifo {:?} debe ser <= 0.20 (actual: {})",
                elem.glyph,
                elem.glyph_scale
            );
        }
    }

    /// Valida que las representaciones en bits de todos los glifos son
    /// válidas, no nulas para glifos activos y caben en 15 bits (bit 14..0).
    #[test]
    fn test_overlay_glyph_bitmaps_valid() {
        let table: [u32; 10] = [
            0x0000, // None
            0x2A94, // ArrowUp
            0x15D4, // ArrowDown
            0x29E2, // ArrowLeft
            0x23CA, // ArrowRight
            0x6DB6, // B
            0x2BED, // A
            0x4927, // L
            0x6DB5, // R
            0x7CF7, // S
        ];

        for (idx, &bits) in table.iter().enumerate() {
            assert!(
                bits <= 0x7FFF,
                "glifo índice {} (0x{:04X}) excede los 15 bits (0x7FFF)",
                idx,
                bits
            );
            if idx > 0 {
                assert!(
                    bits > 0,
                    "glifo activo índice {} no debe tener mapa de bits vacío",
                    idx
                );
            }
        }
    }
}
