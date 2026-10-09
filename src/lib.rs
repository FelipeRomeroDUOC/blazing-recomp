//! Blazing Recomp — punto de entrada nativo para Android.
//!
//! Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
//! arrancar la NativeActivity. Fases integradas:
//!
//! - Fase 4 (propuesta `2026-10-08_renderizado-gpu-android`): presentación
//!   GPU vía [`screen::present::Presenter`] (wgpu: Vulkan/GLES), LUT de
//!   color y temporización temporal del panel.
//! - Fase 5.1 (propuesta `2026-10-08_controles-tactiles-haptica`): entrada
//!   táctil ergonómica (cruceta con cuenca, clúster diagonal GBA SP con
//!   L/B/A/R, Select/Start), auto-rotación de hitboxes, bandera
//!   `touch_enabled` y respuesta háptica por vibración vía JNI.
//!
//! La lógica geométrica de entrada vive en [`crate::input`] (pura,
//! testeable en host, sin dependencias de Android). Todo lo que toca
//! JNI, NativeActivity o Logcat está tras `cfg(target_os = "android")`.
//!
//! La emulación aún no produce fotogramas: la presentación muestra el
//! estado actual del búfer (negro de panel hasta conectar el bucle del
//! motor `gba-core`).

// ---------------------------------------------------------------------------
// Módulos puros, compilables y testeables en host.
// ---------------------------------------------------------------------------
pub mod input;
pub mod overlay;
pub mod overlay_shader;
pub mod error_screen;

#[cfg(test)]
mod packaging_tests;

// ---------------------------------------------------------------------------
// Plataforma Android: ciclo de vida, GPU, entrada y háptica.
// ---------------------------------------------------------------------------
#[cfg(target_os = "android")]
mod platform;

#[cfg(target_os = "android")]
pub use platform::android_main;
