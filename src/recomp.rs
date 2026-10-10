//! Subsistema de despacho y ejecución de bloques C11 recompilados Ahead-Of-Time (AOT).
//!
//! Conecta la tabla maestra de 17,207 funciones nativas (`rcg_blocks`)
//! con el runtime de `gba-core`, ejecutando la lógica del juego directamente
//! en la CPU física sin emulación de instrucciones por software.

use gba_core::capi::{RcgBlock, RT_API};
use gba_core::cpu::FLAG_I;
use gba_core::machine::IRQ_RETURN_ADDR;
use gba_core::{Machine, Mode};
use std::collections::HashMap;

/// Puntero a función de bloque C11 recompilado.
pub type BlockFn = extern "C" fn(*const gba_core::capi::RtApi, *mut std::ffi::c_void) -> u32;

const ROM_BASE: u32 = 0x0800_0000;
const IWRAM_BASE: u32 = 0x0300_0000;
const EWRAM_BASE: u32 = 0x0200_0000;

/// Tabla de despacho indexada O(1) para ejecución nativa de bloques.
pub struct BlockTable {
    rom: Vec<Option<BlockFn>>,
    iwram: Vec<Option<BlockFn>>,
    ewram: Vec<Option<BlockFn>>,
    bios: Vec<Option<BlockFn>>,
    other: HashMap<u32, BlockFn>,
    len: usize,
}

impl BlockTable {
    /// Crea una tabla vacía (fuerza fallback completo al intérprete).
    pub fn empty() -> Self {
        Self {
            rom: Vec::new(),
            iwram: Vec::new(),
            ewram: Vec::new(),
            bios: Vec::new(),
            other: HashMap::new(),
            len: 0,
        }
    }

    /// Construye una tabla indexada a partir de un arreglo de bloques `RcgBlock`.
    pub fn from_blocks(blocks: &[RcgBlock]) -> Self {
        let mut rom_max = 0usize;
        for b in blocks {
            let r = b.key.wrapping_sub(ROM_BASE) as usize;
            if r < 0x0200_0000 {
                rom_max = rom_max.max(r + 1);
            }
        }
        let any_ewram = blocks
            .iter()
            .any(|b| (b.key.wrapping_sub(EWRAM_BASE) as usize) < 0x4_0000);
        let any_bios = blocks.iter().any(|b| b.key < 0x4000);
        let mut t = BlockTable {
            rom: vec![None; rom_max],
            iwram: vec![None; 0x8000],
            ewram: vec![None; if any_ewram { 0x4_0000 } else { 0 }],
            bios: vec![None; if any_bios { 0x4000 } else { 0 }],
            other: HashMap::new(),
            len: blocks.len(),
        };

        for b in blocks {
            let r = b.key.wrapping_sub(ROM_BASE) as usize;
            let w = b.key.wrapping_sub(IWRAM_BASE) as usize;
            if (b.key as usize) < t.bios.len() {
                t.bios[b.key as usize] = Some(b.func);
            } else if r < t.rom.len() {
                t.rom[r] = Some(b.func);
            } else if w < t.iwram.len() {
                t.iwram[w] = Some(b.func);
            } else {
                let e = b.key.wrapping_sub(EWRAM_BASE) as usize;
                if e < t.ewram.len() {
                    t.ewram[e] = Some(b.func);
                } else {
                    t.other.insert(b.key, b.func);
                }
            }
        }
        t
    }

    /// Cantidad total de bloques nativos registrados en la tabla.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Indica si la tabla no contiene ningún bloque nativo.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Resuelve la función nativa para una clave de dirección (con bit 0 = Thumb) en O(1).
    #[inline(always)]
    pub fn get(&self, key: u32) -> Option<BlockFn> {
        let r = key.wrapping_sub(ROM_BASE) as usize;
        if r < self.rom.len() {
            return self.rom[r];
        }
        let w = key.wrapping_sub(IWRAM_BASE) as usize;
        if w < self.iwram.len() {
            return self.iwram[w];
        }
        let e = key.wrapping_sub(EWRAM_BASE) as usize;
        if e < self.ewram.len() {
            return self.ewram[e];
        }
        if (key as usize) < self.bios.len() {
            return self.bios[key as usize];
        }
        if self.other.is_empty() {
            None
        } else {
            self.other.get(&key).copied()
        }
    }
}

/// Ejecuta un fotograma de la consola utilizando bloques C11 nativos con fallback al intérprete.
///
/// Retorna la tupla `(bloques_nativos_ejecutados, pasos_interprete_fallback)`.
pub fn run_frame_native(
    m: &mut Machine,
    table: &BlockTable,
    max_steps: u64,
) -> (u64, u64) {
    m.bus.frame_ready = false;
    let mut native = 0u64;
    let mut fallback = 0u64;
    let mut steps = 0u64;
    let mptr = m as *mut Machine as *mut std::ffi::c_void;

    while !m.bus.frame_ready && steps < max_steps {
        steps += 1;
        // 1. Estados de reposo (Halt) y entrega de interrupciones (IRQ)
        if m.bus.halted
            || (!m.bus.real_bios
                && m.cpu.regs[15] == IRQ_RETURN_ADDR
                && m.cpu.mode() == Mode::Irq)
            || (m.bus.irq_pending() && !m.cpu.flag(FLAG_I))
        {
            m.step();
            continue;
        }

        let key = m.cpu.regs[15] | m.cpu.thumb() as u32;

        if m.bus.real_bios {
            m.bus.pc_in_bios = key < 0x4000;
            // La BIOS real oficial debe ejecutarse en el intérprete instrucción por
            // instrucción para preservar verificaciones de cabecera criptográfica y trampas
            // de bus de memoria (especificación upstream de gba-recomp).
            if key < 0x4000 {
                m.step();
                fallback += 1;
                continue;
            }
        }

        // 2. Hooks de motor de audio HLE en límites de bloque
        if let Some(h) = m.bus.mp2k.as_deref() {
            if h.active && h.hook_match(key) {
                m.bus.mp2k_frame_hook(key);
            }
        }

        // 3. Despacho por tabla nativa O(1) o fallback a intérprete
        match table.get(key) {
            Some(f) => {
                f(&RT_API, mptr);
                native += 1;
            }
            None => {
                m.step();
                fallback += 1;
            }
        }
    }
    (native, fallback)
}

/// Resuelve la ruta a una biblioteca hermana contigua a partir de la ruta absoluta
/// del módulo o ejecutable actual (usado para resolver `libblazing_recompiled.so` junto a `libblazing_recomp.so`).
#[allow(dead_code)]
pub(crate) fn resolve_sibling_library_path(self_path: &str, sibling_name: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(self_path);
    let parent = p.parent()?;
    Some(parent.join(sibling_name))
}

#[cfg(target_os = "android")]
mod dl {
    use std::ffi::{c_char, c_int, c_void, CStr, CString};

    const RTLD_NOW: c_int = 2;

    #[repr(C)]
    struct DlInfo {
        dli_fname: *const c_char,
        dli_fbase: *mut c_void,
        dli_sname: *const c_char,
        dli_saddr: *mut c_void,
    }

    extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlerror() -> *const c_char;
        fn dladdr(addr: *const c_void, info: *mut DlInfo) -> c_int;
    }

    pub unsafe fn open_library(name: &str) -> Option<*mut c_void> {
        // 1. Intentar resolver la ruta canónica absoluta contigua a libblazing_recomp.so vía dladdr
        let mut info = std::mem::MaybeUninit::<DlInfo>::zeroed();
        let res = dladdr(open_library as *const () as *const c_void, info.as_mut_ptr());
        if res != 0 {
            let info = info.assume_init();
            if !info.dli_fname.is_null() {
                let self_path = CStr::from_ptr(info.dli_fname).to_string_lossy();
                if let Some(candidate) = super::resolve_sibling_library_path(&self_path, name) {
                    if let Some(candidate_str) = candidate.to_str() {
                        if let Ok(c_candidate) = CString::new(candidate_str) {
                            let handle = dlopen(c_candidate.as_ptr(), RTLD_NOW);
                            if !handle.is_null() {
                                log::info!("dlopen: biblioteca cargada desde ruta canónica: {candidate_str}");
                                return Some(handle);
                            } else {
                                let err = dlerror();
                                let err_msg = if !err.is_null() {
                                    CStr::from_ptr(err).to_string_lossy().into_owned()
                                } else {
                                    "desconocido".to_string()
                                };
                                log::debug!("dlopen('{candidate_str}') no pudo abrir: {err_msg}");
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback: intentar por nombre relativo estándar
        let c_name = CString::new(name).ok()?;
        let handle = dlopen(c_name.as_ptr(), RTLD_NOW);
        if handle.is_null() {
            let err = dlerror();
            let err_msg = if !err.is_null() {
                CStr::from_ptr(err).to_string_lossy().into_owned()
            } else {
                "desconocido".to_string()
            };
            log::warn!("dlopen('{name}') fallo: {err_msg}");
            None
        } else {
            log::info!("dlopen: biblioteca cargada por nombre relativo: {name}");
            Some(handle)
        }
    }

    pub unsafe fn get_symbol<T>(handle: *mut c_void, name: &str) -> Option<T> {
        let c_name = CString::new(name).ok()?;
        let sym = dlsym(handle, c_name.as_ptr());
        if sym.is_null() {
            let err = dlerror();
            let err_msg = if !err.is_null() {
                CStr::from_ptr(err).to_string_lossy().into_owned()
            } else {
                "desconocido".to_string()
            };
            log::warn!("dlsym('{name}') fallo: {err_msg}");
            None
        } else {
            Some(std::mem::transmute_copy(&sym))
        }
    }
}

/// Intenta cargar la biblioteca de bloques C11 recompilados (`libblazing_recompiled.so`)
/// presente en el paquete APK en Android. Retorna `None` si la biblioteca no está presente
/// (en host o entornos de prueba pura), activando el modo intérprete de respaldo.
pub fn load_native_blocks() -> Option<BlockTable> {
    #[cfg(target_os = "android")]
    {
        unsafe {
            let handle = dl::open_library("libblazing_recompiled.so")?;
            let blocks_ptr: *const RcgBlock = dl::get_symbol(handle, "rcg_blocks")?;
            let count_ptr: *const u64 = dl::get_symbol(handle, "rcg_block_count")?;
            if blocks_ptr.is_null() || count_ptr.is_null() {
                log::warn!("Símbolos rcg_blocks o rcg_block_count nulos en libblazing_recompiled.so");
                return None;
            }
            let count = std::ptr::read_unaligned(count_ptr) as usize;
            if count == 0 || count > 100_000 {
                log::warn!("rcg_block_count ({count}) inválido en libblazing_recompiled.so");
                return None;
            }
            let slice = std::slice::from_raw_parts(blocks_ptr, count);
            log::info!("Recompilación nativa AOT cargada exitosamente: {} bloques C11 activos", count);
            Some(BlockTable::from_blocks(slice))
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

#[cfg(test)]
#[path = "recomp_tests.rs"]
mod tests;
