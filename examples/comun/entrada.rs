//! Lectura de las secuencias de entrada (`<cuadros> <botones>` por línea).
//!
//! El formato es el mismo en los 47 experimentos: cada línea son los cuadros que se
//! mantiene pulsado un conjunto de botones, los comentarios empiezan por `#` y las
//! líneas vacías se ignoran.
//!
//! Había 45 copias de este parser en dos variantes (una devolvía tuplas y otra, el
//! nombre del tramo). Aquí hay **un** parser y dos formas públicas de pedirlo.

/// Nombres de los botones, en el orden de bits del KEYINPUT del GBA.
pub const BOTONES: [&str; 10] = [
    "A", "B", "Select", "Start", "Right", "Left", "Up", "Down", "R", "L",
];

/// Un tramo de la secuencia: cuántos cuadros y con qué botones.
pub struct Tramo {
    /// Texto de botones tal como venía en la línea (`ninguno`, `A+B`, ...).
    pub nombre: String,
    /// Cuántos cuadros dura el tramo.
    pub cuadros: u32,
    /// Bits en 1 = botón presionado. Se convierte a formato activo en bajo al aplicarlo.
    pub presionados: u16,
}

/// El parser, único: las dos formas públicas son proyecciones de esto.
fn analizar(texto: &str) -> Result<Vec<Tramo>, String> {
    let mut tramos = Vec::new();
    for (n, linea) in texto.lines().enumerate() {
        let linea = linea.split('#').next().unwrap_or("").trim();
        if linea.is_empty() {
            continue;
        }
        let mut partes = linea.split_whitespace();
        let cuadros: u32 = partes
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("línea {}: se esperaba un número de cuadros", n + 1))?;
        let nombre = partes
            .next()
            .ok_or_else(|| format!("línea {}: faltan los botones", n + 1))?
            .to_string();
        let mut presionados: u16 = 0;
        if !nombre.eq_ignore_ascii_case("ninguno") {
            for boton in nombre.split('+') {
                let bit = BOTONES
                    .iter()
                    .position(|b| b.eq_ignore_ascii_case(boton))
                    .ok_or_else(|| format!("línea {}: botón desconocido '{}'", n + 1, boton))?;
                presionados |= 1 << bit;
            }
        }
        tramos.push(Tramo {
            nombre,
            cuadros,
            presionados,
        });
    }
    Ok(tramos)
}

/// Forma de 44 experimentos: `(cuadros, botones)` por tramo.
pub fn leer_secuencia(texto: &str) -> Result<Vec<(u32, u16)>, String> {
    Ok(analizar(texto)?
        .into_iter()
        .map(|tramo| (tramo.cuadros, tramo.presionados))
        .collect())
}

/// Forma de `ppu_compare`: el tramo entero, con el texto de botones.
pub fn leer_tramos(texto: &str) -> Result<Vec<Tramo>, String> {
    analizar(texto)
}
