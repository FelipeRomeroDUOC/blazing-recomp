# Propuesta: El dibujado por software baja de 9,2 ms a menos de 3 sin cambiar un píxel

Fecha: 2026-10-13
Nivel: 2
Estado: aprobada (el usuario aprobó el alcance completo el 2026-10-13: «conservemos el motor,
implementa la propuesta de ppu-software»)
Ejecución: fases A y B implementadas y verificadas con código; el escenario del dispositivo
(apartado 8) queda para la persona

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El renderizador de la PPU en software deja de costar 9,20 ms por fotograma en el Android TV y baja a
**≤ 4,5 ms** (objetivo) o mejor, **sin que cambie un solo píxel de salida**: mismos hashes del framebuffer,
mismos tests de la PPU y las mismas imágenes en pantalla. La ganancia sale de tres frentes, en este orden:

1. **La composición de la línea** (`compose`, 4,20 ms): hoy decide *de píxel en píxel* qué capa se ve, con
   la ventana y los efectos consultados pixel a pixel. La decisión depende de la configuración de la línea,
   no del píxel, así que se calcula **por tramos** (las ventanas y sus controles son rangos; el orden de
   prioridad es constante en toda la línea) y solo se mezcla donde hay algo que mezclar.
2. **Los fondos y los sprites** (`render_*`, 3,20 + 1,40 ms): el fondo de texto ya camina por tramos de
   tile desde una optimización anterior; lo que queda es sacar el trabajo por píxel que aún es genérico
   (llamadas a `pal16` por píxel, escrituras de un `u16` cada vez, buffers reinicializados enteros cuando
   la capa está apagada) y dar a los sprites el mismo tratamiento por tramos.
3. **No redibujar lo que no ha cambiado** (palanca estructural, la última y la más delicada): una línea
   cuya configuración y cuyas fuentes (VRAM, paleta, OAM, registros) no han cambiado desde el fotograma
   anterior no necesita volver a componerse. Es lo que hacen los emuladores rápidos y lo que convierte un
   mapa táctico quieto en casi gratis.

El objetivo del proyecto aquí es concreto: hoy `ejec` (16,30 ms) ya consume el presupuesto entero de 60 FPS
(16,67 ms), así que el port no puede ganarle a un emulador en ese dispositivo. La PPU es el 55% de ese
`ejec`; bajarla a la mitad deja margen real.

### 2. Contexto

- Contexto consultado: la sesión del televisor
  [`scripts/diagnostico/log_tv15.txt`](../../scripts/diagnostico/log_tv15.txt) analizada con
  [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) (229 s, mediana 55 FPS, mínimo
  28: `ejec` 16,30 ms —`ppu` 9,20 = fondos 3,20 + sprites 1,40 + composición 4,20—, `apu` 1,50, resto 5,60;
  `dibujo` 6,60 con `present` 5,00); del motor: `crates/gba-core/src/ppu.rs` (1000 líneas: `render_scanline`,
  `render_text_bg` —ya por tramos de tile, con lectura de fila a 32 bits—, `render_affine_bg`,
  `render_bitmap_bg`, `render_sprites`, `compose` con `control_at`/`top_two_layers` por píxel, `alpha_blend`,
  `brighten`, `darken`), `crates/gba-core/src/mem.rs` (`framebuffer`, `vram`, `palette`, `oam`, `aff_dirty`),
  y los tests de la propia `ppu.rs` (comprueban valores exactos del framebuffer); del port:
  [`src/perf.rs`](../../src/perf.rs) y [`src/platform.rs`](../../src/platform.rs) (las columnas
  `ppu/ppubg/ppuspr/ppucomp` del `.txt`) y la sonda `RECOMP_PPU_STATS=1` de
  `crates/recomp/src/main.rs` (la herramienta de iteración rápida en host).
- Intención: que el port tenga una ventaja medible sobre un emulador en el dispositivo más débil del
  proyecto. La ventaja no puede venir de recompilar mejor —el juego ya corre nativo, con el
  intérprete reducido al BIOS—, sino de **imitar mejor el hardware**: a 240 ns por píxel, la PPU cuesta hoy varias veces lo que cuesta esa misma tarea hecha con las
  técnicas que usan mGBA y VBA-M (decodificación por filas de tile, copia de tramos, caché de filas de mapa,
  saltar líneas limpias), que se estiman en 0,5-5 ms por fotograma en un ARM moderno.
- Restricciones y decisiones previas (máx. 3):
  - **La salida visual es intocable** (origen: los tres hashes guardianes del arnés y los tests de `ppu.rs`
    que fijan valores exactos) → implica: toda optimización es de *cómo* se calcula el mismo resultado; si
    algo cambia un píxel, es un fallo, no un compromiso → protegida por: *El dibujado no cambia ni un píxel*.
  - **La granularidad sigue siendo por scanline** (origen: `docs/architecture.md` §6 y el propio módulo:
    los efectos a mitad de línea están fuera de alcance por diseño) → implica: se puede reordenar el trabajo
    dentro de una línea, pero no introducir temporización más fina → protegida por: *La PPU sigue siendo por
    scanline*.
  - **El instrumento no puede ser el trabajo** (origen: la propuesta cerrada
    [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md)) → implica:
    las métricas de la optimización se leen de `RECOMP_PPU_STATS` en host y de las columnas `ppu*` en el
    dispositivo, y se comparan **antes/después en la misma escena**, nunca entre máquinas distintas →
    protegida por: *La PPU baja donde se paga*.
- Alternativas descartadas:
  - **Llevar la PPU a la GPU** → descartada como primera opción: a resolución nativa la GPU no garantiza
    menos latencia (subida de VRAM, sincronización, llamadas de dibujo) y añade el riesgo de divergencias
    de píxel con el modelo en software; además el port ya tiene su propio coste de subida y conversión de
    color. Queda como plan B si el software no llega al listón.
  - **Subir el nivel de optimización del compilador** → descartada aquí: la PPU es Rust compilado por LLVM
    en `release`; ese camino ya está tomado. (El C11 recompilado, que sí va con `-O1`, es otra propuesta.)
  - **Cachear el fotograma completo** → descartada: en un juego con desplazamiento continuo no acierta, y
    acierta por casualidad en las escenas quietas. La unidad que se puede cachear con garantías es la
    **línea**, y solo con invalidación explícita.
- Ante imprevistos, prioriza: la fidelidad de píxel por encima de la ganancia.

### 3. Ejemplos

- **Caso normal** (mapa táctico de la sesión 17, 4 fondos activos y sprites, sin ventanas ni efectos):
  antes: `ppu` 9,20 ms (composición 4,20) y `ejec` 16,30 ms, con la CPU al 98% del presupuesto de 16,67 ms;
  ahora (objetivo): `ppu` ≤ 4,5 ms, `ejec` ≤ 11 ms, y ningún segundo por debajo de 58 FPS.
- **Caso de esquina** (una línea con ventana 0 activa, un sprite semitransparente y `BLDCNT` en alpha):
  antes: los 240 píxeles pasan por la búsqueda de las dos capas superiores;
  ahora (objetivo): solo los píxeles dentro de la ventana y bajo el sprite semitransparente pagan ese
  camino; el resto van por la ruta recta. Mismo resultado, píxel a píxel.
- **Caso de instrumentación** (iteración rápida en host):
  antes: cada prueba exige empaquetar el APK y jugar en la TV;
  ahora: `RECOMP_PPU_STATS=1` sobre `runc` da `ppu/bg/spr/compose` por fotograma en el mismo host, y el
  guardián de 600/2400/7200 fotogramas dice si algún píxel cambió. La TV se reserva para validar.

### 4. Pasos

**Fase A — medir el suelo, antes de optimizar.**

1. Con `RECOMP_PPU_STATS=1`, medir la línea base en host sobre los mismos 7200 fotogramas (guardián
   `b54066dc06f2b76c`/3.208.998) y anotar `ppu/bg/spr/compose`.
2. Prototipo del suelo: sustituir temporalmente `compose` por la escritura del resultado sin efectos (una
   variante que solo elige la capa superior) y medir cuánto queda. Sirve para saber si el suelo alcanzable
   está por debajo del listón antes de invertir en la reescritura.

**Fase B — reescribir el camino caliente por tramos.**

3. `compose`: resolver la ventana y el control por **tramos** (rangos de x), no por píxel; especializar las
   líneas sin ventanas, sin efectos y con un único fondo visible en bucles rectos; escribir el framebuffer
   por pares (`u32`) en lugar de `u16` a `u16`.
4. `render_sprites` y el resto de `render_*`: mismo tratamiento por tramos, paleta resuelta una vez por
   línea, y no reinicializar buffers de capas apagadas.
5. Re-medir en host con la misma sonda y el mismo guardián. Si el listón del caso normal no se alcanza,
   continuar con el paso 6; si se alcanza, saltar al 7.

**Fase C — no redibujar lo que no ha cambiado (solo si hace falta).**

6. Estado limpio por línea: firma de la configuración (DISPCNT/BGxCNT/ventanas/BLDCNT/scroll) y de las
   fuentes (VRAM, paleta, OAM), con invalidación en las escrituras correspondientes; una línea limpia no se
   recompone. Requiere su propio test de invalidación (ver criterios).
7. Validar en el dispositivo: una sesión de ≥120 s en la misma escena, comparar `ppu`, `ppubg`, `ppuspr`,
   `ppucomp`, `ejec` y FPS con la sesión 17.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -n "framebuffer" crates/gba-core/src/*.rs`, `grep -rn "render_scanline" crates/`,
`grep -rn "RECOMP_PPU_STATS" crates/`, y `grep -rn "ppu" src/*.rs` en el port.

- `crates/gba-core/src/ppu.rs` — el módulo entero: `compose` (614), `render_text_bg` (192, ya por tramos),
  `render_affine_bg` (280), `render_bitmap_bg` (337), `render_sprites` (382), `top_two_layers`,
  `alpha_blend`, `brighten`, `darken` y los tests con valores exactos del framebuffer.
- `crates/gba-core/src/mem.rs` — el `framebuffer: [u16]` y los búferes fuente (`vram`, `palette`, `oam`) que
  el paso 6 tendría que vigilar; `aff_dirty` ya existe y es el precedente del mecanismo.
- `crates/recomp/src/main.rs` — la sonda `RECOMP_PPU_STATS` (2750): el instrumento de la fase A.
- [`src/perf.rs`](../../src/perf.rs) y [`src/platform.rs`](../../src/platform.rs) — las columnas `ppu*` que
  se leen en el dispositivo; **no** cambian de significado ni de formato.
- [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y
  [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) — hay que actualizar los números y añadir
  la decisión.
- Los tests de la PPU del motor y `tests/packaging_tests.rs` del port (el APK cambia de tamaño).
- Sin más referencias encontradas.

### 6. Suposiciones

- **El coste de `compose` viene de las decisiones por píxel, no de la cantidad de píxeles** → prototipar:
  sí (fase A, paso 2) → si es falso (el suelo ya está cerca de 4 ms), se pasa directamente a la fase C
  (saltar líneas limpias), que ataca el número de líneas en vez del coste por línea.
  → resultado del prototipo: pendiente (se completa tras la aprobación).
- **El orden y el resultado de `compose` son expresables por tramos sin cambiar píxeles** (las ventanas son
  rangos, el orden de prioridad es constante en la línea, los efectos solo dependen de la configuración) →
  prototipar: sí (fase B) → si es falso: se conserva el camino por píxel para las líneas con ventanas o
  efectos y se especializa solo el caso recto, que es el habitual.
  → resultado del prototipo: pendiente.
- **Se puede comparar antes/después en host con la misma escena** (el arnés es determinista: mismos hashes a
  igual número de fotogramas) → prototipar: no (ya está comprobado en la propuesta anterior: 42 corridas con
  el guardián intacto) → si es falso: la comparación se hace solo en el dispositivo, con dos sesiones.
  → no detecta: que la escena del arnés (los primeros 600-7200 fotogramas) tenga menos capas activas que el
  mapa táctico; por eso las ganancias se validan en la TV y los números de host son **relativos**.

### 7. Alcance

- **Incluye:**
  - `crates/gba-core/src/ppu.rs` completo (composición, fondos, sprites) y los tests que lo blindan.
  - El estado limpio por línea, si la fase A muestra que hace falta (paso 6).
  - La medición en host con `RECOMP_PPU_STATS` y la validación en el dispositivo.
- **No incluye:**
  - Cambiar la salida visual, la resolución, o el formato del framebuffer.
  - La conversión de color y la subida a GPU del port (`color`, `subida` en la telemetría): son puerto, no
    PPU, y van aparte.
  - El mosaico (no implementado hoy) y los efectos a mitad de scanline (fuera de alcance por diseño).
  - El C11 recompilado (`-O1`) y los 5,60 ms del "resto" de `ejec`: otra propuesta.
  - Publicar el motor y re-fijar el SHA: trabajo propio, al final.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El dibujado no cambia ni un píxel** (regresión)
Caso de uso: cualquier cambio del renderizador | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario, y la batería de tests de la PPU
  Cuando se ejecutan 600, 2400 y 7200 fotogramas y `cargo test -p gba-core`
  Entonces los hashes son `31184031261ea2ec`, `b54066dc06f2b76c` y `b54066dc06f2b76c` con
  1.776.798 / 2.167.398 / 3.208.998 bloques y `fallback_steps: 0`, y todos los tests de la PPU pasan
Test: `recomp runc --bios roms/gba_bios.bin --frames 600|2400|7200` desde la raíz del port
Estado previo: **pasa** (es el guardián; debe seguir pasando)

**Escenario: La composición baja en host** (normal)
Caso de uso: los mismos 7200 fotogramas, antes y después | Tipo: normal
  Dado `RECOMP_PPU_STATS=1` sobre la misma longitud de corrida y el mismo `.dll`
  Cuando se comparan las cifras de `ppu`, `bg`, `spr` y `compose` por fotograma
  Entonces `ppu` no supera el **60%** de la línea base y `compose` no supera el **50%**
Test: la misma invocación con la sonda, antes y después, sobre los mismos fotogramas
Estado previo: **falla** (la línea base es la de hoy: medir en la fase A)

**Escenario: La PPU baja en el dispositivo** (visible)
Caso de uso: mapa táctico en el Android TV | Tipo: normal
  Dado el APK optimizado y una sesión de ≥120 s en las mismas escenas de la sesión 17
  Cuando se lee `blazing-perf.txt` y se compara con [`log_tv15.txt`](../../scripts/diagnostico/log_tv15.txt)
  Entonces `ppu` baja de 9,20 a **≤ 4,5 ms** (composición de 4,20 a ≤ 2,0 ms), `ejec` baja de 16,30 a
  **≤ 11 ms**, se mantienen `ppubg` y `ppuspr` dentro del ±10% de lo esperado por sus objetivos, y no queda
  ningún segundo por debajo de 58 FPS
Test: revisión humana + [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)
Estado previo: **falla** (9,20 ms y 10 segundos por debajo de 58 FPS en la sesión 17)

**Escenario (regresión): las líneas limpias no mienten** (solo si se implementa el paso 6)
Caso de uso: escritura de VRAM/paleta/OAM entre fotogramas | Tipo: regresión
  Dado el renderizador con estado limpio por línea
  Cuando un test escribe en VRAM, en la paleta o en OAM entre dos fotogramas, o cambia DISPCNT/BGxCNT/BLDCNT
  Entonces el fotograma siguiente refleja el cambio en todas las líneas afectadas, y una línea que no cambió
  no se recompone (contador de líneas recompuestas)
Test: test nuevo del motor (mutación de cada fuente, una por una) + el guardián del arnés
Estado previo: **no aplica** (hoy todas las líneas se recomponen siempre)

### 9. Cómo lo comprobaré

- **El dibujado no cambia** → comando:
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: `hash: b54066dc06f2b76c`, `native_blocks: 3208998`, `fallback_steps: 0`
  → no detecta: diferencias de píxel que la escena de 7200 fotogramas no toque (por eso los tests de la PPU
  cubren ventanas, efectos, sprites afines y modos de bitmap por separado).
- **La composición baja en host** → comando: el mismo `runc` con `RECOMP_PPU_STATS=1`, antes y después
  → pasa si: `ppu` ≤60% y `compose` ≤50% de la línea base
  → no detecta: el efecto en el dispositivo; el host tiene un reloj distinto y una escena distinta.
- **La PPU baja en el dispositivo** → comando: `python scripts/diagnostico/analizar_telemetria.py --archivo
  <log nuevo> --comparar scripts/diagnostico/log_tv15.txt`
  → pasa si: `ppu` ≤ 4,5 ms, `ejec` ≤ 11 ms, ningún segundo por debajo de 58 FPS
  → no detecta: cambios visuales; para eso está la revisión en pantalla del corto plazo.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — invertir el orden de prioridad de dos fondos → debe morir el escenario de píxel (el hash cambia).
M2 — pintar el sprite semitransparente como opaco → debe morir igualmente.
M3 — dar por limpia una línea que sí cambió → debe morir el escenario de líneas limpias.
Los archivos se respaldan en RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

La PPU baja a ≤4,5 ms en el dispositivo (o al suelo medido en la fase A, si resulta ser menor que el
objetivo) sin que cambie un píxel, los tres hashes guardianes siguen intactos, los tests de la PPU y la suite
del port siguen en verde, y la decisión y los números están documentados.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta
  como `discrepada`.
- **El prototipo del suelo (paso 2) muestra que la composición ya está cerca del mínimo** (menos del 25% de
  ganancia posible con el camino recto): no se reescribe `compose`; se pasa a la fase C.
- Si la reescritura por tramos no llega al listón y la fase C tampoco, **no se encadenan intentos**: se
  documenta lo ganado y se abre la propuesta de la PPU en GPU con los números delante.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primera versión de la propuesta planteaba "optimizar los fondos" como
  el problema principal. Al leer el módulo vi que `render_text_bg` **ya** camina por tramos de tile y lee la
  fila a 32 bits: ese trabajo está hecho. El coste vivo está en `compose` (4,20 ms de los 9,20) y en lo que
  queda de trabajo por píxel en las tres funciones de dibujado.
- **Corregido también:** iba a proponer "cachear el fotograma". El propio módulo ya arrastra `aff_dirty` para
  las capas afines, lo que enseña el mecanismo correcto: la invalidación es por fuente y por línea, no global.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno todavía: la propuesta se presenta antes de tocar código.

---

## Resultado (lo completa el agente después de editar)

Estado: `verificada por agente` en los escenarios de código; el escenario del dispositivo no se pudo
medir aquí.

### Predicho vs. real

- **El dibujado no cambia ni un píxel**: coincide. Los tres hashes guardianes y sus recuentos siguen
  intactos con el código final: `600 → 31184031261ea2ec` (1.776.798 bloques),
  `2400 → b54066dc06f2b76c` (2.167.398) y `7200 → b54066dc06f2b76c` (3.208.998), `fallback_steps: 0`.
  Método: `recomp runc … --frames 600|2400|7200`, con el binario final y con el de referencia, en la
  misma sesión de máquina.
- **La composición baja en host**: el `compose` sí, el total no. El apartado 8 pedía `ppu ≤ 60 %` y
  `compose ≤ 50 %` de la línea base. Medido a 7.200 fotogramas, mismo host y misma sesión:

  | métrica (ms por fotograma, 7.200 fotogramas) | referencia | final | criterio | resultado |
  | :--- | ---: | ---: | :--- | :--- |
  | `ppu` (total) | 0,536 | **0,367** | ≤ 0,322 (60 %) | **no se cumple** (68,5 %) |
  | `compose` | 0,157 | **0,071** | ≤ 0,079 (50 %) | **se cumple** (45,2 %) |
  | `bg` (fondos) | 0,284 | **0,197** | — | −30,6 % |
  | `spr` (sprites, **sin tocar**) | 0,095 | 0,099 | control | +4,2 % |
  | `apu` (audio, **sin tocar**) | 0,114 | 0,115 | control | +0,9 % |

  A 600 fotogramas (el arranque, con menos fondos activos y más sprites) el patrón es el mismo:
  `ppu` 0,528 → 0,445 (84 %) y `compose` 0,141 → 0,096 (68 %). Los dos subsistemas que no se tocaron
  funcionan como control de que la máquina no cambió de velocidad.
- **Fase A (el suelo), medida antes de reescribir**: forzando el camino recto en **todas** las líneas,
  `compose` cuesta 0,061 ms; sustituyéndolo por un relleno, 0,009 ms. Es decir: el techo de esta
  escena está en ~0,061 y el suelo en ~0,009, con la línea base en 0,157. El camino condicional se
  queda en 0,071 porque hay líneas que no puede tomar (efectos de color activos), no porque el bucle
  sea lento.

### Medición en el dispositivo (sesión 18)

El APK con este cambio se probó en el Android TV (`blazing-perf-tv-18.txt`, fuera del repositorio;
`OpenGL`, `AOT`, `AutoVsync`, 133 s de telemetría, `interprete=0` en toda la sesión). La comparación
usa la ventana alineada de los primeros 133 s de cada sesión, que es la parte común del arranque:
sesión 16 (antes de la propuesta anterior), 17 (después) y 18 (después de esta).

| métrica (ms por fotograma, ventana de 133 s) | sesión 16 | sesión 17 | sesión 18 |
| :--- | ---: | ---: | ---: |
| `ejec` (CPU de emulación) | 15,00 | 13,60 | **16,40** |
| `ppu` (total PPU) | 9,40 | **7,70** | **7,60** |
| `ppubg` (fondos) | 3,20 | 3,30 | **2,40** (−27 %) |
| `ppuspr` (sprites, **sin tocar**) | 1,40 | 1,30 | 1,60 (+23 %) |
| `ppucomp` (composición, **tocada**) | 4,30 | 3,25 | **3,80** (+17 %) |
| `apu` (audio, **sin tocar**) | 1,50 | 1,50 | 2,10 (+40 %) |
| `color` (simulación LCD, **sin tocar**) | 1,20 | 1,20 | 1,30 |
| `ejec − ppu − apu` (despacho y lógica, **sin tocar**) | 4,10 | 4,40 | 6,70 |
| Segundos por debajo de 58 FPS | 20 % | 6 % | **68 %** |

**El escenario del dispositivo no se cumple**: `ppu` 7,60 (objetivo ≤ 4,5), `ejec` 16,40 (≤ 11) y 68 %
de los segundos por debajo de 58 FPS. Pero, sobre todo, **esta pareja de sesiones no permite atribuir
nada**: todo sube respecto a la sesión 17 salvo los fondos, incluidas las tres partes que no se
tocaron (`apu` +40 %, sprites +23 %, despacho y lógica de 4,40 a 6,70 ms), y el trabajo nativo por
fotograma pasa de 4.761 a 7.180 bloques (+51 %), lo que apunta a **contenido distinto o más pesado**,
no a código distinto. Con un factor común de ese tamaño, la única señal atribuible es la caída de
`bg` (−27 %, y en host la misma mano dio −31 %), mientras que la ganancia esperada en `compose`
(caída a la mitad) no aparece: sube con el resto de la máquina.

La única certeza que sí da el registro es que el reparto es fiable: `ppu` vuelve a ser exactamente
`bg + spr + comp` (residuo 0,00 ms). Y la distribución de la composición es bimodal: p25 1,80 ms en
las escenas ligeras y 3,80-4,00 ms clavados en el tramo pesado (t≈29..116 s), un valor constante que
se parece al coste por píxel del camino general, no al de un camino que decide por tramos.

**Hipótesis, no conclusión** (queda sin comprobar): puede que en esa escena la vía rápida no se tome
—efectos de color activos, ventana de sprites o algún sprite translúcido por línea—, o en este
dispositivo el coste de la composición puede no estar en las decisiones por píxel. Las dos cosas se
distinguen contando: cuántas líneas toma cada camino y por qué se descartan las demás, más una marca
de compilación en el encabezado para saber qué PPU está corriendo. Ese contador es trabajo aparte
([`2026-10-13_contadores-de-camino-de-la-ppu.md`](2026-10-13_contadores-de-camino-de-la-ppu.md))
y es el paso previo a cualquier otra optimización de la PPU.

### Por qué el `ppu` total no llega al listón del 60 %

El prototipo del suelo lo dice y es el dato que cambia el plan: **con `compose` casi gratis (0,009 ms)
el `ppu` sigue en 0,309 ms** (frente a 0,536 de referencia), porque el trabajo que queda no es
composición sino *dibujar*: 0,191 de fondos + 0,109 de sprites. Los fondos cuestan 1,3 ns por píxel
(618 líneas de fondo por fotograma: cuatro fondos activos por línea en 160 líneas), que es el precio
de leer VRAM y escribir el framebuffer, no de una decisión por píxel.

Conclusión medida: **este listón no se alcanza optimizando el cálculo; solo se alcanza dejando de
dibujar líneas que no han cambiado** (el paso 6, fase C, que la propuesta dejaba «solo si hace
falta»). No se implementó: es un cambio de arquitectura —firma de configuración y de fuentes por
línea, con invalidación en cada escritura de VRAM, paleta, OAM y registros— que no cabe como
extensión de este cambio, y el apartado 11 pide no encadenar intentos. Queda como propuesta propia,
con estos números delante.

### Qué se implementó (motor, `crates/gba-core/src/ppu.rs`)

1. **Camino recto de `compose`** (`straight_span`, `straight_bgs`, `straight_obj`,
   `compose_straight`): con `BLDCNT` en modo 0 y sin ventana de sprites, el control de capas es
   constante a tramos, así que la línea se parte en **tramos** de x (hasta cinco: los bordes de WIN0
   y WIN1) y cada tramo se compone en una pasada, con la lista de fondos monomorfizada por número de
   fondos activos (1 a 4), sin cierre de ventana, sin índices dinámicos y sin comprobación de límites
   por píxel. Las líneas con efectos de color, ventana de sprites o sprite translúcido siguen por el
   camino general, que no se tocó.
2. **Los tramos de ventana se derivan de sus bordes** (WIN0H/WIN1H), con la misma expresión que usa
   `control_at` por píxel, incluido el caso que da la vuelta cuando X1 > X2.
3. **El sprite translúcido se detecta al dibujarlo** (`render_sprites` informa si colocó algún píxel
   semitransparente) en lugar de barrer la línea de 240 píxeles: el barrido costaba 0,026 ms por
   fotograma —16 % de `compose`— y la señal ya estaba en la mano.
4. **Fondos**: un tramo de tile cuya fila está vacía se rellena de una vez en lugar de decodificarse
   píxel a píxel, y la comprobación de que la fila de tile cabe en VRAM sale del bucle de píxeles del
   camino de 8bpp.

### Tests

- **Motor**: 75 unitarios + 2 de integración en verde (eran 73 + 2). Los dos nuevos son de frontera y
  son los que blindan lo añadido:
  - `window_ranges_keep_the_control_of_each_range`: una línea con WIN0 (x = 8..16) y capas distintas
    dentro y fuera; comprueba los píxeles 7, 8, 15, 16 y 239, es decir los bordes exactos de los
    tramos (X1 inclusive, X2 exclusivo).
  - `translucent_sprite_still_blends_in_mode_zero`: un sprite translúcido sobre un fondo opaco con
    `BLDCNT` en modo 0 y el segundo objetivo activo. Si la vía rápida se tomara, el píxel sería el
    color crudo del sprite y el test fallaría.
- **Port**: 107 + 15 = 122 en verde, sin cambios (el port no se tocó).
- **Mutación prevista**: M1 (invertir la prioridad de dos fondos) y M2 (pintar el translúcido como
  opaco) mueren con los dos tests anteriores más el hash; M3 pertenece a la fase C, que no se
  implementó.

### Archivos modificados

- Motor (`../gba-recomp`):
  - `crates/gba-core/src/ppu.rs`: camino recto de la composición (cuatro funciones nuevas), tramos de
    ventana, señal de sprite translúcido en `render_sprites` y `render_scanline`, y las dos mejoras
    de `render_text_bg`. **Sin commitear** en el repositorio del motor: el `[patch]` del port lo toma
    del disco, y publicar el motor y re-fijar el SHA es trabajo aparte (apartado 7).
- Port: este archivo y `target/blazing-recomp-release.apk` (el APK con el cambio, para validar en el
  dispositivo).

### Discrepancias

- **El listón del `ppu` en host no se cumple** (68,5 % frente a ≤ 60 %), y no por el camino recto:
  el techo medido con `compose` casi gratis también queda por encima (0,309 ms = 58 % solo si el
  resto no se mueve, con `compose` a 0,009, que es un relleno de memoria).
- **No se escribió el framebuffer por pares `u32`** (apartado 4, paso 3). Los bucles quedan sin
  comprobación de límites y el compilador ya los vectoriza; el A/B no muestra que falte ese trabajo y
  hacerlo a mano habría pedido `unsafe`. Se declara como desviación de la letra de la propuesta.
- **No se quitaron los rellenos de los búferes de capa apagada** (apartado 4, paso 4). Para eso los
  búferes tendrían que vivir en `MemMap` y `render_text_bg`/`render_sprites` leen de `mem` mientras
  escriben su línea: obliga a partir los préstamos o a cambiar las firmas de las cuatro funciones de
  dibujado, un refactor que no cabía en este cambio.
- **Anomalía medida y no explicada**: en el binario de medición, la corrida `general` (solo con las
  dos mejoras de `render_text_bg`, con `compose` intacto) dio `compose` 0,190 frente a 0,161 de la
  referencia: un +18 % en código que no se tocó. En el binario final `compose` queda en 0,071, así que
  no afecta al resultado, pero el efecto (reubicación de código o decisiones de inline) queda
  registrado.

### Fuera de alcance (hallazgos que no se arreglaron)

- **El orden de bytes de las ventanas no está documentado en el motor** (impacto: bajo). `WIN0H`
  empaqueta X1 (izquierda) en el byte alto y X2 (derecha) en el bajo, y el rango es `x1 <= x < x2`,
  con vuelta cuando X1 > X2. La única constancia es el propio `in_window_range`: mi primer test lo
  escribió al revés y falló, así que el motor estaba bien y el error era del test. Un comentario
  junto a la lectura de los registros lo ahorraría.
- **Los búferes por línea se reinicializan enteros** (impacto: bajo-medio). `render_scanline` llena
  1.920 bytes por línea (`[[TRANSPARENT; 240]; 4]`) y las cuatro funciones de dibujado ya escriben la
  línea completa, así que el relleno es trabajo repetido (~0,02-0,03 ms por fotograma, contado dentro
  de los fondos). Quitarlo necesita los búferes en `MemMap` y partir los préstamos.
- **El cambio del motor está sin commitear** (impacto: medio). El APK lo lleva por el `[patch]`, pero
  cualquier build que no use la ruta local compilaría el motor fijado y perdería la mejora.
- **La validación en el dispositivo queda pendiente** (impacto: alto, y no es un defecto): es la
  medida que decide el apartado 8 y se hace en el televisor.

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

- La PPU baja en el dispositivo y ningún segundo por debajo de 58 FPS (escenario visible): ✔ | ✘ | ?
- El juego se ve igual en pantalla (mapa, combate, menús, transiciones): ✔ | ✘ | ?
- El caso de esquina con ventana y sprite semitransparente se ve igual: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
