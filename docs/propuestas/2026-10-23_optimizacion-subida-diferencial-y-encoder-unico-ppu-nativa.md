# Propuesta: Optimización de subida diferencial, deduplicación por scanline y envío unificado de GPU en la PPU nativa

Fecha: 2026-10-23
Nivel: 2
Estado: discrepada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. **Deduplicación vertical en CPU (`src/ppu_nativa.rs`):** En `EstadoPpuNativa::decodificar_lineas` y `EstadoPpuNativa::decodificar_paletas_linea`, si la línea `py > 0` tiene exactamente los mismos bytes que `py - 1` (`io_lineas[py] == io_lineas[py - 1]` o `paleta_lineas[py] == paleta_lineas[py - 1]`), se copia directamente la fila ya decodificada (`capas[py] = capas[py - 1]` / `destino[py] = destino[py - 1]`) en lugar de volver a parsear los registros o los 512 colores RGB555 en cada una de las 160 líneas.
2. **Subida diferencial (*dirty tracking*) hacia los *Storage Buffers* de GPU (`src/ppu_nativa.rs`):** `PpuNativaPipeline` conserva una copia del último estado subido a la GPU y **solo invoca `queue.write_buffer`** sobre `vram_buffer` (`96 KB`), `palette_buffer` (`320 KB`), `filas_capa_buffer` (`41 KB`) y `filas_comp_buffer` (`10,2 KB`) cuando su contenido realmente cambió respecto al fotograma anterior (y en `quad_buffer` se sube una única vez al crear el pipeline en `PpuNativaPipeline::construir`). En `palette_buffer`, además, si solo cambió un subrango de filas `[fila_min..=fila_max]`, se sube únicamente ese tramo contiguo en vez de los `320 KB` completos.
3. **Poda de la pasada de ventana de sprites (`OBJWIN`) y unificación de `CommandEncoder` (`src/ppu_nativa.rs` y `src/platform.rs`):**
   - Si ninguna de las 160 líneas tiene activo el bit 15 de `DISPCNT` (`0x8000`, `OBJWIN`), se omite por completo el `begin_render_pass("ppu-nativa-pasada-obj-ventana")` (ahorrando un cambio de *Framebuffer Object* y un *tile flush* por fotograma en GPUs móviles *tile-based* como la Mali-G31).
   - Se añade `PpuNativaPipeline::encode_frame_lineas`, que recibe `&mut wgpu::CommandEncoder` del compositor de Android (`src/platform.rs`), fusionando los pases de la PPU nativa y el pase de pantalla/overlay/HUD en **un único `queue.submit()` por fotograma**.

### 2. Contexto

- Contexto consultado: telemetría real del TV Stick (CPU 4× Cortex-A53 @ 1344 MHz, GPU Mali-G31, `backend=OpenGL`) tras el Paso 2, `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/platform.rs` y `src/ppu_nativa_tests.rs`.
- Intención: la telemetría del TV Stick demostró que el salto de la PPU por software redujo el tiempo de CPU (`ejec`) de `~13,5 ms` a `3,5–6,5 ms` (`ppu = 0,0 ms`, `color = 0,0 ms`), pero el tiempo de dibujo en el hilo principal subió a `~30,8 ms` (`32 FPS`) por dos cuellos de botella en el driver OpenGL ES de la Mali-G31:
  1. `subida = 6,8–7,5 ms` (antes `~1,0 ms`), causado por decodificar en CPU `160 × 512 = 81.920` colores cada cuadro y subir incondicionalmente `~475 KB` por `glBufferSubData` (`320 KB` de paleta + `96 KB` de VRAM + `51 KB` de LUTs verticales), lo que fuerza *buffer ghosting* en el driver de Mali incluso cuando la paleta y la VRAM no cambiaron.
  2. `present = 21,5–22,8 ms`, causado porque al sumar `subida (7,2 ms) + 2 submits separados + 4 render passes`, el cuadro supera por pocos milisegundos la barrera del VSync de 60 Hz (`16,66 ms`) y cae al siguiente pulso de VSync (`33,3 ms = 30–32 FPS`). En los segundos `t=86s..90s` del mismo log, en cuanto la carga bajó levemente, el dispositivo entró en un solo VSync y subió a `46 FPS` (`present = 10,9 ms`).
- Restricciones y decisiones previas (máx. 3):
  - Mantener 0 diferencias visuales (100 % bit-exacto frente al resultado actual de la PPU nativa) (origen: propuestas del 2026-10-23) → implica: la deduplicación por línea y la subida diferencial son matemáticamente idénticas al camino completo (si un byte cambia, se actualiza; si es idéntico, el búfer de GPU ya contiene exactamente esos mismos bits) → protegida por «La deduplicación y el dirty tracking producen exactamente el mismo estado y rango sucio» | testeable.
  - Conservar la firma pública `render_frame` y `render_frame_lineas` para no romper `src/bin/ppu_doble.rs`, `examples/comun/gpu.rs` ni los tests existentes (origen: `AGENTS.md` §4.1) → implica: `render_frame_lineas` crea un `CommandEncoder` internamente, delega en `encode_frame_lineas` y hace `queue.submit`, mientras que `Compositor::render` en `src/platform.rs` usa `encode_frame_lineas` con un único `CommandEncoder` compartido → protegida por «El motor base, el shim y los 47 experimentos permanecen intactos» | testeable.
  - En `pasada-obj-ventana`, si una escena activa `DISPCNT` bit 15 (`OBJWIN`) pero tiene `num_sprites_ventana == 0` después de un cuadro que sí tuvo sprites ventana, la textura `vista_obj_ventana` debe limpiarse igualmente para no dejar máscara residual → implica: solo omitir `pasada-obj-ventana` cuando **ninguna línea del cuadro tiene `DISPCNT & 0x8000 != 0`** (es decir, `fs_comp` ni siquiera evalúa `obj_window`) o cuando `vista_obj_ventana` ya está limpia y `num_sprites_ventana == 0` → protegida por «La poda de `pasada-obj-ventana` limpia la máscara si había restos y la omite cuando `OBJWIN` está inactivo» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Reducir el tamaño de `palette_buffer` quitando soporte de paleta por línea → descartada porque reintroduciría el fallo visual de los paneles de unidad (Lyn, Batta) y cajas de texto del mapa.
- Ante imprevistos, prioriza: exactitud visual bit a bit > reducción de bytes subidos por `queue.write_buffer` > unificación de `CommandEncoder`.

### 3. Ejemplos

- Caso normal (cuadro estático o con cambios mínimos en mapa/menú): entre el fotograma $N$ y el fotograma $N+1$ la paleta (`160 × 1024 B`) y la VRAM (`96 KB`) no cambiaron, y dentro del fotograma las 160 líneas tienen la misma paleta → antes: el Cortex-A53 ejecutaba `81.920` conversiones `u16 -> u32` y `queue.write_buffer` enviaba `327.680 B` de paleta + `98.304 B` de VRAM a OpenGL ES en cada fotograma (`subida ≈ 7,2 ms`) → ahora: `decodificar_paletas_linea` convierte solo la línea `0` (`512` iteraciones) y copia las 159 restantes por `memcpy`, detecta que el rango sucio respecto a lo ya residente en `palette_buffer` es `None` y que `vram` no cambió, y realiza **`0 bytes` de subida** en `palette_buffer` y `vram_buffer`.
- Caso normal (degradado HBlank en el cartel de Lyn entre las líneas `80..115`): el DMA de HBlank modifica la paleta únicamente en las líneas `80` a `115` → antes: se subían las 160 líneas (`327.680 B`) → ahora: el comparador por fila detecta que solo cambiaron las filas `80..=115` (`36` filas) y sube únicamente `36 × 2048 = 73.728 B` con el offset `80 * 2048`.
- Caso normal (unificación de `CommandEncoder` en Android): en `Compositor::render` (`src/platform.rs`), con la PPU nativa activa → antes: `nativa.render_frame_lineas` creaba un `CommandEncoder` y llamaba a `queue.submit()`, y luego `Compositor::render` creaba un segundo `CommandEncoder` y llamaba a `queue.submit()` otra vez antes de `frame.present()` → ahora: `nativa.encode_frame_lineas` actualiza los búferes durante la fase de `subida` y graba los pases de la PPU nativa en el mismo `CommandEncoder` que luego graba el pase de composición de pantalla, ejecutando **un único `queue.submit()` por fotograma**.

### 4. Pasos

1. En `src/ppu_nativa.rs`:
   - Optimizar `EstadoPpuNativa::decodificar_lineas(io_lineas)`: para `py > 0`, si `io_lineas[py] == io_lineas[py - 1]`, reutilizar `capas[py] = capas[py - 1]` y `comps[py] = comps[py - 1]` sin volver a invocar `decode_io_regs`.
   - Optimizar `EstadoPpuNativa::decodificar_paletas_linea(paleta_lineas, destino)`: para `py > 0`, si `paleta_lineas[py] == paleta_lineas[py - 1]`, copiar `destino[py] = destino[py - 1]` directamente.
   - Añadir una función pura `rango_filas_modificadas<T: PartialEq, const N: usize>(previo: &[T; N], actual: &[T; N]) -> Option<(usize, usize)>` que devuelva el primer y último índice `(min, max)` donde `previo[i] != actual[i]`, o `None` si son idénticos.
   - En `PpuNativaPipeline`:
     - Subir `QUAD_VERTICES` a `quad_buffer` una sola vez dentro de `PpuNativaPipeline::construir` (usando `mapped_at_creation: true` o en el primer uso).
     - Añadir el seguimiento de estado previo en GPU (`vram_previa: Box<[u8; 96 * 1024]>`, `palette_gpu_previa: Box<[[u32; 512]; 160]>`, `filas_capa_previa: Box<[ConfigCapaGpu; 160]>`, `filas_comp_previa: Box<[ConfigCompGpu; 160]>`, `gpu_inicializado: bool`, `obj_ventana_limpia: bool`).
     - Separar `subir_buffers_lineas(&mut self, queue: &wgpu::Queue, ...)` y `grabar_pases(&mut self, encoder: &mut wgpu::CommandEncoder, target_view: &wgpu::TextureView)`:
       - En `subir_buffers_lineas`, usar `rango_filas_modificadas` (o comparación de slice en VRAM) para llamar a `queue.write_buffer` únicamente sobre el subrango de filas que cambió (o saltar la llamada si no cambió nada).
       - En `grabar_pases`, omitir `ppu-nativa-pasada-obj-ventana` si `self.estado.num_sprites_ventana == 0` y `self.obj_ventana_limpia` es `true` (marcando `obj_ventana_limpia = false` cuando se dibujan sprites ventana y `true` cuando se limpia).
     - Mantener `render_frame_lineas` llamando a `subir_buffers_lineas`, creando un `CommandEncoder`, llamando a `grabar_pases` y ejecutando `queue.submit`.
2. En `src/platform.rs`:
   - En `Compositor::render`:
     - Cuando `es_nativa` es `true`, ejecutar `nativa.subir_buffers_lineas(...)` dentro de la Fase 1 (`subida_ms`), y dentro de la Fase 2 (`codificacion_ms`) llamar a `nativa.grabar_pases(&mut encoder, &self.game_texture_view)` sobre el mismo `encoder` antes de `compositor-pass`, dejando un único `self.queue.submit([encoder.finish()])` para todo el fotograma.
3. En `src/ppu_nativa_tests.rs`:
   - Añadir tests unitarios para `decodificar_lineas` con líneas repetidas, `decodificar_paletas_linea` con líneas repetidas y `rango_filas_modificadas` (caso sin cambios `None`, cambio en una sola fila y cambio en un rango intermedio).
4. Sincronizar `docs/referencia/api-modulos-rust.md` con `python scripts/extraer_referencias.py` y verificar los 5 puntos del Pre-Commit Gatekeeper.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs` — deduplicación por línea, seguimiento de estado sucio (`rango_filas_modificadas`), separación en `subir_buffers_lineas` + `grabar_pases` y poda de `pasada-obj-ventana`.
- `src/platform.rs` — uso de `subir_buffers_lineas` en la fase de subida y `grabar_pases` en el `CommandEncoder` único del compositor.
- `src/ppu_nativa_tests.rs` — pruebas unitarias de deduplicación y cálculo de rangos sucios.
- `docs/referencia/api-modulos-rust.md` — actualización automática de las firmas públicas de `src/ppu_nativa.rs`.

### 6. Suposiciones

- En `wgpu`, grabar los pases que escriben en `self.game_texture_view` (`grabar_pases`) antes del pase que lee `self.game_texture_view` (`compositor-pass`) dentro del mismo `CommandEncoder` inserta automáticamente las barreras de transición de textura necesarias tanto en Vulkan como en OpenGL ES → prototipar: sí, está cubierto por los tests de GPU existentes y el validador de `wgpu`.

### 7. Alcance

- Incluye:
  - Deduplicación de filas idénticas consecutivas en `decodificar_lineas` y `decodificar_paletas_linea`.
  - Subida diferencial (*dirty tracking* por rango de filas) para `palette_buffer`, `filas_capa_buffer`, `filas_comp_buffer` y `vram_buffer`, más inicialización única de `quad_buffer`.
  - Poda de `pasada-obj-ventana` cuando la textura de máscara ya está limpia y no hay sprites ventana.
  - Fusión en un único `CommandEncoder` y un único `queue.submit()` en `src/platform.rs`.
- No incluye:
  - Cambios en `src/ppu_nativa.wgsl` ni en `gba-core`.
  - Eliminación de `src/ppu_shim.*`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El cálculo de rango de filas modificadas detecta cuadros idénticos y acota subrangos parciales
Caso de uso: evitar subir `320 KB` de paleta o `41 KB` de capas a la GPU cuando no cambiaron o cuando el HBlank solo modificó una franja de la pantalla | Tipo: normal
  Dado que dos arreglos de 160 filas donde inicialmente las 160 filas son iguales, y luego se modifican únicamente las filas `80` y `115`
  Cuando se evalúa `rango_filas_modificadas` antes y después de la modificación
  Entonces para los arreglos idénticos devuelve `None`, para una modificación en una única fila `42` devuelve `Some((42, 42))`, y para modificaciones en las filas `80` y `115` devuelve `Some((80, 115))`
Test: `src/ppu_nativa_tests.rs::rango_filas_modificadas_detecta_identicos_y_acota_subrango`
Estado previo: se completa al escribir el test: falla (`rango_filas_modificadas` aún no existe)

Escenario: La deduplicación por scanline produce exactamente las mismas tablas que la decodificación línea a línea
Caso de uso: acelerar en el Cortex-A53 la decodificación de `io_lineas` y `paleta_lineas` sin alterar ni un solo bit del resultado enviado a la GPU | Tipo: normal
  Dado que un conjunto de 160 líneas de registros I/O y de paleta con tramos constantes (líneas `0..50` iguales, cambio en `50..120`, y otro tramo constante en `120..160`)
  Cuando se ejecutan `EstadoPpuNativa::decodificar_lineas` y `EstadoPpuNativa::decodificar_paletas_linea`
  Entonces las 160 filas resultantes coinciden bit a bit con los valores de cada tramo y los tests de fidelidad visual de `tests/ppu_nativa_tests.rs` dan 0 diferencias
Test: `src/ppu_nativa_tests.rs::deduplicacion_por_linea_conserva_resultado_exacto`
Estado previo: se completa al escribir el test: pasa / verifica invariante tras añadir el atajo de deduplicación

Escenario: El motor base, el shim y los 47 experimentos permanecen intactos tras la optimización de subida y encoder
Caso de uso: garantizar que el seguimiento de estado sucio entre cuadros consecutivos y la poda de `obj_ventana` no introduzcan regresiones visuales | Tipo: regresión
  Dado que la suite completa de `blazing-recomp` (incluyendo los cuadros de aceptación con GPU en `tests/ppu_nativa_tests.rs`)
  Cuando se ejecuta `cargo test`
  Entonces el 100 % de los tests unitarios y de integración pasan en verde
Test: `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- El cálculo de rango de filas modificadas detecta cuadros idénticos y acota subrangos parciales → herramienta o comando:
  `cargo test --lib rango_filas_modificadas_detecta_identicos_y_acota_subrango`
  → pasa si: `1 passed`
  → no detecta: comportamiento del driver OpenGL ES en el dispositivo.
- La deduplicación por scanline produce exactamente las mismas tablas que la decodificación línea a línea → herramienta o comando:
  `cargo test --lib deduplicacion_por_linea_conserva_resultado_exacto`
  → pasa si: `1 passed`
  → no detecta: tiempos de `present` en el TV Stick.
- El motor base, el shim y los 47 experimentos permanecen intactos → herramienta o comando:
  `cargo test`
  → pasa si: el 100 % de los tests pasan en verde, seguido de `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check`.
  → no detecta: medición real de `subida` y `fps` en el TV Stick.

### Correcciones previas (si las hubo)

- Ninguna.

---

## Resultado (lo completa el agente después de editar y verificar)

Estado final: discrepada

### Predicho vs. real

- **Deduplicación por scanline:** *Predicho:* copiar la fila anterior cuando los bytes son idénticos. *Real:* implementado en `decodificar_lineas` y `decodificar_paletas_linea`; `deduplicacion_por_linea_conserva_resultado_exacto` compara contra la decodificación ingenua línea a línea y coincide bit a bit.
- **Subida diferencial:** *Predicho:* `write_buffer` solo del rango que cambió, bajando drásticamente `subida`. *Real:* `rango_filas_modificadas` (paleta, capas, composición) y `rango_bytes_modificados` (VRAM, alineado a 4 bytes); el quad se sube una sola vez. En el TV Stick (`blazing-perf-tv-28.txt`), `subida` sí bajó de `6,8–9,5 ms` a `3,1–5,2 ms` (una reducción real de `~3,0 ms` por fotograma), pero el tiempo total de `dibujo` permaneció clavado en `~29,5–31,5 ms` (`31–35 FPS`).
- **Poda de `pasada-obj-ventana`:** *Predicho:* omitirla si no hay sprites de ventana y la máscara ya está limpia. *Real:* campo `obj_ventana_limpia`; la primera pasada siempre se ejecuta.
- **Un solo `queue.submit()`:** *Predicho:* compartir `CommandEncoder` con el compositor para reducir la sobrecarga del driver. *Real:* `render_frame_lineas` se divide en `subir_buffers_lineas` + `grabar_pases`; `Compositor::render` sube en la fase de subida y graba los pases de la PPU nativa en el mismo encoder que el pase de pantalla. En el TV Stick, `codificacion` subió de `1,5–1,9 ms` a `4,0–5,2 ms` porque ahora incluye `nativa.grabar_pases` (que antes se contabilizaba dentro de `subida_ms`), y la suma `subida + codificacion` quedó en `~8,5 ms` (frente a `~8,5–11,0 ms` antes), mientras `present` siguió en `20–23 ms`.
- **Suite:** *Predicho:* 100 % en verde. *Real:* 190 tests en `src/` + 101 en `tests/` en verde; `validar_docs`, `extraer_referencias --check` y `auditar_labels` sin errores.

### Qué se respetó del contexto

- Firmas públicas de `render_frame` y `render_frame_lineas` intactas (`ppu_doble` y `examples/` no cambian).
- `gba-core`, `src/ppu_shim.*` y `src/ppu_nativa.wgsl` sin tocar.
- Sin rutas absolutas locales en el repositorio.

### Fuera de alcance (hallazgos que no se arreglaron)

- `src/ppu_nativa.rs`: la subida de sprites (`sprites_buffer` e instancias) no es diferencial; son pocos bytes, impacto: `bajo`.
- `src/ppu_nativa.wgsl`: en `fs_comp`, la indexación dinámica `cfg_capa.capas[b]` y `prio[b]` dentro del bucle anidado `4 × 4` sobre una estructura local de `272 bytes` (`ConfigCapa`) fuerza *register spilling* a memoria privada (*stack*) en GPUs móviles como la Mali-G31 (OpenGL ES 3.0 no permite indexar registros con variables no constantes), lo que domina el tiempo de ejecución del fragment shader y bloquea `present` en dos intervalos de VSync (`~31 ms`), impacto: `alto`.

---

## Discrepancia

Abierta por: agente
Punto afectado: *Intención del contexto* (bajar `dibujo` por debajo de `16,66 ms` en el TV Stick con Mali-G31 reduciendo `subida` y unificando `CommandEncoder`).
Motivo: la telemetría de la sesión 28 en el TV Stick (`31–35 FPS`, `dibujo = 28,0–31,5 ms`, `subida = 3,5–5,0 ms`, `codificacion = 4,0–5,0 ms`, `present = 18,0–23,5 ms`) demuestra que aunque `subida` bajó de `~7,5 ms` a `~4,3 ms`, el cuello de botella real no está en el envío de búferes desde CPU, sino en el coste de ejecución del shader `fs_comp` (`src/ppu_nativa.wgsl`) en la GPU Mali-G31: copiar `filas_capa[upy]` (`272 bytes` = `68` registros de 32 bits) a una variable local e indexar `cfg_capa.capas[b]` con el índice dinámico `b` obliga al compilador GLSL de Mali a volcar los 68 registros al *stack* de memoria por cada uno de los `38.400` píxeles y evaluarlos en un bucle anidado de `16` iteraciones.
Clasificación: suposición falsa
Propuesta de corrección: [`2026-10-23_eliminar-register-spilling-y-bucle-anidado-fs-comp.md`](2026-10-23_eliminar-register-spilling-y-bucle-anidado-fs-comp.md), que elimina la indexación dinámica de `ConfigCapa` y el bucle anidado `4 × 4` en `fs_comp`.

---

## Revisión humana (solo si el cambio tiene parte visible o de uso)

- [x] Probar el nuevo APK en el TV Stick (Mali-G31) y verificar en `blazing-perf.txt` cuánto bajan `subida` y `present`, y a cuántos FPS sube el juego.

Veredicto del usuario: discrepada (`blazing-perf-tv-28.txt`: `subida` bajó a ~4,3 ms pero el rendimiento total se mantuvo en 31–35 FPS porque la GPU sigue tardando >16,6 ms en ejecutar `fs_comp`).
