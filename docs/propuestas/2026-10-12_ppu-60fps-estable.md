# Propuesta: 60 FPS estables — el hilo de emulación es el cuello de botella

Fecha: 2026-10-12
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)
Ejecución: pasos 1, 2 y 3 implementados y verificados con código; medición del dispositivo recibida
El escenario de presupuesto **no se cumple todavía**: las caídas por debajo de 58 FPS bajan 3,5×, pero no llegan a cero (ver «Medición en el dispositivo»). La marca del apartado «Revisión humana» sigue siendo de la persona.

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El hilo de emulación deja de gastar tiempo en la instrumentación que lo mide y en trabajo
redundante de la PPU, para que sus fotogramas quepan con holgura en los 16,67 ms del presupuesto
de 60 Hz. Hoy los ocupa al **89 %** y por eso hay caídas.

### 2. Contexto

- Contexto consultado: [`scripts/diagnostico/log_tv15.txt`](../../scripts/diagnostico/log_tv15.txt),
  el registro de la sesión más reciente en el Android TV
  (`blazing-perf-tv-16.txt`, fuera del repositorio, analizado con
  [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)),
  [`src/perf.rs`](../../src/perf.rs), [`src/runner.rs`](../../src/runner.rs),
  [`src/pipeline.rs`](../../src/pipeline.rs), [`src/platform.rs`](../../src/platform.rs) y, del
  repositorio del motor, `crates/gba-core/src/ppu.rs` (961 líneas) y
  `crates/gba-core/src/mem.rs:652-660`.
- Intención: que la tasa de fotogramas deje de bajar de 60 en las escenas cargadas, bajando el
  coste de CPU de la emulación, que es el recurso que se agota. El éxito se ve como **cero segundos
  por debajo de 58 FPS** en la telemetría del dispositivo, no como un promedio mejor.
- Restricciones y decisiones previas (máx. 3):
  - El hilo de emulación y el hilo de presentación están desacoplados y corriendo en paralelo
    (origen: [`src/pipeline.rs`](../../src/pipeline.rs), `EmulationWorker` con cadencia propia de 60 Hz)
    → implica: `ejec` y `dibujo` **no se suman**, y el presupuesto que importa es el del hilo de
    emulación (16,67 ms) mientras el de presentación solo bloquea contra VSync → protegida por:
    *La telemetría sigue midiendo las dos fases en sus hilos respectivos*.
  - La corrección del renderizado está protegida por el hash del búfer de fotograma del arnés del
    motor (origen: `recomp runc --frames 7200` → `hash: b54066dc06f2b76c`) → implica: cualquier
    cambio de PPU debe terminar en el mismo hash, o el cambio alteró píxeles → protegida por:
    *El hash del fotograma no cambia en 7200 fotogramas*.
  - El motor es un repositorio independiente y el port se fija a un SHA inmutable
    (origen: `AGENTS.md` §4 y la directiva `[patch]` de [`Cargo.toml`](../../Cargo.toml)) → implica:
    los cambios del motor se desarrollan en local por el `[patch]`, se comprueban con el arnés y
    luego se suben al repositorio oficial para volver a fijar el SHA; el port no puede quedar
    apuntando a un motor sin publicar → protegida por: *El APK final se compila con el `[patch]`
    solo mientras el motor no esté disponible en el rev fijado*.
- Alternativas descartadas:
  - Bajar la resolución de render o tocar el camino de GPU → **los segundos malos son más baratos
    en GPU que los buenos** (`dibujo` 6,05 ms frente a 9,03 ms; `present` 4,40 frente a 7,40): la
    presentación está esperando VSync, no limitando. Bajar resolución no añade CPU y cuesta nitidez.
  - Saltar fotogramas *de render* para «recuperar» FPS → no crea tiempo de CPU; el trabajo de
    emulación faltante sigue faltando en el fotograma siguiente.
  - Optimizar el APU (1,48 ms) o el despacho AOT (~4,4 ms) antes que la PPU → la PPU sola es el
    60 % de `ejec` (8,88 ms); empezar por los demás sería cambiar el orden por preferencia, no por
    medida.
  - Cachear líneas de fotograma entre fotogramas (dirty-line) → es la técnica con más techo, pero
    exige rastrear todas las fuentes de entrada (VRAM, paleta, registros con HDMA) y es un cambio de
    arquitectura de un núcleo generalista; se descarta por riesgo, no por falta de valor.
- Ante imprevistos, prioriza: que el juego siga renderizando exactamente igual (hash intacto) sobre
  la ganancia de rendimiento.

### 3. Ejemplos

- **Caso normal** (mapa táctico con cuatro fondos y ventana de menú, el caso medido):
  antes: `ejec` 14,77 ms de media y 10 segundos de cada 254 por encima del presupuesto, con FPS
  cayendo a 48;
  ahora (objetivo): `ejec` ≤ 10,5 ms y ningún segundo por debajo de 58 FPS.
- **Caso límite** (segundo con la PPU al máximo: `ppu` 11,10 ms, `spr` 3,50 ms, `comp` 5,10 ms):
  antes: `ejec` 19,0 ms → 48 FPS;
  ahora (objetivo): `ejec` ≤ 15 ms → 60 FPS sostenidos, que es justo el margen que falta hoy.
- **Caso de instrumentación** (cualquier fotograma, con `reloj=1891ns` medido en el dispositivo):
  antes: 6 lecturas de reloj por scanline (4 en `ppu.rs`, 2 en `mem.rs:656`) × 160 líneas ≈
  **1,82 ms por fotograma** que se pagan dentro de `ejec` y, en parte, dentro de `ppu`;
  ahora (objetivo): el mismo desglose de telemetría con ≤ 0,1 ms de coste.

### 4. Pasos

Los tres primeros pasos son del repositorio del motor (`../gba-recomp`), que es donde vive la PPU y
donde se instrumentan los contadores. El cuarto es del port.

1. **Instrumentación cara fuera del camino caliente** (`crates/gba-core/src/ppu.rs` y `mem.rs`):
   una lectura de reloj barata (contador de sistema, con alternativa `Instant` en arquitecturas sin
   él) sustituye a `std::time::Instant::now()` en los cuatro puntos por línea; y el envoltorio
   `ppu_ns` de `mem.rs:652-660` —que duplica el contador, porque `ppu` ya es la suma de sus partes—
   deja de medir aparte. Se conserva el desglose `bg/spr/compose` en la telemetría.

   ```rust
   // Antes (mem.rs, dentro del bucle de scanlines)
   let t_start = Instant::now();
   crate::ppu::render_scanline(self, line as usize);
   self.perf_counters.ppu_ns += t_start.elapsed().as_nanos() as u64;

   // Ahora
   crate::ppu::render_scanline(self, line as usize);   // ppu_ns = bg_ns + sprite_ns + compose_ns
   ```

2. **Invariantes de composición fuera del bucle de 160 líneas** (`ppu.rs::compose`, 544-726): el
   orden de prioridad de las capas, los tramos de ventana, el modo de mezcla y qué capas están
   activas no cambian de una línea a otra salvo que el juego escriba esos registros, y el motor ya
   rastrea escrituras de registros afines (`aff_dirty`). Se calculan una vez por fotograma (o cuando
   hay escritura) en lugar de una vez por línea y por píxel.

3. **Camino caliente de la PPU** (`ppu.rs::render_scanline`, `render_text_bg`, `compose`):
   - Los tres búferes por línea (`bg_lines` 4×240 u16, `obj_line` 240, `obj_window` 240 ≈ 3,1 KB)
     se reinicializan 160 veces por fotograma (≈ 500 KB de escritura); pasan a ser búferes de
     trabajo reutilizados en `MemMap` con limpieza mínima.
   - En `compose`, la evaluación de ventana por píxel (hasta dos `in_window_range` por píxel) se
     sustituye por tramos constantes, y el recorrido de capas por píxel se especializa según cuántas
     capas están activas (1..4) para eliminar índices dinámicos y saltos mal predichos.
   - En `render_text_bg`, la comprobación de límites de VRAM se saca del bucle de píxeles.

4. **El port conserva la frontera de hilos y el formato de la telemetría**: no se toca
   [`src/perf.rs`](../../src/perf.rs), ni los campos de las líneas `t=`/`# regiones`, ni la cadencia
   de la `EmulationWorker`. Si el paso 1 deja la telemetría por debajo del ruido, se documenta en
   [`docs/guias/medir-rendimiento.md`](../../docs/guias/medir-rendimiento.md) qué mide cada campo
   tras el cambio.

### 5. Qué más podría verse afectado

Búsqueda hecha: `grep -rn "ppu_ns\|bg_ns\|sprite_ns\|compose_ns" src/ ../gba-recomp/crates/` y
`grep -rn "render_scanline\|perf_counters" src/ ../gba-recomp/crates/`

- [`src/runner.rs:154-159`](../../src/runner.rs) — lee los contadores por región desde
  `take_perf_counters()`. Los campos siguen existiendo; cambia su precisión, no su significado.
- [`src/platform.rs:2126-2129`](../../src/platform.rs) — escribe la línea `# regiones`. No cambia.
- [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) —
  imprime el desglose `ppubg/ppuspr/ppucomp`. No cambia, pero si el paso 1 altera la escala, la
  comparación con registros anteriores queda sesgada y hay que decirlo en la guía.
- `crates/gba-core/tests/perf_counters_tests.rs` (motor) — mide los contadores; es el test que debe
  seguir pasando tras el paso 1.
- `crates/gba-core/src/ppu.rs` (motor) — sus nueve tests unitarios de renderizado son la red del
  paso 3.
- Sin más referencias encontradas.

### 6. Suposiciones

- **El reloj del sistema es mucho más barato que `Instant::now()` en este dispositivo**, así que el
  paso 1 ahorra ~1,8 ms por fotograma. → prototipar: sí, con una medición comparada de los dos
  relojes en el propio equipo → si es falso: el paso 1 se reduce a retirar el contador duplicado
  (~0,6 ms) y se ganan los pasos 2 y 3 con el trabajo del reloj caro; no se pierde nada más.
  → resultado del prototipo: pendiente.
- **El orden de prioridad y las ventanas son estables dentro del fotograma en las escenas medidas**,
  así que el paso 2 no cambia ningún píxel. → prototipar: sí, con el hash de 7200 fotogramas
  → si es falso: el hash cambia y el paso 2 se limita a las partes que no dependan de registros
  escritos a mitad de fotograma (la comparación de hashes lo dirá por escena).
  → resultado del prototipo: pendiente.
- **El coste de la PPU se reparte como dice la telemetría** (`bg` 3,19 / `spr` 1,39 / `comp` 3,83 ms)
  y no hay una parte sin etiquetar mayor. → prototipar: sí, midiendo en el equipo tras el paso 1,
  cuando el reloj deje de contaminar el reparto → si es falso: el objetivo se recalcula con el
  reparto real antes de seguir con el paso 3.

### 7. Alcance

- **Incluye:**
  - El reloj y los contadores de `crates/gba-core/src/ppu.rs` y `mem.rs` (motor).
  - El camino caliente de la PPU: `render_scanline`, `compose`, `render_text_bg` (motor).
  - La verificación en el dispositivo con el APK y la telemetría.
- **No incluye:**
  - El APU (1,48 ms) y el despacho AOT (~4,4 ms por fotograma).
  - El camino de GPU (`present`, `dibujo`), la resolución de render y el modo de presentación.
  - La caché de líneas entre fotogramas (dirty-line), descartada por riesgo.
  - Subir el motor al repositorio oficial y volver a fijar el SHA: se hace cuando el cambio esté
    medido y aprobado, como trabajo propio.
  - Cambiar el formato de la telemetría o el número de campos.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El reparto de la PPU se conserva sin el reloj caro**
Caso de uso: diagnóstico en el dispositivo | Tipo: normal
  Dado un fotograma con las cuatro capas activas y ventana
  Cuando se renderiza con los contadores activos
  Entonces `bg_ns + sprite_ns + compose_ns` sigue siendo el coste de la PPU y el coste de medida es
  ≤ 0,1 ms por fotograma
Test: `crates/gba-core/tests/perf_counters_tests.rs` (motor) más la medición de relojes comparada
Estado previo: —

**Escenario (regresión): el renderizado es idéntico, píxel a píxel**
Caso de uso: cualquier cambio del camino caliente | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario
  Cuando se ejecutan 7200 fotogramas
  Entonces el hash del búfer de fotograma es `b54066dc06f2b76c` y `fallback_steps: 0`
Test: `recomp runc --frames 7200` (comando completo en el apartado 9)
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: el hilo de emulación cabe en el presupuesto**
Caso de uso: mapa táctico en el Android TV | Tipo: normal
  Dado el APK con los cambios y una sesión de al menos 60 s en una zona con caídas
  Cuando se lee `blazing-perf.txt`
  Entonces ningún segundo muestra `fps` por debajo de 58 y `ejec` promedio es ≤ 10,5 ms
Test: revisión humana + `analizar_telemetria.py` | Estado previo: **falla** (43 de 254 segundos por
debajo de 58, 11 por debajo de 55, 5 por debajo de 50; `ejec` 14,77 ms)

**Escenario (límite): la escena más cara medida no rompe el presupuesto**
Caso de uso: el segundo con `ppu` 11,10 ms y `spr` 3,50 ms | Tipo: límite
  Dado ese mismo estado de juego
  Cuando se mide
  Entonces `ejec` ≤ 15 ms y no hay fotogramas perdidos por encima del presupuesto
Test: revisión humana (el escenario lo elige el jugador al provocar el combate) | Estado previo:
**falla** (`ejec` 19,0 ms → 48 FPS)

### 9. Cómo lo comprobaré

- **El renderizado no cambió** → comando:
  `../gba-recomp/gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime `hash: b54066dc06f2b76c` y `fallback_steps: 0`, código de salida 0
  → no detecta: que el `.so` del APK lleve el cambio; eso lo cubre el empaquetado y la telemetría.
- **El reparto de la PPU y el coste del reloj** → comando: el arnés del motor con los contadores de
  rendimiento activos, imprimiendo `bg_ns/sprite_ns/compose_ns` por fotograma (arnés de una medición,
  aún por escribir en el paso 1)
  → pasa si: el reparto se conserva y el coste de medida baja de ~1,8 ms a ≤ 0,1 ms por fotograma
  → no detecta: cómo se comporta el reloj en otros dispositivos; por eso el propio port mide su
  `reloj=` en el encabezado y cada equipo lo reporta.
- **Nada más se rompe** → comando: `cargo test` en este repositorio y `cargo test` en el del motor
  → pasa si: 122 en verde, 0 fallos en el port (107 unitarios + 15 de integración) y el motor en
  verde, incluidos `crates/gba-core/tests/perf_counters_tests.rs` y los tests de `ppu.rs`
  → no detecta: regresiones visuales sutiles que el hash no vea por casualidad en 7200 fotogramas;
  para eso está el escenario visible.
- **Escenario "(visible)"** → revisión humana: instalar el APK, jugar de 60 a 120 s **en una zona
  que hoy cae** (combate con animación o transición de mapa), y leer en `blazing-perf.txt` los
  segundos con `fps` por debajo de 58 y el promedio de `ejec`
  → no detecta: nada más; es la medida que decide.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — volver a envolver `render_scanline` con un contador de reloj caro → debe morir el escenario de
coste de medida.
M2 — devolver el cálculo de prioridades y ventanas al interior del bucle por píxel → debe morir el
escenario del hilo de emulación (o el hash, si además altera píxeles).
M3 — desactivar la limpieza mínima de los búferes por línea → debe morir el hash de 7200 fotogramas.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios de código y verificación pasan, el hash de 7200 fotogramas no cambió, la suite
del port sigue en 122 verdes, la del motor pasa, y tu sesión en el TV no registra ningún segundo por
debajo de 58 FPS.

### 11. Parar si

2 intentos fallidos seguidos con el mismo escenario. Detenerse, explicar qué se probó y marcar la
propuesta como `discrepada`. Si el paso 2 cambia el hash en todas las escenas, se descarta la
optimización de composición y se continúa solo con el paso 1 y el paso 3, que no dependen de
suposiciones sobre registros estables.

### Correcciones previas a presentar

- La lección aplicada de `2026-10-12_ejecucion-nativa-de-bios-real.md` §"Para la próxima vez": al
  anunciar el conteo de tests se usa base + (nuevos − retirados); aquí no se retiran tests del port.
- `AGENTS.md` (checklist de pre-commit) cita 111 tests; la medición de hoy da 107 unitarios + 15 de
  integración = 122. Se usa la medición de hoy como base.
- El análisis histórico de [`analisis_rendimiento.md`](../../docs/explicacion/analisis_rendimiento.md)
  concluye que el cuello de botella es la espera del compositor de pantalla (`present` ~100 ms). Ese
  diagnóstico **ya no describe el estado actual**: `present` mide 7,15 ms de media y los segundos
  con caídas son los que tienen el `present` *más bajo*. Ese documento se redactó con
  `modo=INT` y `latency: 1`; la corrección de rumbo es material de la fase 5 (sincronización
  documental), no de esta propuesta.
- El `reloj=1891ns` de la sesión de hoy es coherente con las 13 sesiones anteriores (1394–2168 ns),
  así que no es un artefacto de una medida: es una propiedad de estos dispositivos.
- `# regiones` de la misma sesión quedó en `rom=0/7435488 iwram=7759/51822188 ewram=0/0
  bios=0/13531994`: el intérprete ya no tiene trabajo relevante (7.759 pasos en 256 s), así que el
  margen que falta **no** está en la cobertura AOT y no se propone cosechar etiquetas aquí.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El renderizado es idéntico, píxel a píxel**: coincide. Los tres hashes guardianes del arnés del
  motor son idénticos a la línea base medida antes de tocar nada, con los contadores deterministas
  intactos:
  `600 → 31184031261ea2ec`, `2400 → b54066dc06f2b76c`, `7200 → b54066dc06f2b76c`;
  `native_blocks` 1.776.798 / 2.167.398 / 3.208.998 y `fallback_steps: 0` sin cambios. Lo mismo con
  la variante intermedia del A/B, así que el guardián cubre las dos formas del código.
  Método: `recomp runc --frames 600|2400|7200` (comando del apartado 9).
- **El reparto de la PPU se conserva sin el reloj caro**: coincide en la parte comprobable. `ppu`
  volvió a ser **exactamente la suma de sus partes** (0,547 = 0,289 bg + 0,097 spr + 0,161 compose +
  0,000 copy), los campos siguen existiendo con el mismo significado y `bg/spr/compose` siguen
  separados. El coste de medida en host **no es medible** por diseño (el reloj del host ya cuesta
  ~30 ns): la cifra de 1,82 ms/fotograma sale del `reloj=1891ns` que el propio dispositivo publica en
  su encabezado, y su comprobación es la del dispositivo: **confirmada por la identidad de los
  contadores** (el hueco `ppu − (bg+spr+comp)` pasa de +0,473 a +0,008 ms por fotograma; ver la
  sección «Medición en el dispositivo»).
- **Nada más se rompe**: coincide. Motor: 75 unitarios + 2 de integración en verde. Port: 107 + 15 =
  **122 en verde**, 0 fallos, exit 0. La ruta aarch64 del reloj nuevo compila
  (`cargo check --target aarch64-linux-android`).
- **El hilo de emulación cabe en el presupuesto**: comprobado en el dispositivo y **no se cumple**,
  aunque mejora. El objetivo era «ningún segundo por debajo de 58 FPS» y `ejec` ≤ 10,5 ms; la sesión
  17 da **10 segundos de 166 por debajo de 58** (28 de 160 en la ventana alineada) con `ejec`
  **13,59 ms**. Frente a la sesión 16, las caídas se reducen y el mínimo sube de 48 a 52 FPS.
- **El caso límite** (el segundo con `ppu` 11,10 ms y `spr` 3,50 ms): **no se reprodujo**, así que su
  objetivo (`ejec` ≤ 15 ms) tampoco se puede dar por bueno. En la sesión 17 la PPU no pasó de 9,3 ms,
  pero los peores segundos siguen en `ejec` 17,8 ms con 52 FPS.
- **Predicho vs. real en el reparto (host)**: el objetivo del apartado 8 era que la PPU bajase; en
  host bajó de 0,579 a 0,539–0,547 ms/fotograma (ver el A/B de abajo), no a la mitad. En el
  dispositivo la PPU pesa el 43 % de `ejec` de forma distinta que en host (25 %), así que la
  proyección de allí es una hipótesis, no un resultado; la medición real está en la sección de abajo.

### Medición en el dispositivo

Dos sesiones del Android TV, mismo juego y misma configuración (`OpenGL`, `AOT`, `present=AutoVsync`).
La línea base es la sesión **16**, que ya tenía el **99,99 % del código en AOT** (133 pasos de
intérprete en 256 s), así que entre ambas sesiones cambia esta propuesta y **no** la cobertura nativa:
no hay cosecha de etiquetas que pueda explicar la diferencia. La comparación fina usa los primeros
160 s de cada sesión (ventana alineada: el mismo arranque, intro y primeros mapas):

| Métrica (ventana alineada, 160 s) | Sesión 16 (antes) | Sesión 17 (después) | Diferencia |
| :--- | ---: | ---: | ---: |
| FPS medio | 58,41 | 59,41 | +1,00 |
| Segundos por debajo de 58 FPS | **28** | **8** | −71 % |
| `ejec` (CPU de emulación) | 14,67 ms | 13,53 ms | **−1,14 ms (−7,8 %)** |
| `ppu` (total PPU) | 8,89 ms | 7,53 ms | **−1,36 ms (−15,3 %)** |
| `ppubg` (fondos, **sin tocar**) | 3,18 ms | 3,21 ms | +0,03 ms (+0,9 %) |
| `ppuspr` (sprites) | 1,43 ms | 1,35 ms | −0,08 ms (−5,6 %) |
| `ppucomp` (composición, **tocada**) | 3,81 ms | 2,96 ms | **−0,85 ms (−22,3 %)** |
| `apu` (audio, **sin tocar**) | 1,48 ms | 1,52 ms | +0,04 ms (+2,7 %) |
| `ejec − ppu` (despacho + lógica, **sin tocar**) | 5,78 ms | 6,00 ms | +0,22 ms (+3,8 %) |

Los tres subsistemas que no se tocaron (`ppubg`, `apu` y el despacho con la lógica) se mueven dentro
del ±4 %: funcionan como control de que la máquina no cambió de velocidad, igual que el A/B de host.
La caída está donde se puso el trabajo.

**El contador duplicado, medido y no modelado.** El paso 1 no se puede validar con el `reloj=` del
encabezado (ver «Fuera de alcance»), pero sí con la identidad de los contadores, que es justo lo que
el paso eliminó. En la sesión 16, `ppu` superaba a `ppubg+ppuspr+ppucomp` en **+0,473 ms de media**,
en **255 de 255** líneas, hasta +0,60 ms: esa era la reconciliación del envoltorio de `mem.rs`,
pagada en cada fotograma. En la sesión 17 la misma diferencia es **+0,008 ms de media**, con un
máximo de 0,10 ms y **ninguna** línea por encima del redondeo de impresión (0,1 ms por campo).

**Reparto de la ganancia.** De las 1,14 ms/fotograma ganadas en `ejec`, **0,47 ms** son la
reconciliación suprimida (medida directamente por el hueco de la identidad, no estimada) y **0,85 ms**
son las etapas de la PPU, con `bg` y `spr` planos. De esos 0,85 ms, la parte puramente algorítmica de
la especialización de `compose` es **como mucho** la que proyecta el A/B de host (−18 % de `compose`,
≈0,35 ms a la escala de este dispositivo): el resto es el reloj barato abaratando también la medición
*dentro* de cada etapa. Este par de sesiones **no permite separar** los dos efectos, así que la cifra
no se reparte a ojo.

### Archivos modificados

- Motor (`../gba-recomp`):
  - `crates/gba-core/src/perf_clock.rs` (**nuevo**, 120 líneas): reloj monótono barato.
  - `crates/gba-core/src/lib.rs`, `mem.rs`, `ppu.rs`: reloj por línea, `ppu_ns` como suma de partes,
    `top_two_layers`, especialización de `compose` y lecturas perezosas de OAM.
  - `crates/recomp/Cargo.toml`, `crates/recomp/src/main.rs`: feature `perf-counters` y salida
    `RECOMP_PPU_STATS` del arnés de medición.
- Port: este archivo. El APK `target/blazing-recomp-release.apk`
  (`sha256 c341420a86a86b38ee25a3a9d53b95d8ccd3ca0d88984c57ae62859005a33517`, 42,38 MB, firmado v2/v3).

¿Coincide con lo anunciado? No del todo: el apartado 7 no incluía un archivo nuevo del motor
(`perf_clock.rs`) ni la feature del arnés, porque la propuesta los describió como «el reloj barato» y
«un arnés de una medición, aún por escribir en el paso 1». Se declaran aquí.

### Discrepancias

- **La primera medición antes/después fue inválida y no se usa.** Dio PPU 1,482 → 0,539 ms/fotograma
  (−64 %) y tiempo de pared 19 s → 6 s (−68 %), una mejora imposible: los cambios solo tocan la PPU,
  que era el 56 % del fotograma en host, así que no pueden explicar un 2,8× del proceso entero. El
  control lo delató: el **APU**, que no se tocó, bajó de 0,318 a 0,114 ms (factor **2,79**), y el
  `bg`, también intacto, coincidió con la predicción de ese factor (0,785/2,79 = 0,281 frente a 0,285
  medido). La conclusión es que **la máquina estaba más lenta durante la línea base**, no que el
  código ganara eso. Los datos quedan aquí como registro, marcados como medidos con máquina caliente
  y no comparables.
- **A/B válido (misma sesión, misma máquina ociosa, con `bg`/`spr`/`apu` como controles):**
  `compose` 0,193 → 0,158 ms/fotograma (**−18 %**) y PPU 0,579 → 0,539 (**−7 %**), mientras los
  controles no se mueven (bg 0,289/0,285, spr 0,098/0,096, apu 0,116/0,114). La especialización de
  `compose` se queda con esa evidencia; el resto del A/B se hizo con `if false &&` sobre la guarda y
  se restauró después, verificando que el hash y los tests se mantenían.
- **El paso 3 (lecturas perezosas de OAM) no tiene A/B propio.** Su corrección está cubierta por el
  hash y los tests, pero no aislé su ganancia. Queda como deuda de verificación.
- La propuesta decía que el arnés del motor estaba «aún por escribir»; resultó que el arnés ya existía
  (`runc`) y lo que faltaba eran los contadores, que son 26 líneas.

### Fuera de alcance (hallazgos que no se arreglaron)

- **La telemetría sigue sin identificar el reloj que calibra** (impacto: bajo-medio). El encabezado
  publica `reloj=1384ns` o `1891ns`, pero ese número mide el `Instant::now()` del **port**
  ([`src/perf.rs:305`](../../src/perf.rs#L305)), no el contador del sistema que el motor usa desde el
  paso 1. Por eso la diferencia 1891 → 1384 ns **no** es evidencia de esta propuesta: es el mismo API
  medido en días distintos. Sin cambiar esto, ninguna sesión futura podrá comprobar el coste del
  reloj del motor desde la telemetría.
- **Pico de presentación de 505,3 ms** (impacto: medio). La sesión 17 tiene un fotograma en t=78 s con
  `dibujo=13.7/507.0` y `presentmax=505.3`; la 16 nunca pasó de 20,2 ms. Es una congelación visible de
  medio segundo y, con una sola aparición, no se puede atribuir ni descartar como regresión de esta
  propuesta. Necesita otra sesión para saber si es sistemático.
- **`docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md` versiona una ruta absoluta local**
  (impacto: bajo, higiene y privacidad). Su apartado de comprobación cita el log de medición con el
  nombre de la carpeta de descargas del equipo delante, contra la regla de `AGENTS.md` §6. Se deja
  como está por no reescribir el registro de otra propuesta; el arreglo es quedarse con el nombre
  del archivo.
- **6,00 ms de despacho y lógica de juego** (impacto: alto). Está sin tocar y ya es el bloque más
  grande del fotograma, por delante de cualquier etapa de la PPU (`comp` 2,96 ms). Es el tema de la
  próxima propuesta, no de esta.
- **7.483 pasos de intérprete en IWRAM** (impacto: bajo). El hotspot superior es `0x030035C8` (código
  copiado a IWRAM en tiempo de ejecución); es el 0,0 % de los eventos de despacho —126 pasos en ROM en
  167 s— y no explica ninguna caída.
- **Evidencia fuera del repositorio** (impacto: bajo). Los registros de las sesiones 16 y 17 que
  sustentan la tabla no están versionados, así que sus números no se pueden reproducir con
  [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) desde un clon limpio.
  El precedente del repositorio es `scripts/diagnostico/log_tv15.txt`; snapshotearlos crea archivos,
  así que es un cambio aparte.

---

### Para la próxima vez

- Una comparación de rendimiento se hace **dentro de la misma sesión**, con un **subsistema intacto
  como control**. Un factor común a todas las fases (incluidas las que no se tocaron) es una máquina
  distinta, no un código mejor. Comparar dos builds con minutos de diferencia y sin control no vale.
- El reloj del sistema se mide antes de confiar en cualquier cronometraje fino: en este dispositivo
  cuesta 1,9 µs, tres órdenes de magnitud por encima de lo normal, y eso cambió la prioridad del
  trabajo.
- El campo `reloj=` de la telemetría **no mide la primitiva que se cambió**: antes de usarlo como
  prueba de un cambio de reloj, comprobar en [`src/perf.rs`](../../src/perf.rs) qué API cronometra.
- Dos sesiones de duración distinta no se comparan enteras: se elige una ventana común (aquí, los
  primeros 160 s) para que el contenido recorrido sea el mismo, y se usan los subsistemas intactos
  como control también en el dispositivo, no solo en host.

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Sesión en el Android TV en una zona que hoy cae: ¿algún segundo por debajo de 58 FPS? (escenario
  visible): ✔ | ✘ | ?
- `ejec` promedio de esa sesión y FPS mínimo observado: ✔ | ✘ | ?
- El juego se ve igual que antes en mapas, menús y combates (sin artefactos de composición): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
