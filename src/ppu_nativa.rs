//! Módulo PPU nativa: dibuja el cuadro de la GBA con la GPU del dispositivo.
//!
//! Sustituye a la PPU por software con un modelo de **composición por píxel**:
//! los fondos y los sprites se rasterizan a un G-buffer (una capa por fondo y una
//! para los sprites) y una pasada final elige, píxel a píxel, las dos capas más
//! altas y les aplica la **unidad de mezcla** (alfa, aclarar, oscurecer) y las
//! **ventanas 0, 1 y OBJ**.
//!
//! La especificación normativa es la PPU por software de `gba-core`
//! (`crates/gba-core/src/ppu.rs`, rev fijado `de4edf59`): su `split6`/`join6`/
//! `alpha_blend`/`brighten`/`darken` y las reglas de `compose()`. Este módulo no
//! reimplementa la PPU completa: dibuja el cuadro en la GPU y reproduce esas
//! reglas, que es lo que el hardware de la GBA hace por sí solo.

use bytemuck::{Pod, Zeroable};

/// Ancho de la pantalla visible de la GBA.
pub const GBA_SCREEN_W: u32 = 240;
/// Alto de la pantalla visible de la GBA.
pub const GBA_SCREEN_H: u32 = 160;
/// Número máximo de entradas de OAM.
pub const MAX_SPRITES: usize = 128;

/// Centinela de «píxel transparente». El mismo valor que usa la referencia; no
/// puede colisionar con un color real porque la paleta va enmascarada a 15 bits.
pub const TRANSPARENTE: u16 = 0xFFFF;

/// Shader de la PPU nativa (capas, sprites y composición).
pub const PPU_NATIVA_WGSL: &str = include_str!("ppu_nativa.wgsl");

/// Nombre del archivo de marca que **apaga** la PPU nativa en runtime.
///
/// La PPU nativa es la ruta de dibujo **por defecto**: un APK recién
/// instalado compone los píxeles en la GPU con la aritmética de la PPU
/// original de la GBA, sin que nadie tenga que tocar nada. Para volver al
/// *shim* (o al render por software) y poder comparar A/B en el propio
/// dispositivo, se crea este archivo; la app lo relee una vez por segundo en
/// los mismos directorios donde busca la telemetría.
///
/// El nombre es explícito a propósito: la marca de apagado convive sin
/// ambigüedad con el `ppu-nativa.flag` de las pruebas anteriores, que ya no se
/// lee (era el interruptor en el otro sentido).
pub const NOMBRE_BANDERA_APAGADO: &str = "ppu-nativa-off.flag";

/// Indica si la PPU nativa debe **apagarse** ahora: existe el archivo
/// [`NOMBRE_BANDERA_APAGADO`] en alguno de los directorios de telemetría.
///
/// Sigue el mismo patrón que [`crate::perf::audio_timing_flag_enabled`]: los
/// directorios son los candidatos de telemetría, que incluyen el directorio
/// externo propio de la app (el único accesible sin permisos especiales).
pub fn apagada_por_marca(search_dirs: &[std::path::PathBuf]) -> bool {
    crate::perf::resolve_perf_candidates(search_dirs)
        .iter()
        .any(|dir| dir.join(NOMBRE_BANDERA_APAGADO).exists())
}

/// Indica si la PPU nativa debe usarse ahora.
///
/// La respuesta es «sí» salvo que exista la marca de apagado: la PPU nativa
/// es la ruta por defecto y el archivo solo sirve para volver atrás.
pub fn ppu_nativa_activada(search_dirs: &[std::path::PathBuf]) -> bool {
    !apagada_por_marca(search_dirs)
}

// ===========================================================================
// Guardián de errores de la GPU
// ===========================================================================
//
// `wgpu` trata un error de validación como fatal: sin un ámbito de error que
// lo recoja, `default_error_handler` hace `panic!("wgpu error: …")` y se
// lleva la app por delante. Con la PPU nativa activa por defecto, eso ya no
// afectaría solo al dispositivo de pruebas: un móvil cuya GPU rechace
// algún recurso se quedaría sin juego.
//
// El guardián envuelve la construcción en un ámbito de error por filtro y
// devuelve el error en vez de dejar que `wgpu` entre en pánico, para que el
// compositor pueda seguir con el *shim*.

/// Motor de errores de `wgpu`: abrir y cerrar un ámbito de error.
///
/// Se abstrae para que el guardián se pueda probar sin GPU: la implementación
/// real delega en `wgpu::Device` y la de mentira devuelve el error que se le
/// configure.
pub trait MotorDeErrores {
    /// Abre un ámbito de error para el filtro dado.
    fn abrir_ambito(&self, filtro: wgpu::ErrorFilter);
    /// Cierra el último ámbito abierto y devuelve el error que recogió, si hubo.
    fn cerrar_ambito(&self) -> Option<String>;
}

impl MotorDeErrores for wgpu::Device {
    fn abrir_ambito(&self, filtro: wgpu::ErrorFilter) {
        self.push_error_scope(filtro);
    }

    fn cerrar_ambito(&self) -> Option<String> {
        // `pop_error_scope` devuelve un futuro **ya listo**: el error se
        // recogió al construirse, no hace falta bombear el dispositivo.
        pollster::block_on(self.pop_error_scope()).map(|e| e.to_string())
    }
}

/// Filtros de error de `wgpu` que cubre el guardián, uno por ámbito.
///
/// `wgpu` clasifica cada error en uno de estos tres filtros y lo guarda en el
/// ámbito más interno **cuyo filtro coincida** con el del error
/// (`ErrorSinkRaw::handle_error`, en `wgpu-26.0.1/src/backend/wgpu_core.rs`).
/// Un solo ámbito no bastaría: un error de memoria agotada no entraría en un
/// ámbito de validación y seguiría entrando en pánico.
pub const FILTROS_ERROR: [wgpu::ErrorFilter; 3] = [
    wgpu::ErrorFilter::Validation,
    wgpu::ErrorFilter::OutOfMemory,
    wgpu::ErrorFilter::Internal,
];

/// Construye un recurso de la GPU sin que un error de validación mate la app.
///
/// Abre un ámbito por cada filtro de error, ejecuta la construcción y cierra
/// los ámbitos. Si alguno recogió un error, lo devuelve como `Err` para que el
/// llamante pueda seguir por otra vía (en este caso, el *shim*).
pub fn construir_con_guardian<T, M: MotorDeErrores>(
    motor: &M,
    etiqueta: &str,
    construir: impl FnOnce() -> T,
) -> Result<T, String> {
    for filtro in FILTROS_ERROR {
        motor.abrir_ambito(filtro);
    }
    let valor = construir();
    let mut errores = Vec::new();
    for _ in FILTROS_ERROR {
        if let Some(e) = motor.cerrar_ambito() {
            errores.push(e);
        }
    }
    if errores.is_empty() {
        Ok(valor)
    } else {
        Err(format!("{etiqueta}: {}", errores.join("; ")))
    }
}

// ===========================================================================
// Bindings del shader
// ===========================================================================
//
// Son la fuente de verdad del lado de Rust: una prueba de integración los
// compara con los que declara el WGSL, para que un cambio en uno no rompa el
// otro en silencio (el fallo solo aparecería en el dispositivo).

/// VRAM completa (96 KB) como búfer de solo lectura.
pub const BINDING_VRAM: u32 = 0;
/// Paleta (512 colores RGB555) como búfer de solo lectura.
pub const BINDING_PALETA: u32 = 1;
/// Configuración de la pasada de capas (modo y los cuatro fondos).
pub const BINDING_CONFIG_CAPA: u32 = 2;
/// Sprites de color, en una textura de solo lectura.
pub const BINDING_SPRITES: u32 = 3;
/// Configuración de la pasada de composición.
pub const BINDING_CONFIG_COMP: u32 = 4;
/// Texturas del G-buffer de fondos, del 0 al 3.
pub const BINDING_TEX_CAPA0: u32 = 5;
/// Textura del G-buffer de sprites (color, prioridad, semitransparencia).
pub const BINDING_TEX_OBJ: u32 = 9;
/// Textura de la máscara de la ventana OBJ.
pub const BINDING_TEX_OBJ_VENTANA: u32 = 10;

// ===========================================================================
// Unidad de mezcla (puro)
// ===========================================================================

/// Canales como los ve la unidad de mezcla: rojo y azul de 5 bits, **verde en un
/// carril de 6 bits** (el valor de 5 bits duplicado).
///
/// El bit bajo real de ese carril es el bit 15 de PRAM, que la referencia
/// enmascara al leer la paleta (`pal16`); aquí el color llega ya enmascarado.
#[inline]
pub fn split6(c: u16) -> (u32, u32, u32) {
    (
        (c & 31) as u32,
        (((c >> 5) & 31) as u32) << 1,
        ((c >> 10) & 31) as u32,
    )
}

/// Reensambla los canales del carril de la unidad de mezcla en un color de 15
/// bits, truncando el verde (el carril de 6 bits vuelve a 5).
#[inline]
pub fn join6(r: u32, g: u32, b: u32) -> u16 {
    (r | ((g >> 1) << 5) | (b << 10)) as u16
}

/// Mezcla alfa de la unidad de mezcla: `a` es la capa superior y `b` la segunda.
///
/// Acumula con redondeo a media hacia arriba (`+8 >> 4`) y satura cada canal.
#[inline]
pub fn alpha_blend(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = split6(a);
    let (br, bg, bb) = split6(b);
    let r = ((ar * eva + br * evb + 8) >> 4).min(31);
    let g = ((ag * eva + bg * evb + 8) >> 4).min(63);
    let b = ((ab * eva + bb * evb + 8) >> 4).min(31);
    join6(r, g, b)
}

/// Aclarar (modo 2 de la unidad de mezcla): suma una fracción `evy/16` de lo que
/// le falta a cada canal para llegar al blanco.
#[inline]
pub fn brighten(c: u16, evy: u32) -> u16 {
    let (r, g, b) = split6(c);
    join6(
        r + (((31 - r) * evy + 8) >> 4),
        g + (((63 - g) * evy + 8) >> 4),
        b + (((31 - b) * evy + 8) >> 4),
    )
}

/// Oscurecer (modo 3 de la unidad de mezcla): resta una fracción `evy/16` de cada
/// canal. El sustraendo redondea a media hacia abajo (`+7`) para que el resultado
/// siga redondeando a media hacia arriba.
#[inline]
pub fn darken(c: u16, evy: u32) -> u16 {
    let (r, g, b) = split6(c);
    join6(
        r - ((r * evy + 7) >> 4),
        g - ((g * evy + 7) >> 4),
        b - ((b * evy + 7) >> 4),
    )
}

// ===========================================================================
// Registros de efectos de color y ventanas
// ===========================================================================

/// Registros de la unidad de mezcla: `BLDCNT`, `BLDALPHA` y `BLDY`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegistrosColor {
    pub bldcnt: u16,
    pub bldalpha: u16,
    pub bldy: u16,
}

impl RegistrosColor {
    /// Lee los tres registros de la I/O de la PPU.
    pub fn de_io(io: &[u8]) -> Self {
        let le = |off: usize| -> u16 {
            if io.len() >= off + 2 {
                u16::from_le_bytes([io[off], io[off + 1]])
            } else {
                0
            }
        };
        Self {
            bldcnt: le(0x50),
            bldalpha: le(0x52),
            bldy: le(0x54),
        }
    }

    /// Modo de la unidad de mezcla (bits 6-7 de `BLDCNT`): 0 ninguna,
    /// 1 alfa, 2 aclarar, 3 oscurecer.
    #[inline]
    pub fn modo(&self) -> u32 {
        ((self.bldcnt >> 6) & 3) as u32
    }

    /// EVA: bits 0-4 de `BLDALPHA`, saturado a 16 (`17..31` significa 16/16).
    #[inline]
    pub fn eva(&self) -> u32 {
        (self.bldalpha & 0x1F).min(16) as u32
    }

    /// EVB: **bits 8-12** de `BLDALPHA` (no 5-9), saturado a 16.
    #[inline]
    pub fn evb(&self) -> u32 {
        (((self.bldalpha >> 8) & 0x1F).min(16)) as u32
    }

    /// EVY: bits 0-4 de `BLDY`, saturado a 16.
    #[inline]
    pub fn evy(&self) -> u32 {
        (self.bldy & 0x1F).min(16) as u32
    }
}

/// Ventanas 0, 1 y OBJ con sus registros de control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ventanas {
    pub win0h: u16,
    pub win1h: u16,
    pub win0v: u16,
    pub win1v: u16,
    /// `WININ`: bits 0-5 para la ventana 0, bits 8-13 para la ventana 1.
    pub winin: u16,
    /// `WINOUT`: bits 0-5 fuera de ventana, bits 8-13 dentro de la ventana OBJ.
    pub winout: u16,
    pub win0_on: bool,
    pub win1_on: bool,
    pub objwin_on: bool,
}

impl Ventanas {
    /// Lee los registros de ventana de la I/O de la PPU y su habilitación en
    /// `DISPCNT` (bits 13, 14 y 15).
    pub fn de_io(io: &[u8], dispcnt: u16) -> Self {
        let le = |off: usize| -> u16 {
            if io.len() >= off + 2 {
                u16::from_le_bytes([io[off], io[off + 1]])
            } else {
                0
            }
        };
        Self {
            win0h: le(0x40),
            win1h: le(0x42),
            win0v: le(0x44),
            win1v: le(0x46),
            winin: le(0x48),
            winout: le(0x4A),
            win0_on: dispcnt & 0x2000 != 0,
            win1_on: dispcnt & 0x4000 != 0,
            objwin_on: dispcnt & 0x8000 != 0,
        }
    }

    /// ¿Hay alguna ventana activa? Sin ventanas la máscara de control es `0x3F`.
    #[inline]
    pub fn alguna(&self) -> bool {
        self.win0_on || self.win1_on || self.objwin_on
    }
}

/// Prueba de rango de ventana: `x1` es el borde izquierdo (incluido) y `x2` el
/// derecho (excluido). Si `x1 > x2` el rango da la vuelta.
#[inline]
pub fn en_rango(x1: u8, x2: u8, v: u8) -> bool {
    if x1 <= x2 {
        (x1..x2).contains(&v)
    } else {
        v >= x1 || v < x2
    }
}

/// Máscara de control de un píxel: qué capas se ven y si hay efectos.
///
/// Bits 0-3: un fondo cada uno. Bit 4: sprites. Bit 5: efectos de color.
pub fn mascara_control(vent: &Ventanas, x: u32, y: u32, obj_window: bool) -> u16 {
    if !vent.alguna() {
        return 0x3F;
    }
    let px = x as u8;
    let py = y as u8;
    if vent.win0_on
        && en_rango((vent.win0v >> 8) as u8, vent.win0v as u8, py)
        && en_rango((vent.win0h >> 8) as u8, vent.win0h as u8, px)
    {
        vent.winin & 0x3F
    } else if vent.win1_on
        && en_rango((vent.win1v >> 8) as u8, vent.win1v as u8, py)
        && en_rango((vent.win1h >> 8) as u8, vent.win1h as u8, px)
    {
        (vent.winin >> 8) & 0x3F
    } else if vent.objwin_on && obj_window {
        (vent.winout >> 8) & 0x3F
    } else {
        vent.wout_mask()
    }
}

impl Ventanas {
    /// Máscara de `WINOUT` para los píxeles fuera de toda ventana.
    #[inline]
    fn wout_mask(&self) -> u16 {
        self.winout & 0x3F
    }
}

// ===========================================================================
// Composición por píxel (puro)
// ===========================================================================

/// Id de capa: 0-3 fondos, 4 sprites, 5 fondo de pantalla.
pub const CAPA_OBJ: u8 = 4;
/// Id de capa del fondo de pantalla (el color 0 de la paleta).
pub const CAPA_FONDO: u8 = 5;

/// Píxel de sprite winner de un píxel de pantalla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelObj {
    pub color: u16,
    /// Prioridad del sprite (0 = delante). `0xFF` si no hay sprite.
    pub prioridad: u8,
    /// ¿El sprite es semitransparente?
    pub semi: bool,
}

impl PixelObj {
    /// Sprite inexistente en ese píxel.
    pub const VACIO: PixelObj = PixelObj {
        color: TRANSPARENTE,
        prioridad: 0xFF,
        semi: false,
    };
}

/// Compone el píxel final aplicando las reglas de `compose()` de la referencia.
///
/// `bg` son los colores de los cuatro fondos (`TRANSPARENTE` si no se ve) y
/// `bg_prio` sus prioridades (`0xFF` si el fondo no se usa en este modo).
pub fn compone_pixel(
    bg: &[u16; 4],
    bg_prio: &[u8; 4],
    obj: PixelObj,
    control: u16,
    reg: &RegistrosColor,
    backdrop: u16,
) -> u16 {
    // Las dos capas más altas: primero por prioridad (0 = delante), y a igual
    // prioridad el sprite gana a los fondos, y entre fondos gana el de índice menor.
    let obj_visible = obj.prioridad != 0xFF && control & 0x10 != 0;

    let mut primera: Option<(u16, u8)> = None;
    let mut segunda: Option<(u16, u8)> = None;
    let mut corte = false;
    for p in 0..4u8 {
        if corte {
            break;
        }
        if obj_visible && obj.prioridad == p {
            match primera {
                None => primera = Some((obj.color, CAPA_OBJ)),
                Some(_) => {
                    segunda = Some((obj.color, CAPA_OBJ));
                    corte = true;
                }
            }
        }
        for bg_idx in 0..4usize {
            if corte {
                break;
            }
            if bg_prio[bg_idx] == p && control & (1 << bg_idx) != 0 {
                let c = bg[bg_idx];
                if c != TRANSPARENTE {
                    match primera {
                        None => primera = Some((c, bg_idx as u8)),
                        Some(_) => {
                            segunda = Some((c, bg_idx as u8));
                            corte = true;
                        }
                    }
                }
            }
        }
    }

    let (fc, fl) = primera.unwrap_or((backdrop, CAPA_FONDO));
    let (sc, sl) = segunda.unwrap_or((backdrop, CAPA_FONDO));

    let efectos_permitidos = control & 0x20 != 0;
    let semi = obj_visible && fl == CAPA_OBJ && obj.semi;
    let modo = reg.modo();

    if semi && reg.bldcnt & (0x100 << sl) != 0 {
        // Un sprite semitransparente fuerza el alfa contra el 2.º objetivo.
        alpha_blend(fc, sc, reg.eva(), reg.evb())
    } else if efectos_permitidos && modo != 0 && reg.bldcnt & (1 << fl) != 0 {
        match modo {
            1 if reg.bldcnt & (0x100 << sl) != 0 => alpha_blend(fc, sc, reg.eva(), reg.evb()),
            2 => brighten(fc, reg.evy()),
            3 => darken(fc, reg.evy()),
            _ => fc,
        }
    } else {
        fc
    }
}

// ===========================================================================
// Configuración de una capa de fondo para la GPU
// ===========================================================================

/// Configuración de un fondo (textos) tal y como la lee la GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct CapaFondoGpu {
    /// ¿Se dibuja este fondo?
    pub enabled: u32,
    /// Prioridad (`BGxCNT` bits 0-1).
    pub priority: u32,
    /// Base de caracteres, en bytes.
    pub char_base: u32,
    /// Base del mapa de pantalla, en bytes.
    pub screen_base: u32,
    /// Tamaño del mapa (0: 32x32, 1: 64x32, 2: 32x64, 3: 64x64 tiles).
    ///
    /// En un fondo afín el mismo campo significa otra cosa: 16, 32, 64 o 128
    /// tiles por lado (16 << bits 14-15 de `BGxCNT`).
    pub screen_size: u32,
    /// ¿8 bits por píxel?
    pub is_8bpp: u32,
    /// Desplazamiento horizontal (`BGxHOFS`).
    pub scroll_x: u32,
    /// Desplazamiento vertical (`BGxVOFS`).
    pub scroll_y: u32,
    /// ¿Es un fondo afín? (BG2 del modo 1; BG2 y BG3 del modo 2).
    pub es_afin: u32,
    /// ¿El mapa afín da la vuelta? (bit 13 de `BGxCNT`).
    pub afin_wrap: u32,
    /// `BGxPA`: paso horizontal por píxel, en 20.8 con signo.
    pub afin_pa: i32,
    /// `BGxPB`: paso del ancla por línea, en 20.8 con signo.
    pub afin_pb: i32,
    /// `BGxPC`: paso vertical por píxel, en 20.8 con signo.
    pub afin_pc: i32,
    /// `BGxPD`: paso del ancla por línea, en 20.8 con signo.
    pub afin_pd: i32,
    /// `BGxX`: ancla horizontal, en 20.8 con 28 bits con signo.
    pub afin_ref_x: i32,
    /// `BGxY`: ancla vertical, en 20.8 con 28 bits con signo.
    pub afin_ref_y: i32,
}

impl CapaFondoGpu {
    /// Lee la configuración de un fondo de la I/O de la PPU.
    pub fn de_io(io: &[u8], bg: usize, enabled: bool) -> Self {
        let le = |off: usize| -> u16 {
            if io.len() >= off + 2 {
                u16::from_le_bytes([io[off], io[off + 1]])
            } else {
                0
            }
        };
        let cnt = le(0x08 + bg * 2);
        Self {
            enabled: u32::from(enabled),
            priority: (cnt & 3) as u32,
            char_base: (((cnt >> 2) & 3) as u32) * 0x4000,
            screen_base: (((cnt >> 8) & 0x1F) as u32) * 0x800,
            screen_size: ((cnt >> 14) & 3) as u32,
            is_8bpp: u32::from(cnt & 0x80 != 0),
            scroll_x: (le(0x10 + bg * 4) & 0x1FF) as u32,
            scroll_y: (le(0x12 + bg * 4) & 0x1FF) as u32,
            es_afin: 0,
            afin_wrap: 0,
            afin_pa: 0,
            afin_pb: 0,
            afin_pc: 0,
            afin_pd: 0,
            afin_ref_x: 0,
            afin_ref_y: 0,
        }
    }

    /// Lee la configuración de un fondo **afín** de la I/O de la PPU.
    ///
    /// Los fondos afines tienen sus propios registros, aparte de `BGxCNT`:
    ///
    /// - `PA`, `PB`, `PC` y `PD`: enteros de 16 bits con signo en punto fijo
    ///   20.8 (`PA` y `PC` son el paso por píxel; `PB` y `PD`, el del ancla).
    /// - `BGxX` y `BGxY`: el ancla, con **28 bits** con signo: el bit 27 es el
    ///   signo y los cuatro bits de arriba no existen en el registro.
    ///
    /// `BG2` usa `0x20`-`0x2C` y `BG3`, `0x30`-`0x3C`. Es la misma aritmética
    /// que la referencia (`sign_extend28` y los registros leídos como `i16`).
    pub fn de_io_afin(io: &[u8], bg: usize, enabled: bool) -> Self {
        let mut capa = Self::de_io(io, bg, enabled);
        let le16 = |off: usize| -> u16 {
            if io.len() >= off + 2 {
                u16::from_le_bytes([io[off], io[off + 1]])
            } else {
                0
            }
        };
        let le32 = |off: usize| -> u32 {
            if io.len() >= off + 4 {
                u32::from_le_bytes([io[off], io[off + 1], io[off + 2], io[off + 3]])
            } else {
                0
            }
        };
        // Los 28 bits con signo: el bit 27 es el signo y los cuatro bits de
        // arriba del registro no existen.
        let signo28 = |v: u32| -> i32 { ((v << 4) as i32) >> 4 };
        let base = if bg == 2 { 0x20 } else { 0x30 };
        let cnt = le16(0x08 + bg * 2);
        capa.es_afin = 1;
        capa.afin_wrap = u32::from(cnt & 0x2000 != 0);
        capa.afin_pa = le16(base) as i16 as i32;
        capa.afin_pb = le16(base + 2) as i16 as i32;
        capa.afin_pc = le16(base + 4) as i16 as i32;
        capa.afin_pd = le16(base + 6) as i16 as i32;
        capa.afin_ref_x = signo28(le32(base + 8));
        capa.afin_ref_y = signo28(le32(base + 12));
        capa
    }
}

/// Configuración de la pasada de capas: el modo de vídeo y los cuatro fondos.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ConfigCapaGpu {
    /// Modo de vídeo (`DISPCNT` bits 0-2).
    pub modo: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub capas: [CapaFondoGpu; 4],
}

/// Configuración de la pasada de composición.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ConfigCompGpu {
    /// `DISPCNT`.
    pub dispcnt: u32,
    /// Prioridad de cada fondo (`0xFF` si no se usa en este modo).
    pub bg_prio0: u32,
    pub bg_prio1: u32,
    pub bg_prio2: u32,
    pub bg_prio3: u32,
    pub win0h: u32,
    pub win1h: u32,
    pub win0v: u32,
    pub win1v: u32,
    pub winin: u32,
    pub winout: u32,
    pub bldcnt: u32,
    pub bldalpha: u32,
    pub bldy: u32,
    /// Relleno hasta 64 bytes: el mínimo que el binding uniforme exige.
    pub _pad0: u32,
    pub _pad1: u32,
}

// ===========================================================================
// Sprites
// ===========================================================================

/// Sprite decodificado y listo para la GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SpriteNativa {
    /// Esquina superior izquierda de la caja del sprite, en píxeles de pantalla.
    pub pos_x: f32,
    pub pos_y: f32,
    /// Tamaño de la caja dibujada (el doble si el afín es de doble tamaño).
    pub caja_x: f32,
    pub caja_y: f32,
    /// Tamaño nominal de la textura del sprite.
    pub tex_x: f32,
    pub tex_y: f32,
    /// Tile base (`OBJATTR2` bits 0-9).
    pub base_tile: u32,
    /// Subpaleta (4bpp) o 0 (8bpp).
    pub palette_idx: u32,
    /// Banderas (ver `FLAG_*`).
    pub flags: u32,
    /// Prioridad del sprite (`OBJATTR2` bits 10-11).
    pub prioridad: u32,
    /// Parámetros de la transformación afín: pa, pb, pc, pd.
    pub afin_a: i32,
    pub afin_b: i32,
    pub afin_c: i32,
    pub afin_d: i32,
    /// Índice de la entrada de OAM de este sprite.
    ///
    /// La resolución de prioridad del sprite winner es «menor número de
    /// prioridad; a igualdad, menor índice de OAM». Como el sprite se dibuja
    /// en un G-buffer, ese desempate se resuelve con una **prueba de
    /// profundidad**: la clave `prioridad * 128 + índice` se escribe en el
    /// adjunto de profundidad y gana la menor.
    pub oam_index: u32,
}

/// Datos de un sprite que necesita la etapa de vértices.
///
/// La lista completa de sprites viaja en un *storage buffer* que solo puede
/// leer la etapa de fragmento: un *storage buffer* visible en la etapa de
/// vértices exige la capacidad `DownlevelFlags::VERTEX_STORAGE`, que el
/// estándar de OpenGL ES permite no tener (un dispositivo puede admitir
/// SSBO solo en fragmento). Por eso el vértice recibe la geometría y la
/// clave de prioridad como atributos de instancia, que son WebGPU básico.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SpriteInstanciaGpu {
    /// `pos_x`, `pos_y`, `caja_x` y `caja_y` del sprite.
    pub geometria: [f32; 4],
    /// `prioridad` e índice de OAM del sprite.
    pub clave: [u32; 2],
}

impl SpriteInstanciaGpu {
    /// Construye la instancia de un sprite ya decodificado.
    pub fn de_sprite(sprite: &SpriteNativa) -> Self {
        Self {
            geometria: [
                sprite.pos_x,
                sprite.pos_y,
                sprite.caja_x,
                sprite.caja_y,
            ],
            clave: [sprite.prioridad, sprite.oam_index],
        }
    }
}

/// Bandera: espejo horizontal.
pub const FLAG_HFLIP: u32 = 1 << 0;
/// Bandera: espejo vertical.
pub const FLAG_VFLIP: u32 = 1 << 1;
/// Bandera: sprite semitransparente.
pub const FLAG_SEMI: u32 = 1 << 2;
/// Bandera: 8 bits por píxel.
pub const FLAG_8BPP: u32 = 1 << 4;
/// Bandera: mapeo 1D de los tiles.
pub const FLAG_1D: u32 = 1 << 5;
/// Bandera: sprite afín.
pub const FLAG_AFIN: u32 = 1 << 6;

/// Tamaños nominales de los sprites: `[forma][tamaño]`.
pub const OBJ_SIZES: [[(u32, u32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)], // cuadrada
    [(16, 8), (32, 8), (32, 16), (64, 32)], // horizontal
    [(8, 16), (8, 32), (16, 32), (32, 64)], // vertical
];

// ===========================================================================
// Estado decodificado de un cuadro
// ===========================================================================

/// Estado de vídeo de un cuadro, decodificado de VRAM, OAM, paleta e I/O.
pub struct EstadoPpuNativa {
    /// Sprites con color (modos normal, semitransparente y prohibido), en orden
    /// de pintado: de fondo a frente.
    pub sprites: [SpriteNativa; MAX_SPRITES],
    pub num_sprites: usize,
    /// Sprites en modo ventana OBJ, que solo pintan la máscara de ventana.
    pub sprites_ventana: [SpriteNativa; MAX_SPRITES],
    pub num_sprites_ventana: usize,
    /// Configuración de los cuatro fondos.
    pub capas: [CapaFondoGpu; 4],
    /// Prioridad de cada fondo; `0xFF` si no se usa en este modo.
    pub bg_prio: [u32; 4],
    /// `DISPCNT`.
    pub dispcnt: u16,
    /// Modo de vídeo.
    pub modo: u32,
    /// Registros de la unidad de mezcla.
    pub color: RegistrosColor,
    /// Ventanas.
    pub ventanas: Ventanas,
}

impl Default for EstadoPpuNativa {
    fn default() -> Self {
        Self::new()
    }
}

impl EstadoPpuNativa {
    /// Crea un estado vacío.
    pub fn new() -> Self {
        Self {
            sprites: [SpriteNativa::default(); MAX_SPRITES],
            num_sprites: 0,
            sprites_ventana: [SpriteNativa::default(); MAX_SPRITES],
            num_sprites_ventana: 0,
            capas: [CapaFondoGpu::default(); 4],
            bg_prio: [0xFF; 4],
            dispcnt: 0,
            modo: 0,
            color: RegistrosColor::default(),
            ventanas: Ventanas::default(),
        }
    }

    /// Decodifica `DISPCNT`, `BGxCNT`, `BGxHOFS`, `BGxVOFS`, los registros de
    /// mezcla y los de ventana.
    pub fn decode_io_regs(&mut self, io: &[u8]) {
        let le = |off: usize| -> u16 {
            if io.len() >= off + 2 {
                u16::from_le_bytes([io[off], io[off + 1]])
            } else {
                0
            }
        };
        self.dispcnt = le(0x00);
        self.modo = (self.dispcnt & 7) as u32;
        self.color = RegistrosColor::de_io(io);
        self.ventanas = Ventanas::de_io(io, self.dispcnt);

        for bg in 0..4usize {
            // Modo 0: los cuatro fondos son de texto. Modo 1: BG0 y BG1 de
            // texto y BG2 afín (lo activa el bit 10 de `DISPCNT`, que en el
            // modo 0 es el bit de habilitación de BG2). El modo 2 (BG2 y BG3
            // afines) y los modos bitmap quedan fuera de esta fase.
            let es_texto = match self.modo {
                0 => bg < 4,
                1 => bg < 2,
                _ => false,
            };
            let es_afin = self.modo == 1 && bg == 2;
            let enabled = if es_texto {
                self.dispcnt & (0x100 << bg) != 0
            } else if es_afin {
                self.dispcnt & 0x400 != 0
            } else {
                false
            };
            self.capas[bg] = if es_afin {
                CapaFondoGpu::de_io_afin(io, bg, enabled)
            } else {
                CapaFondoGpu::de_io(io, bg, enabled)
            };
            self.bg_prio[bg] = if enabled {
                self.capas[bg].priority
            } else {
                0xFF
            };
        }
    }

    /// Decodifica la tabla OAM.
    ///
    /// Se recorre **del último índice al primero** para que el orden estable por
    /// prioridad (de mayor a menor) deje al frente, dentro de cada prioridad, el
    /// sprite de índice OAM más bajo: es el que gana en la referencia.
    ///
    /// La referencia solo dibuja sprites si `DISPCNT` bit 12 está activo.
    pub fn decode_oam(&mut self, oam: &[u8]) -> usize {
        self.num_sprites = 0;
        self.num_sprites_ventana = 0;
        if oam.len() < 1024 || self.dispcnt & 0x1000 == 0 {
            return 0;
        }
        let one_dim = self.dispcnt & 0x40 != 0;

        for i in (0..MAX_SPRITES).rev() {
            let off = i * 8;
            let a0 = u16::from_le_bytes([oam[off], oam[off + 1]]);
            let a1 = u16::from_le_bytes([oam[off + 2], oam[off + 3]]);
            let a2 = u16::from_le_bytes([oam[off + 4], oam[off + 5]]);

            let affine = a0 & 0x100 != 0;
            if !affine && a0 & 0x200 != 0 {
                continue; // deshabilitado
            }
            let shape = ((a0 >> 14) & 3) as usize;
            if shape == 3 {
                continue; // forma prohibida
            }
            let size_idx = ((a1 >> 14) & 3) as usize;
            let (w, h) = OBJ_SIZES[shape][size_idx];
            let doble = affine && a0 & 0x200 != 0;
            let (bw, bh) = if doble { (w * 2, h * 2) } else { (w, h) };

            let mut y = (a0 & 0xFF) as i32;
            if y >= 160 {
                y -= 256;
            }
            let mut x = (a1 & 0x1FF) as i32;
            if x >= 240 {
                x -= 512;
            }
            // Poda de lo que no toca la pantalla visible.
            if x + bw as i32 <= 0
                || x >= GBA_SCREEN_W as i32
                || y + bh as i32 <= 0
                || y >= GBA_SCREEN_H as i32
            {
                continue;
            }

            let modo_obj = (a0 >> 10) & 3;
            let eight_bpp = a0 & 0x2000 != 0;
            let tile_base = (a2 & 0x3FF) as u32;
            let prioridad = ((a2 >> 10) & 3) as u32;
            let palette = ((a2 >> 12) & 0xF) as u32;

            let (pa, pb, pc, pd) = if affine {
                let g = (((a1 >> 9) & 0x1F) as usize) * 32;
                let p = |despl: usize| -> i32 {
                    u16::from_le_bytes([oam[g + despl], oam[g + despl + 1]]) as i16 as i32
                };
                (p(6), p(14), p(22), p(30))
            } else {
                (0x100, 0, 0, 0x100)
            };

            let mut flags = 0u32;
            if !affine {
                if a1 & 0x1000 != 0 {
                    flags |= FLAG_HFLIP;
                }
                if a1 & 0x2000 != 0 {
                    flags |= FLAG_VFLIP;
                }
            }
            if modo_obj == 1 {
                flags |= FLAG_SEMI;
            }
            if eight_bpp {
                flags |= FLAG_8BPP;
            }
            if one_dim {
                flags |= FLAG_1D;
            }
            if affine {
                flags |= FLAG_AFIN;
            }

            let sprite = SpriteNativa {
                pos_x: x as f32,
                pos_y: y as f32,
                caja_x: bw as f32,
                caja_y: bh as f32,
                tex_x: w as f32,
                tex_y: h as f32,
                base_tile: tile_base,
                palette_idx: if eight_bpp { 0 } else { palette },
                flags,
                prioridad,
                afin_a: pa,
                afin_b: pb,
                afin_c: pc,
                afin_d: pd,
                oam_index: i as u32,
            };

            if modo_obj == 2 {
                // Modo ventana OBJ: solo pinta la máscara, no el color.
                if self.num_sprites_ventana < MAX_SPRITES {
                    self.sprites_ventana[self.num_sprites_ventana] = sprite;
                    self.num_sprites_ventana += 1;
                }
            } else {
                if self.num_sprites < MAX_SPRITES {
                    self.sprites[self.num_sprites] = sprite;
                    self.num_sprites += 1;
                }
            }
        }

        // Orden de pintado: prioridad descendente (3 al fondo, 0 al frente). El
        // orden dentro de cada prioridad ya viene del recorrido inverso de OAM.
        // El desempate real lo hace la prueba de profundidad, así que este orden
        // solo fija una secuencia de dibujado determinista.
        self.sprites[..self.num_sprites].sort_by(|a, b| b.prioridad.cmp(&a.prioridad));
        self.num_sprites
    }

    /// Configuración de la pasada de capas.
    pub fn config_capa(&self) -> ConfigCapaGpu {
        ConfigCapaGpu {
            modo: self.modo,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            capas: self.capas,
        }
    }

    /// Configuración de la pasada de composición.
    pub fn config_comp(&self) -> ConfigCompGpu {
        ConfigCompGpu {
            dispcnt: self.dispcnt as u32,
            bg_prio0: self.bg_prio[0],
            bg_prio1: self.bg_prio[1],
            bg_prio2: self.bg_prio[2],
            bg_prio3: self.bg_prio[3],
            win0h: self.ventanas.win0h as u32,
            win1h: self.ventanas.win1h as u32,
            win0v: self.ventanas.win0v as u32,
            win1v: self.ventanas.win1v as u32,
            winin: self.ventanas.winin as u32,
            winout: self.ventanas.winout as u32,
            bldcnt: self.color.bldcnt as u32,
            bldalpha: self.color.bldalpha as u32,
            bldy: self.color.bldy as u32,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

// ===========================================================================
// Pipeline de renderizado GPU
// ===========================================================================

/// Vértice de quad unitario `[0..1]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct QuadVertex {
    pub pos: [f32; 2],
}

/// Los seis vértices de dos triángulos que cubren la pantalla.
pub const QUAD_VERTICES: [QuadVertex; 6] = [
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [0.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
];

/// Pipeline de la PPU nativa: capas en G-buffer y composición por píxel.
pub struct PpuNativaPipeline {
    /// Estado decodificado del cuadro en curso.
    pub estado: EstadoPpuNativa,

    pub pipeline_capa: wgpu::RenderPipeline,
    pub pipeline_obj: wgpu::RenderPipeline,
    pub pipeline_obj_ventana: wgpu::RenderPipeline,
    pub pipeline_comp: wgpu::RenderPipeline,

    pub quad_buffer: wgpu::Buffer,
    pub vram_buffer: wgpu::Buffer,
    pub palette_buffer: wgpu::Buffer,
    pub config_capa_buffer: wgpu::Buffer,
    pub config_comp_buffer: wgpu::Buffer,
    pub sprites_buffer: wgpu::Buffer,
    pub sprites_ventana_buffer: wgpu::Buffer,
    /// Geometría de cada sprite para la etapa de vértices: llega como
    /// atributos de instancia y no como *storage buffer*, que el vértice no
    /// puede leer en todos los dispositivos.
    pub sprites_instancia_buffer: wgpu::Buffer,
    pub sprites_instancia_ventana_buffer: wgpu::Buffer,

    /// Texturas del G-buffer: una por fondo, con el color de 15 bits de cada
    /// píxel.
    pub tex_capa: [wgpu::Texture; 4],
    pub vista_capa: [wgpu::TextureView; 4],
    /// Capa de sprites: color, prioridad, semitransparencia y presencia.
    pub tex_obj: wgpu::Texture,
    pub vista_obj: wgpu::TextureView,
    /// Máscara de la ventana OBJ.
    pub tex_obj_ventana: wgpu::Texture,
    pub vista_obj_ventana: wgpu::TextureView,
    /// Adjunto de profundidad de la pasada de sprites: resuelve la prioridad
    /// (menor número; a igualdad, menor índice de OAM) sin depender del orden
    /// de dibujado.
    pub tex_profundidad: wgpu::Texture,
    pub vista_profundidad: wgpu::TextureView,

    pub grupo_capa: wgpu::BindGroup,
    pub grupo_obj: wgpu::BindGroup,
    pub grupo_obj_ventana: wgpu::BindGroup,
    pub grupo_comp: wgpu::BindGroup,

    staging_palette: Box<[u32; 512]>,
    staging_sprites: Box<[SpriteNativa; MAX_SPRITES]>,
    staging_sprites_ventana: Box<[SpriteNativa; MAX_SPRITES]>,
    staging_sprites_instancia: Box<[SpriteInstanciaGpu; MAX_SPRITES]>,
    staging_sprites_instancia_ventana: Box<[SpriteInstanciaGpu; MAX_SPRITES]>,
}

/// Etiquetas estáticas de las cuatro texturas de capa.
const ETIQUETAS_CAPA: [&str; 4] = [
    "ppu-nativa-capa-0",
    "ppu-nativa-capa-1",
    "ppu-nativa-capa-2",
    "ppu-nativa-capa-3",
];

/// Descriptor de una textura del G-buffer de fondos (un canal de 16 bits).
fn textura_capa(label: &'static str) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: GBA_SCREEN_W,
            height: GBA_SCREEN_H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R16Uint,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }
}

/// Entrada de layout para un búfer de solo lectura.
fn entrada_storage(binding: u32, visibilidad: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: visibilidad,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Entrada de layout para un búfer uniforme.
fn entrada_uniforme(binding: u32, visibilidad: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: visibilidad,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Entrada de layout para una textura entera sin muestrear.
fn entrada_textura(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// Entradas del layout de enlaces de la pasada de capas.
fn entradas_bgl_capa() -> [wgpu::BindGroupLayoutEntry; 3] {
    [
        entrada_storage(BINDING_VRAM, wgpu::ShaderStages::FRAGMENT),
        entrada_storage(BINDING_PALETA, wgpu::ShaderStages::FRAGMENT),
        entrada_uniforme(BINDING_CONFIG_CAPA, wgpu::ShaderStages::FRAGMENT),
    ]
}

/// Entradas del layout de enlaces de la pasada de sprites.
///
/// La lista de sprites se declara **solo para la etapa de fragmento**: verla
/// desde el vértice exigiría la capacidad `DownlevelFlags::VERTEX_STORAGE`,
/// que no todos los dispositivos dan (OpenGL ES permite tener SSBO solo en
/// fragmento) y cuya ausencia aborta la creación del layout.
fn entradas_bgl_obj() -> [wgpu::BindGroupLayoutEntry; 4] {
    [
        entrada_storage(BINDING_VRAM, wgpu::ShaderStages::FRAGMENT),
        entrada_storage(BINDING_PALETA, wgpu::ShaderStages::FRAGMENT),
        entrada_uniforme(BINDING_CONFIG_CAPA, wgpu::ShaderStages::FRAGMENT),
        entrada_storage(BINDING_SPRITES, wgpu::ShaderStages::FRAGMENT),
    ]
}

/// Entradas del layout de enlaces de la pasada de la ventana OBJ.
///
/// No consulta la paleta: solo necesita saber si el píxel está cubierto por
/// un sprite.
fn entradas_bgl_obj_ventana() -> [wgpu::BindGroupLayoutEntry; 3] {
    [
        entrada_storage(BINDING_VRAM, wgpu::ShaderStages::FRAGMENT),
        entrada_uniforme(BINDING_CONFIG_CAPA, wgpu::ShaderStages::FRAGMENT),
        entrada_storage(BINDING_SPRITES, wgpu::ShaderStages::FRAGMENT),
    ]
}

/// Entradas del layout de enlaces de la pasada de composición.
fn entradas_bgl_comp() -> [wgpu::BindGroupLayoutEntry; 8] {
    [
        entrada_storage(BINDING_PALETA, wgpu::ShaderStages::FRAGMENT),
        entrada_uniforme(BINDING_CONFIG_COMP, wgpu::ShaderStages::FRAGMENT),
        entrada_textura(BINDING_TEX_CAPA0),
        entrada_textura(BINDING_TEX_CAPA0 + 1),
        entrada_textura(BINDING_TEX_CAPA0 + 2),
        entrada_textura(BINDING_TEX_CAPA0 + 3),
        entrada_textura(BINDING_TEX_OBJ),
        entrada_textura(BINDING_TEX_OBJ_VENTANA),
    ]
}

/// Layout del búfer de instancias: la geometría y la clave de prioridad de
/// cada sprite.
///
/// Son atributos de vértice (WebGPU básico) y no un *storage buffer`, para
/// que la etapa de vértices no dependa de `DownlevelFlags::VERTEX_STORAGE`.
fn layout_instancia() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteInstanciaGpu>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Uint32x2,
            },
        ],
    }
}

impl PpuNativaPipeline {
    /// Crea el pipeline: texturas de G-buffer, shaders y grupos de enlaces.
    ///
    /// La construcción va dentro del guardián de errores: si la GPU del
    /// dispositivo rechaza algún recurso, se devuelve el error en vez de dejar
    /// que `wgpu` entre en pánico, y el llamante puede seguir con el *shim*.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Result<Self, String> {
        construir_con_guardian(device, "PPU nativa", || {
            Self::construir(device, target_format)
        })
    }

    /// Construcción efectiva del pipeline (sin guardián).
    fn construir(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ppu-nativa-shader"),
            source: wgpu::ShaderSource::Wgsl(PPU_NATIVA_WGSL.into()),
        });

        // --- Texturas del G-buffer ---
        let tex_capa: [wgpu::Texture; 4] = std::array::from_fn(|i| {
            device.create_texture(&textura_capa(ETIQUETAS_CAPA[i]))
        });
        let vista_capa: [wgpu::TextureView; 4] =
            std::array::from_fn(|i| tex_capa[i].create_view(&wgpu::TextureViewDescriptor::default()));

        let tex_obj = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ppu-nativa-obj"),
            size: wgpu::Extent3d {
                width: GBA_SCREEN_W,
                height: GBA_SCREEN_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vista_obj = tex_obj.create_view(&wgpu::TextureViewDescriptor::default());

        let tex_obj_ventana = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ppu-nativa-obj-ventana"),
            size: wgpu::Extent3d {
                width: GBA_SCREEN_W,
                height: GBA_SCREEN_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vista_obj_ventana =
            tex_obj_ventana.create_view(&wgpu::TextureViewDescriptor::default());

        let tex_profundidad = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ppu-nativa-profundidad"),
            size: wgpu::Extent3d {
                width: GBA_SCREEN_W,
                height: GBA_SCREEN_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let vista_profundidad =
            tex_profundidad.create_view(&wgpu::TextureViewDescriptor::default());

        // --- Buffers ---
        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-nativa-quad"),
            size: (std::mem::size_of::<QuadVertex>() * QUAD_VERTICES.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let storage = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let vram_buffer = storage("ppu-nativa-vram", 96 * 1024);
        let palette_buffer = storage("ppu-nativa-palette", 512 * 4);
        let config_capa_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-nativa-config-capa"),
            size: std::mem::size_of::<ConfigCapaGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let config_comp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-nativa-config-comp"),
            size: std::mem::size_of::<ConfigCompGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sprites_buffer = storage(
            "ppu-nativa-sprites",
            (std::mem::size_of::<SpriteNativa>() * MAX_SPRITES) as u64,
        );
        let sprites_ventana_buffer = storage(
            "ppu-nativa-sprites-ventana",
            (std::mem::size_of::<SpriteNativa>() * MAX_SPRITES) as u64,
        );
        // El vértice no puede leer la lista de sprites del *storage buffer* en
        // dispositivos sin `VERTEX_STORAGE`: su geometría llega en un búfer de
        // instancias, que es WebGPU básico.
        let sprites_instancia_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-nativa-sprites-instancia"),
            size: (std::mem::size_of::<SpriteInstanciaGpu>() * MAX_SPRITES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sprites_instancia_ventana_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-nativa-sprites-instancia-ventana"),
            size: (std::mem::size_of::<SpriteInstanciaGpu>() * MAX_SPRITES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Layouts de enlaces ---
        //
        // Se declaran **explícitamente** en vez de dejar que `wgpu` los deduzca
        // del shader: la deducción automática hace que cada pasada acepte solo
        // los enlaces que su punto de entrada menciona, y un desajuste entre
        // el shader y Rust se manifestaría como un error de validación
        // confuso. Con el layout explícito, el grupo de enlaces y el shader
        // dicen lo mismo desde el principio.
        let bgl_capa = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppu-nativa-bgl-capa"),
            entries: &entradas_bgl_capa(),
        });
        let bgl_obj = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppu-nativa-bgl-obj"),
            entries: &entradas_bgl_obj(),
        });
        let bgl_obj_ventana = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppu-nativa-bgl-obj-ventana"),
            entries: &entradas_bgl_obj_ventana(),
        });
        let bgl_comp = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppu-nativa-bgl-comp"),
            entries: &entradas_bgl_comp(),
        });

        let pl_capa = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppu-nativa-pl-capa"),
            bind_group_layouts: &[&bgl_capa],
            push_constant_ranges: &[],
        });
        let pl_obj = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppu-nativa-pl-obj"),
            bind_group_layouts: &[&bgl_obj],
            push_constant_ranges: &[],
        });
        let pl_obj_ventana = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppu-nativa-pl-obj-ventana"),
            bind_group_layouts: &[&bgl_obj_ventana],
            push_constant_ranges: &[],
        });
        let pl_comp = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppu-nativa-pl-comp"),
            bind_group_layouts: &[&bgl_comp],
            push_constant_ranges: &[],
        });

        // --- Pipelines ---
        let layout_quad = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };

        let pipeline_capa = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-nativa-pipeline-capa"),
            layout: Some(&pl_capa),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_quad"),
                compilation_options: Default::default(),
                buffers: &[layout_quad.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_capa"),
                compilation_options: Default::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let pipeline_obj = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-nativa-pipeline-obj"),
            layout: Some(&pl_obj),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_obj"),
                compilation_options: Default::default(),
                buffers: &[layout_quad.clone(), layout_instancia()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_obj"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let pipeline_obj_ventana = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-nativa-pipeline-obj-ventana"),
            layout: Some(&pl_obj_ventana),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_obj"),
                compilation_options: Default::default(),
                buffers: &[layout_quad.clone(), layout_instancia()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_obj_ventana"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let pipeline_comp = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-nativa-pipeline-comp"),
            layout: Some(&pl_comp),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_quad"),
                compilation_options: Default::default(),
                buffers: &[layout_quad.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_comp"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- Grupos de enlaces (uno por pipeline: cada uno usa un subconjunto
        // distinto de los recursos) ---
        let grupo_capa = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppu-nativa-grupo-capa"),
            layout: &bgl_capa,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BINDING_VRAM,
                    resource: vram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_PALETA,
                    resource: palette_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_CONFIG_CAPA,
                    resource: config_capa_buffer.as_entire_binding(),
                },
            ],
        });

        let grupo_obj = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppu-nativa-grupo-obj"),
            layout: &bgl_obj,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BINDING_VRAM,
                    resource: vram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_PALETA,
                    resource: palette_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_CONFIG_CAPA,
                    resource: config_capa_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_SPRITES,
                    resource: sprites_buffer.as_entire_binding(),
                },
            ],
        });

        let grupo_obj_ventana = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppu-nativa-grupo-obj-ventana"),
            layout: &bgl_obj_ventana,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BINDING_VRAM,
                    resource: vram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_CONFIG_CAPA,
                    resource: config_capa_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_SPRITES,
                    resource: sprites_ventana_buffer.as_entire_binding(),
                },
            ],
        });

        let grupo_comp = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppu-nativa-grupo-comp"),
            layout: &bgl_comp,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BINDING_PALETA,
                    resource: palette_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_CONFIG_COMP,
                    resource: config_comp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_CAPA0,
                    resource: wgpu::BindingResource::TextureView(&vista_capa[0]),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_CAPA0 + 1,
                    resource: wgpu::BindingResource::TextureView(&vista_capa[1]),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_CAPA0 + 2,
                    resource: wgpu::BindingResource::TextureView(&vista_capa[2]),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_CAPA0 + 3,
                    resource: wgpu::BindingResource::TextureView(&vista_capa[3]),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_OBJ,
                    resource: wgpu::BindingResource::TextureView(&vista_obj),
                },
                wgpu::BindGroupEntry {
                    binding: BINDING_TEX_OBJ_VENTANA,
                    resource: wgpu::BindingResource::TextureView(&vista_obj_ventana),
                },
            ],
        });

        Self {
            estado: EstadoPpuNativa::new(),
            pipeline_capa,
            pipeline_obj,
            pipeline_obj_ventana,
            pipeline_comp,
            quad_buffer,
            vram_buffer,
            palette_buffer,
            config_capa_buffer,
            config_comp_buffer,
            sprites_buffer,
            sprites_ventana_buffer,
            sprites_instancia_buffer,
            sprites_instancia_ventana_buffer,
            tex_capa,
            vista_capa,
            tex_obj,
            vista_obj,
            tex_obj_ventana,
            vista_obj_ventana,
            tex_profundidad,
            vista_profundidad,
            grupo_capa,
            grupo_obj,
            grupo_obj_ventana,
            grupo_comp,
            staging_palette: Box::new([0u32; 512]),
            staging_sprites: Box::new([SpriteNativa::default(); MAX_SPRITES]),
            staging_sprites_ventana: Box::new([SpriteNativa::default(); MAX_SPRITES]),
            staging_sprites_instancia: Box::new([SpriteInstanciaGpu::default(); MAX_SPRITES]),
            staging_sprites_instancia_ventana: Box::new([SpriteInstanciaGpu::default(); MAX_SPRITES]),
        }
    }

    /// Dibuja un cuadro completo en la vista de textura destino.
    pub fn render_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        vram: &[u8],
        oam: &[u8],
        palette: &[u8],
        io: &[u8],
    ) {
        // 1. Decodificación del estado del cuadro.
        self.estado.decode_io_regs(io);
        self.estado.decode_oam(oam);

        // 2. Paleta: los 512 colores RGB555, enmascarados a 15 bits como hace
        //    la referencia al leerlos.
        {
            let staging = &mut self.staging_palette;
            for (i, slot) in staging.iter_mut().enumerate() {
                let off = i * 2;
                *slot = if palette.len() >= off + 2 {
                    u16::from_le_bytes([palette[off], palette[off + 1]]) & 0x7FFF
                } else {
                    0
                } as u32;
            }
            queue.write_buffer(&self.palette_buffer, 0, bytemuck::cast_slice(&staging[..]));
        }

        // 3. VRAM completa (96 KB).
        let vram_len = (96 * 1024).min(vram.len());
        queue.write_buffer(&self.vram_buffer, 0, bytemuck::cast_slice(&vram[..vram_len]));

        // 4. Configuración de las pasadas.
        queue.write_buffer(
            &self.config_capa_buffer,
            0,
            bytemuck::bytes_of(&self.estado.config_capa()),
        );
        queue.write_buffer(
            &self.config_comp_buffer,
            0,
            bytemuck::bytes_of(&self.estado.config_comp()),
        );

        // 5. Sprites.
        {
            let n = self.estado.num_sprites;
            self.staging_sprites[..n].copy_from_slice(&self.estado.sprites[..n]);
            queue.write_buffer(
                &self.sprites_buffer,
                0,
                bytemuck::cast_slice(&self.staging_sprites[..n]),
            );
            // La geometría que necesita el vértice viaja en un búfer de
            // instancias, en el mismo orden que la lista: así el índice de
            // instancia y el índice del sprite en la lista coinciden.
            for (instancia, sprite) in self.staging_sprites_instancia[..n]
                .iter_mut()
                .zip(self.estado.sprites[..n].iter())
            {
                *instancia = SpriteInstanciaGpu::de_sprite(sprite);
            }
            queue.write_buffer(
                &self.sprites_instancia_buffer,
                0,
                bytemuck::cast_slice(&self.staging_sprites_instancia[..n]),
            );
            let nv = self.estado.num_sprites_ventana;
            self.staging_sprites_ventana[..nv]
                .copy_from_slice(&self.estado.sprites_ventana[..nv]);
            queue.write_buffer(
                &self.sprites_ventana_buffer,
                0,
                bytemuck::cast_slice(&self.staging_sprites_ventana[..nv]),
            );
            for (instancia, sprite) in self.staging_sprites_instancia_ventana[..nv]
                .iter_mut()
                .zip(self.estado.sprites_ventana[..nv].iter())
            {
                *instancia = SpriteInstanciaGpu::de_sprite(sprite);
            }
            queue.write_buffer(
                &self.sprites_instancia_ventana_buffer,
                0,
                bytemuck::cast_slice(&self.staging_sprites_instancia_ventana[..nv]),
            );
        }

        // 6. Geometría.
        queue.write_buffer(&self.quad_buffer, 0, bytemuck::cast_slice(&QUAD_VERTICES));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ppu-nativa-encoder"),
        });

        // 7. Pasada de capas: los cuatro fondos, cada uno a su textura.
        {
            let attachments = [
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_capa[0],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_capa[1],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_capa[2],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_capa[3],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ppu-nativa-pasada-capa"),
                color_attachments: &attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline_capa);
            pass.set_bind_group(0, &self.grupo_capa, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.draw(0..6, 0..1);
        }

        // 8. Pasada de sprites: el color del sprite que gana en cada píxel.
        //    La prueba de profundidad resuelve la prioridad sin depender del
        //    orden de dibujado: gana el sprite con menor número de prioridad y,
        //    a igualdad, el de menor índice de OAM.
        if self.estado.num_sprites > 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ppu-nativa-pasada-obj"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_obj,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.vista_profundidad,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline_obj);
            pass.set_bind_group(0, &self.grupo_obj, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.sprites_instancia_buffer.slice(..));
            pass.draw(0..6, 0..self.estado.num_sprites as u32);
        }

        // 9. Pasada de la ventana OBJ: la máscara de píxeles cubiertos por un
        //    sprite en modo ventana.
        if self.estado.num_sprites_ventana > 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ppu-nativa-pasada-obj-ventana"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.vista_obj_ventana,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline_obj_ventana);
            pass.set_bind_group(0, &self.grupo_obj_ventana, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.sprites_instancia_ventana_buffer.slice(..));
            pass.draw(0..6, 0..self.estado.num_sprites_ventana as u32);
        }

        // 10. Pasada de composición: el píxel final.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ppu-nativa-pasada-comp"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline_comp);
            pass.set_bind_group(0, &self.grupo_comp, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.draw(0..6, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

// ---------------------------------------------------------------------------
// Suite de pruebas satélite (AGENTS.md §4.4)
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "ppu_nativa_tests.rs"]
mod tests;
