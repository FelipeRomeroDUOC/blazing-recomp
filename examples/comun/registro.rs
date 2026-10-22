//! Lectura y variación de registros de hardware del GBA, para las comparaciones por
//! variante de `DISPCNT`.

/// Bit de OBJ en `DISPCNT`.
pub const BIT_OBJ: u16 = 0x1000;
/// Bits de las capas de fondo en `DISPCNT` (BG0-BG3 y OBJ).
pub const BITS_CAPAS: u16 = 0x1F00;

/// Lee un registro de 16 bits de la copia de E/S.
pub fn io16(io: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([io[off], io[off + 1]])
}

/// `DISPCNT` de cada variante: 0 completa, 1 sin OBJ, 2 sólo OBJ, 3 nada.
///
/// `ppu_capas_linea.rs` tiene su propia versión con 11 variantes leídas de una tabla
/// `CAPAS`; no es una copia de esta y se queda donde está.
pub fn dispcnt_de(variante: usize, base: u16) -> u16 {
    match variante {
        0 => base,
        1 => base & !BIT_OBJ,
        2 => (base & !BITS_CAPAS) | BIT_OBJ,
        _ => base & !BITS_CAPAS,
    }
}

/// Copia de `io` con el `DISPCNT` de la variante.
pub fn io_con_variante(io: &[u8], variante: usize) -> Vec<u8> {
    let mut out = io.to_vec();
    let d = dispcnt_de(variante, io16(io, 0));
    out[0] = (d & 0xFF) as u8;
    out[1] = (d >> 8) as u8;
    out
}
