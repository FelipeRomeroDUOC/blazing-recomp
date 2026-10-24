//! Subsistema de despacho y ejecución de bloques C11 recompilados Ahead-Of-Time (AOT).
//!
//! Conecta los 144.875 bloques nativos de ejecución C11 (`rcg_blocks`), agrupados bajo 18.483 funciones catalogadas,
//! con el runtime de `gba-core`, ejecutando la lógica del juego directamente
//! en la CPU física sin emulación de instrucciones por software.

use gba_core::capi::{RcgBlock, RT_API};
use gba_core::cpu::FLAG_I;
use gba_core::machine::IRQ_RETURN_ADDR;
use gba_core::{Machine, Mode};
use std::collections::HashMap;
use std::sync::Mutex;

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
    let mut nativos_por_region = [0u64; 4];
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
            // Los bloques nativos no pasan por `note_fetch`, así que el flag de "la CPU está
            // ejecutando dentro de la BIOS" se mantiene aquí, en el despacho: `gba-core`
            // decide con él si las lecturas de la región 0 devuelven bytes reales.
            //
            // No se desvía la BIOS al intérprete: se ejecuta con sus bloques AOT, igual que
            // en el motor (rev de4edf59b872d887046d6a3b005e2df551b6d44c,
            // `crates/recomp/src/main.rs`, que en este mismo punto solo asigna este flag).
            // Si una dirección de BIOS no tiene bloque, el despacho de más abajo cae al
            // intérprete como en cualquier otra región, y la telemetría lo cuenta en `bios`.
            m.bus.pc_in_bios = key < 0x4000;
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
                if let Some(region) = region_of(key & !1) {
                    nativos_por_region[region.index()] += 1;
                }
            }
            None => {
                record_hotspot(key);
                m.step();
                fallback += 1;
            }
        }
    }
    add_native_region_ticks(&nativos_por_region);
    (native, fallback)
}

/// Rango de direcciones de la memoria ROM de Game Boy Advance (Cartucho).
pub const ROM_REGION_START: u32 = 0x0800_0000;
pub const ROM_REGION_END: u32 = 0x0A00_0000;

/// Región de memoria ejecutable de Game Boy Advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    /// Cartucho (`0x08000000..0x0A000000`).
    Rom = 0,
    /// RAM interna de trabajo (`0x03000000..0x03008000`).
    Iwram = 1,
    /// RAM externa de trabajo (`0x02000000..0x02040000`).
    Ewram = 2,
    /// BIOS (`0x00000000..0x00004000`).
    Bios = 3,
}

impl Region {
    /// Nombres cortos, en el orden canónico de la telemetría `# regiones`.
    pub const NAMES: [&'static str; 4] = ["rom", "iwram", "ewram", "bios"];

    /// Índice de la región dentro de los arreglos de [`RegionCounters`].
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Ejecución acumulada por región de memoria desde el arranque de la sesión.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegionCounters {
    /// Pasos ejecutados por el intérprete de respaldo, indexados por [`Region::index`].
    pub interprete: [u64; 4],
    /// Bloques ejecutados por la tabla nativa AOT, indexados por [`Region::index`].
    ///
    /// Cuenta **bloques**, no instrucciones, así que no es comparable término a término con
    /// `interprete`: dividir ambos da una fracción de eventos de despacho, no de instrucciones.
    pub nativos: [u64; 4],
}

impl RegionCounters {
    /// Indica si todavía no se ha registrado ninguna ejecución.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.interprete.iter().all(|&c| c == 0) && self.nativos.iter().all(|&c| c == 0)
    }

    /// Formatea la línea de telemetría `# regiones`, con un par `interprete/nativos` por región.
    pub fn to_line(&self) -> String {
        use std::fmt::Write as _;
        let mut linea = String::from("# regiones");
        for (indice, nombre) in Region::NAMES.iter().enumerate() {
            let _ = write!(
                linea,
                " {}={}/{}",
                nombre, self.interprete[indice], self.nativos[indice]
            );
        }
        linea
    }
}

/// Reparto acumulado de las líneas de la PPU entre los caminos de la composición.
///
/// Las cuatro cuentas suman las líneas dibujadas desde el arranque de la sesión: cada línea cae en
/// una sola razón, la primera que impide usar la vía rápida (efectos de color, ventana de sprites o
/// sprite translúcido). Es el dato que dice si la vía rápida se ejecuta en juego real y, si no, por
/// qué.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PpuPathCounters {
    /// Líneas compuestas por la vía rápida (tramos de control constante).
    pub rectas: u64,
    /// De las `rectas`, cuántas lo son porque el efecto de color es la identidad (oscurecer o
    /// aclarar con `BLDY = 0`, o alpha sin segundos objetivos): líneas que antes se descartaban.
    pub inertes: u64,
    /// De las `rectas`, cuántas aplicaron un aclarado u oscurecido real sin buscar la segunda
    /// capa (modos 2 y 3).
    pub efecto: u64,
    /// Líneas con efecto real que piden alpha (modo 1), contadas entre las descartadas.
    pub efalpha: u64,
    /// De las líneas con efecto real, cuántas piden aclarar (modo 2).
    pub efbrillo: u64,
    /// De las líneas con efecto real, cuántas piden oscurecer (modo 3).
    pub efoscuro: u64,
    /// Líneas descartadas porque `BLDCNT` pide un efecto de color que necesita la segunda capa.
    pub mezcla: u64,
    /// Líneas descartadas por la ventana de sprites (`OBJWIN`).
    pub ventana: u64,
    /// Líneas descartadas por un píxel de sprite translúcido.
    pub semi: u64,
}

impl PpuPathCounters {
    /// Indica si todavía no se ha contado ninguna línea.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Total de líneas contadas, que debe cerrar en múltiplos de 160 (las líneas de un fotograma).
    #[inline]
    pub fn total(&self) -> u64 {
        self.rectas + self.mezcla + self.ventana + self.semi
    }

    /// Porcentaje de líneas que tomaron la vía rápida, redondeado al entero (0 sin líneas contadas).
    pub fn rectas_pct(&self) -> u64 {
        match self.total() {
            0 => 0,
            total => (self.rectas * 100 + total / 2) / total,
        }
    }

    /// Formatea la línea de telemetría `# ppucamino` con las cuatro cuentas, el reparto de los
    /// efectos y el porcentaje de líneas por la vía rápida.
    pub fn to_line(&self) -> String {
        format!(
            "# ppucamino rectas={} mezcla={} ventana={} semi={} inertes={} efecto={} efalpha={} \
             efbrillo={} efoscuro={} (rectas={}%)",
            self.rectas,
            self.mezcla,
            self.ventana,
            self.semi,
            self.inertes,
            self.efecto,
            self.efalpha,
            self.efbrillo,
            self.efoscuro,
            self.rectas_pct()
        )
    }
}

/// Clasifica una dirección de PC normalizada (bit 0 en cero) en su región ejecutable.
#[inline]
fn region_of(pc: u32) -> Option<Region> {
    if (ROM_REGION_START..ROM_REGION_END).contains(&pc) {
        Some(Region::Rom)
    } else if (0x0300_0000..0x0300_8000).contains(&pc) {
        Some(Region::Iwram)
    } else if (0x0200_0000..0x0204_0000).contains(&pc) {
        Some(Region::Ewram)
    } else if pc < 0x4000 {
        Some(Region::Bios)
    } else {
        None
    }
}

/// Acumulador global de hotspots, de ejecución por región de memoria y del reparto
/// de líneas de la PPU por camino de composición.
#[derive(Default)]
struct HotspotState {
    map: HashMap<u32, u64>,
    region: RegionCounters,
    camino: PpuPathCounters,
}

static HOTSPOTS: Mutex<Option<HotspotState>> = Mutex::new(None);

// El acumulador de hotspots es estado global del proceso y `cargo test` ejecuta los
// tests en paralelo, así que cualquier test que escriba en él (directamente o a través
// de `run_frame_native`) debe serializarse con este candado para que sus aserciones
// por diferencia sean deterministas.
#[cfg(test)]
pub(crate) static LOCK_ACUMULADORES: Mutex<()> = Mutex::new(());

/// Registra una dirección ejecutada en el intérprete de respaldo si pertenece a regiones ejecutables
/// (ROM, IWRAM, EWRAM o BIOS).
#[inline]
pub fn record_hotspot(pc: u32) {
    let clean_pc = pc & !1;
    let Some(region) = region_of(clean_pc) else {
        return;
    };
    if let Ok(mut guard) = HOTSPOTS.lock() {
        let estado = guard.get_or_insert_with(HotspotState::default);
        *estado.map.entry(clean_pc).or_insert(0) += 1;
        estado.region.interprete[region.index()] += 1;
    }
}

/// Extrae las `limit` direcciones ejecutadas con más frecuencia y **consume solo esas**.
///
/// Las direcciones que no entran en el `top` conservan su contador y vuelven a competir en el
/// volcado siguiente, de modo que ninguna ejecución registrada se descarta. Un `limit` mayor o
/// igual que el número de direcciones registradas deja el acumulador vacío.
pub fn drain_top_hotspots(limit: usize) -> Vec<(u32, u64)> {
    let mut guard = match HOTSPOTS.lock() {
        Ok(guard) => guard,
        Err(_) => return Vec::new(),
    };
    let Some(estado) = guard.as_mut() else {
        return Vec::new();
    };
    let mut entries: Vec<(u32, u64)> = estado.map.iter().map(|(addr, hits)| (*addr, *hits)).collect();
    entries.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    entries.truncate(limit);
    for (addr, _) in &entries {
        estado.map.remove(addr);
    }
    entries
}

/// Acumula los bloques nativos ejecutados en un fotograma, agrupados por región.
///
/// Se invoca una sola vez por fotograma y no una vez por bloque, para no añadir
/// sincronización al camino caliente de despacho nativo.
pub(crate) fn add_native_region_ticks(counts: &[u64; 4]) {
    if counts.iter().all(|&c| c == 0) {
        return;
    }
    if let Ok(mut guard) = HOTSPOTS.lock() {
        let estado = guard.get_or_insert_with(HotspotState::default);
        for (indice, &conteo) in counts.iter().enumerate() {
            estado.region.nativos[indice] += conteo;
        }
    }
}

/// Acumula el reparto de líneas del fotograma que el motor acaba de medir.
///
/// Se invoca una sola vez por fotograma (no una vez por línea) desde el bucle de emulación, así
/// que no añade sincronización al camino caliente: el motor cuenta por línea sin bloqueo y aquí
/// se vuelca el total del fotograma.
pub(crate) fn add_ppu_path(
    rectas: u64,
    mezcla: u64,
    ventana: u64,
    semi: u64,
    inertes: u64,
    efecto: u64,
    efalpha: u64,
    efbrillo: u64,
    efoscuro: u64,
) {
    if rectas | mezcla | ventana | semi | inertes | efecto | efalpha | efbrillo | efoscuro == 0 {
        return;
    }
    if let Ok(mut guard) = HOTSPOTS.lock() {
        let estado = guard.get_or_insert_with(HotspotState::default);
        estado.camino.rectas += rectas;
        estado.camino.mezcla += mezcla;
        estado.camino.ventana += ventana;
        estado.camino.semi += semi;
        estado.camino.inertes += inertes;
        estado.camino.efecto += efecto;
        estado.camino.efalpha += efalpha;
        estado.camino.efbrillo += efbrillo;
        estado.camino.efoscuro += efoscuro;
    }
}

/// Devuelve una copia del reparto acumulado de líneas por camino. No reinicia los contadores.
pub fn ppu_path_counters() -> PpuPathCounters {
    match HOTSPOTS.lock() {
        Ok(guard) => guard
            .as_ref()
            .map_or_else(PpuPathCounters::default, |estado| estado.camino),
        Err(_) => PpuPathCounters::default(),
    }
}

/// Devuelve una copia de la ejecución acumulada por región de memoria. No reinicia los contadores.
pub fn region_counters() -> RegionCounters {
    match HOTSPOTS.lock() {
        Ok(guard) => guard
            .as_ref()
            .map_or_else(RegionCounters::default, |estado| estado.region),
        Err(_) => RegionCounters::default(),
    }
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
            if count == 0 || count > 500_000 {
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
