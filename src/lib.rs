//! Blazing Recomp — punto de entrada nativo para Android.
//!
//! Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
//! arrancar la NativeActivity. Fases integradas:
//!
//! - Renderizado GPU nativo (ADR 42): composición acelerada directamente
//!   en GPU mediante [`crate::ppu_nativa::NativePpu`] con shaders WGSL, tabla
//!   LUT vertical de paleta y presentación sobre superficie Android en
//!   [`crate::present`] (wgpu: Vulkan con respaldo automático a OpenGL ES).
//! - Entrada táctil ergonómica: cruceta con cuenca, clúster diagonal GBA SP con
//!   L/B/A/R, Select/Start, auto-rotación de hitboxes, bandera `touch_enabled` y
//!   respuesta háptica por vibración vía JNI.
//!
//! La lógica geométrica de entrada vive en [`crate::input`] (pura,
//! testeable en host, sin dependencias de Android). Todo lo que toca
//! JNI, NativeActivity o Logcat está tras `cfg(target_os = "android")`.
//!
//! El motor `gba-core` ejecuta la lógica del juego (acelerada por los bloques
//! C11 nativos de `libblazing_recompiled.so` cargados por [`crate::recomp`]),
//! y la PPU nativa compone cada fotograma en la GPU coordinada con el refresco
//! del panel.

// ---------------------------------------------------------------------------
// Módulos puros, compilables y testeables en host.
// ---------------------------------------------------------------------------
pub mod input;
pub mod overlay;
pub mod overlay_shader;
pub mod error_screen;
pub mod runner;
pub mod storage;
pub mod recomp;
pub mod fps;
pub mod perf;
pub mod telemetria;
pub mod pipeline;
pub mod present;

pub mod ppu_nativa;


// ---------------------------------------------------------------------------
// Plataforma Android: ciclo de vida, GPU, entrada y háptica.
// ---------------------------------------------------------------------------
#[cfg(target_os = "android")]
mod platform;

#[cfg(target_os = "android")]
pub use platform::android_main;
