# Propuesta: Sincronizar la referencia del visor doble con el estado de fin de cuadro y diagnosticar el temblor de fondos y carteles

Fecha: 2026-10-22
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En el visor interactivo `ppu_doble`, la comparación automática en segundo plano pasa a contrastar la PPU nativa contra la referencia de estado congelado (`sw_fin`) además del framebuffer por línea (`sw_linea`), eliminando las falsas alarmas por escrituras en VBlank, se añade una tecla (`L`) para alternar en vivo qué referencia muestra la pantalla izquierda y se registra en el informe de cada captura qué registros de desplazamiento (`BGxHOFS`/`BGxVOFS`), ventana o mezcla cambiaron dentro del cuadro para explicar el temblor del panel de estadísticas al subir de nivel y el cartel blanco de unidad.

### 2. Contexto

- Contexto consultado: `src/bin/ppu_doble.rs`, `examples/comun/diagnostico.rs`, `examples/ppu_nativa_compara.rs`, `examples/ppu_barrido_nativa.rs`, `src/pipeline.rs`, `src/runner.rs`, `../gba-recomp/crates/gba-core/src/mem.rs`, `../gba-recomp/crates/gba-core/src/ppu.rs`, `tests/arnes_experimentos_tests.rs` y las 1.274 capturas de `volcados/diferencias/`.
- Intención: en la primera sesión con `ppu_doble` se generaron 1.274 capturas porque el hilo de diagnóstico recibió `sw_rgb` (`m.bus.framebuffer`, dibujado línea a línea) tanto en el parámetro `sw_fin` como en `sw_linea`. Eso hizo que cualquier escritura del juego después de las líneas visibles o a mitad de cuadro se etiquetara falsamente como `render_fin_de_cuadro`, y además la persona observó en vivo que al subir de nivel a Lyn (`cuadros 17656..17922`) el cuadro de estadísticas vibraba verticalmente 1 píxel arriba y abajo en la PPU nativa mientras en la PPU por software permanecía quieto. El éxito es que `ppu_doble` separe de verdad `sw_fin` (bugs del shader de la GPU) de `sw_linea` (cambios de registros como `BGxVOFS` o `BLDCNT` entre líneas), no llene la carpeta con miles de capturas repetidas y deje escrito el informe completo de `diag.informe()` en cada `.txt`.
- Restricciones y decisiones previas (máx. 3):
  - Mantener el guardado automático ante diferencias nuevas más captura manual con `F5`, pero aislando los fallos reales del shader y registrando también por qué tiemblan los paneles cuando cambian los registros entre líneas (origen: respuesta humana, 2026-10-22) → implica: reconstruir `sw_fin` en el hilo trabajador (o con una máquina auxiliar de replay en `ppu_doble`) con el mismo estado congelado que recibe la GPU, pasar `sw_fin` y `sw_linea` reales a `diagnosticar`, y cuantizar la caja de la firma (`caja_cuantizada` en bloques de 16 px) cuando la causa es `estado_por_linea` para que una cortina de transición no dispare 60 capturas seguidas → protegida por «El diagnosticador en vivo separa un fallo real del shader de un cambio de estado entre líneas» y «El filtro de firmas agrupa transiciones graduales sin omitir fallos nuevos» | testeable.
  - Conservar intactos los 47 experimentos de `examples/`, `tests/datos/ppu-nativa-hashes-base.txt` y el árbol de dependencias de Android (origen: `tests/arnes_experimentos_tests.rs` y `tests/ppu_nativa_tests.rs`) → implica: editar únicamente `examples/comun/diagnostico.rs`, `src/bin/ppu_doble.rs` y `tests/arnes_experimentos_tests.rs`, declarando `required-features = ["proto"]` en `[[bin]] name = "ppu_doble"` en `Cargo.toml` para que `ppu_doble` tenga acceso a `m.bus.capturas_linea` y pueda reportar qué registro exacto (`BG0VOFS`, `BG1VOFS`, `WIN0V`, `BLDCNT`) causó la vibración o el cartel blanco → protegida por «El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos» | testeable.
  - Prohibición de versionar rutas locales absolutas o capturas de la ROM (origen: `AGENTS.md` §3 y §7) → implica: mantener todas las salidas en `volcados/diferencias/` → protegida por «El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Ignorar por completo `sw_linea` y comparar sólo contra `sw_fin` → descartada porque ocultaría bugs visibles que ocurren en la PPU nativa en vivo (como la vibración del cuadro de estadísticas al subir de nivel o el cartel de Lyn en blanco si provienen de un registro escrito a mitad de cuadro).
  - Reconstruir `sw_fin` (160 líneas de PPU por software) en el hilo principal en cada uno de los 60 cuadros por segundo → descartada si no está activa la vista `sw_fin`, porque la máquina auxiliar en el hilo trabajador puede reconstruir `sw_fin` únicamente cuando un cuadro difiere, manteniendo el bucle principal ligero.
- Ante imprevistos, prioriza: exactitud del diagnóstico (`sw_fin` vs `sw_linea` + registros que cambiaron entre líneas) > reducción del número de capturas redundantes > controles extra en la ventana.

### 3. Ejemplos

- Caso normal (subida de nivel de Lyn donde el cuadro de estadísticas vibra, ej. `cuadros 17656..17922`): en los cuadros pares la PPU nativa dibuja los textos y números del panel `Lord / PV / Fuer / Hab / Vel` 1 píxel desplazados respecto a la PPU por software → antes: `ppu_doble` pasaba `&tarea.sw_rgb` dos veces a `diagnosticar`, escribía 140 archivos `.txt` idénticos diciendo `causa: render_fin_de_cuadro` y `resuelve: []`, y no decía qué capa ni qué registro se movió → ahora: el hilo trabajador reconstruye `sw_fin` con el estado congelado de fin de cuadro, compara `nativa` contra `sw_fin` y contra `sw_linea`, guarda en `cuadro-<N>-<caso>.txt` el informe completo de `diag.informe()` (indicando si `dif_fin == 0` y nombrando el registro exacto que cambió entre la línea del panel y el final del cuadro, p. ej. `BG0VOFS` / `BG1VOFS` o `oam`), y el filtro no repite la captura en los 139 cuadros siguientes del mismo parpadeo.
- Caso normal (cartel de unidad de Lyn en blanco en el mapa, ej. `cuadro 14618`): el cartel sale azul translúcido en software y blanco en la nativa → antes: el `.txt` sólo tenía 5 líneas truncadas sin los residuos de ablación ni el píxel testigo → ahora: el `.txt` incluye la tabla completa de ablación (`sin BG0..BG3`, `sin OBJ`, `sin WIN0..OBJWIN`, `sin BLDCNT`), los registros que cambiaron en el cuadro y el píxel testigo `(x, y)` con su ventana y sprites.
- Caso límite (transición gradual de cortina o *fade* donde la caja crece píxel a píxel): durante un *fade* o cortina de rombos por HBlank (`dif_fin == 0`, `dif_linea > 0`), la caja de diferencias cambia unos pocos píxeles por cuadro → antes: generaba hasta 87 PNG seguidos de la misma transición → ahora: para fallos puramente de `estado_por_linea` (`dif_fin == 0`), la firma agrupa la caja en celdas de `32x32` píxeles, reduciendo una transición entera a un puñado de capturas representativas, mientras que cualquier fallo real del shader (`dif_fin > 0`) se captura siempre con precisión de 1 píxel.

### 4. Pasos

1. En `examples/comun/diagnostico.rs`:
   - Añadir `render_software_fin(rom: &[u8], vram: &[u8], oam: &[u8], palette: &[u8], io: &[u8]) -> Vec<u8>` (reconstrucción limpia en RGB8 de las 160 líneas con estado congelado de fin de cuadro) para que tanto `ppu_doble` como los tests calculen `sw_fin` de verdad y nunca se pase `sw_linea` duplicado por error.
   - Actualizar `decidir_captura_en_vivo` (o añadir `evaluar_cuadro_en_vivo`) para que distinga diferencias en `sw_fin` (precisión exacta) de diferencias sólo en `sw_linea` (caja cuantizada en bloques de 32 px para no inundar el disco durante transiciones HBlank).
2. En `Cargo.toml`: añadir `required-features = ["proto"]` al bloque `[[bin]] name = "ppu_doble"` para habilitar `m.bus.capturas_linea` en el visor y poder detectar qué registros (`BG0VOFS`, `BLDCNT`, `WIN0H`, etc.) o memorias (`vram`, `oam`, `paleta`) mutaron a mitad de cuadro.
3. En `src/bin/ppu_doble.rs`:
   - Activar `maquina.bus.capturar_linea = true` antes de `run_frame` y enviar las capturas de línea al hilo trabajador cuando se detecte una diferencia nueva o se pulse `F5`.
   - En el hilo trabajador, calcular `sw_fin_rgb = render_software_fin(...)`, guardar `cuadro-<N>-fin.png` si `dif_fin > 0` y `cuadro-<N>-linea.png` si `dif_linea > 0`, ejecutar `diagnostico::diagnosticar` pasándole `sw_fin_rgb`, `sw_linea_rgb` y las 160 `lineas_estado` reales, y escribir el informe completo (`diag.informe().join("\n")`) en `volcados/diferencias/cuadro-<N>.txt`.
   - Añadir la tecla `L` en la ventana para alternar en vivo el panel izquierdo entre `PPU Software (por línea, emulador)` y `PPU Software (estado fin de cuadro)`, mostrando en la barra de título `dif_fin` y `dif_linea` por separado.
4. Añadir los tests en `tests/arnes_experimentos_tests.rs` y verificar el checklist de Pre-Commit.

### 5. Qué más podría verse afectado

- `Cargo.toml` — añade `required-features = ["proto"]` en `[[bin]] name = "ppu_doble"`.
- `examples/comun/diagnostico.rs` — añade `render_software_fin` y cuantización de firma para `linea`.
- `src/bin/ppu_doble.rs` — conecta `sw_fin`, `capturas_linea`, tecla `L` e informe completo en `.txt`.
- `tests/arnes_experimentos_tests.rs` — añade los tests de los nuevos escenarios.
- Búsqueda hecha: revisión de `tests/arnes_experimentos_tests.rs` y `tests/ppu_nativa_tests.rs`.

### 6. Suposiciones

- Activar `maquina.bus.capturar_linea = true` en `ppu_doble` en modo `--release` mantiene los 60 FPS en PC (copia ~15 MB por segundo en RAM) → prototipar: sí
  → si es falso: en lugar de activar `capturar_linea` en todos los cuadros, cuando el bucle principal detecta una diferencia nueva guarda una copia del estado de la máquina antes del cuadro o activa `capturar_linea` únicamente en el cuadro siguiente para diagnosticar la racha sin coste continuo.
  → resultado del prototipo: [se completa tras la aprobación] | no detecta: equipos con poca ancho de banda de RAM.

### 7. Alcance

- Incluye:
  - Corrección de `src/bin/ppu_doble.rs` para calcular `sw_fin` real, capturar el estado por línea (`proto`), alternar la referencia izquierda con la tecla `L` y guardar el informe causal completo en `volcados/diferencias/cuadro-<N>.txt`.
  - Reducción de capturas redundantes en `examples/comun/diagnostico.rs` cuantizando la caja de firmas cuando el desajuste es sólo de `estado_por_linea` (transiciones HBlank/VBlank) y manteniendo precisión exacta cuando `dif_fin > 0`.
  - Pruebas unitarias en `tests/arnes_experimentos_tests.rs`.
- No incluye:
  - Modificar todavía `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl` ni `src/pipeline.rs` hasta confirmar con el nuevo informe si la vibración del Level Up y el cartel blanco son mutaciones de línea o de shader.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El diagnosticador en vivo separa un fallo real del shader de un cambio de scroll vertical entre líneas
Caso de uso: al subir de nivel el cuadro de estadísticas vibra o cambia un registro a mitad de cuadro y queremos saber si es bug del shader o de sincronización de línea | Tipo: normal
  Dado que un estado de prueba donde en la línea 0..79 `BG0VOFS` vale `0` y al final del cuadro (`io_fin`) `BG0VOFS` vale `1` (desplazando el panel 1 píxel verticalmente), y la PPU nativa dibuja con `io_fin`
  Cuando se reconstruye `sw_fin` con `render_software_fin` y se llama a `diagnosticar` con `nativa`, `sw_fin` y `sw_linea`
  Entonces `dif_fin` vale `0`, `dif_linea` es mayor que `0`, `diag.causa` es `"estado_por_linea"` (y no `"render_fin_de_cuadro"`), y `diag.registros` contiene `"BG0VOFS"`
Test: `tests/arnes_experimentos_tests.rs::test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea`
Estado previo: se completa al escribir el test: falla (`render_software_fin` aún no existe)

Escenario: El filtro de firmas agrupa transiciones graduales de línea sin omitir fallos de fin de cuadro
Caso de uso: durante una cortina o transición por HBlank la caja crece píxel a píxel y no debe generar decenas de capturas seguidas | Tipo: límite
  Dado que tres cuadros consecutivos donde `sw_fin` coincide con `nativa` (`dif_fin == 0`) pero `sw_linea` difiere en cajas que crecen dentro del mismo bloque de 32 px: `(10, 10, 40, 40)`, `(10, 10, 42, 41)` y `(10, 10, 45, 43)`, seguidos de un cuarto cuadro donde `sw_fin` sí difiere de `nativa` (`dif_fin > 0`) en `(10, 10, 45, 43)`
  Cuando `decidir_captura_en_vivo` evalúa los cuatro cuadros
  Entonces en el primer cuadro devuelve `Some("linea")`, en el segundo y tercer cuadro devuelve `None` (agrupados como misma transición), y en el cuarto cuadro devuelve `Some("fin")` inmediatamente
Test: `tests/arnes_experimentos_tests.rs::test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin`
Estado previo: se completa al escribir el test: falla (la función actual no recibe `sw_fin` ni agrupa cajas de transición)

Escenario: El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos
Caso de uso: actualizar `ppu_doble` con `proto` no rompe la suite del repositorio | Tipo: regresión
  Dado que `Cargo.toml`, los 47 experimentos de `examples/` y `src/bin/ppu_doble.rs`
  Cuando se ejecutan `cargo test --test arnes_experimentos_tests`, `cargo build --release --features proto --bin ppu_doble` y `python scripts/extraer_referencias.py --check`
  Entonces todos terminan con código `0` y sin advertencias
Test: `tests/arnes_experimentos_tests.rs::test_el_inventario_coincide_con_cargo_toml`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- El diagnosticador en vivo separa un fallo real del shader de un cambio de scroll vertical entre líneas → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea`
  → pasa si: `1 passed`
  → no detecta: si el cambio de `BG0VOFS` ocurrió en HBlank (líneas 0..159) o en VBlank (líneas 160..227); el informe nombra ambos si la línea 0 del cuadro tenía el valor previo al VBlank final.
- El filtro de firmas agrupa transiciones graduales de línea sin omitir fallos de fin de cuadro → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin`
  → pasa si: `1 passed`
  → no detecta: transiciones que saltan más de 32 píxeles de golpe por cuadro (que generarán ~5 capturas a lo largo de los 240 px de ancho en vez de 60).
- El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos → herramienta o comando:
  `cargo test --test arnes_experimentos_tests && cargo build --release --features proto --bin ppu_doble && python scripts/extraer_referencias.py --check`
  → pasa si: todos salen con código `0`
  → no detecta: ergonomía visual al alternar la tecla `L` en la ventana.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Ninguna.

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- ninguno: todo se comprobó con tests (`test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea`, `test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin`, `test_el_inventario_coincide_con_cargo_toml`).
- Detectado solo por el humano: [lo completa la persona]

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- El diagnosticador en vivo separa un fallo real del shader de un cambio de scroll vertical entre líneas: coincide
  Resultado real: `test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea` pasó al 100 % comprobando que cuando `BG0VOFS` cambia entre líneas a mitad de cuadro, `dif_fin` resulta en 0, `dif_linea` mayor que 0, `diag.causa` se identifica como `"estado_por_linea"` (y no `"render_fin_de_cuadro"`) y `diag.registros` lista `"BG0VOFS"`.
  Método: el anunciado (`cargo test --test arnes_experimentos_tests test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea`).
- El filtro de firmas agrupa transiciones graduales de línea sin omitir fallos de fin de cuadro: coincide
  Resultado real: `test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin` pasó al 100 % comprobando que las cajas que crecen gradualmente dentro de un mismo bloque de 32 px de una transición HBlank se deduplican evitando capturas redundantes, mientras que cualquier desajuste real de `sw_fin` dispara la captura de inmediato.
  Método: el anunciado (`cargo test --test arnes_experimentos_tests test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin`).
- El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos: coincide
  Resultado real: `cargo test` ejecutó los 285 tests del repositorio en verde (184 unitarios en `src/` y 101 de integración en `tests/`), `cargo build --release --features proto --bin ppu_doble` compiló limpiamente en 31.8s, `validar_docs.py` arrojó 0 errores y `python scripts/extraer_referencias.py --check` finalizó con código 0.
  Método: el anunciado.

### Contexto: ¿se respetó?

- Restricción de guardado automático deduplicado, tecla `L` para alternar la referencia izquierda en vivo y separación real de `sw_fin` (GPU) frente a `sw_linea` (emulador por scanline) → respetada.
- Restricción de no alterar los 47 experimentos de `examples/` ni agregar dependencias no deseadas a Android → respetada: `required-features = ["proto"]` agregado exclusivamente al binario opcional `ppu_doble`.
- Prohibición de rutas locales absolutas y de versionar assets propietarios → respetada: cero rutas absolutas y cero archivos binarios propietarios añadidos.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: `Machine::new(rom)` toma posesión del vector de la ROM, requiriendo usar `&maquina.bus.rom` para el renderizado de `sw_fin` en el bucle principal → resuelto de forma limpia sin duplicar buffers.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `Cargo.toml`: agrega `required-features = ["proto"]` a `[[bin]] name = "ppu_doble"`.
- `examples/comun/diagnostico.rs`: implementa `render_software_fin`, `evaluar_cuadro_en_vivo` y cuantización a 32 px de las cajas de transición en línea.
- `src/bin/ppu_doble.rs`: conecta `sw_fin_rgb`, `capturar_linea`, alternancia de vista con `L` y volcado del informe diagnóstico causal en `.txt`.
- `tests/arnes_experimentos_tests.rs`: añade tests unitarios para la separación causal y el filtro de firmas.
- `docs/propuestas/2026-10-22_sincronizar-referencia-visor-y-temblor.md`: propuesta verificada con reporte de resultados.
- ¿Coincide con lo anunciado? sí.

### Fuera de alcance (hallazgos que no se arreglaron)

- Corrección de sincronización de scanline / multiplexado en el shader WGSL para efectos de scroll en línea (impacto medio): el visor ahora detecta y explica con precisión matemática en el `.txt` cada caso de vibración o cartel de unidad, pero la renderización por scanlines en la GPU nativa formará parte de una propuesta de renderizado posterior una vez clasificados los patrones observados.

### Para la próxima vez

- Utilizar directamente `&maquina.bus.rom` cuando se requiera el slice inmutable de la ROM tras instanciar la máquina.
