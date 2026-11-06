# Propuesta: Limpieza de Volcado Diagnóstico y Hash de Cuadros en Platform y PpuDoble

Estado: revisada por humano (confirmada)
Fecha: 2026-11-06
Autor: Antigravity
Nivel: 2

## 1. Contexto y Motivación

Durante la investigación de los parpadeos en la escena del dragón, se introdujeron mecanismos temporales de instrumentación diagnóstica en `src/platform.rs` y `src/bin/ppu_doble.rs`:
- Flags diagnósticos en `AppState` (`hash_cuadros_activo`, `sync_vram_activo`, `rel_dragon`).
- Lógica de volcado y hashing de búferes de cuadro específicos (cuadros 86 y 150) a disco/logs.

Una vez aislada y resuelta la causa raíz en `chain_gate` y `run_frame_native`, y tras la poda de heurísticas y monitores del dragón, esta instrumentación ad-hoc ya no tiene propósito operativo y añade ruido, ramas muertas y campos innecesarios en `AppState`.

## 2. Archivos Afectados y Cambios Propuestos

### 2.1. `src/platform.rs`
- **Eliminar campos de `AppState`**:
  - `pub hash_cuadros_activo: bool`
  - `pub sync_vram_activo: bool`
  - `pub rel_dragon: bool`
- **Eliminar inicialización** de dichos campos en el constructor de `AppState`.
- **Eliminar bloque de volcado/hash**: En el ciclo de renderizado de cuadros (líneas ~2696-2823), retirar el bloque que intercepta los cuadros 86/150 y calcula hashes o vuelca VRAM a disco.

### 2.2. `src/bin/ppu_doble.rs`
- **Eliminar variable/flag `rel_dragon`**.
- **Eliminar bloque condicional de volcado de cuadros 86/150** en el bucle principal de comparación dual de PPU.

## 3. Criterios de Aceptación y Verificación

1. `cargo check` y `cargo test` compilan sin errores ni advertencias sobre campos inexistentes.
2. `python scripts/run_tests_summary.py` pasa el 100% de la suite de pruebas.
3. Se mantiene intacto el bucle de ejecución de emulación y la renderización en `src/platform.rs` y `src/bin/ppu_doble.rs`.
4. La documentación de API (`docs/referencia/api-modulos-rust.md`) se sincroniza si cambian las estructuras públicas de `AppState`.

## 4. Resultados de Implementación

- **Archivos modificados:** `src/platform.rs`, `src/bin/ppu_doble.rs`, `docs/referencia/api-modulos-rust.md`.
- **Lógica purgada:** Eliminación completa de `hash-cuadros.flag`, `sync-vram.flag`, `rel_dragon`, `reevaluar_hash_cuadros`, el parámetro `sync_vram_activo` en `CompositorState::render` y el volcado de cuadros 86/150 a PPM/TXT.
- **Suite de pruebas:** 324/324 tests pasados en verde (100% éxito) verificado con `python scripts/run_tests_summary.py`.

