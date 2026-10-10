//! Módulo del shader del overlay: incluye el WGSL como constante y lo
//! expone como `&str` para el pipeline y para el test de validación con
//! Naga en host (criterio `test_overlay_shader_validates`).

/// Fuente WGSL del overlay (`src/overlay.wgsl`).
pub const WGSL: &str = include_str!("overlay.wgsl");

/// Fuente WGSL del blit de juego GBA (`src/game_blit.wgsl`).
pub const GAME_BLIT_WGSL: &str = include_str!("game_blit.wgsl");

/// Fuente WGSL del blit de elementos HUD (`src/hud_blit.wgsl`).
pub const HUD_BLIT_WGSL: &str = include_str!("hud_blit.wgsl");

