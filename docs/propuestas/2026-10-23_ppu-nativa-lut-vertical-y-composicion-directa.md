# Propuesta: Arquitectura moderna de PPU nativa con LUT vertical 1D, composición perezosa en una pasada y corrección de artefactos del G-Buffer

Fecha: 2026-10-23
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La PPU nativa en GPU pasa de evaluar un único estado de fin de cuadro sobre cuatro texturas intermedias de fondo a evaluar en paralelo una tabla vertical 1D de 160 filas (`filas_capa[py]` y `filas_comp[py]`) y muestrear los fondos bajo demanda directamente en la pasada de composición, eliminando el cartel blanco de unidad, la vibración de 1 píxel en el cuadro de estadísticas al subir de nivel, los sprites fantasma cuando `num_sprites == 0` y la saturación de brillo por precedencia de operadores en `brighten()`.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs`, `tests/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs`, `examples/comun/gpu.rs`, `src/bin/ppu_doble.rs`, `docs/fuentes/Manual Técnico_ Recompilación Estática y Arquitectura de Hardware de Game Boy Advance.md` y los 391 informes diagnósticos de `volcados/diferencias/`.
- Intención: la auditoría de las 391 capturas reveló que el 53 % de los fallos visibles en *Fire Emblem* (`dif_fin = 0, dif_linea > 0`, como el panel de unidad de Lyn en blanco, el cartel «Turno: enemigo», la vibración vertical del cuadro de estadísticas al subir de nivel y las cortinas de rombos) ocurren porque el juego reprograma `BLDCNT`, `BLDALPHA`, `BGxHOFS/VOFS` y `WIN0/1` entre líneas o en el DMA de VBlank (línea 160), mientras que la GPU leía un único uniforme escalar al final del cuadro. El 47 % restante proviene de dos fallos en el pipeline actual (`tex_obj` y `tex_obj_ventana` no se limpian cuando `num_sprites == 0`, dejando sprites fantasma en pantalla, y `brighten()` en WGSL escribe `x.r + ((31u - x.r) * evy + 8u) >> 4u` sin paréntesis alrededor del `>> 4u`). El objetivo es dotar a la PPU nativa de una arquitectura moderna en GPU (LUT vertical 1D indexada por `py` + evaluación perezosa de fondos sin texturas intermedias `tex_capa0..3`) que resuelva de raíz todos estos problemas en una sola pasada paralela 2D.
- Restricciones y decisiones previas (máx. 3):
  - Construir una arquitectura nativa de GPU (LUT vertical 1D `fila[py]` en un único despacho 2D paralelo + composición directa) en vez de emular barridos línea por línea desde la CPU (origen: respuesta humana, 2026-10-23) → implica: sustituir los uniformes escalares por tablas de 160 entradas indexadas por `py` en el fragment shader, evaluar `color_capa` bajo demanda dentro de `fs_comp` solo para las capas habilitadas por `control` hasta hallar `primera` y `segunda`, y eliminar las 4 texturas intermedias `tex_capa[0..4]` y `pipeline_capa` → protegida por «La LUT vertical 1D reproduce cambios de scroll y mezcla a mitad de cuadro en una sola pasada» y «Los desplazamientos y bindings del shader coinciden con las estructuras de Rust» | testeable.
  - Compatibilidad con OpenGL ES 3.0 en Android sin `DownlevelFlags::VERTEX_STORAGE` (origen: `src/ppu_nativa.rs` líneas 1138-1141 y `src/ppu_nativa_tests.rs::ningun_storage_buffer_es_visible_en_la_etapa_de_vertices`) → implica: todos los *storage buffers* (`vram`, `paleta`, `filas_capa`, `sprites`, `filas_comp`) deben declararse exclusivamente con visibilidad `wgpu::ShaderStages::FRAGMENT` y nunca leerse desde `vs_quad` ni `vs_obj` → protegida por «Ningún storage buffer es visible en la etapa de vértices y el shader valida con naga» | testeable.
  - Conservar intactos el motor base `gba-core` (`rev = de4edf5`), `src/ppu_shim.*` y los 47 experimentos de `examples/` (origen: `AGENTS.md` §4.1, `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian` y `tests/arnes_experimentos_tests.rs`) → implica: capturar los registros I/O por línea (`io_lineas: [[u8; 0x60]; 160]`) en `src/recomp.rs` / `src/runner.rs` y desde `capturas_linea` en `src/bin/ppu_doble.rs` sin tocar `gba-core`, manteniendo además `render_frame` compatible con los ejemplos que pasan un solo `io: &[u8]` (rellenando las 160 filas con ese estado uniforme) → protegida por «El motor base, el shim y los 47 experimentos permanecen intactos» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Lanzar 160 pases de renderizado en GPU (uno por scanline) → descartada porque serializaría el trabajo en la GPU y multiplicaría por 160 la sobrecarga del driver en Android.
  - Conservar las 4 texturas `tex_capa0..3` y el pase `fs_capa` → descartada porque obliga a escribir y volver a leer `240 × 160 × 4` píxeles en VRAM móvil aunque una capa esté oculta por una ventana o tapada por otra de mayor prioridad; evaluar `color_capa` directamente en `fs_comp` detiene la búsqueda en cuanto se encuentran `primera` y `segunda`.
- Ante imprevistos, prioriza: exactitud visual píxel a píxel frente a la especificación de la GBA > ahorro de ancho de banda en GPU móvil > simplicidad de estructuras auxiliares.

### 3. Ejemplos

- Caso normal (cartel de unidad de Lyn en el mapa y vibración del panel de estadísticas al subir de nivel, `cuadros 5235..5283` y `12461`): durante el cuadro, el juego activa `BLDCNT = 0x0344` / `BLDALPHA = 0x060D` en las líneas del cartel y lo apaga más abajo, o cambia `BG0VOFS` entre la mitad superior e inferior → antes: `PpuNativaPipeline` leía únicamente `io` al final del cuadro (tras el DMA de VBlank en la línea 160), dibujando el cartel opaco en blanco y haciendo vibrar el cuadro de estadísticas 1 píxel arriba y abajo → ahora: `PpuNativaPipeline::render_frame_lineas` recibe los registros de las 160 líneas, rellena `filas_capa[0..160]` y `filas_comp[0..160]`, y `fs_comp` consulta `filas_comp[py]` y `filas_capa[py]`, produciendo `0` píxeles de diferencia en el cartel translúcido y manteniendo quieto el cuadro de estadísticas.
- Caso normal (aclarado con `brighten()` en WGSL cuando `c` tiene canales mayores que 0, ej. `cuadro 172`): un píxel con canal rojo `x.r = 16` y `evy = 8` se aclara en modo 2 → antes: en `src/ppu_nativa.wgsl` la expresión `x.r + ((31u - x.r) * evy + 8u) >> 4u` sumaba primero `16 + 128 = 144` y luego desplazaba `144 >> 4 = 9` (oscureciendo el canal en lugar de aclararlo a `16 + 8 = 24`) → ahora: con los paréntesis corregidos `x.r + (((31u - x.r) * evy + 8u) >> 4u)`, el shader devuelve `24`, idéntico a `src/ppu_nativa.rs::brighten`.
- Caso límite (cuadro con `num_sprites == 0` o `num_sprites_ventana == 0` después de un cuadro con sprites, ej. `cuadro 172`): el juego apaga `DISPCNT` bit 12 o deshabilita todos los OAM al entrar en una transición → antes: `render_frame` saltaba `ppu-nativa-pasada-obj` (`if self.estado.num_sprites > 0`), dejando sin limpiar `tex_obj` y mostrando congelados en pantalla la bandera de idioma y el cursor del cuadro anterior → ahora: `ppu-nativa-pasada-obj` y `ppu-nativa-pasada-obj-ventana` ejecutan siempre su `LoadOp::Clear(wgpu::Color::TRANSPARENT)` (y solo invocan `pass.draw(...)` si `num_sprites > 0`), limpiando `tex_obj` y `tex_obj_ventana` en el 100 % de los cuadros.

### 4. Pasos

1. En `src/ppu_nativa.wgsl`:
   - Corregir la precedencia de operadores en `brighten()`: cambiar `x.r + ((31u - x.r) * evy + 8u) >> 4u` (y canales `g`, `b`) a `x.r + (((31u - x.r) * evy + 8u) >> 4u)`.
   - Declarar las tablas por línea `@group(0) @binding(2) var<storage, read> filas_capa: array<ConfigCapa, 160>;` y `@group(0) @binding(4) var<storage, read> filas_comp: array<ConfigComp, 160>;` (eliminando los bindings de `tex_capa0..3` y el punto de entrada `fs_capa`, y moviendo `tex_obj` y `tex_obj_ventana` a sus constantes actualizadas).
   - En `indice_paleta_obj`, leer `filas_capa[u32(py)].modo` para la comprobación de solapamiento de tiles en modos bitmap.
   - En `fs_comp`, leer `let cfg_comp = filas_comp[u32(py)];` y `let cfg_capa = filas_capa[u32(py)];`, y dentro del bucle de prioridades `for (var p: u32 = 0u; p < 4u; p = p + 1u)` evaluar perezosamente `color_capa(cfg_capa.capas[b], u32(px), u32(py))` solo cuando `prio[b] == p && (control & (1u << b)) != 0u`, deteniendo el bucle en cuanto se obtienen `primera` y `segunda`.
2. En `src/ppu_nativa.rs`:
   - Añadir soporte para el Modo 2 en `EstadoPpuNativa::decode_io_regs` (`es_afin = (self.modo == 1 && bg == 2) || (self.modo == 2 && (bg == 2 || bg == 3))`) y un método `EstadoPpuNativa::decodificar_lineas(io_lineas: &[[u8; 0x60]; 160]) -> ([ConfigCapaGpu; 160], [ConfigCompGpu; 160])` que acumule también el punto de referencia afín (`BG2X/Y`, `BG3X/Y`) línea a línea con `PB/PD` cuando el juego no lo reescribe en esa línea.
   - Eliminar las 4 texturas `tex_capa` / `vista_capa`, `pipeline_capa` y `grupo_capa`, convirtiendo `config_capa_buffer` (`160 * size_of::<ConfigCapaGpu>()`) y `config_comp_buffer` (`160 * size_of::<ConfigCompGpu>()`) en *storage buffers* de solo lectura para fragmento (`ShaderStages::FRAGMENT`), e incluir `vram_buffer` y `config_capa_buffer` en `grupo_comp`.
   - Ejecutar siempre los pases de limpieza `ppu-nativa-pasada-obj` y `ppu-nativa-pasada-obj-ventana` con `LoadOp::Clear`, condicionando únicamente la llamada `pass.draw(...)` a `num_sprites > 0` / `num_sprites_ventana > 0`.
   - Exponer `render_frame_lineas(..., io_fin: &[u8], io_lineas: Option<&[[u8; 0x60]; 160]>)` y mantener `render_frame(..., io: &[u8])` delegando en `render_frame_lineas(..., io, None)` (que replica `io` en las 160 filas) para no romper ningún llamante existente.
3. En `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs`, `examples/comun/gpu.rs` y `src/bin/ppu_doble.rs`:
   - Añadir `pub io_lineas: Box<[[u8; 0x60]; 160]>` a `PpuFrameState` y capturar los primeros `0x60` bytes de `bus.io` al cambiar de scanline visible (`0..160`) durante la ejecución del cuadro en `GameRunner` / `run_frame_native`, así como desde `capturas_linea` en `src/bin/ppu_doble.rs` (`gpu.renderiza_con_lineas`).
4. Actualizar las pruebas estructurales en `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs` para reflejar el pipeline unificado sin `fs_capa` y añadir los tests de los nuevos escenarios.
5. Sincronizar `docs/referencia/api-modulos-rust.md` ejecutando `python scripts/extraer_referencias.py` y verificar los 5 puntos del Pre-Commit Gatekeeper.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.wgsl` — corrección de `brighten()`, LUT vertical 1D (`filas_capa`, `filas_comp`) y composición perezosa directa en `fs_comp`.
- `src/ppu_nativa.rs` — eliminación de `tex_capa`, limpieza incondicional de `tex_obj`/`tex_obj_ventana`, soporte de Modo 2 y `render_frame_lineas`.
- `src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs` — transporte de `io_lineas` en `PpuFrameState` hacia `PpuNativaPipeline`.
- `examples/comun/gpu.rs` y `src/bin/ppu_doble.rs` — método `renderiza_con_lineas` para alimentar las 160 filas capturadas en el visor doble.
- `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs` — actualización de tests de reflexión WGSL y nuevos tests de LUT por línea, `brighten` y limpieza de sprites.
- `docs/referencia/api-modulos-rust.md` — actualización automática mediante `python scripts/extraer_referencias.py`.

### 6. Suposiciones

- Evaluar `color_capa` directamente dentro de `fs_comp` leyendo `filas_capa[py]` y `filas_comp[py]` desde *storage buffers* de solo lectura cabe holgadamente dentro del límite `max_storage_buffers_per_shader_stage` de WebGPU / OpenGL ES (usa 4 storage buffers en `fs_comp`: `vram`, `paleta`, `filas_capa`, `filas_comp`, que es el mínimo garantizado por el estándar WebGPU downlevel) → prototipar: sí
  → si es falso: si algún límite downlevel exigiera ≤ 3 storage buffers por etapa, empaquetar `ConfigCapaGpu` y `ConfigCompGpu` en una única estructura `FilaPpuGpu` en un solo *storage buffer* (dejando solo 3 storage buffers en `fs_comp`: `vram`, `paleta` y `filas_ppu`).
  → resultado del prototipo: [se completa tras la aprobación] | no detecta: drivers propietarios con bugs fuera del estándar.

### 7. Alcance

- Incluye:
  - Tabla de parámetros verticales 1D (`160` líneas) para fondos, ventanas y mezcla de color en `src/ppu_nativa.rs` y `src/ppu_nativa.wgsl`.
  - Evaluación perezosa de fondos directamente en `fs_comp` (eliminando `tex_capa0..3` y `fs_capa`).
  - Limpieza garantizada de `tex_obj` y `tex_obj_ventana` cuando `num_sprites == 0` o `num_sprites_ventana == 0`.
  - Corrección de precedencia de operadores en `brighten()` en `src/ppu_nativa.wgsl` y soporte de Modo 2 afín.
  - Conexión de `io_lineas` en `PpuFrameState` (`src/pipeline.rs`, `src/runner.rs`, `src/recomp.rs`, `src/platform.rs`) y en `src/bin/ppu_doble.rs`.
- No incluye:
  - Modificar `gba-core` ni `src/ppu_shim.*` (congelados por `test_el_motor_base_y_el_shim_no_cambian`).
  - Particionado de VRAM u OAM a mitad de cuadro (ninguna de las 391 capturas de *Fire Emblem* presentó mutaciones de VRAM u OAM entre líneas visibles).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La LUT vertical 1D reproduce cambios de scroll y mezcla a mitad de cuadro en una sola pasada
Caso de uso: en *Fire Emblem* el cartel de unidad de Lyn activa mezcla alfa solo en sus líneas y el cuadro de estadísticas al subir de nivel cambia `BG0VOFS` a mitad de pantalla | Tipo: normal
  Dado que un arreglo de 160 estados de línea `io_lineas` donde las líneas `0..80` tienen `BG0VOFS = 0` y `BLDCNT = 0x0041` (mezcla alfa activa) y las líneas `80..160` tienen `BG0VOFS = 4` y `BLDCNT = 0x0000` (sin mezcla)
  Cuando `EstadoPpuNativa::decodificar_lineas` genera las 160 filas para la GPU
  Entonces las filas `0..80` conservan `scroll_y == 0` y `bldcnt == 0x0041`, y las filas `80..160` conservan `scroll_y == 4` y `bldcnt == 0x0000`, permitiendo que la GPU dibuje ambas mitades sin usar el estado único de fin de cuadro
Test: `src/ppu_nativa_tests.rs::la_lut_vertical_conserva_scroll_y_mezcla_por_linea`
Estado previo: se completa al escribir el test: falla (`decodificar_lineas` aún no existe)

Escenario: El aclarado en el shader WGSL respeta la precedencia del desplazamiento y coincide con Rust
Caso de uso: durante fundidos a blanco y destellos de magia o subida de nivel (`BLDCNT` modo 2), los colores intermedios deben aclararse hacia 31 sin truncar el canal base | Tipo: límite
  Dado que el código fuente de `fn brighten` en `src/ppu_nativa.wgsl` y un color con canales no nulos (`c = 0x4210`, `evy = 8`)
  Cuando se valida la expresión aritmética del shader y se compara con `brighten(0x4210, 8)` en Rust
  Entonces cada canal en WGSL agrupa la suma del incremento antes del desplazamiento (`+ (((... ) * evy + 8u) >> 4u)`) y el resultado en Rust y WGSL para `r = 16, evy = 8` es `24` (y no `9`)
Test: `tests/ppu_nativa_tests.rs::test_brighten_wgsl_parentesis_y_no_cero_coinciden`
Estado previo: se completa al escribir el test: falla (en `src/ppu_nativa.wgsl` falta el paréntesis alrededor de `((31u - x.r) * evy + 8u) >> 4u`)

Escenario: Ningún storage buffer es visible en la etapa de vértices y el shader valida con naga
Caso de uso: el pipeline con LUT vertical 1D y composición directa debe arrancar en móviles y Android TV con OpenGL ES 3.0 sin errores de validación de `wgpu` | Tipo: límite
  Dado que el shader `src/ppu_nativa.wgsl` y los layouts de enlaces de `src/ppu_nativa.rs`
  Cuando `naga` valida el módulo y se inspeccionan todas las entradas de los `BindGroupLayout`
  Entonces el shader valida sin errores, ningún `Storage` buffer tiene visibilidad `ShaderStages::VERTEX`, y los desplazamientos de `CapaFondo`, `ConfigCapa`, `ConfigComp` y `SpriteNativa` coinciden byte a byte entre Rust y WGSL
Test: `tests/ppu_nativa_tests.rs::los_desplazamientos_de_las_estructuras_coinciden`
Estado previo: se completa al escribir el test: pasa (regresión estructural actualizada)

Escenario: El motor base, el shim y los 47 experimentos permanecen intactos
Caso de uso: la evolución de la PPU nativa no altera los archivos congelados ni rompe la suite de pruebas | Tipo: regresión
  Dado que `tests/datos/ppu-nativa-hashes-base.txt` y los 47 experimentos de `examples/`
  Cuando se ejecuta `cargo test`
  Entonces `test_el_motor_base_y_el_shim_no_cambian` y `test_el_inventario_coincide_con_cargo_toml` pasan al 100 %
Test: `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- La LUT vertical 1D reproduce cambios de scroll y mezcla a mitad de cuadro en una sola pasada → herramienta o comando:
  `cargo test --lib la_lut_vertical_conserva_scroll_y_mezcla_por_linea`
  → pasa si: `1 passed`
  → no detecta: fallos de presentación en ventana Win32.
- El aclarado en el shader WGSL respeta la precedencia del desplazamiento y coincide con Rust → herramienta o comando:
  `cargo test --test ppu_nativa_tests test_brighten_wgsl_parentesis_y_no_cero_coinciden`
  → pasa si: `1 passed`
  → no detecta: diferencias de redondeo fuera del modo 2.
- Ningún storage buffer es visible en la etapa de vértices y el shader valida con naga → herramienta o comando:
  `cargo test --test ppu_nativa_tests`
  → pasa si: todos los tests de `ppu_nativa_tests` pasan en verde.
  → no detecta: tiempos de GPU en hardware real.
- El motor base, el shim y los 47 experimentos permanecen intactos → herramienta o comando:
  `cargo test`
  → pasa si: el 100 % de la suite unitaria y de integración pasa en verde, seguido de `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check`.
  → no detecta: inspección visual en sesión interactiva.

### Correcciones previas (si las hubo)

- Se verificó el límite de *storage buffers* por etapa en WebGPU downlevel (OpenGL ES 3.0): el mínimo garantizado en fragmento es 4 (`max_storage_buffers_per_shader_stage = 4`). Al eliminar `tex_capa0..3` y evaluar los fondos en `fs_comp`, `fs_comp` utiliza exactamente 4 *storage buffers* (`vram`, `paleta`, `filas_capa`, `filas_comp`) o bien 3 si se unifican `ConfigCapa` y `ConfigComp`, cumpliendo estrictamente el límite de compatibilidad Android.

---

## Resultado (lo completa el agente después de editar y verificar)

Estado final: verificada por agente

### Predicho vs. real

- **Eliminación de texturas intermedias y pase previo:**
  - *Predicho:* Eliminar `fs_capa` y las texturas `tex_capa0..3` del pipeline, resolviendo capas directamente en `fs_comp`.
  - *Real:* `src/ppu_nativa.wgsl` y `src/ppu_nativa.rs` eliminan `pipeline_capa`, `tex_capa0..3` y `fs_capa`. `fs_comp` invoca `color_capa` de forma perezosa por cada píxel para las capas habilitadas hasta identificar `primera` y `segunda`.
- **LUT vertical 1D de 160 entradas:**
  - *Predicho:* Sustituir uniformes escalares por tablas de 160 entradas (`filas_capa` y `filas_comp`) indexadas por `py`.
  - *Real:* Implementado en WGSL como `var<storage, read> filas_capa: array<ConfigCapa>` y `var<storage, read> filas_comp: array<ConfigComp>`. Rust decodifica las 160 líneas en `decodificar_lineas` y sube los buffers correspondientes.
- **Limpieza de texturas de sprites:**
  - *Predicho:* Limpiar `tex_obj` y `tex_obj_ventana` incondicionalmente para evitar sprites fantasma cuando `num_sprites == 0`.
  - *Real:* El pase de renderizado de objetos usa `LoadOp::Clear` al inicio y solo ejecuta `draw` si `num_sprites > 0`.
- **Precedencia de operadores en `brighten()`:**
  - *Predicho:* Corregir `x.r + (((31u - x.r) * evy + 8u) >> 4u)`.
  - *Real:* Corregido en WGSL y blindado con el test de integración `test_brighten_wgsl_parentesis_y_no_cero_coinciden`.
- **Suite de pruebas:**
  - *Predicho:* 285 tests existentes + nuevos tests pasando al 100%.
  - *Real:* 287/287 tests en verde (185 de `src/lib.rs` + 102 de integración en `tests/`).

### Qué se respetó del contexto

- Se respetó la arquitectura nativa en GPU en una sola pasada 2D paralela, sin emular scanlines en bucles de CPU ni emitir 160 pases de renderizado.
- Se respetó la compatibilidad WebGPU downlevel / OpenGL ES 3.0: ningún *storage buffer* es visible en `ShaderStages::VERTEX`.
- Se preservaron intactos `gba-core` (`rev = de4edf5`), `src/ppu_shim.*` y los 47 ejemplos de `examples/`.

### Cambios tras prototipos (si los hubo)

- No hubo prototipos HTML necesarios al tratarse de lógica de shaders y pipelines en Rust/WGSL. Se verificó con naga en tiempo de compilación de tests.

### Fuera de alcance (hallazgos que no se arreglaron)

- En el motor base `gba-core` (`crates/gba-core/src/ppu.rs`, línea 976), la clave rápida `clave_de_tramo` calcula `mascara | if prioridad != 0xFF { 0x100 | prioridad as u16 } else { 0 }` sin desplazar `prioridad`, por lo que un sprite con `prioridad = 2` activa el bit 1 (`0x0002`), colisionando con el bit de opacidad de `BG1` en `mascara` durante `compose_alpha_tramos` (causa de los 77 píxeles de diferencia en el bocadillo `¡Aquí! ▼` del `cuadro 9254`, donde la PPU nativa dibuja correctamente el borde del globo encima del retrato y la PPU por software de `gba-core` pisa el borde). No se modifica `gba-core` por la restricción de congelación de `tests/datos/ppu-nativa-hashes-base.txt` (`impacto: bajo`).

### Revisión humana

Lo que el agente no pudo comprobar (escenarios `(visible)`, restricciones "no testeable" y lo que resultó "no pudo comprobarse"). La persona elige una marca por línea: ✔ (sí) | ✘ (no) | ? (duda).

- ✔ | ✘ | ? — En `ppu_doble`, el cartel de unidad de Lyn en el mapa y el cartel de cambio de turno se ven azules translúcidos (y no blancos) y el cuadro de estadísticas al subir de nivel permanece quieto sin vibrar (`nota: `)

Detectado solo por el humano (problemas que el agente no vio; lo rellena la persona):
-
