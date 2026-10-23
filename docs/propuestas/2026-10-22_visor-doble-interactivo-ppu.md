# Propuesta: Visor interactivo de doble pantalla para comparar PPU por software y PPU nativa en vivo

Fecha: 2026-10-22
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se añade la herramienta de escritorio `ppu_doble` (`cargo run --release --bin ppu_doble -- <ruta.gba> [ruta_bios.bin]`) que abre una ventana dividida lado a lado (`PPU por software | PPU nativa`), responde en vivo al teclado y compara los fotogramas en segundo plano guardando en `volcados/diferencias/` las capturas PNG compuestas y el diagnóstico causal cuando aparece una diferencia nueva o se pulsa `F5`.

### 2. Contexto

- Contexto consultado: `Cargo.toml`, `Cargo.lock`, `examples/comun/{mod,color,diagnostico,entrada,gpu,registro}.rs`, `examples/ppu_nativa_compara.rs`, `examples/ppu_barrido_nativa.rs`, `tests/arnes_experimentos_tests.rs`, `tests/packaging_tests.rs` y `scripts/extraer_referencias.py`.
- Intención: hoy para comprobar la PPU nativa hay que escribir secuencias de botones en `secuencias/*.txt` y esperar un barrido sin ver el juego en movimiento. El éxito es iniciar el juego en PC con una sola orden, jugar con el teclado viendo ambas PPU lado a lado a 60 FPS y que el arnés detecte, diagnostique y guarde en `volcados/diferencias/` cualquier desajuste visual mientras se juega.
- Restricciones y decisiones previas (máx. 3):
  - Una sola ventana dividida lado a lado (`Izquierda: PPU por software | Derecha: PPU nativa`) y guardado automático de PNG + diagnóstico ante diferencias nuevas (con filtro de firmas consecutivas) más tecla manual para forzar captura (origen: respuesta humana, 2026-10-22) → implica: componer un lienzo de `480x160` escalado por entero (ej. `3x` → `1440x480`) y enviar cada cuadro con diferencias (o forzado con `F5`) por un canal síncrono a un hilo trabajador que ejecuta `guardar_captura_si_difiere` y `diagnosticar` sin frenar los 60 FPS → protegida por «El despachador del visor filtra firmas repetidas y permite forzar captura manual» y «El lienzo doble coloca la PPU por software a la izquierda y la PPU nativa a la derecha» | testeable.
  - El inventario de `examples/` está fijado en exactamente 47 archivos por test y el APK de Android no debe incorporar dependencias de ventanas de escritorio (origen: `tests/arnes_experimentos_tests.rs` líneas 65 y 170; `AGENTS.md` §4.1) → implica: declarar la herramienta como binario `[[bin]] name = "ppu_doble"` en `src/bin/ppu_doble.rs` (fuera de `examples/` y fuera de la lista fija de `scripts/extraer_referencias.py`) y crear la ventana en Windows con `windows-sys` (ya presente en `Cargo.lock` v0.61.2) bajo `[target.'cfg(not(target_os = "android"))'.dependencies]` sin añadir `winit` ni tocar el árbol de Android → protegida por «El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos» | testeable.
  - Prohibición de versionar rutas locales absolutas o assets del juego (origen: `AGENTS.md` §3 y §7) → implica: recibir la ruta de la ROM por argumento de línea de comandos (con detección opcional de `roms/gba_bios.bin` si existe) y escribir las capturas e informes en `volcados/diferencias/`, cubierto por `.gitignore` → protegida por «El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Añadir `winit` o `softbuffer` a `Cargo.toml` → descartada porque descarga ~40 crates nuevas en `Cargo.lock` cuando `windows-sys` (`CreateWindowExW` + `StretchDIBits` GDI) ya está en `Cargo.lock` y resuelve la ventana y el teclado en ~120 líneas sin dependencias nuevas.
  - Ubicar el visor como `examples/ppu_doble.rs` → descartada porque `tests/arnes_experimentos_tests.rs` exige `assert_eq!(rutas.len(), 47)` y que todo archivo en `examples/` sea un experimento por lotes sin ventana.
- Ante imprevistos, prioriza: fluidez de juego a 60 FPS sin tirones al guardar capturas > exactitud del diagnóstico en segundo plano > atajos de teclado adicionales.

### 3. Ejemplos

- Caso normal (jugando en vivo cuando aparece una diferencia): se ejecuta `cargo run --release --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba"`, se juega con las flechas y `Z`/`X` (botones `A`/`B`) y en el cuadro `900` la PPU nativa difiere de la PPU por software → antes: no existía forma de jugar viendo ambas PPU ni de capturar fallos al vuelo → ahora: la ventana muestra a la izquierda la PPU por software y a la derecha la PPU nativa; el título actualiza `Cuadro 900 | dif fin: 485 px | capturas: 1`, y el hilo en segundo plano guarda `volcados/diferencias/cuadro-900-fin.png`, `volcados/diferencias/cuadro-900-fin.txt` e imprime en consola la causa raíz sin detener la partida.
- Caso límite (diferencia estática durante varios segundos o captura manual con `F5`): una pantalla de menú mantiene exactamente la misma diferencia durante 180 cuadros seguidos y luego la persona pulsa `F5` en un cuadro con `0` diferencias → antes: habría llenado el disco con 180 PNG idénticos y no permitiría capturar un cuadro sin error numérico → ahora: el filtro de firmas guarda sólo el primer cuadro de la racha y omite los 179 repetidos; al pulsar `F5`, fuerza el volcado de `volcados/diferencias/cuadro-<N>-manual.png` aunque `dif == 0`.

### 4. Pasos

1. Añadir en `examples/comun/diagnostico.rs` dos funciones puras reutilizables por el visor y probadas por tests:
   - `lienzo_doble_bgrx(sw_rgb: &[u8], nat_rgb: &[u8], resaltar_dif: bool) -> Vec<u32>` que arma el fotograma de `480x160` (`240x160` software a la izquierda + `240x160` nativa a la derecha, con opción de tintar en magenta los píxeles donde difieren al pulsar `Tab`).
   - ` guardar_captura_forzada(dir, cuadro, etiqueta, sw_rgb, nat_rgb)` y el selector `DecisionCaptura` que combina `FiltroFirmas` con el flag `forzar_manual` para decidir si un cuadro jugado debe enviarse al hilo de diagnóstico.
2. Declarar en `Cargo.toml` la dependencia `windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_UI_Input_KeyboardAndMouse", "Win32_Graphics_Gdi", "Win32_System_LibraryLoader"] }` bajo `[target.'cfg(windows)'.dependencies]` y el binario `[[bin]] name = "ppu_doble" path = "src/bin/ppu_doble.rs"`.
3. Crear `src/bin/ppu_doble.rs` con:
   - Ventana Win32 de `1440x480` (escala `3x` de `480x160`, redimensionable manteniendo aspecto) y mapa de teclado GBA (`Flechas` = D-Pad, `Z` = A, `X` = B, `Enter` = Start, `Backspace` = Select, `A` = L, `S` = R) más controles del arnés (`F5` = forzar captura PNG + diagnóstico, `Tab` = resaltar diferencias en vivo sobre el panel derecho, `Espacio` = pausar/reanudar, `N` = avanzar 1 cuadro en pausa, `Shift` = turbo).
   - Bucle principal a 60 FPS que avanza `gba_core::Machine::step_frame()`, obtiene `sw_fin` (y `sw_linea` si se compila con `--features proto`) y `nativa` (`Gpu::nueva_nativa()`), presenta el `lienzo_doble_bgrx` en la ventana y actualiza la barra de título con FPS, cuadro actual, píxeles distintos (`fin` y `linea`) y total de capturas.
   - Hilo trabajador en segundo plano conectado por `mpsc::sync_channel` que recibe los cuadros con diferencias nuevas (o forzados con `F5`), ejecuta `guardar_captura_si_difiere` / `guardar_captura_forzada` y `diagnosticar` (usando su propia instancia de `Gpu::nueva_nativa()` para las variantes de ablación), escribe `volcados/diferencias/cuadro-<N>-<caso>.png` y `.txt`, e imprime el veredicto en consola.
4. Añadir los tests en `tests/arnes_experimentos_tests.rs` para el compositor de doble pantalla (`480x160`) y la lógica de decisión de captura en vivo (automática deduplicada + manual forzada), y comprobar toda la suite y el checklist de Pre-Commit.

### 5. Qué más podría verse afectado

- `Cargo.toml` — añade `[target.'cfg(windows)'.dependencies] windows-sys` (ya resuelto en `Cargo.lock` v0.61.2) y `[[bin]] name = "ppu_doble"`.
- `examples/comun/diagnostico.rs` — añade `ANCHO_DOBLE` (`480`), `lienzo_doble_bgrx`, `guardar_captura_forzada` y `decidir_captura_en_vivo`.
- `tests/arnes_experimentos_tests.rs` — añade los tests de los nuevos escenarios y verifica que los 47 ejemplos siguen intactos.
- `scripts/extraer_referencias.py` — escanea una lista cerrada `archivos_ordenados` en `src/` (`lib.rs`, `input.rs`, etc.), por lo que `src/bin/ppu_doble.rs` no altera `docs/referencia/api-modulos-rust.md`.
- Búsqueda hecha: revisión de `tests/arnes_experimentos_tests.rs`, `tests/packaging_tests.rs` y `scripts/extraer_referencias.py`.

### 6. Suposiciones

- Dos instancias de `comun::gpu::Gpu::nueva_nativa()` pueden coexistir en el mismo proceso (una en el hilo principal para dibujar a 60 FPS y otra en el hilo trabajador para correr las 9 pasadas de ablación cuando hay diferencias sin bloquear la ventana) → prototipar: sí
  → si es falso: el hilo principal calcula también las variantes de ablación sólo en el cuadro donde cambia la firma (tarda ~15 ms una sola vez por escena distinta) y envía al hilo trabajador únicamente los búferes RGB ya renderizados para codificar el PNG y escribir el informe en disco.
  → resultado del prototipo: [se completa tras la aprobación] | no detecta: contención de driver en GPUs integradas antiguas.
- `windows-sys = "0.61"` con las features `Win32_UI_WindowsAndMessaging` y `Win32_Graphics_Gdi` compila directamente con el `Cargo.lock` actual sin acceso a red → prototipar: sí
  → si es falso: usamos `raw-window-handle` con carga dinámica `extern "system"` de `user32.dll` y `gdi32.dll` (que siempre están en Windows) sin tocar `[dependencies]` en `Cargo.toml`.
  → resultado del prototipo: [se completa tras la aprobación] | no detecta: ejecución en Linux o macOS (donde se compila un stub informativo con `#[cfg(not(windows))]`).

### 7. Alcance

- Incluye:
  - Binario interactivo `src/bin/ppu_doble.rs` con ventana dividida lado a lado (`PPU por software | PPU nativa`), entrada de teclado GBA en vivo y atajos de inspección (`F5`, `Tab`, `Espacio`, `N`, `Shift`).
  - Conexión en segundo plano con `examples/comun/diagnostico.rs` para guardar automáticamente en `volcados/diferencias/` el PNG compuesto `960x160` y el informe `.txt` con la causa raíz ante cada diferencia nueva, más captura manual con `F5`.
  - Pruebas unitarias en `tests/arnes_experimentos_tests.rs`.
- No incluye:
  - Audio en vivo ni soporte de mandos Bluetooth en el visor de PC (sólo teclado).
  - Modificar el shader `src/ppu_nativa.wgsl` ni el empaquetado de Android (`scripts/package_apk.ps1`).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El lienzo doble coloca la PPU por software a la izquierda y la PPU nativa a la derecha
Caso de uso: al jugar en la ventana dividida, cada mitad muestra su PPU correspondiente y permite resaltar diferencias | Tipo: normal
  Dado que un cuadro de PPU por software con el píxel `(5, 10)` en rojo `(255, 0, 0)` y un cuadro de PPU nativa con el píxel `(5, 10)` en azul `(0, 0, 255)` y el píxel `(20, 30)` idéntico en verde `(0, 255, 0)` en ambos
  Cuando se construye el búfer de ventana con `lienzo_doble_bgrx` (`480x160`) sin resaltado y con resaltado de diferencias
  Entonces sin resaltado el píxel `(5, 10)` (mitad izquierda) vale `0x00FF0000` (rojo) y el píxel `(240 + 5, 10)` (mitad derecha) vale `0x000000FF` (azul); con resaltado activo el píxel distinto `(240 + 5, 10)` se tinta en magenta `0x00FF00FF` y el píxel idéntico `(240 + 20, 30)` conserva `0x0000FF00` (verde)
Test: `tests/arnes_experimentos_tests.rs::test_lienzo_doble_coloca_software_izquierda_y_nativa_derecha`
Estado previo: se completa al escribir el test: falla (la función `lienzo_doble_bgrx` aún no existe)

Escenario: El despachador del visor filtra firmas repetidas y permite forzar captura manual
Caso de uso: jugando en vivo se guardan diferencias nuevas automáticamente sin saturar el disco y se puede capturar cualquier cuadro con `F5` | Tipo: límite
  Dado que una secuencia en vivo de 4 cuadros: el cuadro 1 tiene `0` diferencias y `forzar = false`; los cuadros 2 y 3 tienen la misma diferencia (`dif = 120`, misma firma) con `forzar = false`; y el cuadro 4 tiene `0` diferencias pero `forzar = true` (`F5`)
  Cuando se evalúa cada cuadro y se ejecuta el guardado en una carpeta temporal
  Entonces el cuadro 1 no guarda nada, el cuadro 2 guarda `cuadro-2-fin.png`, el cuadro 3 se omite por firma repetida, y el cuadro 4 guarda `cuadro-4-manual.png` (`960x160` válido) aunque tenga `0` diferencias
Test: `tests/arnes_experimentos_tests.rs::test_visor_filtra_firmas_en_vivo_y_permite_captura_forzada`
Estado previo: se completa al escribir el test: falla (la función de decisión y captura forzada aún no existe)

Escenario: El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos
Caso de uso: añadir el binario `ppu_doble` no rompe los tests estructurales del repositorio ni contamina el crate de Android | Tipo: regresión
  Dado que `Cargo.toml` con el nuevo binario `ppu_doble`, los 47 experimentos de `examples/` y `docs/referencia/api-modulos-rust.md`
  Cuando se ejecutan `cargo test --test arnes_experimentos_tests`, `cargo build --release --bin ppu_doble` y `python scripts/extraer_referencias.py --check`
  Entonces los tests pasan al 100 %, `ppu_doble` compila sin errores ni advertencias y `extraer_referencias.py --check` devuelve código `0`
Test: `tests/arnes_experimentos_tests.rs::test_el_inventario_coincide_con_cargo_toml`
Estado previo: se completa al escribir el test: pasa (regresión)

Escenario: La ventana dividida muestra ambas PPU en vivo y responde al mismo teclado (visible)
Caso de uso: iniciar el juego en PC y comprobar visualmente ambas PPU lado a lado con los mismos controles | Tipo: normal (visible)
  Dado que la ROM local en `roms/1575 - Fire Emblem (E)(Independent).gba`
  Cuando la persona ejecuta `cargo run --release --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba"` y pulsa teclas de dirección o `Enter`/`Z`
  Entonces se abre la ventana dividida con `PPU por software` a la izquierda y `PPU nativa` a la derecha respondiendo al mismo tiempo a las teclas, y en `volcados/diferencias/` aparecen los PNG compuestos y `.txt` de los cuadros donde difieren
Test: revisión humana (ejecución interactiva en escritorio)
Estado previo: pendiente de revisión humana

### 9. Cómo lo comprobaré

- El lienzo doble coloca la PPU por software a la izquierda y la PPU nativa a la derecha → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_lienzo_doble_coloca_software_izquierda_y_nativa_derecha`
  → pasa si: `1 passed`
  → no detecta: que la ventana del sistema operativo se vea fluida en el monitor de la persona.
- El despachador del visor filtra firmas repetidas y permite forzar captura manual → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_visor_filtra_firmas_en_vivo_y_permite_captura_forzada`
  → pasa si: `1 passed`
  → no detecta: tirones de disco si el usuario pulsa `F5` decenas de veces por segundo.
- El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos → herramienta o comando:
  `cargo test --test arnes_experimentos_tests && cargo build --release --bin ppu_doble && python scripts/extraer_referencias.py --check`
  → pasa si: todos terminan con código `0`
  → no detecta: errores en tiempo de ejecución al pulsar teclas dentro de la ventana Win32.
- La ventana dividida muestra ambas PPU en vivo y responde al mismo teclado (visible) → revisión humana:
  ejecutar `cargo run --release --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba"` y comprobar que las dos mitades avanzan con el mismo input y guardan las diferencias en `volcados/diferencias/`
  → no detecta: fallos en escenas a las que no se llegue durante la partida de prueba.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Se verificó en `tests/arnes_experimentos_tests.rs` (líneas 65 y 170) que añadir un archivo en `examples/` rompería el invariante de `47` experimentos; por eso la herramienta se ubica en `src/bin/ppu_doble.rs` como `[[bin]]` y no en `examples/`.
- Se verificó en `Cargo.lock` (línea 1860) que `windows-sys 0.61.2` ya está presente en el proyecto, evitando descargar `winit` o `softbuffer`.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- El lienzo doble coloca la PPU por software a la izquierda y la PPU nativa a la derecha: coincide
  Resultado real: `test_lienzo_doble_coloca_software_izquierda_y_nativa_derecha` pasó al 100 % comprobando que la mitad izquierda tiene el píxel rojo `0x00FF0000`, la mitad derecha el píxel azul `0x000000FF`, y que al activar resaltado el píxel discordante se tiñe de magenta `0x00FF00FF`.
  Método: el anunciado (`cargo test --test arnes_experimentos_tests test_lienzo_doble_coloca_software_izquierda_y_nativa_derecha`).
- El despachador del visor filtra firmas repetidas y permite forzar captura manual: coincide
  Resultado real: `test_visor_filtra_firmas_en_vivo_y_permite_captura_forzada` pasó al 100 % comprobando que un cuadro limpio no captura, un cuadro con diferencia captura `cuadro-2-fin.png`, un cuadro consecutivo idéntico se omite por firma repetida, y pulsar F5 genera `cuadro-4-manual.png` incondicionalmente.
  Método: el anunciado (`cargo test --test arnes_experimentos_tests test_visor_filtra_firmas_en_vivo_y_permite_captura_forzada`).
- El inventario de los 47 experimentos y la referencia de módulos se mantienen intactos: coincide
  Resultado real: `test_el_inventario_coincide_con_cargo_toml` y `test_los_experimentos_usan_el_modulo_comun` pasaron al 100 % (47 experimentos), `ppu_doble` compiló en `--release` sin errores ni advertencias, y `python scripts/extraer_referencias.py --check` devolvió 0.
  Método: el anunciado.
- La ventana dividida muestra ambas PPU en vivo y responde al mismo teclado (visible): pendiente de revisión humana
  Resultado real: binario compilado y listo para ser probado de forma interactiva en PC con `cargo run --release --bin ppu_doble -- <ruta.gba>`.
  Método: revisión humana en entorno de escritorio.

### Contexto: ¿se respetó?

- Restricción de una sola ventana dividida lado a lado (`Izquierda: PPU por software | Derecha: PPU nativa`) con guardado automático deduplicado y manual por tecla → respetada.
- Restricción de no alterar el inventario de 47 experimentos en `examples/` ni contaminar dependencias de Android → respetada: `ppu_doble` se declaró en `[[bin]]` y `windows-sys` bajo `[target.'cfg(windows)'.dependencies]`.
- Prohibición de rutas locales absolutas y de versionar assets propietarios → respetada: cero rutas locales ni binarios propietarios versionados.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: tipos de punteros nulos de `windows-sys` 0.61.2 (`HBRUSH`, `*mut c_void`) y método de avance de frame en `Machine` (`run_frame(5_000_000)` en lugar de `step_frame()`) → corregidos inmediatamente durante la edición.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `Cargo.toml`: añade dependencia condicional `windows-sys` y binario `ppu_doble`.
- `examples/comun/diagnostico.rs`: añade `ANCHO_DOBLE`, `lienzo_doble_bgrx`, `guardar_captura_forzada`, `decidir_captura_en_vivo` y visibilidad pública de `ANCHO`/`ALTO`.
- `tests/arnes_experimentos_tests.rs`: añade tests unitarios para lienzo doble y decisión de captura interactiva.
- `src/bin/ppu_doble.rs`: archivo nuevo con la herramienta interactiva de escritorio.
- ¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Consultar la firma exacta del método de frame en `Machine` (`run_frame`) antes de redactar el boceto del bucle principal.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

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

- La ventana dividida muestra ambas PPU en vivo y responde al mismo teclado (visible): ✔ | ✘ | ?  [nota de la persona, opcional]
- Detectado solo por el humano: [lo completa la persona]
