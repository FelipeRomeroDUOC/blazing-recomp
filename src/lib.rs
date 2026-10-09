//! Blazing Recomp — punto de entrada nativo para Android.
//!
//! Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
//! arrancar la NativeActivity. Fase 4 (propuesta
//! `2026-10-08_renderizado-gpu-android`): presentación gráfica acelerada
//! por GPU vía [`screen::present::Presenter`] (wgpu: Vulkan/GLES) sobre el
//! `ANativeWindow` del sistema, con temporización temporal de panel
//! (`screen::Temporal`), LUT de color (BGR555 → RGBA8) y escalado 3:2
//! centrado.
//!
//! El bucle de eventos maneja el ciclo de vida gráfico: el presenter se
//! crea al recibir `InitWindow` (con la ventana viva) y se destruye al
//! recibir `TerminateWindow` o `Pause`, para no sostener punteros ni
//! recursos GPU de una ventana huérfana.
//!
//! La emulación aún no produce fotogramas: la presentación muestra el
//! estado actual del búfer (negro de panel hasta Fase 5 de emulación en
//! banda con el motor `gba-core`).
//!
//! El audio (Fase 6) y los controles táctiles (Fase 5) quedan fuera.

use std::sync::Arc;

use android_activity::{MainEvent, PollEvent};
use gba_core::Machine;
use raw_window_handle::{
    AndroidDisplayHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, RawDisplayHandle,
};
use screen::blend::default_rho;
use screen::present::{GridParams, Presenter};
use screen::{ColorLut, ColorSettings, DisplayTarget, ResponseMode, ScreenKind, Temporal};

/// Dimensiones del framebuffer de la GBA (3:2).
const GBA_W: u32 = 240;
const GBA_H: u32 = 160;
const GBA_PIXELS: usize = (GBA_W * GBA_H) as usize;

/// Envoltorio seguro de la ventana nativa Android para `wgpu`.
///
/// `ndk::native_window::NativeWindow` ya implementa `HasWindowHandle`
/// (handle `AndroidNdkWindowHandle` con el puntero `ANativeWindow`); este
/// tipo delega en él y añade el lado display (`AndroidDisplayHandle`,
/// vacío por diseño: Android es un display único), completando el contrato
/// que `screen::present::Presenter::new` exige.
/// El `NativeWindow` interno es dueño de su referencia adquirida: se
/// libera al descartar esta estructura.
struct AndroidWindow {
    window: android_activity::ndk::native_window::NativeWindow,
}

impl AndroidWindow {
    /// Toma propiedad de la ventana entregada por `AndroidApp::native_window`.
    fn new(window: android_activity::ndk::native_window::NativeWindow) -> AndroidWindow {
        AndroidWindow { window }
    }

    /// Tamaño físico de la superficie en píxeles.
    fn size(&self) -> (u32, u32) {
        (
            self.window.width().max(0) as u32,
            self.window.height().max(0) as u32,
        )
    }
}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        // SAFETY: el puntero `ANativeWindow` es válido mientras esta
        // estructura exista (el `NativeWindow` interno retiene su
        // referencia adquirida) y no cambia durante la vida del handle
        // prestado; contrato documentado en `ndk::native_window`.
        let raw = raw_window_handle::RawWindowHandle::AndroidNdk(
            raw_window_handle::AndroidNdkWindowHandle::new(
                self.window.ptr().cast::<std::ffi::c_void>(),
            ),
        );
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: `AndroidDisplayHandle` no contiene punteros prestados
        // (Android es un display único y el handle queda vacío), tal como
        // documenta raw-window-handle para este caso.
        let raw = RawDisplayHandle::Android(AndroidDisplayHandle::new());
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

/// Estado de la aplicación: lo que persiste entre pausas y reanudaciones
/// del sistema Android.
struct AppState {
    /// Modelo de hardware de la consola. `None` hasta que se carguen la
    /// ROM y el BIOS del usuario con hashes verificados (AGENTS.md §3).
    machine: Option<Machine>,
    /// Presenter GPU (`wgpu`) sobre la ventana Android. `None` cuando no
    /// hay ventana viva (`TerminateWindow`, `Pause`, `Destroy`).
    presenter: Option<Presenter>,
    /// Ventana propiedad del presenter, para consultar su tamaño físico.
    window: Option<Arc<AndroidWindow>>,
    /// Panel físico que se simula (perfil de `screen`). Se usará para
    /// reconstruir la LUT cuando el usuario cambie de perfil (Fase 5+).
    #[allow(dead_code)]
    screen_kind: ScreenKind,
    /// BGR555 → RGBA8 precalibrado para el panel y destino elegidos.
    lut: ColorLut,
    /// Resolución temporal del panel LCD (respuesta entre fotogramas).
    temporal: Temporal,
    /// Parámetros de rejilla/escalado por fotograma.
    grid: GridParams,
    /// Búfer RGBA de presentación (salida de `temporal.compose`).
    rgba: Vec<[u8; 4]>,
}

impl AppState {
    fn new() -> AppState {
        // Perfil por defecto de la biblioteca: panel original con
        // frontlight (la vista "diseñada" del juego), destino sRGB.
        let screen_kind = ScreenKind::Frontlit;
        let lut = ColorLut::build(&ColorSettings::default());
        let temporal = Temporal::new(
            GBA_PIXELS,
            // `Smart` es el modo documentado como default en `blend.rs`.
            ResponseMode::Smart,
            1,
            default_rho(screen_kind),
        );
        AppState {
            machine: None,
            presenter: None,
            window: None,
            screen_kind,
            lut,
            temporal,
            grid: GridParams::with_strength(1.0),
            rgba: vec![[0, 0, 0, 255]; GBA_PIXELS],
        }
    }

    /// Carga ROM y BIOS en el modelo de hardware de la consola (perfiles de
    /// AGENTS.md §3). Se invoca una vez verificados los hashes SHA-256.
    #[allow(dead_code)]
    fn load(&mut self, rom: Vec<u8>, bios: &[u8]) {
        self.machine = Some(Machine::new_with_bios(rom, bios));
    }

    /// Libera los recursos GPU y de emulación al pausar; Android puede
    /// matar la NativeActivity sin aviso y conviene no sostener RAM del
    /// juego ni una superficie de una ventana que ya no existe.
    fn suspend(&mut self) {
        if self.presenter.take().is_some() || self.window.take().is_some() {
            log::debug!("recursos GPU liberados (suspend)");
        }
        if let Some(machine) = self.machine.take() {
            log::debug!("Machine liberada (suspend)");
            drop(machine);
        }
    }

    /// Destruye el presenter y su ventana (evento `TerminateWindow`).
    fn remove_presenter(&mut self) {
        self.presenter = None;
        self.window = None;
    }

    /// Crea el presenter GPU sobre `window`. Solo con ventana viva:
    /// llamarlo fuera de `InitWindow` dejaría el presenter con un puntero
    /// a ventana huérfana.
    fn create_presenter(&mut self, window: Option<Arc<AndroidWindow>>) {
        self.remove_presenter();
        let Some(window) = window else {
            log::warn!("event: InitWindow sin ventana nativa");
            return;
        };
        let (w, h) = window.size();
        log::info!("ventana nativa {}x{}: creando presenter", w, h);
        match Presenter::new(
            // Ownership compartido: la superficie conserva un clon, la
            // garantía de vida es interna a `screen` (sin `unsafe` aquí).
            window.clone(),
            GBA_W,
            GBA_H,
            DisplayTarget::Auto,
            true, // vsync: pacing de presentación (el ritmo del juego lo
                  // fijará el reloj de audio; ver docs de `screen`).
        ) {
            Ok(p) => {
                self.window = Some(window);
                self.presenter = Some(p);
                log::info!("presenter GPU listo");
            }
            Err(e) => log::error!("Presenter::new: {e}"),
        }
    }

    /// Compone el fotograma visible y lo presenta con escalado 3:2
    /// centrado. No-op si no hay ventana viva.
    fn redraw(&mut self) {
        let Some(window) = &self.window else { return };
        let (w, h) = window.size();
        let Some(presenter) = self.presenter.as_mut() else { return };
        // Pipeline (ver docs de `screen`): histograma temporal del panel →
        // LUT BGR555→RGBA8 → swapchain del presenter.
        self.temporal.compose(&self.lut, &mut self.rgba);
        if let Err(e) = presenter.present(&self.rgba, (w, h), &self.grid) {
            log::warn!("present: {e}");
        }
    }
}

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    // Logging a Logcat con el tag "BlazingRecomp" (visible en `adb logcat`).
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("BlazingRecomp"),
    );
    log::info!("Blazing Recomp: android_main (fase renderizado GPU)");

    let mut state = AppState::new();
    let mut finishing = false;

    // Bucle principal: procesa eventos hasta que Android destruya la app.
    // API de android-activity 0.6: callback, no iterator; `None` bloquea
    // hasta el siguiente evento (sin busy-waiting; AGENTS.md §2). Cuando
    // haya emulación real, este bucle cederá antes del vsync del frame.
    while !finishing {
        app.poll_events(None, |event| {
            let PollEvent::Main(main) = event else { return };
            match main {
                MainEvent::InitWindow { .. } => {
                    log::info!("ciclo de vida: InitWindow");
                    state.create_presenter(
                        app.native_window().map(AndroidWindow::new).map(Arc::new),
                    );
                    state.redraw();
                }
                MainEvent::WindowResized { .. } | MainEvent::RedrawNeeded { .. } => {
                    log::info!("ciclo de vida: redibujar ventana");
                    state.redraw();
                }
                MainEvent::TerminateWindow { .. } => {
                    log::info!("ciclo de vida: TerminateWindow");
                    state.remove_presenter();
                }
                MainEvent::Resume { .. } => {
                    log::info!("ciclo de vida: Resume");
                    if state.machine.is_none() {
                        log::info!("sin Machine cargada: esperando ROM/BIOS del usuario");
                    }
                }
                MainEvent::Pause => {
                    log::info!("ciclo de vida: Pause");
                    state.suspend();
                }
                MainEvent::Destroy => {
                    log::info!("ciclo de vida: Destroy");
                    finishing = true;
                }
                _ => {
                    // Fases 5-6 añadirán InputAvailable, ContentRectChanged,
                    // LowMemory y demás; aquí quedan silenciosos.
                }
            }
        });
    }

    log::info!("Blazing Recomp: android_main finalizado");
}
