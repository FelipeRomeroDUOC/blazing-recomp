# Propuesta: Pulido final de subida diferencial de sprites, uniformes del compositor y carga selectiva de capas en `ppu_nativa`

Fecha: 2026-10-23
Nivel: 2
Estado: revisada por humano

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se aplica el pulido final sobre los tres puntos donde aún se realizan transferencias `queue.write_buffer` o copias de registros redundantes en cada fotograma, con el objetivo de recortar entre `1,0` y `2,0 ms` adicionales de margen en dispositivos de gama de entrada como el TV Stick (CPU 4× Cortex-A53 @ 1344 MHz, GPU Mali-G31 con `backend=OpenGL`):

1. **Subida diferencial de OAM y búferes de sprites en [`src/ppu_nativa.rs`](../../src/ppu_nativa.rs):**
   - Actualmente, `PpuNativaPipeline::subir_buffers_lineas` invoca `self.estado.decode_oam(oam)` y ejecuta 4 llamadas incondicionales a `queue.write_buffer` (`sprites_buffer`, `sprites_instancia_buffer`, `sprites_ventana_buffer` y `sprites_instancia_ventana_buffer`) en **todos** los fotogramas, incluso cuando la OAM no ha cambiado o cuando `num_sprites == 0` / `num_sprites_ventana == 0`.
   - En el backend OpenGL ES de `wgpu`, cada llamada a `queue.write_buffer` emite `glBindBuffer` + `glBufferSubData`, lo que fuerza al driver de Mali a sincronizar si la GPU aún tiene el búfer en vuelo.
   - Se añaden cachés del último estado subido en `PpuNativaPipeline` (`oam_previa: Box<[u8; 1024]>`, `dispcnt_oam_previo: u16`, `sprites_previos: Box<[SpriteNativa; MAX_SPRITES]>`, `num_sprites_previo: usize`, `sprites_ventana_previos: Box<[SpriteNativa; MAX_SPRITES]>` y `num_sprites_ventana_previo: usize`):
     - Si `!primera`, `oam[..1024] == self.oam_previa[..]` y `(self.estado.dispcnt & 0x1040) == self.dispcnt_oam_previo`, se omite por completo `decode_oam` y las 4 subidas de búferes de sprites.
     - Cuando OAM o los bits `0x1040` de `DISPCNT` cambian, solo se suben `sprites_buffer` y `sprites_instancia_buffer` si `n > 0` y `(primera || n != self.num_sprites_previo || self.sprites_previos[..n] != self.estado.sprites[..n])`.
     - Del mismo modo, solo se suben `sprites_ventana_buffer` y `sprites_instancia_ventana_buffer` si `nv > 0` y `(primera || nv != self.num_sprites_ventana_previo || self.sprites_ventana_previos[..nv] != self.estado.sprites_ventana[..nv])`. En el TV Stick (`win=0` o sin sprites de ventana la mayor parte del tiempo), esto elimina 2 llamadas `glBufferSubData` en el 100 % de los fotogramas normales.
2. **Subida diferencial de uniformes del compositor y omisión del overlay táctil cuando está desactivado en [`src/platform.rs`](../../src/platform.rs):**
   - Actualmente, `Compositor::render` ejecuta en cada fotograma `build_globals`, `build_elems_with_orientation` (reservando un `Vec<ElemParams>` en el heap) y 4 llamadas a `queue.write_buffer` (`game_params_buffer`, `overlay_globals_buffer`, `overlay_elems_buffer` y `hud_params_buffer`), aunque en Android TV (`touch.touch_enabled == false`) el pase del overlay ni siquiera se dibuja y aunque `game_params` y `hud_params` son constantes mientras no cambie la geometría de la ventana.
   - Se guardan en `Compositor` los últimos uniformes subidos (`ultimo_game_params: Option<[f32; 8]>`, `ultimo_hud_params: Option<[f32; 8]>`, `ultimo_overlay_globals: Option<[f32; 4]>`, `ultimo_overlay_elems: Option<Box<[f32; 12 * 16]>>`):
     - `game_params_buffer` y `hud_params_buffer` solo se escriben con `queue.write_buffer` cuando cambian respecto al valor cacheado.
     - Cuando `!touch.touch_enabled`, se omite por completo el cálculo del layout del overlay (`build_globals`, `build_elems_with_orientation`, `Vec::collect`) y las 2 escrituras a `overlay_globals_buffer` y `overlay_elems_buffer`. Cuando `touch.touch_enabled` está activo (en móviles), también se suben solo si cambiaron (por ejemplo, al pulsar o soltar un botón táctil).
3. **Eliminación del paso por valor de `CapaFondo` (`64 bytes`) en `color_capa` dentro de [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl):**
   - Actualmente, `fs_comp` invoca `color_capa(filas_capa[upy].capas[0u], upx, upy)`. Como en WGSL los parámetros de función se pasan por valor, el compilador Naga/GLSL copia los 16 campos de `CapaFondo` (`64 bytes`, incluyendo los 6 parámetros afines `afin_pa..afin_ref_y`) desde el SSBO a registros temporales antes de entrar en `color_capa`, incluso en el Modo 0 (`98 %` de los fotogramas del juego) donde ningún fondo es afín y solo se usan 5 campos (`screen_size`, `scroll_x`, `scroll_y`, `screen_base`, `char_base`, `is_8bpp`). Además, `if (c.enabled == 0u)` dentro de `color_capa` es redundante cuando se llama desde `fs_comp` porque `cfg_comp.bg_prioX == p` (`p < 4u`) solo ocurre cuando `enabled != 0u` (`bg_prio` vale `0xFF` si la capa está deshabilitada).
   - Se refactoriza `color_capa(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32` y `color_capa_afin(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32` para que reciban `(upy, idx_capa)` y lean directamente del SSBO `filas_capa[upy].capas[idx_capa]` únicamente los campos que necesita la rama activa (en fondos de texto, solo los campos de texto; en fondos afines, solo los campos afines), evitando copiar los 6 coeficientes afines en cada muestreo de fondo de texto.

### 2. Contexto

- Contexto consultado: telemetría real del TV Stick (`blazing-perf-tv-29.txt`), [`src/ppu_nativa.rs`](../../src/ppu_nativa.rs), [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl), [`src/platform.rs`](../../src/platform.rs) y [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs).
- Intención: tras alcanzar `59,6 FPS` de media en `blazing-perf-tv-29.txt` (264 de 272 segundos a 59–60 FPS), las 7 microcaídas puntuales a 56–57 FPS ocurren cuando `subida` (`3,1–5,2 ms`) coincide con un pico de rasterización en una escena con más capas o sprites, rozando el presupuesto de `16,66 ms` del VSync. Eliminar las hasta 8 llamadas `glBufferSubData` redundantes por fotograma (`4` de sprites + `4` de uniformes del compositor) y la copia por valor de `64 bytes` por capa en `fs_comp` amplía el colchón de tiempo por fotograma tanto en CPU/driver como en ALU/L2 de la Mali-G31.
- Restricciones y decisiones previas (máx. 3):
  - Mantener el 100 % de exactitud visual píxel a píxel frente a la PPU de referencia de `gba-core` (origen: `2026-10-19_ppu-nativa-fase-1.md` y `tests/ppu_nativa_tests.rs`) → implica: la caché de OAM/sprites debe invalidarse ante cualquier cambio en los 1024 bytes de OAM o en los bits de `DISPCNT` que afectan a la decodificación de sprites (`bit 6` mapeo 1D/2D y `bit 12` activación de OBJ, máscara `0x1040`), y `color_capa(upy, idx_capa, px, py)` debe producir exactamente el mismo color RGB555 en modos de texto y afines → protegida por «La subida diferencial de sprites y el muestreo directo de capas conservan 0 diferencias visuales» | testeable.
  - Conservar intacto el layout de memoria de las estructuras compartidas entre Rust y WGSL (`CapaFondo`, `ConfigCapa`, `ConfigComp`, `SpriteNativa`) (origen: `tests/ppu_nativa_tests.rs::los_desplazamientos_de_las_estructuras_coinciden`) → implica: no alterar los campos ni el tamaño de `CapaFondo` (`64 bytes`), sino cambiar únicamente la firma interna de `color_capa` y `color_capa_afin` en `src/ppu_nativa.wgsl` → protegida por «El shader valida con naga y conserva todos los desplazamientos» | testeable.
  - No romper el renderizado del overlay táctil en smartphones cuando `touch.touch_enabled == true` (origen: `AGENTS.md` §2.4) → implica: cuando `touch.touch_enabled` sea `true`, los búferes del overlay siguen actualizándose en cuanto cambie cualquier parámetro geométrico o estado de botón presionado → protegida por «Los uniformes del compositor y del overlay se actualizan cuando cambian» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Reducir el tamaño de `CapaFondo` eliminando los campos afines → descartada porque los modos 1 y 2 usan fondos afines por línea (efectos HDMA) y requieren esos coeficientes en el SSBO.
- Ante imprevistos, prioriza: exactitud visual bit a bit > eliminación de escrituras `queue.write_buffer` redundantes en `subir_buffers_lineas` y `Compositor::render` > reducción de copias de estructura en `color_capa`.

### 3. Ejemplos

- Caso normal (OAM sin cambios entre dos fotogramas consecutivos): en la pantalla de título o cuando el cursor está quieto entre dos fotogramas → antes: `subir_buffers_lineas` ejecutaba `decode_oam(oam)` (recorriendo los 128 entradas OAM y ordenando sprites) y hacía 4 llamadas a `queue.write_buffer` (`sprites_buffer`, `sprites_instancia_buffer`, `sprites_ventana_buffer`, `sprites_instancia_ventana_buffer`) → ahora: ` decidir_subida_sprites` detecta que `oam` y `dispcnt & 0x1040` son idénticos al fotograma previo y realiza `0` decodificaciones y `0` llamadas a `queue.write_buffer`.
- Caso normal (TV Stick con mando y `touch_enabled == false` en resolución estable): durante toda la partida en el TV Stick → antes: `Compositor::render` reservaba un `Vec<ElemParams>` en cada fotograma y ejecutaba 4 llamadas a `queue.write_buffer` (`game_params_buffer`, `overlay_globals_buffer`, `overlay_elems_buffer`, `hud_params_buffer`) → ahora: tras el primer fotograma, mientras `(w, h, rw, rh)` no cambie y `touch_enabled` sea `false`, `Compositor::render` ejecuta `0` llamadas a `queue.write_buffer` para esos 4 búferes y `0` reservas en el heap.
- Caso normal (muestreo de un fondo de texto en Modo 0 en `src/ppu_nativa.wgsl`): cuando `fs_comp` evalúa `BG0` en el píxel `(upx, upy)` → antes: `color_capa(filas_capa[upy].capas[0u], upx, upy)` copiaba por valor los 16 campos (`64 bytes`) de `CapaFondo` (incluyendo los 6 enteros de la matriz afín) a registros locales antes de comprobar `if (c.es_afin != 0u)` → ahora: `color_capa(upy, 0u, upx, upy)` consulta `filas_capa[upy].capas[idx_capa].es_afin` y, al ser `0u`, lee únicamente los 5 campos de texto (`screen_size`, `scroll_x`, `scroll_y`, `screen_base`, `char_base`, `is_8bpp`), sin tocar las 6 palabras de la matriz afín.

### 4. Pasos

1. En [`src/ppu_nativa.rs`](../../src/ppu_nativa.rs):
   - Extraer una función auxiliar pura y testeable ` sprites_modificados(primera: bool, previos: &[SpriteNativa], actuales: &[SpriteNativa]) -> bool` que devuelva `true` únicamente cuando `actuales` no esté vacío (`!actuales.is_empty()`) y `(primera || previos != actuales)`.
   - Añadir a `PpuNativaPipeline` los campos `oam_previa: Box<[u8; 1024]>`, `dispcnt_oam_previo: u16`, `sprites_previos: Box<[SpriteNativa; MAX_SPRITES]>`, `num_sprites_previo: usize`, `sprites_ventana_previos: Box<[SpriteNativa; MAX_SPRITES]>` y `num_sprites_ventana_previo: usize`.
   - En `PpuNativaPipeline::subir_buffers_lineas`:
     - Comprobar si `primera || oam.len() < 1024 || self.oam_previa[..] != oam[..1024] || (self.estado.dispcnt & 0x1040) != self.dispcnt_oam_previo`.
     - Solo si cambió: actualizar `oam_previa` y `dispcnt_oam_previo`, ejecutar `self.estado.decode_oam(oam)`, subir `sprites_buffer` + `sprites_instancia_buffer` únicamente cuando `sprites_modificados(primera, &self.sprites_previos[..self.num_sprites_previo], &self.estado.sprites[..n])` sea `true`, y subir `sprites_ventana_buffer` + `sprites_instancia_ventana_buffer` únicamente cuando `sprites_modificados(primera, &self.sprites_ventana_previos[..self.num_sprites_ventana_previo], &self.estado.sprites_ventana[..nv])` sea `true`, actualizando `num_sprites_previo` y `num_sprites_ventana_previo`.
2. En [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl):
   - Cambiar la firma de `color_capa_afin(c: CapaFondo, px: u32, py: u32) -> u32` a `color_capa_afin(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32`, leyendo los campos afines desde `filas_capa[upy].capas[idx_capa]`.
   - Cambiar la firma de `color_capa(c: CapaFondo, px: u32, py: u32) -> u32` a `color_capa(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32`, leyendo desde `filas_capa[upy].capas[idx_capa]` únicamente `enabled`, `es_afin` y, si es de texto, los campos de texto (`screen_size`, `scroll_x`, `scroll_y`, `screen_base`, `char_base`, `is_8bpp`), sin copiar la estructura `CapaFondo` completa por valor.
   - Actualizar las 4 llamadas en `fs_comp` a `color_capa(upy, 0u, upx, upy)`, `color_capa(upy, 1u, upx, upy)`, `color_capa(upy, 2u, upx, upy)` y `color_capa(upy, 3u, upx, upy)`.
3. En [`src/platform.rs`](../../src/platform.rs):
   - Añadir a `Compositor` los campos `ultimo_game_params: Option<[f32; 8]>`, `ultimo_hud_params: Option<[f32; 8]>`, `ultimo_overlay_globals: Option<[f32; 4]>` y `ultimo_overlay_elems: Option<Box<[f32; 12 * 16]>>`.
   - En `Compositor::render`:
     - Escribir `game_params_buffer` solo si `self.ultimo_game_params != Some(game_params)`.
     - Si `touch.touch_enabled` es `true`, calcular `globals_packed` y `elems_packed` (sin alocar un `Vec` intermedio en el heap, iterando directamente sobre el array devuelto por `build_elems_with_orientation`) y escribir `overlay_globals_buffer` / `overlay_elems_buffer` solo si cambiaron respecto a `ultimo_overlay_globals` / `ultimo_overlay_elems`. Si `touch.touch_enabled` es `false`, omitir todo el bloque del overlay.
     - Escribir `hud_params_buffer` solo si `self.ultimo_hud_params != Some(hud_params)`.
4. En [`src/ppu_nativa_tests.rs`](../../src/ppu_nativa_tests.rs) y [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs):
   - Añadir pruebas unitarias para `sprites_modificados` (lista vacía no sube, lista idéntica al cuadro previo no sube, cambio en un sprite o en la cantidad sí sube).
   - Ampliar el test estático del shader en `tests/ppu_nativa_tests.rs` para verificar que `color_capa` no recibe `CapaFondo` por valor.
5. Ejecutar los 5 criterios del Pre-Commit Gatekeeper (`cargo test`, `validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py` y auditoría de rutas).

### 5. Qué más podría verse afectado

- [`src/ppu_nativa.rs`](../../src/ppu_nativa.rs) — caché de OAM/sprites y función `sprites_modificados` en `PpuNativaPipeline::subir_buffers_lineas`.
- [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl) — firma de `color_capa` y `color_capa_afin` por índice `(upy, idx_capa)` en lugar de copia por valor de `CapaFondo`.
- [`src/platform.rs`](../../src/platform.rs) — subida diferencial de `game_params_buffer`, `hud_params_buffer` y omisión/subida diferencial de `overlay_globals_buffer` y `overlay_elems_buffer` en `Compositor::render`.
- [`src/ppu_nativa_tests.rs`](../../src/ppu_nativa_tests.rs) y [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs) — tests unitarios y de integración.

### 6. Suposiciones

- Omitir las escrituras `glBufferSubData` redundantes de sprites y uniformes del compositor reduce el tiempo de `subida` en la CPU Cortex-A53 y evita bloqueos de sincronización implícita del driver OpenGL ES de Mali-G31 → prototipar: comprobable en la telemetría `subida` del TV Stick.

### 7. Alcance

- Incluye:
  - Deduplicación de `decode_oam` y subida diferencial de los 4 búferes de sprites en `src/ppu_nativa.rs`.
  - Subida diferencial de los uniformes del compositor y omisión del cálculo/subida del overlay cuando `!touch.touch_enabled` en `src/platform.rs`.
  - Eliminación del paso por valor de `CapaFondo` (`64 bytes`) en `color_capa` y `color_capa_afin` en `src/ppu_nativa.wgsl`.
  - Pruebas en `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs`.
- No incluye:
  - Cambios en `gba-core`, `src/ppu_shim.*` ni en los shaders del compositor (`src/overlay_shader.rs`).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: `sprites_modificados` omite subidas cuando la lista de sprites está vacía o no ha cambiado respecto al cuadro previo
Caso de uso: evitar llamadas `queue.write_buffer` innecesarias sobre los 4 búferes de sprites y ventana OBJ en `subir_buffers_lineas` | Tipo: normal y borde
  Dado un arreglo previo de `SpriteNativa` y el arreglo decodificado del cuadro actual
  Cuando la lista actual está vacía (`n == 0`) o es idéntica a la del cuadro previo (`!primera && previos == actuales`)
  Entonces `sprites_modificados` devuelve `false`; y devuelve `true` cuando `n > 0` y (`primera` es `true` o algún sprite/longitud cambió)
Test: `src/ppu_nativa_tests.rs::test_sprites_modificados_detecta_cambios_y_omite_vacios`
Estado previo: se completa al escribir el test: no existe (función nueva)

Escenario: `color_capa` y `color_capa_afin` acceden a `filas_capa` por índice sin copiar `CapaFondo` (`64 bytes`) por valor y conservan 0 diferencias visuales
Caso de uso: ahorrar copias de los 6 registros afines en cada muestreo de fondo de texto en Modo 0 | Tipo: normal y regresión
  Dado el shader `src/ppu_nativa.wgsl` y la suite de pruebas visuales de `tests/ppu_nativa_tests.rs`
  Cuando se valida con `naga` y se ejecuta `cargo test`
  Entonces `color_capa` y `color_capa_afin` reciben `(upy: u32, idx_capa: u32, px: u32, py: u32)` sin parámetro `c: CapaFondo` por valor, y todas las escenas de Modo 0, Modo 1, Modo 2, ventanas y mezcla producen 0 píxeles de diferencia
Test: `tests/ppu_nativa_tests.rs::test_fs_comp_no_indexa_arreglos_locales_dinamicamente`
Estado previo: se completa al escribir el test: falla (actualmente `color_capa` recibe `c: CapaFondo` por valor)

### 9. Métodos de comprobación

- Tests:
  - `cargo test` (suite completa de tests unitarios y de integración).
- Validación documental y de referencias:
  - `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  - `python scripts/extraer_referencias.py --check`
  - `python scripts/diagnostico/auditar_labels.py`
- Empaquetado y prueba en dispositivo (revisión humana):
  - `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`

### 10. Puntos de revisión humana

- Qué mirar: probar el APK en el TV Stick (CPU Cortex-A53, GPU Mali-G31) y verificar en `blazing-perf.txt` que `subida` baja aún más respecto a `blazing-perf-tv-29.txt` y que la fluidez a 60 FPS se mantiene sin errores gráficos.
- Cómo probarlo: instalar `target/android-artifacts/blazing-recomp-debug.apk` en el TV Stick y compartir el archivo `blazing-perf.txt`.

---

## Resultado (lo completa el agente después de ejecutar)

- Estado final: verificada por agente
- Criterios: 2 de 2 cumplidos en verde (test unitario `test_sprites_modificados_detecta_cambios_y_omite_vacios` y test estático/integración `test_fs_comp_no_indexa_arreglos_locales_dinamicamente` + 191 tests de `src/` y 101 tests de `tests/`).
- Predicho vs. real:
  - Predicho: OAM y sprites solo se decodifican y suben a la GPU cuando cambian respecto al cuadro previo, omitiendo las 4 llamadas a `queue.write_buffer` en fotogramas estáticos y cuando `nv == 0`.
  - Real: Implementado con `oam_previa`, `dispcnt_oam_previo`, `sprites_previos`, `sprites_ventana_previos` y la función `sprites_modificados`.
  - Predicho: `Compositor::render` omite el cálculo del overlay y la subida de sus 2 búferes cuando `!touch.touch_enabled`, y solo sube `game_params_buffer` y `hud_params_buffer` cuando cambian.
  - Real: Implementado con `ultimo_game_params`, `ultimo_hud_params`, `ultimo_overlay_globals` y `ultimo_overlay_elems`, eliminando 4 `write_buffer` por fotograma en TV Sticks y evitando recalcular la geometría táctil en cada frame.
  - Predicho: `color_capa` y `color_capa_afin` en `src/ppu_nativa.wgsl` reciben `(upy: u32, idx_capa: u32, px: u32, py: u32)` y leen del SSBO sin copiar `CapaFondo` (`64 bytes`) por valor.
  - Real: Implementado y validado con `naga` y la suite de regresión visual de la PPU nativa (`15 passed`).
- Respetado del contexto: exactitud visual bit a bit intacta, 0 rutas absolutas en el repositorio, suite completa de 292 tests en verde y APK multi-ABI compilado y firmado limpiamente.

### Cambios tras prototipos

- En `src/platform.rs`, el test estático `test_render_escala_bufer_y_wiring_de_calibracion` requería la presencia exacta de `.map(|e| e.scaled(escala_render))`, por lo que se mantuvo dicha invocación funcional dentro del bloque condicional `if touch.touch_enabled` preservando el contrato con el harness de tests.

### Fuera de alcance (hallazgos que no se arreglaron)

*(Ninguno; todos los cambios se ciñeron al alcance acordado).*

### Revisión humana

- Veredicto: revisada por humano y confirmada con telemetría real de 400 segundos (`blazing-perf-tv-30.txt`).
  - La media de `dibujo` bajó de `12.24 ms` a `11.26 ms` (ahorro neto de `~1.0 ms`), con `subida` bajando a `3.29 ms` y `codificacion` a `3.09 ms`.
  - La tasa promedio se consolidó en `59.7 FPS` a lo largo de 400 segundos (373 de 395 segundos estables en 59–61 FPS, ~94.4 % del tiempo).
  - Las variaciones a 56–58 FPS (5.6 % del tiempo) se deben a la concurrencia entre los hilos del worker de emulación y el compositor sobre los 4 núcleos Cortex-A53 a 1344 MHz compartidos con el sistema Android, manteniendo una jugabilidad totalmente fluida y sin cuellos de botella gráficos en la GPU Mali-G31.
