use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Firma criptogrfica SHA-256 oficial de la ROM europea de Fire Emblem (Europe) (16 MB).
pub const OFFICIAL_ROM_SHA256: &str =
    "6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9";

/// Firma criptogrfica SHA-256 oficial de la BIOS de Game Boy Advance (16 KB).
pub const OFFICIAL_BIOS_SHA256: &str =
    "fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570";

/// Tamao exacto de la ROM oficial de Fire Emblem (16 MB en bytes).
pub const OFFICIAL_ROM_SIZE: usize = 16 * 1024 * 1024;

/// Tamao exacto del binario de BIOS oficial de GBA (16 KB en bytes).
pub const OFFICIAL_BIOS_SIZE: usize = 16 * 1024;

/// Assets del juego cargados y verificados en memoria listos para la ejecucin.
#[derive(Debug, Clone)]
pub struct LoadedAssets {
    /// Bfer con los 16 MB de la ROM oficial.
    pub rom: Vec<u8>,
    /// Bfer con los 16 KB de la BIOS oficial (si fue encontrada).
    pub bios: Option<Vec<u8>>,
    /// Ruta del archivo de ROM desde donde se carg.
    pub rom_path: PathBuf,
    /// Ruta del archivo de BIOS desde donde se carg (si aplica).
    pub bios_path: Option<PathBuf>,
}

/// Errores posibles al buscar, leer o validar los assets del juego.
#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// No se encontr ningn archivo cuya firma SHA-256 coincida con la ROM oficial.
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

/// Calcula la firma criptogrfica SHA-256 de un bfer de bytes y la retorna en formato hexadecimal en minsculas.
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

/// Verifica si un bfer de datos coincide exactamente en tamao y firma SHA-256 con la ROM oficial europea.
pub fn verify_rom(data: &[u8]) -> bool {
    if data.len() != OFFICIAL_ROM_SIZE {
        return false;
    }
    compute_sha256(data).eq_ignore_ascii_case(OFFICIAL_ROM_SHA256)
}

/// Verifica si un bfer de datos coincide exactamente en tamao y firma SHA-256 con la BIOS oficial de GBA.
pub fn verify_bios(data: &[u8]) -> bool {
    if data.len() != OFFICIAL_BIOS_SIZE {
        return false;
    }
    compute_sha256(data).eq_ignore_ascii_case(OFFICIAL_BIOS_SHA256)
}

/// Genera la lista ordenada de directorios candidatos de bsqueda, priorizando la carpeta Downloads.
pub fn resolve_search_directories(internal_data_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // 1. Android Downloads principal (almacenamiento compartido primario)
    dirs.push(PathBuf::from("/sdcard/Download"));
    // 2. Android Downloads secundario/emulado
    dirs.push(PathBuf::from("/storage/emulated/0/Download"));
    // 3. Carpeta Download relativa local (desarrollo o emulador de escritorio)
    dirs.push(PathBuf::from("Download"));
    // 4. Carpeta roms relativa local (para pruebas en host de desarrollo)
    dirs.push(PathBuf::from("roms"));
    // 5. Directorio interno privado de la aplicacin provisto por AndroidApp
    if let Some(dir) = internal_data_dir {
        dirs.push(dir.to_path_buf());
    }
    dirs
}

/// Escanea los directorios especificados en busca de archivos de ROM y BIOS oficiales.
///
/// La bsqueda inspecciona archivos con extensin `.gba` o `.bin` comprobando su tamao
/// y firma SHA-256 oficial antes de cargarlos a memoria.
pub fn scan_and_load_game(search_dirs: &[PathBuf]) -> Result<LoadedAssets, StorageError> {
    let mut found_rom: Option<(PathBuf, Vec<u8>)> = None;
    let mut found_bios: Option<(PathBuf, Vec<u8>)> = None;

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

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

        // Si ya encontramos la ROM, no necesitamos seguir buscando en carpetas secundarias
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

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
