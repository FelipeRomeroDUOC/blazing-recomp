//! Conversión de color y comparación de píxeles entre el juego y una ruta de dibujo.

/// Diferencia máxima por canal que se considera ruido de redondeo (±1 nivel).
pub const TOLERANCIA: u8 = 1;

/// Canal de 5 bits a 8 bits, con la misma fórmula que el shim.
pub fn canal_8bit(v: u16) -> u8 {
    (((v & 0x1F) as u32 * 255 + 15) / 31) as u8
}

/// Pasa el framebuffer RGB555 de gba-core a RGB de 8 bits por píxel.
pub fn software_a_rgb(fb: &[u16]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(fb.len() * 3);
    for &c in fb {
        rgb.push(canal_8bit(c));
        rgb.push(canal_8bit(c >> 5));
        rgb.push(canal_8bit(c >> 10));
    }
    rgb
}

/// Dos imágenes RGB de 8 bits: el píxel `p` coincide si cada canal difiere como mucho
/// `TOLERANCIA`.
pub fn cerca(a: &[u8], b: &[u8], p: usize) -> bool {
    (0..3).all(|k| a[p * 3 + k].abs_diff(b[p * 3 + k]) <= TOLERANCIA)
}

/// Lo mismo para un par de píxeles sueltos, que es la forma que usan la mayoría de los
/// experimentos al recorrer los dos búferes a la vez.
pub fn cerca_pixel(a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|k| a[k].abs_diff(b[k]) <= TOLERANCIA)
}
