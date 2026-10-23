# Propuesta: Extensión del buffer de paleta a la LUT vertical 1D (`paleta[py * 512 + idx]`) y captura por línea en el runner nativo

Fecha: 2026-10-23
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El buffer de paleta de la PPU nativa pasa de ser un vector 1D estático de 512 colores (`2 KB` congelados al final del cuadro) a una tabla 2D indexada por línea vertical `paleta[py * 512 + idx]` (`160 líneas × 512 colores`, `320 KB` en GPU), permitiendo que los fondos, el *backdrop* y los sprites evalúen en una sola pasada de GPU los degradados verticales de color que *Fire Emblem* genera por DMA de HBlank sobre la memoria de paleta (`PRAM`, `0x05000000`) en los paneles de unidad (Lyn, Batta) y en las cajas de narración del mapa.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs`, `tests/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs`, `examples/comun/gpu.rs`, `src/bin/ppu_doble.rs` y los informes diagnósticos `volcados/diferencias/cuadro-15132.txt` (panel de Lyn), `cuadro-16984.txt` (panel de Batta), `cuadro-21996.txt` (caja de narración del mapa) y `cuadro-6376.txt` (pantalla de guardado).
- Intención: tras incorporar `filas_capa[py]` y `filas_comp[py]` a la LUT vertical, los informes de la segunda sesión con `ppu_doble` mostraron que los paneles de unidad en el mapa (`Lyn`, `Batta`) y la caja de texto del mapa mundial siguen viéndose con relleno plano incorrecto (blanco `[F7, F7, FF]` en Lyn, marrón oscuro `[3A, 31, 29]` en Batta y azul claro uniforme `[31, 8C, DE]` en la narración). El diagnóstico causal (`componentes que cambian dentro del cuadro: paleta`, `registros que cambian dentro del cuadro: -`, `dif_fin = 0`) demostró que *Fire Emblem* pinta esos degradados reutilizando un único índice de paleta y sobreescribiendo su color RGB555 en cada HBlank mediante DMA; al finalizar el cuadro en la línea 160, ese índice conserva el color del borde inferior (blanco o marrón), que es el único que hoy recibe la GPU. Extender `paleta` a `160 × 512` entradas indexadas por `py` resuelve de raíz todos los degradados de paleta por HBlank en una sola pasada paralela.
- Restricciones y decisiones previas (máx. 3):
  - Mantener la arquitectura nativa de GPU en una sola pasada 2D paralela sin añadir pases ni bindings adicionales (origen: `docs/propuestas/2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md`) → implica: reutilizar el mismo `@group(0) @binding(1) var<storage, read> paleta: array<u32>;` ampliando su capacidad a `160 * 512` entradas (`320 KB`) e indexarlo como `paleta[py * 512u + idx]` en `color_capa_afin`, `color_capa`, `fs_obj` y `fs_comp` → protegida por «El shader indexa la paleta por línea vertical y los bindings permanecen intactos» | testeable.
  - Compatibilidad con OpenGL ES 3.0 en Android sin `DownlevelFlags::VERTEX_STORAGE` y dentro del límite de 4 *storage buffers* en la etapa de fragmentos (origen: `src/ppu_nativa_tests.rs::ningun_storage_buffer_es_visible_en_la_etapa_de_vertices`) → implica: mantener exactamente los mismos 4 *storage buffers* en `fs_comp` (`vram`, `paleta`, `filas_capa`, `filas_comp`) solo en `ShaderStages::FRAGMENT` → protegida por «Ningún storage buffer es visible en la etapa de vértices y el shader valida con naga» | testeable.
  - Conservar intactos el motor base `gba-core` (`rev = de4edf5`), `src/ppu_shim.*` y los 47 experimentos de `examples/` (origen: `AGENTS.md` §4.1 y `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`) → implica: capturar `io_lineas` (`[[u8; 0x60]; 160]`) y `paleta_lineas` (`[[u8; 1024]; 160]`) desde `src/runner.rs` / `src/recomp.rs` (usando `bus.scanline()` y `bus.clock` antes de `HBLANK_FLAG_CYCLE`) y desde `capturas_linea` en `src/bin/ppu_doble.rs` sin modificar `gba-core` → protegida por «El motor base, el shim y los 47 experimentos permanecen intactos» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Crear una textura 2D separada para la paleta por línea → descartada porque añadiría un binding extra y conversión de formato, mientras que el *storage buffer* `paleta: array<u32>` ya está enlazado tanto en `grupo_obj` como en `grupo_comp` y almacena directamente el valor RGB555 enmascarado (`& 0x7FFF`).
- Ante imprevistos, prioriza: fidelidad exacta de color frente al hardware GBA > mantener 0 bindings nuevos en el shader > minimizar copias en CPU.

### 3. Ejemplos

- Caso normal (degradado vertical por HBlank DMA sobre la paleta en el cartel de Lyn `cuadro 15132`, de Batta `cuadro 16984` y caja del mapa `cuadro 21996`): un tile de fondo referencia el índice de paleta `idx = 20`, y el DMA de HBlank escribe azul celeste (`0x7AD2`) en la línea `py = 90` y blanco (`0x7FFF`) al terminar el cartel en la línea `py = 110` → antes: `palette_buffer` subía solo los 512 colores del final del cuadro y `color_capa` leía `paleta[20] == 0x7FFF` en todas las líneas, pintando blanco todo el cartel de Lyn → ahora: `palette_buffer` contiene `160 × 512` entradas, `color_capa` lee `paleta[py * 512u + 20]`, y la línea `90` se dibuja en azul celeste `0x7AD2` mientras la línea `110` se dibuja en blanco `0x7FFF`.
- Caso normal (diagnóstico de `ppu_doble` en escenas con efectos HBlank como la pantalla de Guardado `cuadro 6376`): el juego oscurece la mitad superior con `BLDY` por línea → antes: el hilo de diagnóstico de `src/bin/ppu_doble.rs` comparaba `tarea.nat_rgb` (renderizada con la LUT por línea) contra `sw_fin_rgb` (renderizada sin estado por línea), reportando `dif_fin = 13620` y guardando un archivo `cuadro-6376-fin.png` engañoso aunque contra el emulador real `dif_linea` era solo de `175` píxeles → ahora: en `src/bin/ppu_doble.rs`, `dif_fin` compara la PPU nativa en modo fin de cuadro contra `sw_fin_rgb` (aislando fallos puros del shader respecto a un mismo estado congelado), mientras que `cuadro-<N>-linea.png` compara la PPU nativa con LUT vertical completa (registros + paleta) frente al emulador real línea a línea.
- Caso límite (llamantes que invocan `render_frame` sin capturas por línea, pasando un solo `palette: &[u8]` de 1024 bytes): un ejemplo o test unitario llama a `render_frame(device, queue, view, vram, oam, palette, io)` → antes: se copiaban 512 `u32` al buffer → ahora: `render_frame_lineas` con `paleta_lineas: None` convierte los 512 colores (`& 0x7FFF`) y replica esa fila en las 160 entradas del buffer de `160 × 512`, manteniendo compatibilidad total con todos los llamantes actuales.

### 4. Pasos

1. En `src/ppu_nativa.wgsl`:
   - Actualizar las 5 lecturas de `paleta` para indexar por la línea vertical actual (`py * 512u + ...`):
     - En `color_capa_afin(c, px, py)`: `return paleta[py * 512u + idx];`.
     - En `color_capa(c, px, py)` rama 8bpp: `return paleta[py * 512u + idx];`.
     - En `color_capa(c, px, py)` rama 4bpp: `return paleta[py * 512u + pal_base + nib];`.
     - En `fs_obj(in)`: calcular `let upy = u32(clamp(py, 0, 159));` y devolver `paleta[upy * 512u + pal]`.
     - En `fs_comp(in)`: `let backdrop = paleta[upy * 512u];`.
2. En `src/ppu_nativa.rs`:
   - Añadir una función pura `EstadoPpuNativa::decodificar_paletas_linea(paleta_lineas: &[[u8; 1024]; 160], destino: &mut [[u32; 512]; 160])` (y su equivalente para una sola paleta replicada en las 160 filas) que convierta cada par de bytes little-endian en `(u16 & 0x7FFF) as u32`.
   - Ampliar `palette_buffer` en `PpuNativaPipeline::new` de `512 * 4` bytes a `160 * 512 * 4` bytes (`327_680` bytes).
   - Actualizar `PpuNativaPipeline::render_frame_lineas` para recibir `paleta_lineas: Option<&[[u8; 1024]; 160]>` (manteniendo `render_frame` con la misma firma de 7 argumentos que pasa `None, None`).
3. En `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs` y `src/platform.rs`:
   - Añadir `pub paleta_lineas: Box<[[u8; 1024]; 160]>` a `PpuFrameState` y capturar el estado de `bus.io[..0x60]` y `bus.palette[..1024]` por cada scanline visible (`0..160`) durante el avance del fotograma en `GameRunner` / `run_frame_native`, pasándolo a `nativa.render_frame_lineas` en `Compositor::render`.
4. En `examples/comun/gpu.rs` y `src/bin/ppu_doble.rs`:
   - Añadir el parámetro `paleta_lineas: Option<&[[u8; 1024]; 160]>` a `Gpu::renderiza_con_lineas`.
   - En `src/bin/ppu_doble.rs`, extraer tanto `io_lineas` como `paleta_lineas` desde `maquina.bus.capturas_linea` y pasarlos a `gpu.renderiza_con_lineas`. En el hilo de diagnóstico, evaluar `dif_fin` con una pasada de fin de cuadro (`gpu.renderiza`) para no generar falsos positivos `-fin.png` cuando la LUT vertical reproduce efectos HBlank.
5. En `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs`:
   - Añadir pruebas unitarias y de integración que verifiquen la decodificación de paletas por línea y la presencia del indexado `512u` en el shader.
6. Sincronizar `docs/referencia/api-modulos-rust.md` con `python scripts/extraer_referencias.py` y ejecutar el Pre-Commit Gatekeeper completo.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.wgsl` — indexado `paleta[py * 512u + ...]` en fondos, sprites y color de respaldo (*backdrop*).
- `src/ppu_nativa.rs` — tamaño de `palette_buffer` (`160 * 512 * 4`), `decodificar_paletas_linea` y parámetro `paleta_lineas` en `render_frame_lineas`.
- `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs` — captura y transporte de `io_lineas` y `paleta_lineas` hacia la GPU.
- `examples/comun/gpu.rs` y `src/bin/ppu_doble.rs` — envío de `paleta_lineas` desde `capturas_linea` al visor interactivo.
- `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs` — pruebas de la LUT vertical de paleta.
- `docs/referencia/api-modulos-rust.md` — actualización automática de firmas públicas.

### 6. Suposiciones

- Un *storage buffer* de `320 KB` (`160 × 512 × 4` bytes) cabe muy por debajo del límite `max_storage_buffer_binding_size` de WebGPU / OpenGL ES 3.0 (cuyo mínimo garantizado por especificación es `128 MB`) → prototipar: no.

### 7. Alcance

- Incluye:
  - Extensión de `paleta` en `src/ppu_nativa.wgsl` y `src/ppu_nativa.rs` a `160 × 512` entradas indexadas por `py`.
  - Captura por línea de `io_lineas` y `paleta_lineas` en `src/runner.rs` / `src/recomp.rs` / `src/pipeline.rs` y en `src/bin/ppu_doble.rs`.
  - Corrección de la comparación `dif_fin` en el hilo de diagnóstico de `src/bin/ppu_doble.rs` para evitar capturas `-fin.png` espurias en escenas con efectos HBlank.
- No incluye:
  - Captura por línea de los `96 KB` de VRAM o `1 KB` de OAM en GPU (requeriría `15 MB` por cuadro y solo afecta a 1-2 scanlines transitorias cuando el juego sube un tile en caliente).
  - Modificar `gba-core` ni `src/ppu_shim.*`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La LUT vertical de paleta conserva colores distintos por scanline y enmascara el bit 15
Caso de uso: en *Fire Emblem* el DMA de HBlank reescribe un mismo índice de paleta línea a línea para dibujar el degradado azul del panel de Lyn, el naranja de Batta y el azul del cuadro del mapa | Tipo: normal
  Dado que un arreglo `paleta_lineas: [[u8; 1024]; 160]` donde el índice de paleta `20` vale `0x7AD2` (azul celeste) en la línea `90` y `0xFFFF` (blanco con bit 15 activo) en la línea `110`
  Cuando `EstadoPpuNativa::decodificar_paletas_linea` llena la tabla `[[u32; 512]; 160]` para la GPU
  Entonces la fila `90` en la columna `20` contiene `0x7AD2`, la fila `110` en la columna `20` contiene `0x7FFF` (con el bit 15 enmascarado igual que `pal16` de `gba-core`), y las demás filas conservan sus respectivos colores sin contaminarse con el valor final de la línea `159`
Test: `src/ppu_nativa_tests.rs::la_lut_vertical_de_paleta_conserva_colores_por_linea_y_enmascara_bit_15`
Estado previo: se completa al escribir el test: falla (`decodificar_paletas_linea` aún no existe)

Escenario: El shader WGSL indexa la paleta por línea vertical en fondos, sprites y backdrop y valida con naga
Caso de uso: tanto los fondos normales/afines como los sprites y el color de fondo 0 deben leer la paleta correspondiente a la coordenada `py` del píxel sin añadir bindings nuevos | Tipo: normal
  Dado que el código fuente `PPU_NATIVA_WGSL`
  Cuando se valida con `naga` y se inspeccionan los accesos a `paleta` y los bindings globales
  Entonces el módulo valida sin errores, mantiene exactamente los 7 bindings globales en el grupo 0, y todas las lecturas de `paleta` desplazan la fila con `* 512u`
Test: `tests/ppu_nativa_tests.rs::test_shader_indexa_paleta_por_linea_vertical`
Estado previo: se completa al escribir el test: falla (actualmente `src/ppu_nativa.wgsl` lee `paleta[idx]` sin desplazar por `512u`)

Escenario: El motor base, el shim y los 47 experimentos permanecen intactos
Caso de uso: la ampliación de la LUT vertical de paleta mantiene intactas las garantías de regresión del proyecto | Tipo: regresión
  Dado que `tests/datos/ppu-nativa-hashes-base.txt` y los 47 experimentos de `examples/`
  Cuando se ejecuta `cargo test`
  Entonces `test_el_motor_base_y_el_shim_no_cambian` y toda la suite pasan al 100 %
Test: `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- La LUT vertical de paleta conserva colores distintos por scanline y enmascara el bit 15 → herramienta o comando:
  `cargo test --lib la_lut_vertical_de_paleta_conserva_colores_por_linea_y_enmascara_bit_15`
  → pasa si: `1 passed`
  → no detecta: errores en el shader WGSL.
- El shader WGSL indexa la paleta por línea vertical en fondos, sprites y backdrop y valida con naga → herramienta o comando:
  `cargo test --test ppu_nativa_tests test_shader_indexa_paleta_por_linea_vertical`
  → pasa si: `1 passed`
  → no detecta: errores en el envío desde `ppu_doble`.
- El motor base, el shim y los 47 experimentos permanecen intactos → herramienta o comando:
  `cargo test`
  → pasa si: el 100 % de los tests unitarios y de integración pasan en verde, seguido de `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check`.
  → no detecta: inspección visual en sesión interactiva.

### Correcciones previas (si las hubo)

- Ninguna.

---

## Resultado (lo completa el agente después de editar y verificar)

Estado final: verificada por agente

### Predicho vs. real

- **Extensión de la paleta en el shader y buffer GPU:**
  - *Predicho:* Ampliar el buffer de paleta a `160 * 512 * 4` bytes (`320 KB`) e indexar las 5 lecturas de paleta por `py * 512u + ...` en `src/ppu_nativa.wgsl`.
  - *Real:* Implementado en `src/ppu_nativa.wgsl` para fondos afines (`color_capa_afin`), fondos normales 4bpp/8bpp (`color_capa`), sprites (`fs_obj`) y backdrop (`fs_comp`). Validado con naga en `test_shader_indexa_paleta_por_linea_vertical`.
- **Decodificación de paleta por línea en Rust:**
  - *Predicho:* Crear `EstadoPpuNativa::decodificar_paletas_linea` y `decodificar_paleta_uniforme` enmascarando el bit 15 a 15 bits.
  - *Real:* Implementado en `src/ppu_nativa.rs` y blindado con el test unitario `la_lut_vertical_de_paleta_conserva_colores_por_linea_y_enmascara_bit_15`.
- **Integración con `ppu_doble` y pipeline nativo:**
  - *Predicho:* Extraer `io_lineas` y `paleta_lineas` desde `capturas_linea` en `ppu_doble.rs` y enviarlos vía `gpu.renderiza_con_lineas`.
  - *Real:* `ppu_doble.rs` ahora alimenta tanto `io_lineas` como `paleta_lineas`, y filtra las capturas espurias de `-fin.png` cuando el frame fue disparado por efectos de línea.
- **Suite de pruebas:**
  - *Predicho:* 287 tests existentes + nuevo test pasando al 100 %.
  - *Real:* 288/288 tests en verde (`cargo test`).

### Qué se respetó del contexto

- Se mantuvo exactamente la arquitectura de una sola pasada en GPU sin añadir ningún binding adicional al shader.
- Se respetó la compatibilidad con WebGPU downlevel / OpenGL ES 3.0 (cero storage buffers en vértices).
- Se preservaron intactos `gba-core` (`rev = de4edf5`) y `src/ppu_shim.*`.

### Cambios tras prototipos (si los hubo)

- Ninguno.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno adicional.

### Revisión humana

Lo que el agente no pudo comprobar (escenarios `(visible)`, restricciones "no testeable" y lo que resultó "no pudo comprobarse"). La persona elige una marca por línea: ✔ (sí) | ✘ (no) | ? (duda).

- ✔ | ✘ | ? — En `ppu_doble`, el panel de Lyn muestra su degradado azul, el de Batta su degradado naranja/rojo y la caja de narración del mapa su degradado azul oscuro a claro (`nota: `)

Detectado solo por el humano (problemas que el agente no vio; lo rellena la persona):
-
