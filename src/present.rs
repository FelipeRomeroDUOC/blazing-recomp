//! Calibración de presentación del swapchain (módulo puro, testeable en host).
//!
//! **Contexto medido (tablet Snapdragon 610, `blazing-perf-4.txt`):** el
//! fotograma se bloquea en `frame.present()` (`present` ≈ 52 ms en retrato y
//! ≈ 33 ms en apaisado, esto es 3,34× y 2,18× el intervalo de VSync), mientras
//! que el coste propio del port (`subida + codificacion + espera`) es de
//! ~2,5 ms y en retrato se dibuja *menos* que en apaisado. La causa no es
//! nuestra carga, es el acoplamiento con el compositor.
//!
//! `wgpu-hal` pide al driver `min_image_count = latencia + 1`
//! (`vulkan/device.rs`, `create_swapchain`), de modo que
//! `desired_maximum_frame_latency: 1` deja el swapchain con **2 imágenes**:
//! `Mailbox` no puede reemplazar un fotograma pendiente (necesita una tercera
//! imagen donde renderizar) y `queue_present` degenera en bloqueo hasta que el
//! compositor libera un búfer.
//!
//! Este módulo recorre unas pocas configuraciones al arrancar, mide `present`
//! en cada una y elige la mejor. La política es conservadora a propósito:
//! solo adopta cambios con mejora medible sobre la referencia.
//!
//! Dos de los tramos son **sondas** (rol [`Rol::Sonda`]), medidas pero nunca
//! adoptadas:
//! - presentar sin VSync, para saber si el bloqueo de `present` es de
//!   sincronización o trabajo real del compositor (adoptarlo produciría
//!   *tearing*);
//! - renderizar a [`ESCALA_REDUCIDA`], para saber si el coste escala con los
//!   píxeles. Bajarla de forma permanente es una decisión de nitidez que ya se
//!   descartó explícitamente en
//!   `docs/propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`,
//!   así que esta calibración solo aporta el dato, no la aplica.

use wgpu::PresentMode;

/// Segundos de telemetría (ventanas de 1 s) que dura cada configuración.
pub const SEGUNDOS_POR_TRAMO: usize = 3;

/// Mejora mínima exigida para adoptar otra configuración no sonda.
pub const MARGEN_MEJORA: f64 = 0.05;

/// Factor de resolución del tramo de diagnóstico que reduce píxeles a renderizar.
pub const ESCALA_REDUCIDA: f32 = 0.5;

/// Papel de un tramo dentro de la calibración.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rol {
    /// Configuración vigente: referencia contra la que se mide cualquier mejora.
    Referencia,
    /// Candidata que la calibración puede adoptar.
    Adoptable,
    /// Sonda de diagnóstico: se mide para aprender, pero jamás se adopta
    /// (presentar sin VSync provocaría *tearing*; reducir la resolución de
    /// render tocaría una decisión de nitidez ya tomada).
    Sonda,
}

/// Una configuración de presentación concreta del swapchain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidato {
    /// Latencia deseada de fotogramas (`desired_maximum_frame_latency`);
    /// determina `min_image_count = latencia + 1`.
    pub latency: u32,
    /// Modo de presentación del swapchain.
    pub present_mode: PresentMode,
    /// Resolución de render relativa a la ventana física (1.0 = nativa).
    pub scale: f32,
    /// Papel del tramo que mide esta configuración.
    pub rol: Rol,
}

impl Candidato {
    /// Crea un candidato.
    pub fn new(latency: u32, present_mode: PresentMode, scale: f32, rol: Rol) -> Candidato {
        Candidato {
            latency,
            present_mode,
            scale,
            rol,
        }
    }

    /// Descripción legible para la telemetría (`lat=2 modo=Mailbox escala=1.00`).
    /// Las sondas y la referencia se marcan para que el log no invite a
    /// preguntarse por qué no se adoptaron.
    pub fn descripcion(&self) -> String {
        let base = format!(
            "lat={} modo={:?} escala={:.2}",
            self.latency, self.present_mode, self.scale
        );
        match self.rol {
            Rol::Referencia => format!("{} referencia", base),
            Rol::Sonda => format!("{} sonda", base),
            Rol::Adoptable => base,
        }
    }
}

/// Medición acumulada de una configuración durante `SEGUNDOS_POR_TRAMO` segundos.
#[derive(Clone, Debug, PartialEq)]
pub struct Tramo {
    /// Configuración medida por este tramo.
    pub candidato: Candidato,
    muestras_present: Vec<f64>,
    muestras_fps: Vec<f64>,
}

impl Tramo {
    /// Crea un tramo vacío para un candidato.
    pub fn new(candidato: Candidato) -> Tramo {
        Tramo {
            candidato,
            muestras_present: Vec::new(),
            muestras_fps: Vec::new(),
        }
    }

    /// Registra una ventana de telemetría (1 s) de este tramo.
    pub fn muestra(&mut self, present_ms: f64, fps: f64) {
        self.muestras_present.push(present_ms);
        self.muestras_fps.push(fps);
    }

    /// Descarta las muestras acumuladas (reinicio de la calibración).
    fn limpiar(&mut self) {
        self.muestras_present.clear();
        self.muestras_fps.clear();
    }

    /// `true` si el tramo ya reunió las ventanas necesarias.
    pub fn completa(&self) -> bool {
        self.muestras_fps.len() >= SEGUNDOS_POR_TRAMO
    }

    /// `true` si al menos una ventana dibujó fotogramas: un tramo en el que el
    /// compositor falló (por ejemplo, swapchain rechazado) no es comparable.
    pub fn valido(&self) -> bool {
        self.muestras_fps.iter().any(|f| *f > 0.0)
    }

    /// Promedio de `present` de las ventanas que sí dibujaron fotogramas.
    pub fn present_medio(&self) -> Option<f64> {
        let validas: Vec<f64> = self
            .muestras_present
            .iter()
            .zip(self.muestras_fps.iter())
            .filter(|(_, fps)| **fps > 0.0)
            .map(|(p, _)| *p)
            .collect();
        if validas.is_empty() {
            None
        } else {
            Some(validas.iter().sum::<f64>() / validas.len() as f64)
        }
    }

    /// Promedio de fotogramas por segundo de las ventanas medidas.
    pub fn fps_medio(&self) -> f64 {
        if self.muestras_fps.is_empty() {
            0.0
        } else {
            self.muestras_fps.iter().sum::<f64>() / self.muestras_fps.len() as f64
        }
    }
}

/// Elige la configuración a adoptar a partir de los tramos medidos.
///
/// Devuelve el candidato elegido y el motivo (para la telemetría). Reglas:
/// 1. Sin muestras válidas de la referencia → se mantiene la referencia.
/// 2. La mejor candidata (`Rol::Adoptable`) gana a la referencia solo si la
///    mejora en al menos `MARGEN_MEJORA` (5 %).
/// 3. En cualquier otro caso se mantiene la referencia.
///
/// Los tramos con `Rol::Sonda` nunca se adoptan, por buenos que sean: son
/// mediciones para decidir, no configuraciones para aplicar.
pub fn elegir_mejor(tramos: &[Tramo]) -> (Candidato, String) {
    let referencia = match tramos
        .iter()
        .find(|t| t.candidato.rol == Rol::Referencia)
    {
        Some(t) => t,
        None => {
            // Sin referencia declarada no hay nada seguro que aplicar.
            let fallback = tramos
                .first()
                .map(|t| t.candidato)
                .unwrap_or(Candidato::new(1, PresentMode::Fifo, 1.0, Rol::Referencia));
            return (fallback, "sin tramo de referencia".to_string());
        }
    };
    let Some(present_ref) = referencia.present_medio() else {
        return (
            referencia.candidato,
            "la referencia no tiene muestras válidas; se mantiene".to_string(),
        );
    };

    // Solo compiten las candidatas adoptables; las sondas quedan fuera.
    let mut mejor: Option<(Candidato, f64)> = None;
    for tramo in tramos.iter().filter(|t| t.candidato.rol == Rol::Adoptable) {
        let Some(present) = tramo.present_medio() else {
            continue;
        };
        if mejor.map_or(true, |(_, p)| present < p) {
            mejor = Some((tramo.candidato, present));
        }
    }

    if let Some((candidato, present)) = mejor {
        if present <= present_ref * (1.0 - MARGEN_MEJORA) {
            return (
                candidato,
                format!(
                    "mejora medible sobre la referencia ({:.1} ms vs {:.1} ms)",
                    present, present_ref
                ),
            );
        }
    }

    (
        referencia.candidato,
        format!("sin mejora medible sobre la referencia ({:.1} ms)", present_ref),
    )
}

/// Calibración de presentación: recorre una lista de configuraciones, mide
/// `present` en cada una y decide cuál aplicar.
pub struct PresentCalibration {
    tramos: Vec<Tramo>,
    indice: usize,
    /// Configuración pendiente de aplicar al compositor.
    pendiente: Option<Candidato>,
    /// Configuración elegida al terminar (`Some` ⇒ calibración finalizada).
    elegido: Option<Candidato>,
    /// Tamaño de ventana con el que arrancó el recorrido actual.
    tamano_inicial: Option<(u32, u32)>,
    /// Reinicios consumidos por cambios de orientación (máximo 1).
    reinicios: u8,
}

impl PresentCalibration {
    /// Construye el recorrido de calibración.
    ///
    /// `modo_base` es el modo de presentación ya configurado por el compositor
    /// (referencia); `modos_disponibles` son los que ofrece el driver, y solo
    /// se incluye la sonda sin VSync si `Immediate` está entre ellos.
    pub fn new(modo_base: PresentMode, modos_disponibles: &[PresentMode]) -> PresentCalibration {
        let mut tramos = vec![
            Tramo::new(Candidato::new(1, modo_base, 1.0, Rol::Referencia)),
            Tramo::new(Candidato::new(2, modo_base, 1.0, Rol::Adoptable)),
            Tramo::new(Candidato::new(3, modo_base, 1.0, Rol::Adoptable)),
            Tramo::new(Candidato::new(
                3,
                modo_base,
                ESCALA_REDUCIDA,
                Rol::Sonda,
            )),
        ];
        if modos_disponibles.contains(&PresentMode::Immediate) {
            // Sonda: mide el techo sin VSync para distinguir si el bloqueo de
            // `present` es de sincronización o trabajo real del compositor.
            tramos.push(Tramo::new(Candidato::new(
                3,
                PresentMode::Immediate,
                1.0,
                Rol::Sonda,
            )));
        }
        PresentCalibration {
            pendiente: Some(tramos[0].candidato),
            tramos,
            indice: 0,
            elegido: None,
            tamano_inicial: None,
            reinicios: 0,
        }
    }

    /// Configuración que debe estar activa ahora mismo.
    pub fn actual(&self) -> Candidato {
        self.elegido
            .unwrap_or_else(|| self.tramos[self.indice.min(self.tramos.len() - 1)].candidato)
    }

    /// Retira y devuelve la configuración pendiente de aplicar, si la hay.
    pub fn tomar_pendiente(&mut self) -> Option<Candidato> {
        self.pendiente.take()
    }

    /// `true` cuando ya se eligió configuración (no se miden más tramos).
    pub fn finalizada(&self) -> bool {
        self.elegido.is_some()
    }

    /// Número de tramos del recorrido (para diagnósticos y pruebas).
    pub fn total_tramos(&self) -> usize {
        self.tramos.len()
    }

    /// Registra una ventana de telemetría y devuelve las líneas de comentario
    /// (`# ...`) que correspondan: apertura de tramo, cierre con promedio y
    /// veredicto final.
    pub fn registrar_segundo(&mut self, present_ms: f64, fps: f64) -> Vec<String> {
        let mut lineas = Vec::new();
        if self.finalizada() {
            return lineas;
        }
        let indice = self.indice;
        let Some(tramo) = self.tramos.get_mut(indice) else {
            return lineas;
        };
        if tramo.muestras_fps.is_empty() {
            lineas.push(format!(
                "# tramo idx={} {}",
                indice,
                tramo.candidato.descripcion()
            ));
        }
        tramo.muestra(present_ms, fps);
        if !tramo.completa() {
            return lineas;
        }

        lineas.push(format!(
            "# resultado idx={} present={:.1}ms fps={:.1}",
            indice,
            tramo.present_medio().unwrap_or(0.0),
            tramo.fps_medio()
        ));

        self.indice += 1;
        if self.indice >= self.tramos.len() {
            let (elegido, motivo) = elegir_mejor(&self.tramos);
            lineas.push(format!("# elegido {} ({})", elegido.descripcion(), motivo));
            self.elegido = Some(elegido);
            self.pendiente = Some(elegido);
        } else {
            self.pendiente = Some(self.tramos[self.indice].candidato);
        }
        lineas
    }

    /// Informa del tamaño de ventana observado. Un cambio de orientación
    /// invalida la comparación entre tramos (el coste de composición depende
    /// de la rotación): se reinicia una vez y, si vuelve a cambiar, se cierra
    /// la calibración con lo medido hasta el momento.
    pub fn notificar_tamano(&mut self, tamano: (u32, u32)) -> Vec<String> {
        let mut lineas = Vec::new();
        if self.finalizada() {
            return lineas;
        }
        match self.tamano_inicial {
            None => self.tamano_inicial = Some(tamano),
            Some(inicial) if inicial == tamano => {}
            Some(_) => {
                if self.reinicios == 0 {
                    self.reinicios = 1;
                    for tramo in &mut self.tramos {
                        tramo.limpiar();
                    }
                    self.indice = 0;
                    self.pendiente = Some(self.tramos[0].candidato);
                    self.tamano_inicial = Some(tamano);
                    lineas.push("# calibracion reiniciada (cambio de orientacion)".to_string());
                } else {
                    let (elegido, motivo) = elegir_mejor(&self.tramos);
                    lineas.push(format!(
                        "# elegido {} (abortada por cambio de orientacion: {})",
                        elegido.descripcion(),
                        motivo
                    ));
                    self.elegido = Some(elegido);
                    self.pendiente = Some(elegido);
                    self.tamano_inicial = Some(tamano);
                }
            }
        }
        lineas
    }
}

#[cfg(test)]
#[path = "present_tests.rs"]
mod tests;
