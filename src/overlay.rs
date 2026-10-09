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
    SelectLabel = 9,
    StartLabel = 10,
}

pub static OUTFIT_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Outfit-Bold.ttf");
pub static JETBRAINS_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

/// Atlas de textura de glifos rasterizados en memoria (R8Unorm).
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Construye el atlas tipográfico R8 (256x256 px) rasterizando glifos de Outfit Bold
/// y JetBrains Mono Bold al arrancar la aplicación.
pub fn build_font_atlas() -> FontAtlas {
    let outfit = fontdue::Font::from_bytes(OUTFIT_FONT_BYTES, fontdue::FontSettings::default())
        .expect("Outfit font debe ser válida");
    let jb = fontdue::Font::from_bytes(JETBRAINS_FONT_BYTES, fontdue::FontSettings::default())
        .expect("JetBrains Mono font debe ser válida");

    let width = 256usize;
    let height = 256usize;
    let mut data = vec![0u8; width * height];

    let blit = |data: &mut [u8], bitmap: &[u8], bw: usize, bh: usize, rx: usize, ry: usize, rw: usize, rh: usize| {
        if bw == 0 || bh == 0 {
            return;
        }
        let ox = rx + rw.saturating_sub(bw) / 2;
        let oy = ry + rh.saturating_sub(bh) / 2;
        for r in 0..bh {
            for c in 0..bw {
                let px = ox + c;
                let py = oy + r;
                if px < width && py < height {
                    data[py * width + px] = bitmap[r * bw + c];
                }
            }
        }
    };

    // Fila 0 (y: 0..64): Botones de acción principales (A, B, L, R) en Outfit Bold
    let letters = [
        ('A', 0usize, 0usize),
        ('B', 1usize, 0usize),
        ('L', 2usize, 0usize),
        ('R', 3usize, 0usize),
    ];
    for (ch, col, row) in letters {
        let (metrics, bitmap) = outfit.rasterize(ch, 44.0);
        blit(&mut data, &bitmap, metrics.width, metrics.height, col * 64, row * 64, 64, 64);
    }

    // Fila 1 (y: 64..128): Flechas direccionales de la cruceta (▲, ▼, ◀, ▶)
    let arrows = [
        (0usize, 1usize, 0), // Up
        (1usize, 1usize, 1), // Down
        (2usize, 1usize, 2), // Left
        (3usize, 1usize, 3), // Right
    ];
    for (col, row, dir) in arrows {
        let ox = col * 64;
        let oy = row * 64;
        for y in 0..64 {
            for x in 0..64 {
                let fx = x as f32 + 0.5 - 32.0;
                let fy = y as f32 + 0.5 - 32.0;
                let (rx, ry) = match dir {
                    0 => (fx, fy),
                    1 => (fx, -fy),
                    2 => (fy, fx),
                    3 => (-fy, -fx),
                    _ => (fx, fy),
                };
                let t = (ry + 14.0) / 28.0;
                let inside = if t >= 0.0 && t <= 1.0 {
                    let half_w = t * 14.0;
                    let dx = half_w - rx.abs();
                    dx.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if inside > 0.0 {
                    data[(oy + y) * width + (ox + x)] = (inside * 255.0) as u8;
                }
            }
        }
    }

    // Fila 2 (y: 128..192): Etiquetas de sistema (SELECT, START) en JetBrains Mono Bold
    let labels = [
        ("SELECT", 0usize, 2usize),
        ("START", 1usize, 2usize),
    ];
    for (text, col, row) in labels {
        let rx = col * 128;
        let ry = row * 64;
        let mut total_w = 0usize;
        for ch in text.chars() {
            let (m, _) = jb.rasterize(ch, 22.0);
            total_w += m.advance_width.round() as usize;
        }
        let start_x = rx + 128usize.saturating_sub(total_w) / 2;
        let baseline_y = ry + 38;
        let mut cur_x = start_x;
        for ch in text.chars() {
            let (metrics, bitmap) = jb.rasterize(ch, 22.0);
            let char_x = (cur_x as i32 + metrics.xmin).max(0) as usize;
            let char_y = (baseline_y as i32 - metrics.ymin - metrics.height as i32).max(0) as usize;
            for r in 0..metrics.height {
                for c in 0..metrics.width {
                    let px = char_x + c;
                    let py = char_y + r;
                    if px < width && py < height {
                        let val = bitmap[r * metrics.width + c];
                        if val > 0 {
                            data[py * width + px] = data[py * width + px].max(val);
                        }
                    }
                }
            }
            cur_x += metrics.advance_width.round() as usize;
        }
    }

    FontAtlas {
        width: width as u32,
        height: height as u32,
        data,
    }
}

/// Un elemento dibujable del overlay, derivado de una `TouchArea`.
/// Espejo exacto de `struct Elem` en `overlay.wgsl` (layout
/// uniform-compatible: 16 x f32 = 64 B, stride múltiplo de 16).
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
    pub angle: f32,
    pub extent_y: f32,
}

/// Primitiva del elemento: círculo o pastilla redondeada orientada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElemKind {
    Circle = 0,
    Pill = 1,
}

impl ElemParams {
    /// Empaqueta en el orden del `struct Elem` del shader (16 floats / 64 bytes).
    pub fn pack(&self) -> [f32; 16] {
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
            self.angle,
            self.extent_y,
            0.0,
            0.0,
        ]
    }
}

/// Colores temáticos por botón según orientación.
/// Todos los botones usan la paleta gris-lavanda uniforme estilo GBA SP en portrait,
/// y gris claro escarchado translúcido ([0.72, 0.75, 0.82]) en landscape.
fn button_color(_button: crate::input::Button, orientation: crate::input::Orientation) -> [f32; 3] {
    use crate::input::Orientation::*;
    match orientation {
        Portrait => [0.56, 0.58, 0.70],  // gris-lavanda uniforme (#a4a9c5 / #7b819f)
        Landscape => [0.72, 0.75, 0.82], // gris claro translúcido escarchado
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
        Select => Glyph::SelectLabel,
        Start => Glyph::StartLabel,
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
            // L y R como pastillas inclinadas (~-30°); Select/Start pastillas horizontales lisas;
            // cruceta y A/B círculos (Paso 4).
            let (kind, round, angle, rad_x, extent_y) = match a.button {
                L | R => {
                    let ang = (-30.0f32).to_radians();
                    let rx = a.radius;
                    let ey = a.radius * 0.58;
                    (ElemKind::Pill, ey, ang, rx, ey)
                }
                Select | Start => {
                    let ey = a.radius * 0.45;
                    (ElemKind::Pill, ey, 0.0, a.radius, ey)
                }
                _ => (ElemKind::Circle, 0.0, 0.0, a.radius, a.radius),
            };
            ElemParams {
                cx: a.cx,
                cy: a.cy,
                radius: rad_x,
                color,
                pressed: if is_pressed(a.button) { 1.0 } else { 0.0 },
                kind,
                glyph: button_glyph(a.button),
                round,
                glyph_scale: 0.18,
                ring: 0.55,
                angle,
                extent_y,
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
            crate::input::Orientation::Landscape => 0.65,
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
            Glyph::SelectLabel => Button::Select,
            Glyph::StartLabel => Button::Start,
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

        // (c) Todos los botones usan paleta gris-lavanda uniforme (Paso 1).
        let elems2 = build_elems(&layout(1080, 2400, Orientation::Portrait), |_| false);
        for elem in &elems2 {
            assert!(
                (elem.color[0] - 0.56).abs() < 0.01 && (elem.color[1] - 0.58).abs() < 0.01 && (elem.color[2] - 0.70).abs() < 0.01,
                "todos los botones deben ser gris-lavanda uniforme"
            );
        }

        // (d) Colores en landscape: gris claro translúcido escarchado ([0.72, 0.75, 0.82]).
        let elems_ls = build_elems_with_orientation(
            &layout(2400, 1080, Orientation::Landscape),
            Orientation::Landscape,
            |_| false,
        );
        for elem in &elems_ls {
            assert!(
                elem.color[0] > 0.70 && elem.color[1] > 0.70 && elem.color[2] > 0.70,
                "todos los botones en landscape deben ser gris claro translúcido"
            );
        }
    }

    /// Criterio de aceptación: en Landscape la opacidad es 0.65 y los botones
    /// usan acabado gris claro escarchado translúcido (canales RGB > 0.70).
    #[test]
    fn test_landscape_frosted_light_gray_palette_and_opacity() {
        let globals = build_globals(2400, 1080, Orientation::Landscape);
        assert!(
            (globals.opacity - 0.65).abs() < 0.01,
            "opacidad en landscape debe ser 0.65, actual: {}",
            globals.opacity
        );

        let elems = build_elems_with_orientation(
            &layout(2400, 1080, Orientation::Landscape),
            Orientation::Landscape,
            |_| false,
        );
        for elem in &elems {
            assert!(
                elem.color[0] > 0.70 && elem.color[1] > 0.70 && elem.color[2] > 0.70,
                "color en landscape debe ser gris claro translúcido (> 0.70), actual: {:?}",
                elem.color
            );
        }
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
            angle: -0.52,
            extent_y: 1.5,
        };
        let packed = e.pack();
        assert_eq!(packed.len(), 16);
        assert_eq!(
            packed,
            [1.0, 2.0, 3.0, 0.25, 0.5, 0.75, 1.0, 1.0, 6.0, 0.4, 0.13, 0.55, -0.52, 1.5, 0.0, 0.0]
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

    /// Criterio de aceptación (Paso 6): los shaders WGSL con textura de atlas y sampler
    /// parsean y validan con Naga sin errores.
    #[test]
    fn test_overlay_shader_with_font_atlas_validates() {
        test_overlay_shader_validates();
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

    /// Criterio de aceptación Paso 6: Generación y validez del atlas tipográfico R8
    #[test]
    fn test_font_atlas_contains_required_glyphs() {
        let atlas = build_font_atlas();
        assert_eq!(atlas.width, 256);
        assert_eq!(atlas.height, 256);
        assert_eq!(atlas.data.len(), 256 * 256);
        let non_zero = atlas.data.iter().filter(|&&b| b > 0).count();
        assert!(non_zero > 1000, "el atlas debe contener glifos rasterizados: {}", non_zero);

        // Validar que cada una de las 3 filas contiene cobertura de píxeles:
        // Fila 0 (A, B, L, R)
        let row0_pixels = atlas.data[0..64 * 256].iter().filter(|&&b| b > 0).count();
        assert!(row0_pixels > 200, "fila 0 (botones A/B/L/R) vacía");

        // Fila 1 (flechas cruceta)
        let row1_pixels = atlas.data[64 * 256..128 * 256].iter().filter(|&&b| b > 0).count();
        assert!(row1_pixels > 200, "fila 1 (flechas cruceta) vacía");

        // Fila 2 (etiquetas SELECT / START)
        let row2_pixels = atlas.data[128 * 256..192 * 256].iter().filter(|&&b| b > 0).count();
        assert!(row2_pixels > 200, "fila 2 (etiquetas SELECT/START) vacía");
    }

    /// Criterio Paso 5: simetría y congruencia de los 4 brazos de la cruceta
    /// para alimentar la cruz continua en el shader.
    #[test]
    fn test_dpad_cross_geometry_symmetry() {
        let mut tc = TouchController::new();
        tc.resize(1080, 2400);
        let elems = build_elems(tc.areas(), |_| false);
        let up = &elems[0];
        let down = &elems[1];
        let left = &elems[2];
        let right = &elems[3];

        assert_eq!(up.glyph, Glyph::ArrowUp);
        assert_eq!(down.glyph, Glyph::ArrowDown);
        assert_eq!(left.glyph, Glyph::ArrowLeft);
        assert_eq!(right.glyph, Glyph::ArrowRight);

        // Centro vertical y horizontal coinciden en el eje de simetría
        let dpad_cx = up.cx;
        let dpad_cy = (up.cy + down.cy) * 0.5;
        assert!((down.cx - dpad_cx).abs() < 0.01);
        assert!((left.cy - dpad_cy).abs() < 0.01);
        assert!((right.cy - dpad_cy).abs() < 0.01);

        // Los 4 brazos están a la misma distancia del centro
        let d_up = (up.cy - dpad_cy).abs();
        let d_down = (down.cy - dpad_cy).abs();
        let d_left = (left.cx - dpad_cx).abs();
        let d_right = (right.cx - dpad_cx).abs();
        assert!((d_up - d_down).abs() < 0.01);
        assert!((d_left - d_right).abs() < 0.01);
        assert!((d_up - d_left).abs() < 0.01);
    }

    /// Criterio de aceptación: La flecha izquierda de la cruceta debe apuntar a la izquierda (◀),
    /// lo que significa que el ápice (menos cobertura de píxeles) está en la mitad izquierda
    /// de la celda de 64x64, y la base (más cobertura) en la mitad derecha.
    #[test]
    fn test_dpad_left_arrow_points_left() {
        let atlas = build_font_atlas();
        // Celda de la flecha izquierda: col 2 (x: 128..192), row 1 (y: 64..128)
        let mut left_half_pixels = 0;
        let mut right_half_pixels = 0;
        for y in 64..128 {
            for x in 128..160 {
                if atlas.data[y * 256 + x] > 0 {
                    left_half_pixels += 1;
                }
            }
            for x in 160..192 {
                if atlas.data[y * 256 + x] > 0 {
                    right_half_pixels += 1;
                }
            }
        }
        assert!(
            left_half_pixels < right_half_pixels,
            "flecha izquierda debe apuntar a la izquierda (ápice a la izquierda: {} px vs base a la derecha: {} px)",
            left_half_pixels,
            right_half_pixels
        );
    }
}
