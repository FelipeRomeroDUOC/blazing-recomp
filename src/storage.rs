use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Firma criptográfica SHA-256 oficial de la ROM europea de Fire Emblem (Europe) (16 MB).
pub const OFFICIAL_ROM_SHA256: &str =
    "6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9";

/// Firma criptográfica SHA-256 oficial de la BIOS de Game Boy Advance (16 KB).
pub const OFFICIAL_BIOS_SHA256: &str =
    "fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570";

/// Tamaño exacto de la ROM oficial de Fire Emblem (16 MB en bytes).
pub const OFFICIAL_ROM_SIZE: usize = 16 * 1024 * 1024;

/// Tamaño exacto del binario de BIOS oficial de GBA (16 KB en bytes).
pub const OFFICIAL_BIOS_SIZE: usize = 16 * 1024;

/// Nombres de archivo comunes con los que suele guardarse la ROM de Fire Emblem.
pub const KNOWN_ROM_FILENAMES: &[&str] = &[
    "fe7.gba",
    "Fire Emblem (Europe).gba",
    "Fire Emblem.gba",
    "1575 - Fire Emblem (E)(Independent).gba",
    "Fire Emblem (E).gba",
    "fe7_europe.gba",
    "game.gba",
];

/// Nombres de archivo comunes para la BIOS oficial de GBA.
pub const KNOWN_BIOS_FILENAMES: &[&str] = &[
    "gba_bios.bin",
    "bios.bin",
    "gba.bin",
];

/// Assets del juego cargados y verificados en memoria listos para la ejecución.
#[derive(Debug, Clone)]
pub struct LoadedAssets {
    /// Búfer con los 16 MB de la ROM oficial.
    pub rom: Vec<u8>,
    /// Búfer con los 16 KB de la BIOS oficial (si fue encontrada).
    pub bios: Option<Vec<u8>>,
    /// Ruta del archivo de ROM desde donde se cargó.
    pub rom_path: PathBuf,
    /// Ruta del archivo de BIOS desde donde se cargó (si aplica).
    pub bios_path: Option<PathBuf>,
}

/// Errores posibles al buscar, leer o validar los assets del juego.
#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// No se encontró ningún archivo cuya firma SHA-256 coincida con la ROM oficial.
    RomNotFound,
    /// Error de entrada/salida al leer los archivos en disco.
    IoError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RomNotFound => write!(
                f,
                "ROM oficial de Fire Emblem no encontrada o con firma no coincidente"
            ),
            Self::IoError(e) => write!(f, "Error de E/S al leer archivos del juego: {}", e),
        }
    }
}

impl std::error::Error for StorageError {}

/// Calcula la firma criptográfica SHA-256 de un búfer de bytes y la retorna en formato hexadecimal en minúsculas.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in result {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

/// Verifica si un búfer de datos coincide exactamente en tamaño y firma SHA-256 con la ROM oficial europea.
pub fn verify_rom(data: &[u8]) -> bool {
    if data.len() != OFFICIAL_ROM_SIZE {
        return false;
    }
    compute_sha256(data).eq_ignore_ascii_case(OFFICIAL_ROM_SHA256)
}

/// Verifica si un búfer de datos coincide exactamente en tamaño y firma SHA-256 con la BIOS oficial de GBA.
pub fn verify_bios(data: &[u8]) -> bool {
    if data.len() != OFFICIAL_BIOS_SIZE {
        return false;
    }
    compute_sha256(data).eq_ignore_ascii_case(OFFICIAL_BIOS_SHA256)
}

/// Genera la lista ordenada de directorios candidatos de búsqueda, priorizando la carpeta Downloads
/// y el almacenamiento propio de la app en Android 11-16.
pub fn resolve_search_directories(
    internal_data_dir: Option<&Path>,
    external_data_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // 1. Directorio externo propio de la app (/sdcard/Android/data/com.blazingrecomp.game/files/)
    // En Android 11 a 16 NUNCA requiere permisos y siempre es accesible directamente.
    if let Some(dir) = external_data_dir {
        dirs.push(dir.to_path_buf());
    }
    // 2. Android Downloads principal (almacenamiento compartido primario)
    dirs.push(PathBuf::from("/sdcard/Download"));
    // 3. Android Downloads secundario/emulado
    dirs.push(PathBuf::from("/storage/emulated/0/Download"));
    // 4. Subcarpeta GBA dentro de Downloads
    dirs.push(PathBuf::from("/sdcard/Download/GBA"));
    // 5. Carpeta Download relativa local (desarrollo o emulador de escritorio)
    dirs.push(PathBuf::from("Download"));
    // 6. Carpeta roms relativa local (para pruebas en host de desarrollo)
    dirs.push(PathBuf::from("roms"));
    // 7. Rutas alternativas en emuladores (BlueStacks, Nox, AVD Android 9)
    dirs.push(PathBuf::from("/mnt/sdcard/Download"));
    dirs.push(PathBuf::from("/storage/self/primary/Download"));
    // 8. Directorio interno privado de la aplicación provisto por AndroidApp
    if let Some(dir) = internal_data_dir {
        dirs.push(dir.to_path_buf());
    }
    dirs
}

/// Escanea los directorios especificados en busca de archivos de ROM y BIOS oficiales.
///
/// Combina sondeo directo de nombres estándar (`fe7.gba`, `Fire Emblem (Europe).gba`) con
/// escaneo general del directorio para máxima compatibilidad con Scoped Storage en Android 16.
pub fn scan_and_load_game(search_dirs: &[PathBuf]) -> Result<LoadedAssets, StorageError> {
    let mut found_rom: Option<(PathBuf, Vec<u8>)> = None;
    let mut found_bios: Option<(PathBuf, Vec<u8>)> = None;

    for dir in search_dirs {
        // Intento 1: Sondeo directo de archivos conocidos (funciona incluso si read_dir falla por permisos en Android 16)
        if found_rom.is_none() {
            for name in KNOWN_ROM_FILENAMES {
                let candidate = dir.join(name);
                if let Ok(meta) = std::fs::metadata(&candidate) {
                    if meta.len() as usize == OFFICIAL_ROM_SIZE {
                        if let Ok(bytes) = std::fs::read(&candidate) {
                            if verify_rom(&bytes) {
                                found_rom = Some((candidate, bytes));
                                break;
                            }
                        }
                    }
                }
            }
        }

        if found_bios.is_none() {
            for name in KNOWN_BIOS_FILENAMES {
                let candidate = dir.join(name);
                if let Ok(meta) = std::fs::metadata(&candidate) {
                    if meta.len() as usize == OFFICIAL_BIOS_SIZE {
                        if let Ok(bytes) = std::fs::read(&candidate) {
                            if verify_bios(&bytes) {
                                found_bios = Some((candidate, bytes));
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Intento 2: Si read_dir está permitido, inspeccionar todos los archivos con extensión .gba y .bin
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_size = match entry.metadata() {
                    Ok(meta) => meta.len() as usize,
                    Err(_) => continue,
                };

                // Evaluar posible ROM de 16 MB
                if found_rom.is_none() && file_size == OFFICIAL_ROM_SIZE {
                    let is_gba = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("gba"))
                        .unwrap_or(false);

                    if is_gba {
                        if let Ok(bytes) = std::fs::read(&path) {
                            if verify_rom(&bytes) {
                                found_rom = Some((path.clone(), bytes));
                            }
                        }
                    }
                }

                // Evaluar posible BIOS de 16 KB
                if found_bios.is_none() && file_size == OFFICIAL_BIOS_SIZE {
                    let is_bin = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("bin"))
                        .unwrap_or(false);

                    if is_bin {
                        if let Ok(bytes) = std::fs::read(&path) {
                            if verify_bios(&bytes) {
                                found_bios = Some((path.clone(), bytes));
                            }
                        }
                    }
                }
            }
        }

        // Si ya encontramos la ROM, podemos detener la búsqueda en otros directorios
        if found_rom.is_some() {
            break;
        }
    }

    match found_rom {
        Some((rom_path, rom_bytes)) => {
            let (bios_path, bios_bytes) = match found_bios {
                Some((bp, bb)) => (Some(bp), Some(bb)),
                None => (None, None),
            };

            Ok(LoadedAssets {
                rom: rom_bytes,
                bios: bios_bytes,
                rom_path,
                bios_path,
            })
        }
        None => Err(StorageError::RomNotFound),
    }
}

/// Renderiza un mensaje instructivo nítido en el framebuffer de juego de 240x160 px.
///
/// Permite que el Compositor GPU presente la carcasa y controles táctiles de la consola
/// mostrando la guía de instalación en la pantalla virtual, sin recurrir a bloqueos de CPU.
pub fn render_guide_screen(buffer: &mut [[u8; 4]], title: &str, lines: &[&str]) {
    if buffer.len() != 240 * 160 {
        return;
    }
    let font = match fontdue::Font::from_bytes(
        crate::overlay::JETBRAINS_FONT_BYTES,
        fontdue::FontSettings::default(),
    ) {
        Ok(f) => f,
        Err(_) => return,
    };

    // 1. Limpieza de fondo: azul noche elegante (#0d1b2a)
    let bg = [13u8, 27u8, 42u8, 255u8];
    for px in buffer.iter_mut() {
        *px = bg;
    }

    // 2. Barra de título superior: (#1b4965)
    let header_h = 24usize;
    let header_bg = [27u8, 73u8, 101u8, 255u8];
    for y in 0..header_h {
        for x in 0..240 {
            buffer[y * 240 + x] = header_bg;
        }
    }

    // 3. Dibujar título en la barra superior (en blanco, tamaño 11px)
    let mut pen_x = 8.0f32;
    let title_y = 4usize;
    for ch in title.chars() {
        let (metrics, bitmap) = font.rasterize(ch, 11.0);
        let start_x = (pen_x as usize).saturating_add(metrics.xmin.max(0) as usize);
        let start_y = title_y;
        for row in 0..metrics.height {
            let py = start_y + row;
            if py >= header_h {
                continue;
            }
            for col in 0..metrics.width {
                let px = start_x + col;
                if px >= 240 {
                    continue;
                }
                let alpha = bitmap[row * metrics.width + col];
                if alpha > 30 {
                    buffer[py * 240 + px] = [255, 255, 255, 255];
                }
            }
        }
        pen_x += metrics.advance_width;
    }

    // 4. Dibujar líneas de texto (en celeste claro, tamaño 9.0px)
    let mut cur_y = 30usize;
    for line in lines {
        let mut line_x = 8.0f32;
        for ch in line.chars() {
            let (metrics, bitmap) = font.rasterize(ch, 9.0);
            let start_x = (line_x as usize).saturating_add(metrics.xmin.max(0) as usize);
            let start_y = cur_y;
            for row in 0..metrics.height {
                let py = start_y + row;
                if py >= 160 {
                    continue;
                }
                for col in 0..metrics.width {
                    let px = start_x + col;
                    if px >= 240 {
                        continue;
                    }
                    let alpha = bitmap[row * metrics.width + col];
                    if alpha > 30 {
                        buffer[py * 240 + px] = [224, 251, 252, 255];
                    }
                }
            }
            line_x += metrics.advance_width;
        }
        cur_y += 12;
        if cur_y >= 155 {
            break;
        }
    }
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
