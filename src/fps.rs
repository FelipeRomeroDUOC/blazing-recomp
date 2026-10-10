//! Contador de FPS y generador de HUD visual en pantalla.
//!
//! Calcula la tasa de refresco en tiempo real con amortiguación y rasteriza
//! una pastilla (badge) visual estilizada en un búfer RGBA de 80x26 px utilizando
//! la tipografía vectorial [`crate::overlay::JETBRAINS_FONT_BYTES`].
//!
//! Este módulo es puro, sin dependencias de Android, y testeable en host.

use std::time::{Duration, Instant};

pub const BADGE_WIDTH: usize = 216;
pub const BADGE_HEIGHT: usize = 34;

/// Contador de fotogramas y generador del badge visual de FPS.
pub struct FpsCounter {
    font: fontdue::Font,
    last_interval_start: Instant,
    frames_in_interval: u32,
    current_fps: f32,
    last_rendered: Option<(u32, bool, &'static str)>,
    is_native: bool,
    renderer: &'static str,
    dirty: bool,
    buffer: Vec<[u8; 4]>,
}

impl FpsCounter {
    /// Inicializa el contador cargando la tipografía JetBrains Mono de assets.
    pub fn new() -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(
            crate::overlay::JETBRAINS_FONT_BYTES,
            fontdue::FontSettings::default(),
        )
        .map_err(|e| format!("fallo al cargar tipografía JetBrains Mono: {e}"))?;

        let mut counter = Self {
            font,
            last_interval_start: Instant::now(),
            frames_in_interval: 0,
            current_fps: 60.0,
            last_rendered: None,
            is_native: true,
            renderer: "Vulkan",
            dirty: true,
            buffer: vec![[0, 0, 0, 0]; BADGE_WIDTH * BADGE_HEIGHT],
        };

        counter.render_badge(60);
        counter.last_rendered = Some((60, true, "Vulkan"));
        counter.dirty = true;
        Ok(counter)
    }

    /// Actualiza el modo del motor (true = AOT nativo C11, false = Intérprete).
    pub fn set_native(&mut self, is_native: bool) {
        if self.is_native != is_native {
            self.is_native = is_native;
            let rounded = self.current_fps.round().clamp(0.0, 999.0) as u32;
            self.render_badge(rounded);
            self.last_rendered = Some((rounded, is_native, self.renderer));
            self.dirty = true;
        }
    }

    /// Indica si el motor está reportando modo nativo AOT.
    #[inline]
    pub fn is_native(&self) -> bool {
        self.is_native
    }

    /// Actualiza el nombre del renderer activo de la GPU (ej. "Vulkan" u "OpenGL").
    pub fn set_renderer(&mut self, renderer: &'static str) {
        if self.renderer != renderer {
            self.renderer = renderer;
            let rounded = self.current_fps.round().clamp(0.0, 999.0) as u32;
            self.render_badge(rounded);
            self.last_rendered = Some((rounded, self.is_native, renderer));
            self.dirty = true;
        }
    }

    /// Nombre del backend gráfico activo reportado en el badge.
    #[inline]
    pub fn renderer(&self) -> &'static str {
        self.renderer
    }

    /// Dimensiones del badge en píxeles `(ancho, alto)`.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (BADGE_WIDTH as u32, BADGE_HEIGHT as u32)
    }

    /// Tasa de fotogramas actual estimada.
    #[inline]
    pub fn fps(&self) -> f32 {
        self.current_fps
    }

    /// Indica si la textura en GPU necesita actualizarse con nuevos píxeles.
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marca la textura como sincronizada tras subirla a la GPU.
    #[inline]
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Píxeles RGBA del badge actual (216x34).
    #[inline]
    pub fn badge_rgba(&self) -> &[[u8; 4]] {
        &self.buffer
    }

    /// Registra el avance de un solo fotograma en el instante actual.
    pub fn on_frame(&mut self) {
        self.on_frames(1);
    }

    /// Registra el avance de `count` fotogramas de simulación en el instante actual.
    pub fn on_frames(&mut self, count: u32) {
        self.on_frames_at(count, Instant::now());
    }

    /// Registra el avance de un solo fotograma con un timestamp explícito (tests deterministas).
    pub fn on_frame_at(&mut self, now: Instant) {
        self.on_frames_at(1, now);
    }

    /// Registra el avance de `count` fotogramas con un timestamp explícito (tests deterministas).
    pub fn on_frames_at(&mut self, count: u32, now: Instant) {
        self.frames_in_interval += count;
        let elapsed = now.duration_since(self.last_interval_start);

        // Actualizar medición cada 350 ms para estabilidad visual sin jitter
        if elapsed >= Duration::from_millis(350) {
            let secs = elapsed.as_secs_f32();
            if secs > 0.0 {
                self.current_fps = (self.frames_in_interval as f32) / secs;
                let rounded = self.current_fps.round().clamp(0.0, 999.0) as u32;

                if self.last_rendered != Some((rounded, self.is_native, self.renderer)) {
                    self.render_badge(rounded);
                    self.last_rendered = Some((rounded, self.is_native, self.renderer));
                    self.dirty = true;
                }
            }
            self.frames_in_interval = 0;
            self.last_interval_start = now;
        }
    }

    /// Rasteriza el badge con fondo de pastilla translúcida y texto con la tipografía de assets.
    pub fn render_badge(&mut self, fps_val: u32) {
        // 1. Limpieza de fondo: pastilla oscura con bordes redondeados (radio 8)
        let w = BADGE_WIDTH;
        let h = BADGE_HEIGHT;
        let corner_r = 8.0f32;

        let bg_color = [12u8, 16u8, 24u8, 205u8]; // Slate translúcido al 80%
        let border_color = [255u8, 255u8, 255u8, 38u8]; // Borde sutil al 15%

        for y in 0..h {
            for x in 0..w {
                // Cálculo de distancia a esquinas redondeadas
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                let dx = (px - corner_r).min(0.0).abs().max((px - (w as f32 - corner_r)).max(0.0));
                let dy = (py - corner_r).min(0.0).abs().max((py - (h as f32 - corner_r)).max(0.0));
                let dist = (dx * dx + dy * dy).sqrt();

                let inside = dist <= corner_r;
                let idx = y * w + x;

                if inside {
                    // Detectar si está en el borde de 1px
                    let is_border = x == 0 || x == w - 1 || y == 0 || y == h - 1 || dist >= (corner_r - 1.0);
                    if is_border {
                        self.buffer[idx] = border_color;
                    } else {
                        self.buffer[idx] = bg_color;
                    }
                } else {
                    self.buffer[idx] = [0, 0, 0, 0];
                }
            }
        }

        // 2. Selección de color del texto según rendimiento
        let text_color = if fps_val >= 58 {
            [74u8, 222u8, 128u8] // Verde esmeralda (#4ADE80)
        } else if fps_val >= 45 {
            [251u8, 191u8, 36u8] // Ámbar (#FBBF24)
        } else {
            [248u8, 113u8, 113u8] // Carmesí (#F87171)
        };

        // 3. Rasterizado del texto (ej. "60 FPS · AOT · Vulkan")
        let mode_tag = if self.is_native { "AOT" } else { "INT" };
        let text = format!("{fps_val} FPS · {mode_tag} · {}", self.renderer);
        let font_size = 15.5f32;

        // Calcular ancho total para centrado
        let mut total_w = 0i32;
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, font_size);
            total_w += metrics.advance_width.round() as i32;
        }

        let start_x = ((w as i32 - total_w) / 2).max(4);
        let baseline_y = 23i32; // Línea base visual dentro de 34px de alto

        let mut cur_x = start_x;
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, font_size);
            let char_x = cur_x + metrics.xmin;
            let char_y = baseline_y - metrics.ymin - metrics.height as i32;

            for r in 0..metrics.height {
                for c in 0..metrics.width {
                    let px = char_x + c as i32;
                    let py = char_y + r as i32;

                    if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                        let alpha = bitmap[r * metrics.width + c];
                        if alpha > 0 {
                            let idx = (py as usize) * w + (px as usize);
                            let bg = self.buffer[idx];
                            let a_factor = (alpha as f32) / 255.0;

                            // Alpha compositing (over)
                            let r_out = ((text_color[0] as f32) * a_factor + (bg[0] as f32) * (1.0 - a_factor)).round() as u8;
                            let g_out = ((text_color[1] as f32) * a_factor + (bg[1] as f32) * (1.0 - a_factor)).round() as u8;
                            let b_out = ((text_color[2] as f32) * a_factor + (bg[2] as f32) * (1.0 - a_factor)).round() as u8;
                            let a_out = ((255.0 * a_factor) + (bg[3] as f32) * (1.0 - a_factor)).round().min(255.0) as u8;

                            self.buffer[idx] = [r_out, g_out, b_out, a_out];
                        }
                    }
                }
            }
            cur_x += metrics.advance_width.round() as i32;
        }
    }
}

#[cfg(test)]
#[path = "fps_tests.rs"]
mod tests;
