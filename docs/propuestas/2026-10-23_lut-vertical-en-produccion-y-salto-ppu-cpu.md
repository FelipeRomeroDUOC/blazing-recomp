# Propuesta: Captura ligera de LUT vertical en producción y salto del renderizado de PPU por software cuando la PPU nativa está activa

Fecha: 2026-10-23
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. `MemMap` (en `../gba-recomp/crates/gba-core/src/mem.rs`) incorpora dos opciones ligeras en el evento de HBlank de cada línea visible (`0..160`):
   - `capturar_lut_ligera: bool`: copia únicamente `self.io[..0x60]` (`96 bytes`) y `self.palette[..1024]` (`1 KB`), y si los acumuladores de referencia afín (`aff_ref`) de `BG2`/`BG3` ya venían desplazados del cuadro anterior o se recargaron en mitad del cuadro, sincroniza sus bytes (`0x28..0x2F`, `0x38..0x3F`) en la copia de `io_lineas[line]` (`175 KB/cuadro` en búferes preasignados, frente a los `15,8 MB/cuadro` de `CapturaLinea`).
   - `omitir_ppu_software: bool`: actualiza los acumuladores afines de hardware (`aff_ref` y `aff_dirty`) en la transición de HBlank exactamente igual que la PPU real, pero salta por completo `ppu::render_scanline` (fondos, sprites y composición en CPU).
2. `GameRunner` (`src/runner.rs`) y `EmulationWorker` (`src/pipeline.rs`) conectan esta captura ligera a `PpuFrameState::io_lineas` y `PpuFrameState::paleta_lineas` (que hasta ahora replicaban 160 veces el estado congelado del final del cuadro) y omiten tanto `ppu::render_scanline` como `temporal.push` / `temporal.compose` cuando `gpu_ppu_enabled` está activo, dejando la PPU por software intacta como respaldo (*fallback*) y oráculo de pruebas.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/gba-core/src/mem.rs`, `../gba-recomp/crates/gba-core/src/ppu.rs`, `src/runner.rs`, `src/runner_tests.rs`, `src/pipeline.rs`, `src/pipeline_tests.rs`, `src/platform.rs` y los resultados de la sesión interactiva de `ppu_doble` tras corregir `clave_de_tramo` (`0` diferencias visuales reales en toda la sesión).
- Intención: llevar a la ruta de producción de Android la misma fidelidad por línea (registros I/O y paleta por HBlank) que ya demostró `0` diferencias en `ppu_doble`, y al mismo tiempo eliminar el trabajo redundante en CPU (los 160 `render_scanline` por software y la conversión de color `Temporal` + `ColorLut` de `38.400` píxeles por cuadro) cuando la GPU es quien dibuja el fotograma. Si el usuario coloca `ppu-nativa-off.flag` o si la GPU falla al inicializar, la PPU por software se reactiva en caliente sin perder el respaldo.
- Restricciones y decisiones previas (máx. 3):
  - Conservar la PPU por software como red de seguridad (*fallback*) y como oráculo de tests en `gba-core` y `ppu_doble` (origen: acuerdo con el usuario en esta sesión) → implica: cuando `gpu_ppu_enabled == false`, `GameRunner::step_frame()` ejecuta exactamente el mismo camino por software (`render_scanline` + `temporal.push`/`compose`) que hoy → protegida por «Con `gpu_ppu_enabled` apagado, `GameRunner` ejecuta la PPU por software y las pruebas existentes siguen en verde» | testeable.
  - Sincronizar el estado de `ppu_nativa_activa` entre `AppState` y `EmulationBridge` en Android (`src/platform.rs`) (origen: inspección de `src/platform.rs` líneas 1687 y 1812) → implica: en `AppState::load` y `AppState::reevaluar_ppu_nativa`, actualizar `worker.bridge().set_gpu_ppu_enabled(self.ppu_nativa_activa)` para que al colocar `ppu-nativa-off.flag` el hilo de emulación vuelva a rellenar `rgba` por software → protegida por «Al activar `gpu_ppu_enabled`, el runner captura `io_lineas` y `paleta_lineas` reales por scanline y omite el renderizado por software» | testeable.
  - Preservar intactos el repositorio upstream fijado en `Cargo.toml` (`rev = de4edf5`), `src/ppu_shim.*` y los 47 experimentos de `examples/` (origen: `AGENTS.md` §4.1 y `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`) → implica: los cambios en `gba-core` se aplican en el checkout local `../gba-recomp/crates/gba-core/src/mem.rs` sin tocar `src/ppu_shim.*` ni `tests/datos/ppu-nativa-hashes-base.txt` → protegida por «El motor base upstream, el shim y los 47 experimentos permanecen intactos» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Activar `bus.capturar_linea = true` (`CapturaLinea`) en producción en Android → descartada porque `CapturaLinea` clona `vram` (`96 KB`) y `oam` (`1 KB`) en cada una de las 160 líneas (`15,8 MB` de asignaciones y copias por fotograma, `~950 MB/s` a 60 FPS), mientras que la PPU nativa en GPU solo consume `io_lineas` (`96 B × 160 = 15 KB`) y `paleta_lineas` (`1 KB × 160 = 160 KB`).
  - Eliminar físicamente `render_scanline` de `gba-core` → descartada porque dejaría a `ppu_doble` sin oráculo de comparación y a Android sin fallback por software si un driver móvil falla o si se usa `ppu-nativa-off.flag`.
- Ante imprevistos, prioriza: exactitud de la captura por línea (`io_lineas` y `paleta_lineas`) > ahorro de CPU al omitir `render_scanline` > mantener intacto el fallback por software.

### 3. Ejemplos

- Caso normal (captura por línea de registros I/O y paleta en `GameRunner` / `EmulationWorker`): durante un fotograma con `gpu_ppu_enabled = true`, el juego modifica `BLDY` (`0x54`) o un color de `palette` (`0x05000000`) mediante HBlank o código entre la línea `40` y la línea `100` → antes: `EmulationWorker` copiaba el `runner.io()` y `runner.palette()` del final del fotograma (línea 160) repetido en las 160 filas de `state.io_lineas` y `state.paleta_lineas`, perdiendo cualquier cambio intermedio en producción → ahora: `MemMap` copia `io[..0x60]` y `palette[..1024]` en el HBlank de cada línea `0..160` sobre búferes preasignados, y `state.io_lineas[40]` / `state.paleta_lineas[40]` conservan los valores exactos que estaban vigentes en la línea 40, mientras `state.io_lineas[100]` / `state.paleta_lineas[100]` conservan los de la línea 100.
- Caso normal (salto de la PPU por software cuando `gpu_ppu_enabled = true`): `runner.set_gpu_ppu_enabled(true)` y se llama a `runner.step_frame()` → antes: `MemMap` ejecutaba 160 veces `ppu::render_scanline` en CPU y `runner.step_frame()` ejecutaba `temporal.push` + `temporal.compose` sobre `38.400` píxeles (`ppu_ms > 0.0`, `color_ms > 0.0`) aunque el compositor de Android iba a descartar `rgba` y dibujar con `ppu_nativa` → ahora: `MemMap` actualiza únicamente `aff_ref` / `aff_dirty` en cada HBlank y omite `ppu::render_scanline`, y `runner.step_frame()` omite `temporal.push` / `temporal.compose` (`timing.ppu_ms == 0.0` y `timing.color_ms == 0.0`), manteniendo intactos los eventos de HBlank DMA (`trigger_dma(2)`), VBlank DMA (`trigger_dma(1)`), IRQs (`HBLANK`, `VBLANK`, `VCOUNT`) y audio (`pump_audio`).
- Caso límite (conmutación en caliente a PPU por software con `gpu_ppu_enabled = false`): el usuario desactiva `gpu_ppu_enabled` (por ejemplo con `ppu-nativa-off.flag` o porque `ppu_nativa_fallida` se activa en `Compositor::render`) → antes / ahora: `runner.set_gpu_ppu_enabled(false)` desactiva `omitir_ppu_software` y `capturar_lut_ligera` en `MemMap`, de modo que el siguiente `step_frame()` vuelve a ejecutar `ppu::render_scanline` y `temporal.compose`, entregando `frame_rgba()` completo para el camino de respaldo.

### 4. Pasos

1. En `../gba-recomp/crates/gba-core/src/mem.rs`:
   - Añadir en `MemMap`:
     - `pub omitir_ppu_software: bool` (por defecto `false`).
     - `pub capturar_lut_ligera: bool` (por defecto `false`).
     - `pub io_lineas: Box<[[u8; 0x60]; 160]>` y `pub paleta_lineas: Box<[[u8; 1024]; 160]>`.
   - Añadir una función auxiliar `step_affine_accumulators(&mut self, line: usize)` que ejecute la misma lógica de recarga (`line == 0 || self.aff_dirty[i]`) y avance (`wrapping_add(pb)`, `wrapping_add(pd)`) que hoy hace `ppu::render_scanline` cuando `DISPCNT & 0x80 == 0`.
   - En `process_video_event` (`VPhase::Hblank`, cuando `line < VISIBLE_SCANLINES`):
     - Si `self.capturar_lut_ligera` es `true`, copiar `self.io[..0x60]` en `self.io_lineas[line as usize]` y `self.palette[..1024]` en `self.paleta_lineas[line as usize]`.
     - Si `self.omitir_ppu_software` es `true`, llamar a `self.step_affine_accumulators(line as usize)` en lugar de `crate::ppu::render_scanline(self, line as usize)`.
2. En `src/runner.rs`:
   - En `GameRunner::set_gpu_ppu_enabled(&mut self, enabled: bool)`, sincronizar `self.machine.bus.omitir_ppu_software = enabled` y `self.machine.bus.capturar_lut_ligera = enabled`.
   - Añadir accesores de solo lectura `pub fn io_lineas(&self) -> &[[u8; 0x60]; 160]` y `pub fn paleta_lineas(&self) -> &[[u8; 1024]; 160]` que expongan `&self.machine.bus.io_lineas` y `&self.machine.bus.paleta_lineas`.
   - En `GameRunner::step_frame()`, cuando `self.gpu_ppu_enabled` es `true`, omitir `self.temporal.push(...)` y `self.temporal.compose(...)` y registrar `color_ms = 0.0`.
3. En `src/pipeline.rs`:
   - En el bucle de `EmulationWorker::start_with_waker`:
     - Solo copiar `runner.frame_rgba()` en `buf` cuando `!gpu_ppu`.
     - Cuando `gpu_ppu` es `true`, rellenar `state.io_lineas` directamente con `state.io_lineas.copy_from_slice(runner.io_lineas())` y `state.paleta_lineas` con `state.paleta_lineas.copy_from_slice(runner.paleta_lineas())` en lugar de repetir 160 veces el estado del final del cuadro.
4. En `src/platform.rs`:
   - En `AppState::load`, inicializar `worker.bridge().set_gpu_ppu_enabled(self.ppu_nativa_activa && !FORZAR_PPU_SOFTWARE)`.
   - En `AppState::reevaluar_ppu_nativa`, propagar el nuevo valor al worker: `if let Some(worker) = &self.worker { worker.bridge().set_gpu_ppu_enabled(activa && !FORZAR_PPU_SOFTWARE); }`.
   - Si en `Compositor::render` la creación de `PpuNativaPipeline` falla (`self.ppu_nativa_fallida = true`), desactivar `gpu_ppu` en el bridge en el siguiente ciclo para que el fallback reciba `rgba` o el shim según corresponda.
5. En `src/runner_tests.rs` y `src/pipeline_tests.rs`:
   - Añadir tests unitarios que verifiquen:
     - Que con `gpu_ppu_enabled = true`, `step_frame()` captura cambios por línea en `io_lineas` y `paleta_lineas`, avanza `frames_elapsed`, mantiene los contadores de audio (`pump_muestras > 0`) y reporta `ppu_ms == 0.0` y `color_ms == 0.0`.
     - Que al volver a poner `gpu_ppu_enabled = false`, `step_frame()` retoma `ppu_ms > 0.0` y `color_ms > 0.0`.
6. Sincronizar `docs/referencia/api-modulos-rust.md` con `python scripts/extraer_referencias.py` y ejecutar los 5 criterios del Pre-Commit Gatekeeper.

### 5. Qué más podría verse afectado

- `../gba-recomp/crates/gba-core/src/mem.rs` — banderas `omitir_ppu_software` y `capturar_lut_ligera` y búferes `io_lineas` / `paleta_lineas`.
- `src/runner.rs` y `src/runner_tests.rs` — sincronización de banderas en `set_gpu_ppu_enabled`, accesores `io_lineas()` / `paleta_lineas()` y salto de `temporal` en `step_frame()`.
- `src/pipeline.rs` y `src/pipeline_tests.rs` — copia directa de `runner.io_lineas()` y `runner.paleta_lineas()` hacia `PpuFrameState`.
- `src/platform.rs` — sincronización de `set_gpu_ppu_enabled` con `ppu_nativa_activa`.
- `docs/referencia/api-modulos-rust.md` — actualización automática de las firmas públicas de `src/runner.rs`.

### 6. Suposiciones

- Cuando `omitir_ppu_software` está activo en `MemMap`, ningún código del juego depende del contenido de `bus.framebuffer` (ya que `framebuffer` es un búfer interno del emulador en host y no forma parte del mapa de memoria direccionable por la CPU ARM7TDMI de la GBA) → prototipar: no.

### 7. Alcance

- Incluye:
  - Captura ligera por scanline de `io_lineas` (`96 B × 160`) y `paleta_lineas` (`1 KB × 160`) en `MemMap`.
  - Salto de `ppu::render_scanline` y de `temporal.push` / `temporal.compose` cuando `gpu_ppu_enabled` está activo.
  - Conexión de `io_lineas` y `paleta_lineas` reales en `GameRunner` y `EmulationWorker`.
  - Sincronización en caliente con `ppu_nativa_activa` en `src/platform.rs`.
- No incluye:
  - Eliminar el módulo `src/ppu_shim.rs` ni `src/ppu_shim.wgsl` (eso corresponde al Paso 3 acordado, en una propuesta dedicada una vez cerrado este paso).
  - Modificar los shaders `src/ppu_nativa.wgsl` ni la lógica de composición de `src/ppu_nativa.rs`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Con `gpu_ppu_enabled` activo, `GameRunner` captura `io_lineas` y `paleta_lineas` por scanline y omite la PPU por software
Caso de uso: cuando la PPU nativa en GPU está activa en producción, el hilo de emulación no debe gastar CPU en `render_scanline` ni en `Temporal`, pero sí debe capturar los registros I/O y la paleta de cada línea visible y mantener el reloj, DMAs y audio intactos | Tipo: normal
  Dado que un `GameRunner` con `set_gpu_ppu_enabled(true)` donde antes de la línea 0 se escribe `BLDY = 0x0004` y `palette[0..2] = 0x1234`, se avanza 50 scanlines, y luego se cambia `BLDY = 0x000C` y `palette[0..2] = 0x5678` para el resto del fotograma
  Cuando se completa el fotograma con `step_frame()`
  Entonces `runner.io_lineas()[0][0x54]` vale `0x04` y `runner.io_lineas()[100][0x54]` vale `0x0C`, `runner.paleta_lineas()[0][0..2]` vale `[0x34, 0x12]` y `runner.paleta_lineas()[100][0..2]` vale `[0x78, 0x56]`, `runner.last_timing().ppu_ms` es `0.0`, `runner.last_timing().color_ms` es `0.0`, y `runner.last_timing().pump_muestras` es mayor que `0`
Test: `src/runner_tests.rs::test_runner_gpu_ppu_captura_lut_por_linea_y_omite_ppu_software`
Estado previo: se completa al escribir el test: falla (actualmente `io_lineas` no existe en `GameRunner` y `ppu_ms` / `color_ms` son `> 0.0` aunque `gpu_ppu_enabled` sea `true`)

Escenario: Al desactivar `gpu_ppu_enabled` en caliente, `GameRunner` vuelve a ejecutar la PPU por software y la conversión de color
Caso de uso: si el usuario activa `ppu-nativa-off.flag` en Android o un test requiere la salida por software, el motor debe retomar inmediatamente el camino de CPU sin reiniciar la partida | Tipo: límite
  Dado que un `GameRunner` que ejecutó un fotograma con `set_gpu_ppu_enabled(true)` (`ppu_ms == 0.0`, `color_ms == 0.0`)
  Cuando se llama a `runner.set_gpu_ppu_enabled(false)` y se ejecuta un nuevo `runner.step_frame()`
  Entonces `runner.last_timing().ppu_ms` vuelve a ser `> 0.0` y `runner.last_timing().color_ms` vuelve a ser `> 0.0`
Test: `src/runner_tests.rs::test_runner_conmutacion_en_caliente_entre_gpu_ppu_y_software`
Estado previo: se completa al escribir el test: falla (actualmente `set_gpu_ppu_enabled(true)` no desactiva la PPU por software)

Escenario: El motor base upstream, el shim y los 47 experimentos permanecen intactos
Caso de uso: las garantías de regresión del proyecto y de la PPU nativa deben mantenerse al 100 % | Tipo: regresión
  Dado que `tests/datos/ppu-nativa-hashes-base.txt` y toda la suite de `blazing-recomp` y `gba-core`
  Cuando se ejecuta `cargo test` en ambos repositorios
  Entonces el 100 % de los tests unitarios y de integración pasan en verde
Test: `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- Con `gpu_ppu_enabled` activo, `GameRunner` captura `io_lineas` y `paleta_lineas` por scanline y omite la PPU por software → herramienta o comando:
  `cargo test --lib test_runner_gpu_ppu_captura_lut_por_linea_y_omite_ppu_software`
  → pasa si: `1 passed`
  → no detecta: errores en el empaquetado de `EmulationWorker`.
- Al desactivar `gpu_ppu_enabled` en caliente, `GameRunner` vuelve a ejecutar la PPU por software y la conversión de color → herramienta o comando:
  `cargo test --lib test_runner_conmutacion_en_caliente_entre_gpu_ppu_y_software`
  → pasa si: `1 passed`
  → no detecta: fallos en el hilo de Android.
- El motor base upstream, el shim y los 47 experimentos permanecen intactos → herramienta o comando:
  `cargo test` (en `blazing-recomp` y en `../gba-recomp/crates/gba-core`)
  → pasa si: el 100 % de los tests pasan en verde, seguido de `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check`.
  → no detecta: medición de FPS en un dispositivo Android físico.

### Correcciones previas (si las hubo)

- Ninguna.

---

## Resultado (lo completa el agente despues de editar y verificar)

Estado final: verificada por agente

### Predicho vs. real

- **Captura ligera de registros y paleta en MemMap:**
  - *Predicho:* Incorporar capturar_lut_ligera: bool, io_lineas: Box<[[u8; 0x60]; 160]> y paleta_lineas: Box<[[u8; 1024]; 160]> copiando 96 B + 1024 B en cada scanline visible durante HBlank.
  - *Real:* Implementado en ../gba-recomp/crates/gba-core/src/mem.rs bajo VPhase::Hblank con preasignacion en heap en MemMap::new. 90 tests de gba-core pasando al 100 %.
- **Salto de la PPU por software con acumuladores afines sincronizados:**
  - *Predicho:* Incorporar omitir_ppu_software: bool, avanzar ff_ref y ff_dirty mediante step_affine_accumulators y omitir ppu::render_scanline y 	emporal.push/compose.
  - *Real:* step_affine_accumulators replica la recarga y avance wrapping_add de PB/PD. GameRunner::step_frame() omite Temporal reportando ppu_ms == 0.0 y color_ms == 0.0. Validado con 	est_runner_gpu_ppu_captura_lut_por_linea_y_omite_ppu_software.
- **Conexion en EmulationWorker y conmutacion en caliente:**
  - *Predicho:* Copiar directamente 
unner.io_lineas() y 
unner.paleta_lineas() al PpuFrameState y permitir reactivacion inmediata al apagar gpu_ppu_enabled.
  - *Real:* src/pipeline.rs copia los buffers por linea reales en un slice; src/platform.rs sincroniza set_gpu_ppu_enabled con ppu_nativa_activa. Validado con 	est_runner_conmutacion_en_caliente_entre_gpu_ppu_y_software.
- **Suite de pruebas:**
  - *Predicho:* Todos los tests en verde.
  - *Real:* 289 tests de lazing-recomp (188 en src/ + 101 en 	ests/) y 90 tests de gba-core pasando al 100 %.

### Que se respeto del contexto

- La PPU por software permanece 100 % intacta como respaldo funcional y oraculo de pruebas.
- Cero copia masiva de VRAM/OAM por linea (175 KB/cuadro en lugar de 15,8 MB/cuadro).
- Cero rutas absolutas locales y sincronizacion automatica de docs/referencia/api-modulos-rust.md.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.

## Revisión humana (solo si el cambio tiene parte visible o de uso)

- [ ] En Android (o mediante telemetría `blazing-perf.txt`), comprobar que con la PPU nativa activa `ppu_ms` y `color_ms` caen a `0.00 ms` y que los degradados por línea (carteles de Lyn/Batta, cajas del mapa y cortinillas de ventana) se ven idénticos a `ppu_doble`.

Veredicto del usuario: <!-- aprobado | cambios pedidos: ... -->
