//! Blazing Recomp — punto de entrada nativo para Android.
//!
//! Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
//! arrancar la NativeActivity. En esta fase (Fase 3 de la propuesta
//! `2026-10-08_crate-nativo-android`) solo:
//!
//! 1. Inicializa el logging a Logcat (`android_logger`).
//! 2. Gestiona los eventos de ciclo de vida (`Resume`, `Pause`, `Terminate`)
//!    a través de `android-activity` con backend `native-activity`.
//! 3. Prepara el tipo `gba_core::Machine` (el modelo de hardware de la GBA
//!    provisto por el motor base, sin duplicar su código).
//!
//! El renderizado (Vulkan/GLES por wgpu), el audio y los controles táctiles
//! llegan en fases posteriores; no se anticipan aquí.

use gba_core::Machine;

/// Estado de la aplicación: lo que persiste entre pausas y reanudaciones
/// del sistema Android.
struct AppState {
    /// Modelo de hardware de la consola. `None` hasta que la app esté en
    /// `Resume` con permiso de acceso al almacenamiento y el usuario haya
    /// cargado su copia de la ROM y del BIOS (los hashes se verifican antes
    /// de permitir jugar; ver AGENTS.md §3).
    machine: Option<Machine>,
}

impl AppState {
    fn new() -> AppState {
        AppState { machine: None }
    }

    /// Carga ROM y BIOS en el modelo de hardware de la consola (perfiles de
    /// AGENTS.md §3). Pensado para Fase 4: se invocará una vez verificados
    /// los hashes SHA-256 del directorio `roms/`.
    #[allow(dead_code)]
    fn load(&mut self, rom: Vec<u8>, bios: &[u8]) {
        self.machine = Some(Machine::new_with_bios(rom, bios));
    }

    /// Libera el modelo de hardware al pausar; Android puede matar la
    /// NativeActivity sin aviso y conviene no sostener RAM del juego.
    fn suspend(&mut self) {
        if let Some(machine) = self.machine.take() {
            log::debug!("Machine liberada (suspend)");
            drop(machine);
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
    log::info!("Blazing Recomp: android_main");

    let mut state = AppState::new();

    // Superficie inicial: el rect de la ventana (si ya existe) se reporta
    // a Logcat; render real llega en Fase 4.
    let rect = app.content_rect();
    log::info!(
        "content rect inicial: {}x{}",
        rect.right - rect.left,
        rect.bottom - rect.top
    );

    // Bucle principal: procesa eventos hasta que Android destruya la app.
    // API de android-activity 0.6: callback, no iterator; `None` bloquea
    // hasta el siguiente evento (sin busy-waiting; AGENTS.md §2).
    loop {
        let mut finishing = false;
        app.poll_events(None, |event| match event {
            android_activity::PollEvent::Main(main) => match main {
                android_activity::MainEvent::Resume { .. } => {
                    log::info!("ciclo de vida: Resume");
                    // La máquina se recarga en la fase de selección de
                    // ROM/BIOS (AGENTS.md §3).
                    if state.machine.is_none() {
                        log::info!("sin Machine cargada: esperando ROM/BIOS del usuario");
                    }
                }
                android_activity::MainEvent::Pause => {
                    log::info!("ciclo de vida: Pause");
                    state.suspend();
                }
                android_activity::MainEvent::Destroy => {
                    log::info!("ciclo de vida: Destroy");
                    finishing = true;
                }
                _ => {
                    // Fases 4-6 añadirán InitWindow, WindowResized,
                    // RedrawNeeded, InputAvailable y demás; aquí quedan
                    // silenciosos para el log.
                }
            },
            _ => {}
        });
        if finishing {
            break;
        }
    }

    log::info!("Blazing Recomp: android_main finalizado");
}
