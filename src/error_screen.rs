//! Renderizador de pantalla de error por software y blindaje de GPU.
//!
//! Diseñado para presentar mensajes de diagnóstico legibles directamente en el
//! framebuffer de la ventana nativa (`ANativeWindow`) cuando la inicialización
//! gráfica acelerada (`wgpu`) falla (por ejemplo, en emuladores como BlueStacks
//! que carecen de driver Vulkan funcional o fallan en la negociación EGL) o
//! ante cualquier pánico no capturado en el hilo nativo.
//!
//! Este módulo es puro y compilable/testeable en host.

/// Negocia un formato de textura y un modo alfa seguros a partir de las
/// capacidades de la superficie reportadas por el adaptador.
///
/// Si la lista de formatos está vacía (comportamiento observado en ciertos
/// emuladores y controladores antes de negociar la ventana), devuelve un
/// formato estándar seguro (`Rgba8UnormSrgb`) en lugar de entrar en pánico por
/// indexación ávida fuera de rango (`caps.formats[0]`).
pub fn select_surface_format_and_alpha(
    formats: &[wgpu::TextureFormat],
    alpha_modes: &[wgpu::CompositeAlphaMode],
) -> (wgpu::TextureFormat, wgpu::CompositeAlphaMode) {
    let format = formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .or_else(|| formats.first().copied())
        .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

    let alpha_mode = alpha_modes
        .first()
        .copied()
        .unwrap_or(wgpu::CompositeAlphaMode::Auto);

    (format, alpha_mode)
}

/// Renderizador por software para presentar pantallas de error y diagnóstico.
pub struct ErrorScreenRenderer {
    font: fontdue::Font,
}

impl ErrorScreenRenderer {
    /// Carga el rasterizador de fuentes tipográficas para la pantalla de error.
    pub fn new() -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(
            crate::overlay::JETBRAINS_FONT_BYTES,
            fontdue::FontSettings::default(),
        )
        .map_err(|e| format!("fallo al cargar fuente para pantalla de error: {e}"))?;
        Ok(Self { font })
    }

    /// Renderiza un marco de error con fondo temático, barra de título y
    /// mensaje multilínea legible directamente en el búfer de píxeles provisto.
    ///
    /// - `buffer`: slice mutable con los bytes del framebuffer nativo.
    /// - `width`, `height`: dimensiones visibles de la ventana en píxeles.
    /// - `stride`: número de píxeles por fila en la memoria (stride >= width).
    /// - `bytes_per_pixel`: típicamente 4 para `RGBA_8888` / `RGBX_8888`.
    /// - `title`: título de la alerta (ej. `[BLAZING RECOMP] ERROR DE INICIALIZACIÓN`).
    /// - `message`: cuerpo del mensaje de error o volcado de pánico.
    pub fn render(
        &self,
        buffer: &mut [u8],
        width: usize,
        height: usize,
        stride: usize,
        bytes_per_pixel: usize,
        title: &str,
        message: &str,
    ) {
        if width == 0 || height == 0 || stride < width || bytes_per_pixel < 3 {
            return;
        }

        let total_needed = stride * height * bytes_per_pixel;
        if buffer.len() < total_needed {
            return;
        }

        // 1. Limpieza de fondo: rojo vino muy oscuro (#140505)
        let bg_r = 20u8;
        let bg_g = 5u8;
        let bg_b = 5u8;
        let bg_a = 255u8;

        for y in 0..height {
            let row_offset = y * stride * bytes_per_pixel;
            for x in 0..width {
                let px = row_offset + x * bytes_per_pixel;
                buffer[px] = bg_r;
                buffer[px + 1] = bg_g;
                buffer[px + 2] = bg_b;
                if bytes_per_pixel >= 4 {
                    buffer[px + 3] = bg_a;
                }
            }
        }

        // 2. Barra de encabezado superior (altura proporcional, mín 40px)
        let header_height = (height / 10).clamp(44, 70);
        let hdr_r = 160u8;
        let hdr_g = 25u8;
        let hdr_b = 25u8;

        for y in 0..header_height.min(height) {
            let row_offset = y * stride * bytes_per_pixel;
            for x in 0..width {
                let px = row_offset + x * bytes_per_pixel;
                buffer[px] = hdr_r;
                buffer[px + 1] = hdr_g;
                buffer[px + 2] = hdr_b;
            }
        }

        // 3. Dibujar título en blanco en la cabecera
        let title_font_size = (header_height as f32 * 0.45).clamp(16.0, 24.0);
        self.draw_text_line(
            buffer,
            width,
            height,
            stride,
            bytes_per_pixel,
            title,
            20,
            (header_height as f32 * 0.25) as usize,
            title_font_size,
            (255, 255, 255),
        );

        // 4. Dibujar cuerpo del mensaje con wrapping
        let body_font_size = (title_font_size * 0.8).clamp(14.0, 18.0);
        let line_spacing = (body_font_size * 1.45) as usize;
        let start_y = header_height + 24;
        let start_x = 24;
        let max_text_width = width.saturating_sub(start_x * 2);

        // Estimación de caracteres por línea para ajuste de texto
        let char_width_est = (body_font_size * 0.6).max(1.0) as usize;
        let max_chars_per_line = (max_text_width / char_width_est).max(20);

        let mut current_y = start_y;
        for raw_line in message.lines() {
            if current_y + line_spacing > height.saturating_sub(40) {
                // Si el mensaje es muy largo, indicar truncamiento
                self.draw_text_line(
                    buffer,
                    width,
                    height,
                    stride,
                    bytes_per_pixel,
                    "... [mensaje truncado por espacio en pantalla] ...",
                    start_x,
                    current_y,
                    body_font_size,
                    (200, 200, 100),
                );
                break;
            }

            if raw_line.is_empty() {
                current_y += line_spacing / 2;
                continue;
            }

            // Wrapping de palabras
            let mut line_buf = String::new();
            for word in raw_line.split_whitespace() {
                if !line_buf.is_empty() && line_buf.len() + word.len() + 1 > max_chars_per_line {
                    self.draw_text_line(
                        buffer,
                        width,
                        height,
                        stride,
                        bytes_per_pixel,
                        &line_buf,
                        start_x,
                        current_y,
                        body_font_size,
                        (240, 240, 240),
                    );
                    current_y += line_spacing;
                    line_buf.clear();
                    if current_y + line_spacing > height.saturating_sub(40) {
                        break;
                    }
                }
                if !line_buf.is_empty() {
                    line_buf.push(' ');
                }
                line_buf.push_str(word);
            }

            if !line_buf.is_empty() && current_y + line_spacing <= height.saturating_sub(40) {
                self.draw_text_line(
                    buffer,
                    width,
                    height,
                    stride,
                    bytes_per_pixel,
                    &line_buf,
                    start_x,
                    current_y,
                    body_font_size,
                    (240, 240, 240),
                );
                current_y += line_spacing;
            }
        }

        // 5. Pie de página con instrucción
        let footer = "Presiona Atrás para salir | Registro en logcat (BlazingRecomp)";
        let footer_y = height.saturating_sub(30);
        self.draw_text_line(
            buffer,
            width,
            height,
            stride,
            bytes_per_pixel,
            footer,
            start_x,
            footer_y,
            14.0,
            (180, 180, 140),
        );
    }

    /// Dibuja una línea de texto simple rasterizando cada glifo sobre el buffer.
    fn draw_text_line(
        &self,
        buffer: &mut [u8],
        width: usize,
        height: usize,
        stride: usize,
        bytes_per_pixel: usize,
        text: &str,
        origin_x: usize,
        origin_y: usize,
        font_size: f32,
        color: (u8, u8, u8),
    ) {
        let (cr, cg, cb) = color;
        let mut cur_x = origin_x;

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, font_size);
            if metrics.width > 0 && metrics.height > 0 {
                let glyph_x = (cur_x as i32 + metrics.xmin).max(0) as usize;
                let glyph_y = (origin_y as i32 + (font_size as i32 - metrics.height as i32 - metrics.ymin)).max(0) as usize;

                for row in 0..metrics.height {
                    let py = glyph_y + row;
                    if py >= height {
                        break;
                    }
                    let row_start = py * stride * bytes_per_pixel;
                    for col in 0..metrics.width {
                        let px = glyph_x + col;
                        if px >= width {
                            break;
                        }
                        let alpha = bitmap[row * metrics.width + col];
                        if alpha > 0 {
                            let idx = row_start + px * bytes_per_pixel;
                            let a = alpha as u32;
                            let inv_a = 255 - a;

                            // Alpha blending sobre el fondo existente
                            let r = (cr as u32 * a + buffer[idx] as u32 * inv_a) / 255;
                            let g = (cg as u32 * a + buffer[idx + 1] as u32 * inv_a) / 255;
                            let b = (cb as u32 * a + buffer[idx + 2] as u32 * inv_a) / 255;

                            buffer[idx] = r as u8;
                            buffer[idx + 1] = g as u8;
                            buffer[idx + 2] = b as u8;
                        }
                    }
                }
            }

            cur_x += metrics.advance_width.round().max(1.0) as usize;
            if cur_x >= width {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_fallback_empty_capabilities() {
        let empty_formats = [];
        let empty_alphas = [];

        // Debe devolver formato y alpha por defecto sin entrar en pánico
        let (format, alpha) = select_surface_format_and_alpha(&empty_formats, &empty_alphas);
        assert_eq!(
            format,
            wgpu::TextureFormat::Rgba8Unorm,
            "Con formatos vacíos debe usar Rgba8Unorm como fallback seguro"
        );
        assert_eq!(
            alpha,
            wgpu::CompositeAlphaMode::Auto,
            "Con alphas vacíos debe usar Auto como fallback seguro"
        );
    }

    #[test]
    fn test_surface_prefers_non_srgb() {
        let formats = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm,
        ];
        let alphas = [wgpu::CompositeAlphaMode::Opaque];

        let (format, alpha) = select_surface_format_and_alpha(&formats, &alphas);
        assert_eq!(
            format,
            wgpu::TextureFormat::Rgba8Unorm,
            "Debe priorizar el formato no-sRGB si está disponible"
        );
        assert_eq!(alpha, wgpu::CompositeAlphaMode::Opaque);
    }

    #[test]
    fn test_error_screen_renders_multiline_text() {
        let renderer = ErrorScreenRenderer::new().expect("Debe crear el renderer de pantalla de error");
        let width = 400;
        let height = 300;
        let stride = 400;
        let bpp = 4;
        let mut buffer = vec![0u8; stride * height * bpp];

        let title = "TEST ERROR TITLE";
        let message = "Línea 1: Fallo crítico de inicialización GPU.\nLínea 2: Código de adaptador no encontrado en BlueStacks.\nLínea 3: Reintentando...";

        renderer.render(&mut buffer, width, height, stride, bpp, title, message);

        // Verificar que los píxeles no son todos ceros (se dibujó el fondo y el texto)
        let modified_count = buffer.iter().filter(|&&b| b > 0).count();
        assert!(
            modified_count > (width * height * 2),
            "El buffer debe contener píxeles de color de fondo y glifos renderizados"
        );
    }

    #[test]
    fn test_error_screen_handles_zero_or_small_dimensions() {
        let renderer = ErrorScreenRenderer::new().expect("Debe crear el renderer");
        let mut buffer = vec![0u8; 100];

        // Dimensiones cero o buffer insuficiente no deben provocar pánicos
        renderer.render(&mut buffer, 0, 0, 0, 4, "T", "M");
        renderer.render(&mut buffer, 100, 100, 100, 4, "T", "M"); // buffer demasiado pequeño para 100x100
    }

    #[test]
    fn test_error_screen_truncates_extremely_long_messages_safely() {
        let renderer = ErrorScreenRenderer::new().expect("Debe crear el renderer");
        let width = 300;
        let height = 200;
        let stride = 300;
        let bpp = 4;
        let mut buffer = vec![0u8; stride * height * bpp];

        // Mensaje con 80 líneas que sobrepasa con creces la altura de 200 px
        let mut huge_msg = String::new();
        for i in 0..80 {
            huge_msg.push_str(&format!("Volcado línea {i}: 0xDEADBEEF 0xCAFEBABE registro de depuración\n"));
        }

        // No debe entrar en pánico ni escribir fuera del slice
        renderer.render(&mut buffer, width, height, stride, bpp, "PÁNICO CRÍTICO", &huge_msg);

        let modified_count = buffer.iter().filter(|&&b| b > 0).count();
        assert!(modified_count > 0, "Debe haber píxeles dibujados a pesar del truncamiento");
    }

    #[test]
    fn test_surface_fallback_all_srgb_formats() {
        let srgb_formats = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ];
        let alphas = [wgpu::CompositeAlphaMode::PreMultiplied];

        // Si solo hay sRGB, debe tomar el primer formato disponible sin fallar
        let (fmt, alpha) = select_surface_format_and_alpha(&srgb_formats, &alphas);
        assert_eq!(fmt, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(alpha, wgpu::CompositeAlphaMode::PreMultiplied);
    }

    #[test]
    fn test_error_screen_stride_padding_and_memory_alignment() {
        let renderer = ErrorScreenRenderer::new().expect("Debe crear el renderer");
        let width = 200;
        let stride = 260; // 60 píxeles de padding por fila
        let height = 100;
        let bpp = 4;
        let mut buffer = vec![0u8; stride * height * bpp];

        renderer.render(&mut buffer, width, height, stride, bpp, "STRIDE TEST", "Línea con padding");

        // Comprobar que ninguna columna entre width (200) y stride (260) fue tocada
        for y in 0..height {
            let row_offset = y * stride * bpp;
            for x in width..stride {
                let px = row_offset + x * bpp;
                assert_eq!(
                    buffer[px..px + bpp],
                    [0, 0, 0, 0],
                    "El padding en fila {y}, columna {x} no debe ser modificado por el renderer"
                );
            }
        }
    }
}
