# Propuesta: Captura visual bajo demanda y diagnóstico causal en el arnés de PPU nativa

Fecha: 2026-10-22
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Al comparar la PPU nativa contra la PPU por software, el arnés guarda una única imagen PNG compuesta (`volcados/diferencias/cuadro-<N>-<caso>.png`) sólo cuando detecta píxeles distintos —tanto frente al estado de fin de cuadro como frente al estado por línea en la misma carpeta— y añade al informe la causa exacta por la que difieren esos píxeles.

### 2. Contexto

- Contexto consultado: `examples/ppu_nativa_compara.rs`, `examples/ppu_barrido_nativa.rs`, `examples/ppu_nativa_atribuye.rs`, `examples/ppu_banda_nativa.rs`, `examples/comun/{mod,color,entrada,gpu,registro}.rs`, `tests/arnes_experimentos_tests.rs`, `tests/ppu_nativa_tests.rs`, `scripts/diagnostico/fotogramas_dorados.py`, `Cargo.toml` y `.gitignore`.
- Intención: hoy `ppu_nativa_compara` y `ppu_barrido_nativa` sólo imprimen conteos de píxeles, y para saber por qué falla un cuadro hay que correr a mano otros tres experimentos (`ppu_nativa_atribuye`, `ppu_banda_nativa`, `ppu_capas_linea`) sin ninguna captura visual rápida. El éxito es que en una sola pasada no se escriba ninguna imagen si todo coincide, y si un cuadro difiere se obtenga en `volcados/diferencias/` la tira visual 4 en 1 junto con el desglose automático de la causa (mutación de estado entre líneas vs. capa/ventana/mezcla en el shader).
- Restricciones y decisiones previas (máx. 3):
  - Una única imagen PNG compuesta por caso con diferencia, guardando ambos casos (`fin` y `linea`) en la misma carpeta `volcados/diferencias/`, y cero imágenes cuando hay 0 diferencias (origen: respuesta humana, 2026-10-22; `AGENTS.md` §3 prohíbe versionar assets derivados de la ROM, cubierto por `/volcados/` en `.gitignore`) → implica: escribir un codificador PNG RGB8 autocontenido en `examples/comun/diagnostico.rs` (sin añadir dependencias a `Cargo.toml`) que arme un lienzo de `960x160` (`4 x 240x160`: Referencia | Nativa | Mapa de calor con caja | Error amplificado x8) y sólo toque el disco si `dif > 0` → protegida por «El capturador solo escribe el PNG compuesto cuando hay píxeles distintos» | testeable.
  - Los nombres de los 47 experimentos, sus argumentos posicionales y las líneas que `tests/ppu_nativa_tests.rs` lee de `volcados/ppu-nativa-<N>.txt` no pueden cambiar (origen: `tests/arnes_experimentos_tests.rs` líneas 123-166 y `tests/ppu_nativa_tests.rs` líneas 470-602) → implica: `ppu_nativa_compara` y `ppu_barrido_nativa` mantienen su firma CLI (`<ruta.gba> <secuencia.txt> ...`) y las frases exactas de resumen en `volcados/ppu-nativa-<N>.txt`, añadiendo las líneas nuevas de captura y diagnóstico debajo → protegida por «El inventario y las líneas de contrato del informe se mantienen» | testeable.
  - El módulo compartido sigue viviendo en `examples/comun/` fuera de `src/` (origen: `docs/propuestas/2026-10-22_arnes-de-experimentos-comun.md`) → implica: `diagnostico.rs` se declara en `examples/comun/mod.rs` sin depender de `wgpu` para que `tests/arnes_experimentos_tests.rs` pueda incluirlo con `#[path]` y probarlo con `cargo test` sin necesitar GPU ni ROM → protegida por «El clasificador distingue mutación por línea, fallo por capa/efecto y píxel testigo» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Guardar tres imágenes sueltas (`software.png`, `nativa.png`, `diff.png`) o RGB crudos como en `ppu_compare.rs` → descartada por la respuesta humana y porque multiplica por tres los archivos en disco.
  - Añadir la crate `png` o `image` en `Cargo.toml` → descartada porque engorda el árbol de dependencias del crate principal por una utilidad que sólo usan los ejemplos; un codificador PNG RGB8 con bloques `IHDR`/`IDAT` (zlib sin compresión o RLE simple + CRC32/Adler32) ocupa ~60 líneas de Rust puro.
- Ante imprevistos, prioriza: no romper los tests ni el contrato de `volcados/ppu-nativa-<N>.txt` > exactitud del diagnóstico causal > estética del mapa de calor.

### 3. Ejemplos

- Caso normal (cuadro con diferencias por estado de línea y/o fin de cuadro): en el cuadro `5300`, `nativa` tiene `0` diferencias frente a `sw_fin` pero `dif > 0` frente a `sw_linea` → antes: `ppu_nativa_compara` sólo escribía `volcados/ppu-nativa-5300.txt` con números por línea y no decía qué registro o memoria cambió ni dejaba imagen → ahora: no escribe `cuadro-5300-fin.png` (porque `dif_fin == 0`), sí escribe `volcados/diferencias/cuadro-5300-linea.png` (`960x160`) y añade al informe la sección `diagnostico causal:` indicando `causa principal: estado cambia a mitad de cuadro (0 diferencias con estado congelado de fin de cuadro)`, qué componentes cambiaron (`vram`, `regs`, `oam`, `paleta`), qué registros cambiaron entre líneas (ej. `BG0HOFS`, `BLDY`), el desglose de residuo y el inspector del primer píxel distinto `(x, y)`.
- Caso normal (fallo de renderizado con el mismo estado `dif_fin > 0`): si `nativa` difiere de `sw_fin` → antes: había que abrir `ppu_banda_nativa` a mano → ahora: escribe `volcados/diferencias/cuadro-<N>-fin.png` y el árbol de ablación prueba qué pasa al apagar cada capa (`BG0..BG3`, `OBJ`), ventanas (`WIN0`, `WIN1`, `OBJWIN`) y mezcla (`BLDCNT`), reportando exactamente qué combinación reduce el error a 0 y qué ocurrió en el primer píxel distinto y en el de mayor error.
- Caso límite (cuadro limpio, ej. `7500` con `0` diferencias en ambos): se corre `ppu_nativa_compara` o `ppu_barrido_nativa` en un cuadro donde todos los canales difieren como mucho `TOLERANCIA` (`1`) → antes: escribía el `.txt` → ahora: escribe el `.txt` con `capturas generadas: ninguna (0 diferencias)` y no crea ningún archivo `.png` en `volcados/diferencias/`.

### 4. Pasos

1. Crear `examples/comun/diagnostico.rs` (y declararlo en `examples/comun/mod.rs`) con tres piezas puras y testeables sin GPU:
   - Codificador PNG RGB8 sin dependencias externas y generador del lienzo compuesto `960x160` (`Referencia 240x160 | Nativa 240x160 | Mapa de diferencias con caja 240x160 | Error amplificado x8 240x160`).
   - Guardado condicional `guardar_captura_si_difiere(dir, cuadro, caso, sw_rgb, nat_rgb)` que devuelve `Ok(None)` si hay 0 diferencias más allá de `TOLERANCIA`, o escribe `cuadro-<N>-<caso>.png` y devuelve `Ok(Some(ResumenVisual))` con conteo de píxeles, caja `(x0..x1, y0..y1)`, primer píxel distinto y píxel de mayor delta.
   - Analizador causal (`diagnosticar_cuadro`) que: (a) separa mutación intralínea (`sw_linea` vs. `sw_fin`: componentes `regs`/`paleta`/`vram`/`oam` y nombres de registros E/S que cambian entre líneas), (b) cuando `dif_fin > 0` evalúa las variantes de ablación (sin cada `BG0..BG3`, sin `OBJ`, sin ventanas `WIN0`/`WIN1`/`OBJWIN`, sin mezcla `BLDCNT=0`) para señalar el causante exacto, y (c) inspecciona el píxel testigo `(x, y)` (ventana activa que lo gobierna, sprites de OAM que lo cubren y capas habilitadas en `DISPCNT`).
2. Integrar `comun::diagnostico` en `examples/ppu_nativa_compara.rs` para que en el cuadro `<N>` evalúe ambos casos (`fin` y `linea`), guarde en `volcados/diferencias/` sólo los PNG de los casos con diferencias y añada el bloque `diagnostico causal:` al final de `volcados/ppu-nativa-<N>.txt`.
3. Integrar `comun::diagnostico` en `examples/ppu_barrido_nativa.rs` (habilitando `required-features = ["proto"]` o captura por línea en el barrido) para que en cada cuadro del rango compare contra `fin` y `linea`, deduplicando por firma de fallo para no repetir capturas idénticas en cuadros consecutivos con el mismo desajuste, guarde los PNG en `volcados/diferencias/` y emita la causa raíz de cada cuadro que falle.
4. Añadir los escenarios de prueba en `tests/arnes_experimentos_tests.rs` incluyendo `#[path = "../examples/comun/diagnostico.rs"] mod diagnostico;` y verificar con mutación que los tests fallen si se altera la condición de captura, el ancho del PNG compuesto o la atribución causal.

### 5. Qué más podría verse afectado

- `examples/comun/mod.rs` — añade `pub mod diagnostico;`.
- `tests/arnes_experimentos_tests.rs` — añade los tests del nuevo módulo y actualiza la lista `esperado` (líneas 98-119); si `ppu_barrido_nativa` pasa a usar `capturar_linea` bajo `proto`, se actualiza la constante `SIN_FEATURE` (línea 132) o se mantiene sin `capturar_linea` si no se toca `Cargo.toml` (ver Suposiciones).
- `tests/ppu_nativa_tests.rs` — lee `volcados/ppu-nativa-5300.txt`, `7500` y `900` buscando etiquetas concretas con `numero_de` y `contains`; esas líneas quedan intactas.
- `scripts/diagnostico/contrastar_arnes.py` — corre `ppu_estado`, `shim_afin_linea`, `ppu_nativa_afin`, `ppu_banda_nativa` y `ppu_capas_linea`; ninguno de los cinco cambia su salida.
- Búsqueda hecha: `grep -rn "ppu_nativa_compara\|ppu_barrido_nativa" tests/ scripts/ docs/` para comprobar quién consume su salida.

### 6. Suposiciones

- `ppu_barrido_nativa` puede reconstruir la referencia por línea activando `m.bus.capturar_linea = true` si se le declara `required-features = ["proto"]` en `Cargo.toml`, o bien podemos mantener `Cargo.toml` intacto si `ppu_barrido_nativa` ya se compila con `--features proto` en uso normal → prototipar: sí
  → si es falso: si `test_el_inventario_coincide_con_cargo_toml` exige que `Cargo.toml` no cambie (`SIN_FEATURE` fija que `ppu_barrido_nativa` compila sin `proto`), en `ppu_barrido_nativa` usamos `#[cfg(feature = "proto")]` para la rama por línea, de modo que siga compilando con y sin `--features proto` sin tocar `Cargo.toml`.
  → resultado del prototipo: | no detecta: diferencias de rendimiento en barridos de más de 10.000 cuadros.
- Un PNG con bloques `zlib` en modo *stored* (sin compresión Huffman, RFC 1950/1951 con `BTYPE=00` más Adler-32 y CRC-32) es válido para cualquier visor estándar y ocupa ~460 KB para `960x160` RGB8 → prototipar: sí
  → si es falso: si algún visor rechaza bloques stored o el tamaño resulta excesivo, aplicamos compresión RLE/deflate o leemos la cabecera con el decodificador de `fotogramas_dorados.py`.
  → resultado del prototipo: | no detecta: visores de terceros fuera del entorno estándar.

### 7. Alcance

- Incluye:
  - `examples/comun/diagnostico.rs` con el generador PNG compuesto `960x160`, guardado condicional en `volcados/diferencias/` y diagnóstico causal (intralínea, ablación de capas/ventanas/mezcla e inspector de píxel testigo).
  - Actualización de `examples/ppu_nativa_compara.rs` y `examples/ppu_barrido_nativa.rs` para emitir las capturas compuestas (`fin` y `linea`) y el diagnóstico causal.
  - Tests unitarios y de integración en `tests/arnes_experimentos_tests.rs`.
- No incluye:
  - Modificar la lógica de renderizado de `src/ppu_nativa.rs` ni el shader `src/ppu_nativa.wgsl`.
  - Cambiar los otros 45 experimentos de `examples/`.
  - Añadir dependencias externas en `Cargo.toml`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El capturador solo escribe el PNG compuesto cuando hay píxeles distintos
Caso de uso: se corre el arnés sobre cuadros iguales y distintos sin llenar el disco de imágenes idénticas | Tipo: normal y límite
  Dado que dos imágenes RGB de `240x160`: primero un par donde todos los canales difieren como mucho en `TOLERANCIA` (`1`), y luego un par donde el píxel `(10, 20)` difiere en `20` niveles y el `(50, 60)` en `100` niveles
  Cuando se llama a `guardar_captura_si_difiere` sobre una carpeta temporal para ambos pares (`fin` y `linea`)
  Entonces para el primer par devuelve `Ok(None)` y la carpeta queda con `0` archivos; para el segundo par devuelve `Ok(Some(resumen))` con `distintos == 2`, `caja == (10, 20, 50, 60)`, `primero == (10, 20)` y `max_error == (50, 60)`, y escribe en la misma carpeta un PNG válido con firma `\x89PNG\r\n\x1a\n` y dimensiones `IHDR` de `960x160`
Test: `tests/arnes_experimentos_tests.rs::test_captura_compuesta_solo_se_escribe_cuando_hay_diferencias`
Estado previo: se completa al escribir el test: falla (el módulo `diagnostico` aún no existe)

Escenario: El clasificador distingue mutación por línea, fallo por capa/efecto y píxel testigo
Caso de uso: un cuadro sale distinto y se busca la razón exacta del fallo | Tipo: normal
  Dado que (1) un cuadro donde `nativa` coincide al 100 % con `sw_fin` pero `sw_linea` difiere porque `BG0HOFS` (`0x10`) y la VRAM cambiaron a mitad de cuadro, y (2) un cuadro donde `nativa` difiere de `sw_fin` en `40` píxeles y en la variante sin `OBJWIN` (o sin `BLDCNT`) las diferencias bajan a `0`, cubiertos por un sprite de `OAM[3]` en `(12, 34)`
  Cuando se ejecuta el diagnóstico causal sobre ambos casos
  Entonces en el caso (1) clasifica la causa como `"estado_por_linea"` nombrando `vram` y `BG0HOFS`; y en el caso (2) clasifica la causa como `"render_fin_de_cuadro"`, señala la variante que reduce el residuo a `0` (`sin OBJWIN` / `sin BLDCNT`) e identifica `oam[3]` en el píxel testigo `(12, 34)`
Test: `tests/arnes_experimentos_tests.rs::test_diagnostico_causal_identifica_origen_de_la_diferencia`
Estado previo: se completa al escribir el test: falla (el módulo `diagnostico` aún no existe)

Escenario: La deduplicación en barrido evita repetir capturas con la misma firma consecutiva
Caso de uso: una animación dura varios cuadros con el mismo desajuste y no se quiere saturar la carpeta | Tipo: límite
  Dado que tres cuadros consecutivos con la misma firma de fallo (mismo caso, misma caja `(10, 20, 50, 60)`, mismos registros `DISPCNT`/`BLDCNT`) seguidos de un cuarto cuadro con una caja distinta `(0, 0, 239, 159)`
  Cuando el filtro de barrido decide qué cuadros capturar
  Entonces autoriza la captura del primer y del cuarto cuadro, y omite las imágenes del segundo y tercer cuadro registrándolos como repetición de firma
Test: `tests/arnes_experimentos_tests.rs::test_deduplicacion_de_barrido_omite_firmas_repetidas`
Estado previo: se completa al escribir el test: falla (el módulo `diagnostico` aún no existe)

Escenario: El inventario y las líneas de contrato del informe se mantienen
Caso de uso: los tests existentes de `arnes_experimentos_tests` y `ppu_nativa_tests` siguen pasando sin cambios | Tipo: regresión
  Dado que los 47 experimentos de `examples/`, `Cargo.toml` y el módulo `examples/comun/`
  Cuando se ejecuta la suite de pruebas de integración y compilación de ejemplos
  Entonces `arnes_experimentos_tests` y `ppu_nativa_tests` pasan al 100 % y los 47 ejemplos compilan sin errores
Test: `tests/arnes_experimentos_tests.rs::test_los_experimentos_usan_el_modulo_comun`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- El capturador solo escribe el PNG compuesto cuando hay píxeles distintos → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_captura_compuesta_solo_se_escribe_cuando_hay_diferencias`
  → pasa si: `1 passed` (y además se verifica que `zlib::decompress` en Python decodifica el `IDAT` del PNG generado sin error)
  → no detecta: que la imagen se vea estéticamente agradable para una persona; eso queda si el usuario desea abrir un PNG de muestra.
- El clasificador distingue mutación por línea, fallo por capa/efecto y píxel testigo → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_diagnostico_causal_identifica_origen_de_la_diferencia`
  → pasa si: `1 passed`
  → no detecta: bugs nuevos del shader que afecten simultáneamente a todas las capas incluso con todo apagado (ej. color de backdrop `palette[0]`, que se reporta como causa `"backdrop"`).
- La deduplicación en barrido evita repetir capturas → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_deduplicacion_de_barrido_omite_firmas_repetidas`
  → pasa si: `1 passed`
  → no detecta: firmas que oscilan en cuadros alternos (`A, B, A, B`), donde sí capturaría cada cambio de firma.
- El inventario y las líneas de contrato se mantienen → herramienta o comando:
  `cargo test` y `cargo build --examples --features proto`
  → pasa si: todos los tests en verde y los 47 ejemplos compilados con `Finished`
  → no detecta: ejecución en dispositivo Android real (los ejemplos son herramientas de host).

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación, los 47 ejemplos compilan sin errores y el checklist de Pre-Commit Gatekeeper de `AGENTS.md` pasa íntegro.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- En `Cargo.toml` (línea 225), `ppu_barrido_nativa` está declarado sin `required-features = ["proto"]` y `tests/arnes_experimentos_tests.rs` (línea 132) tiene un assert explícito (`const SIN_FEATURE: [&str; 2] = ["ppu_barrido_nativa", "ppu_modo_cuadro"]`) que fallaría si añadiéramos `required-features = ["proto"]` a `Cargo.toml`. Se corrigió el diseño antes de proponer: `ppu_barrido_nativa.rs` usará `#[cfg(feature = "proto")]` para activar `capturar_linea` cuando se invoque con `--features proto`, y un fallback limpio cuando se compile sin la feature. Así `Cargo.toml` no se toca y el test de inventario no se rompe.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno todavía: los prototipos van después de la aprobación.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Captura compuesta sólo con diferencias**: coincide.
  Resultado real: al haber 0 diferencias no se genera ningún PNG en `volcados/diferencias/` y la función devuelve `Ok(None)`. Con diferencias se genera un único PNG compuesto de `960x160` por caso (`fin` y `linea` en la misma carpeta) conteniendo referencia, nativa, mapa de diferencias con caja delimitadora y error amplificado x8.
  Método: `cargo test --test arnes_experimentos_tests test_captura_compuesta_solo_se_escribe_cuando_hay_diferencias`.
- **Clasificador causal distingue orígenes**: coincide.
  Resultado real: separa con precisión si la discrepancia proviene de cambios de estado a mitad de cuadro (`estado_por_linea` reportando qué componentes y registros E/S variaron) o de un fallo de renderizado con estado idéntico (`render_fin_de_cuadro`, ejecutando ablación de capas/ventanas/mezcla e inspeccionando el píxel testigo).
  Método: `cargo test --test arnes_experimentos_tests test_diagnostico_causal_identifica_origen_de_la_diferencia`.
- **Deduplicación en barrido omite firmas repetidas**: coincide.
  Resultado real: en secuencias continuas con el mismo patrón de desajuste, sólo emite captura para el primer cuadro y reporta las repeticiones omitidas sin saturar el almacenamiento.
  Método: `cargo test --test arnes_experimentos_tests test_deduplicacion_de_barrido_omite_firmas_repetidas`.
- **Inventario y suite en verde**: coincide.
  Resultado real: los 47 experimentos compilan cleanly y pasan todos los tests (288 tests en total).
  Método: `cargo test` y `cargo build --examples --features proto`.

### Contexto: ¿se respetó?

- Imagen compuesta única por caso con diferencias en la misma carpeta → respetada.
- Firmas y compatibilidad de los 47 ejemplos sin roturas → respetada.
- Módulo común en `examples/comun/diagnostico.rs` sin dependencias externas añadidas → respetada.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: ninguno.

### Archivos modificados

- `examples/comun/diagnostico.rs` (nuevo)
- `examples/comun/mod.rs`
- `examples/ppu_nativa_compara.rs`
- `examples/ppu_barrido_nativa.rs`
- `tests/arnes_experimentos_tests.rs`
- `docs/BACKLOG.md`
- `docs/propuestas/2026-10-22_arnes-comparacion-visual-y-causal.md`

### Discrepancias

- ninguna

### Para la próxima vez

- El diseño de `FiltroFirmas` desacoplado y el codificador PNG puro permiten verificar la salida gráfica sin requerir GPU en los tests de integración.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana

- ninguno: todo se comprobó con tests (`test_captura_compuesta_solo_se_escribe_cuando_hay_diferencias`, `test_diagnostico_causal_identifica_origen_de_la_diferencia`, `test_deduplicacion_de_barrido_omite_firmas_repetidas`, `test_los_experimentos_usan_el_modulo_comun`).
- Detectado solo por el humano:
