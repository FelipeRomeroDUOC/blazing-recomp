# Carga de bloques AOT C11 en Windows para paridad de `ppu_doble` con Android

Fecha: 2026-11-06
Nivel: 2
Estado: revisada por humano
Hereda de: docs/propuestas/2026-11-06_mezcla-unica-fuego-sobre-dragon-wgsl.md
Conservadas sin cambios: S2, R1, R3, C1, C3
Reemplazadas: S1 → S3, R2 → R4, C2 → C4

## Para decidir

- **Qué cambia:** En `src/recomp.rs`, habilitamos la carga dinámica de la biblioteca recompilada C11 en Windows (`out/1575 - Fire Emblem (E)(Independent)-bios.dll` o `blazing_recompiled.dll`) mediante `LoadLibraryW` y `GetProcAddress` de `windows-sys` (ya presente en `Cargo.toml`), exponiendo `load_native_blocks_from_path` y activando `load_native_blocks()` en ejecuciones de escritorio fuera de `#[cfg(test)]`.
- **Para qué:** Hacer que `ppu_doble` en PC deje de caer al intérprete ciclo a ciclo (`maquina.run_frame`) y ejecute exactamente los mismos 144.875 bloques C11 AOT y el mismo bucle `run_frame_native` que Android, para comprobar de inmediato si los parpadeos de la escena del dragón aparecen también en PC.
- **Ejemplo concreto (arranque y avance de fotograma en `ppu_doble` en Windows):**
  - *Antes:*
    - `blazing_recomp::recomp::load_native_blocks()` devolvía siempre `None` en Windows (`#[cfg(not(target_os = "android"))] { None }`).
    - `ppu_doble` imprimía `[PpuDoble] Aviso: Bloques AOT no disponibles, usando intérprete.` y ejecutaba `maquina.run_frame(5_000_000)` (intérprete puro ciclo a ciclo, donde las interrupciones HBlank se despachan al instante tras cada instrucción).
  - *Ahora:*
    - `blazing_recomp::recomp::load_native_blocks()` carga `out/1575 - Fire Emblem (E)(Independent)-bios.dll`, resuelve `rcg_blocks` y `rcg_block_count` (`144875` bloques) y devuelve `Some(BlockTable)`.
    - `ppu_doble` imprime `[PpuDoble] Bloques AOT cargados para paridad de timing 1:1 con Android.` y ejecuta `blazing_recomp::recomp::run_frame_native(&mut maquina, table, 2_000_000)` en cada cuadro.
- **Pasos desde quien usa la app:**
  1. Ejecutar `cargo run --release --bin ppu_doble` en PC.
  2. Verificar en consola el mensaje `[PpuDoble] Bloques AOT cargados para paridad de timing 1:1 con Android.`.
  3. Observar la escena del dragón en la ventana de `PPU Nativa (WGSL)` en PC y comprobar si aparecen los parpadeos que ocurrían en Android.
- **Qué podría romperse:** Los tests unitarios de `src/runner_tests.rs` que crean un `GameRunner::new(create_dummy_rom(), ...)` con una ROM ficticia rellena de ceros: si `load_native_blocks()` cargara los bloques reales de *Fire Emblem* durante `cargo test`, `GameRunner` ejecutaría bloques AOT del juego real sobre una ROM vacía. Para evitarlo, `load_native_blocks()` sigue devolviendo `None` bajo `#[cfg(test)]`, mientras que la carga real de la DLL en Windows se prueba directamente mediante `load_native_blocks_from_path`.
- **Qué no se toca:** No se modifican los shaders ni `src/ppu_nativa.wgsl` (se conservan `S2`, `R1`, `R3`, `C1`, `C3` de la propuesta anterior), no se altera la lógica de Android (`#[cfg(target_os = "android")]`), ni se necesita tocar `src/bin/ppu_doble.rs` porque ya invoca `load_native_blocks()` y `run_frame_native()`.
- **Qué necesito de ti:** Aprobar la propuesta para implementarla y lanzar `ppu_doble` en PC (1 punto de revisión visual en la ventana de `ppu_doble`).

## Detalle para verificar

### Suposiciones

- **S3 (verificada en disco y código, reemplaza a `S1`):** El archivo `out/1575 - Fire Emblem (E)(Independent)-bios.dll` ya existe en el repositorio local y exporta los símbolos C `rcg_blocks` y `rcg_block_count` con el mismo ABI (`RcgBlock`) que `libblazing_recompiled.so` en Android.
  - *Si fuera falsa:* Regeneraríamos la DLL en `out/` ejecutando `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1`.
- **S4 (verificada en código, no se prueba antes de editar):** `src/bin/ppu_doble.rs:281-286` y `497-502` ya llaman a `blazing_recomp::recomp::load_native_blocks()` al iniciar y a `blazing_recomp::recomp::run_frame_native(&mut maquina, table, 2_000_000)` en cada fotograma cuando `block_table` es `Some`.
  - *Si fuera falsa:* Añadiríamos la llamada en `src/bin/ppu_doble.rs`.

### Reglas del proyecto que cambian este plan

- **R4 (Fuente: `AGENTS.md §1` y `Cargo.toml:41-49`, reemplaza a `R2`):** No añadir dependencias externas nuevas para cargar bibliotecas en Windows; reutilizar `windows_sys::Win32::System::LibraryLoader` (`LoadLibraryW` y `GetProcAddress`) que ya está declarada en `Cargo.toml`. Protegida por `C4`.
- **R5 (Fuente: `AGENTS.md §3`):** Mantener el aislamiento de los tests unitarios con ROMs sintéticas en `src/runner_tests.rs` y `src/recomp_tests.rs` sin debilitar ningún `assert!`. Protegida por `C6`.

### Comprobaciones

- **C4 — Carga real de la DLL AOT en Windows mediante `load_native_blocks_from_path` (normal, reemplaza a `C2`):**
  - *Situación real:* En Windows x86_64, el archivo `out/1575 - Fire Emblem (E)(Independent)-bios.dll` está presente en disco y contiene la tabla de bloques C11 generada por `recomp build`.
  - *Dado que* `load_native_blocks_from_path` abre la ruta con `LoadLibraryW` y busca `rcg_blocks` y `rcg_block_count` con `GetProcAddress`,
  - *Cuando* se invoca sobre `out/1575 - Fire Emblem (E)(Independent)-bios.dll`,
  - *Entonces* devuelve `Some(table)` con `table.len() > 100_000` y resuelve en `O(1)` el punto de entrada de la ROM (`table.get(0x0800_0000).is_some()` o bloques Thumb mapeados).
  - *Nombre del test:* `test_windows_load_native_blocks_from_dll_real` en `src/recomp_tests.rs`.
  - *Comando:* `python scripts/run_tests_summary.py test_windows_load_native_blocks_from_dll_real`
  - *Qué no cubre:* La visualización interactiva en la ventana de `ppu_doble`.

- **C5 — Ruta inexistente o inválida devuelve `None` limpiamente sin pánico (extremo):**
  - *Situación real:* En un entorno limpio o ruta inexistente (`out/no_existe_recomp.dll`), la biblioteca AOT no está disponible.
  - *Dado que* `load_native_blocks_from_path` valida el puntero de `LoadLibraryW` y los símbolos antes de construir la tajada,
  - *Cuando* se invoca con una ruta inexistente,
  - *Entonces* devuelve `None` sin abortar ni corromper memoria.
  - *Nombre del test:* `test_load_native_blocks_from_path_inexistente_retorna_none` en `src/recomp_tests.rs`.
  - *Comando:* `python scripts/run_tests_summary.py test_load_native_blocks_from_path_inexistente_retorna_none`
  - *Qué no cubre:* Archivos DLL corruptos a mitad de escritura.

- **C6 — Aislamiento de `load_native_blocks()` bajo `#[cfg(test)]` y no regresión en `runner_tests` (no debe cambiar):**
  - *Situación real:* Los tests de `src/runner_tests.rs` instancian `GameRunner::new(create_dummy_rom(), ...)` que internamente llama a `load_native_blocks()`.
  - *Dado que* bajo `#[cfg(test)]` `load_native_blocks()` retorna `None`,
  - *Cuando* se ejecuta la suite de `runner` y `recomp`,
  - *Entonces* `load_native_blocks()` devuelve `None` en contexto de test unitario y todos los tests de `runner` y `recomp` pasan en verde.
  - *Nombre del test:* `test_load_native_blocks_en_cfg_test_retorna_none` (más las suites `recomp` y `runner`).
  - *Comando:* `python scripts/run_tests_summary.py recomp`
  - *Qué no cubre:* El binario `ppu_doble` compilado fuera de `#[cfg(test)]` (se verifica al correr `cargo check --bin ppu_doble` y en la revisión humana).

### Archivos que se modificarán

- `src/recomp.rs`: Añadir el módulo `dl_win` bajo `#[cfg(target_os = "windows")]` usando `windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW}`, exponer `pub fn load_native_blocks_from_path(path: &std::path::Path) -> Option<BlockTable>` y conectar `load_native_blocks()` bajo `#[cfg(all(target_os = "windows", not(test)))]` buscando en `out/1575 - Fire Emblem (E)(Independent)-bios.dll` y `blazing_recompiled.dll`.
- `src/recomp_tests.rs`: Añadir los tests `test_windows_load_native_blocks_from_dll_real`, `test_load_native_blocks_from_path_inexistente_retorna_none` y `test_load_native_blocks_en_cfg_test_retorna_none`.

### Evidencia

- `src/recomp.rs:482-605`: Implementación actual de `mod dl` (solo Android) y `load_native_blocks` (retorna `None` fuera de Android).
- `src/bin/ppu_doble.rs:281-286` y `497-502`: El visor `ppu_doble` ya invoca `load_native_blocks()` y `run_frame_native(&mut maquina, table, 2_000_000)` cuando `block_table` es `Some`.
- `Cargo.toml:41-49`: `windows-sys` v0.61 ya tiene habilitada la feature `"Win32_System_LibraryLoader"`.
- `src-c/1575 - Fire Emblem (E)(Independent)-bios.def:1-5`: Exporta `rcg_block_count` y `rcg_blocks`.

## Revisión humana

- Ejecutar `ppu_doble` en PC con los bloques AOT cargados y comprobar en la escena del dragón si los parpadeos de Android aparecen también en PC: ✔
