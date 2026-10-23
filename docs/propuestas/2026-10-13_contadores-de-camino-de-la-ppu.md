# Propuesta: contadores de camino de la PPU y marca de compilación en el encabezado

Fecha: 2026-10-13
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La telemetría del dispositivo pasa a decir **qué camino de composición tomó cada línea** de la PPU y
**qué versión del renderizador** está corriendo. En concreto:

1. Cuatro contadores por fotograma que reparten las 160 líneas entre las cuatro razones posibles:
   compuesta por la **vía rápida**, o descartada por **efectos de color** (`BLDCNT` ≠ 0), por
   **ventana de sprites** (`OBJWIN`) o por un **sprite translúcido**. Los cuatro suman 160, así que no
   hay líneas sin explicar.
2. Una línea de telemetría `# ppucamino rectas=N mezcla=N ventana=N semi=N (rectas=X%)`, con el mismo
   estilo y cadencia que `# regiones` y `# hotspots`.
3. Una marca de compilación en el encabezado: `# backend=… reloj=…ns ppu=recto-1`.

El objetivo no es optimizar nada: es que la próxima sesión del televisor **conteste por sí sola** si
la vía rápida de [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)
se ejecuta en juego real y, si no se ejecuta, por qué.

### 2. Contexto

- Contexto consultado: la sesión 18 del Android TV (`blazing-perf-tv-18.txt`, fuera del repositorio),
  comparada por ventana alineada de 133 s con las sesiones 16 y 17 del mismo equipo
  (`blazing-perf-tv-16.txt`, `blazing-perf-tv-17.txt`, también fuera del repositorio); el cierre de
  [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md);
  y del motor, `crates/gba-core/src/ppu.rs` (`compose`, `render_sprites`) y `mem.rs`
  (`GbaPerfCounters`).
- Intención: cerrar la duda que dejó la sesión 18. Esa sesión **no permite atribuir nada al cambio**:
  junto a las fases tocadas subieron las tres que no se tocaron (`apu` +40 %, sprites +23 %, despacho
  y lógica de 4,40 a 6,70 ms), con +51 % de trabajo nativo por fotograma, y el usuario confirma que
  fue el mismo sitio que la sesión 17 **pero sin saltar la intro ni las cutscenes**. La distribución
  de la composición es bimodal (p25 1,80 ms; 3,80-4,00 ms clavados en el tramo pesado), un valor
  constante que se parece al coste por píxel del camino general. Todo apunta a que en esa escena la
  vía rápida no se toma —los desvanecidos de color de la intro y de los diálogos ponen `BLDCNT` ≠ 0—
  pero hoy no hay forma de comprobarlo desde el registro.
- Restricciones y decisiones previas (máx. 3):
  - **El formato de las líneas `t=` no se toca** (origen: `2026-10-12_telemetria-sin-coste-en-el-apk.md`
    y los tests de `src/perf_tests.rs`) → implica: el dato nuevo viaja en una línea de comentario `#`
    nueva, como `# regiones`, y en el encabezado → protegida por: *el reparto por fases no cambia*.
  - **El instrumento no puede ser el trabajo** (origen: la misma propuesta cerrada) → implica: los
    contadores son cuatro sumas enteras por línea, sin reloj, y el contador se decide una vez por
    línea dentro de la guarda que ya existe → protegida por: *el coste de medida es despreciable*.
  - **La salida visual es intocable** (origen: los tres hashes guardianes del arnés) → implica: contar
    no puede cambiar un píxel → protegida por: *el dibujado no cambia ni un píxel*.
- Alternativas descartadas:
  - **Deducirlo del log actual** → imposible: `ppucomp` mide tiempo, no la razón de cada línea, y
    «3,8 ms constantes» admite las dos hipótesis.
  - **Medir solo con el arnés de host** → el host ya dice que la vía rápida cubre ~74 % de las líneas
    de *esa* escena, pero la escena del televisor es otra: el arnés no ve menús ni desvanecidos.
  - **Añadir campos a la línea `t=`** → rompe el formato que leen el analizador y los tests.
- Ante imprevistos, prioriza: que el instrumento no cambie el comportamiento medido por encima de la
  comodidad del formato.

### 3. Ejemplos

- **Caso normal** (una línea con `BLDCNT` en modo 0, sin ventana de sprites y sin píxel translúcido):
  antes: se compone con la vía rápida y no queda constancia;
  ahora: `lineas_rectas` sube 1 en esa línea y `# ppucamino` lo ve.
- **Caso de esquina** (línea con desvanecido en curso, `BLDCNT` modo 1):
  antes: se compone por el camino general y la telemetría solo enseñaba el tiempo;
  ahora: `descartes_mezcla` sube 1, y si el 100 % de las líneas cae ahí el registro lo dice sin
  ambigüedad.
- **Caso de identidad** (cualquier fotograma): `rectas + mezcla + ventana + semi == 160`.
- **Caso de versión** (cualquier sesión): el encabezado pasa de
  `# backend=OpenGL modo=AOT present=AutoVsync alpha=Opaque reloj=2549ns` a
  `# backend=OpenGL modo=AOT present=AutoVsync alpha=Opaque reloj=2549ns ppu=recto-1`, así que un
  registro dice por sí solo qué PPU lo produjo.

### 4. Pasos

Motor (`../gba-recomp`):

1. `crates/gba-core/src/mem.rs`: `GbaPerfCounters` incorpora `lineas_rectas`, `descartes_mezcla`,
   `descartes_ventana` y `descartes_semi` (contadores de suma, sin reloj).
2. `crates/gba-core/src/ppu.rs`: en `compose`, la guarda que decide el camino cuenta una vez por
   línea, con la misma precedencia que la evaluación (efectos → ventana → translúcido → recto).
3. `crates/gba-core/src/ppu.rs`: constante pública `PPU_CAMINO_VERSION` con la marca de compilación.

Port:

4. `src/runner.rs`: los cuatro contadores del fotograma se acumulan en el port al leer los contadores
   del motor, con el mismo patrón que los de región.
5. `src/recomp.rs`: `PpuPathCounters` (sumas acumuladas + `to_line()` con porcentajes de líneas
   rectas) y `ppu_path_counters()`, en el estilo de `RegionCounters`.
6. `src/platform.rs`: línea `# ppucamino` junto a `# regiones` (cada 2 s) y `ppu=…` en el encabezado
   de la telemetría.
7. Tests: motor (`crates/gba-core/tests/perf_counters_tests.rs`, suma de 160 y conteo por razón) y
   port (`src/recomp_tests.rs`, formato y porcentajes con casos límite de cero líneas).
8. Documentación: `docs/guias/medir-rendimiento.md` (la línea nueva) y
   `docs/referencia/api-modulos-rust.md` (regenerada con `scripts/extraer_referencias.py`).

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "perf_counters" ../gba-recomp/crates/`, `grep -rn "take_perf_counters"
src/`, y `grep -n "reloj=" src/`.

- `src/perf.rs` y el formato de las líneas `t=`: **no cambian** (es el punto).
- `src/perf_tests.rs`, `src/pipeline_tests.rs`: no construyen `GbaPerfCounters`; no se tocan.
- `scripts/diagnostico/analizar_telemetria.py`: hoy ignora las líneas `#` que no conoce, así que la
  línea nueva no lo rompe; **no se le añade parseo** en esta propuesta.
- `tests/packaging_tests.rs`: comprueba el APK, no el formato de la telemetría; el APK cambia de
  tamaño por el cambio del motor, y eso ya lo cubre su propio test.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Los cuatro contadores suman 160 por fotograma** (una línea, una decisión) → prototipar: no, es
  aritmética de la guarda → si es falso (una línea pasara dos veces por `compose`), la identidad lo
  delata en el test y el porcentaje deja de ser interpretable.
- **El reparto de razones de la escena del televisor es estable dentro de un segundo** → prototipar:
  no → si es falso, la línea acumulada sigue valiendo (se leen diferencias entre líneas) y el
  porcentaje se vuelve ruidoso pero no falso.

### 7. Alcance

- **Incluye:**
  - Los cuatro contadores en el motor y su conteo por línea, y la marca de compilación.
  - La acumulación en el port, la línea `# ppucamino` y el campo del encabezado.
  - Los tests de las dos partes y la documentación de la línea nueva.
  - El APK con el instrumento, para la siguiente sesión.
- **No incluye:**
  - Optimizar nada de la PPU ni cambiar qué líneas toma la vía rápida.
  - Tocar el formato de las líneas `t=`, el analizador de telemetría o la guía de campaña de medidas.
  - Subir el motor al repositorio oficial y re-fijar el SHA (trabajo aparte, ya abierto).

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: las líneas se reparten sin dejar ninguna sin explicar** (normal)
Caso de uso: cualquier fotograma del motor | Tipo: normal
  Dado un fotograma con `BLDCNT` en modo 0, sin ventana de sprites y sin sprites translúcidos
  Cuando se llama a `take_perf_counters()` tras el fotograma
  Entonces `descartes_mezcla == 0`, `descartes_ventana == 0`, `descartes_semi == 0` y
  `lineas_rectas == 160`
Test: `crates/gba-core/tests/perf_counters_tests.rs` | Estado previo: **falla** (los campos no existen)

**Escenario: los efectos de color explican el descarte** (límite)
Caso de uso: un fotograma con `BLDCNT` pidiendo brillo | Tipo: límite
  Dado el mismo fotograma con `BLDCNT` en modo 1
  Cuando se leen los contadores
  Entonces `descartes_mezcla == 160` y `lineas_rectas == 0`
Test: `crates/gba-core/tests/perf_counters_tests.rs` | Estado previo: **falla**

**Escenario: la línea de camino y el encabezado se formatean** (normal)
Caso de uso: la telemetría del port | Tipo: normal
  Dado un acumulado de `rectas=1280 mezcla=320 ventana=0 semi=0` sobre 10 fotogramas
  Cuando se formatea
  Entonces la línea es `# ppucamino rectas=1280 mezcla=320 ventana=0 semi=0 (rectas=80%)` y con
  acumulado vacío no se escribe ninguna línea
Test: `src/recomp_tests.rs` | Estado previo: **falla**

**Escenario (regresión): el dibujado no cambia ni un píxel** (regresión)
Caso de uso: el instrumento en el camino caliente | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario
  Cuando se ejecutan 600, 2400 y 7200 fotogramas
  Entonces los hashes son `31184031261ea2ec`, `b54066dc06f2b76c` y `b54066dc06f2b76c` con
  `fallback_steps: 0`
Test: `recomp runc --bios roms/gba_bios.bin --frames 600|2400|7200` | Estado previo: **pasa**

### 9. Cómo lo comprobaré

- **El reparto es exacto y el dibujado no cambia** → comandos:
  `cargo test -p gba-core` en el motor y
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: los tests están en verde y el hash sigue siendo `b54066dc06f2b76c`
  → no detecta: que la escena de 7200 fotogramas use la vía rápida; precisamente para eso se cuenta.
- **La línea nueva llega al registro** → comando: `cargo test` en el port y, si hay TV disponible,
  una sesión corta leyendo `blazing-perf.txt` → pasa si: aparece `# ppucamino` con los cuatro campos
  y su porcentaje, y el encabezado lleva `ppu=recto-1`.
- **El coste del instrumento es despreciable** → comando: el mismo `runc` con la sonda de la PPU
  (`RECOMP_PPU_STATS=1`) antes y después → pasa si: `ppu` se mantiene dentro del ±2 % de la línea base
  de la sesión de medición anterior en el mismo host.
  → no detecta: el efecto en otros dispositivos.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — contar la línea en el camino recto sin evaluar la guarda → debe morir la identidad de 160.
M2 — intercambiar `descartes_mezcla` por `descartes_ventana` → debe morir el escenario de límite.
Los archivos se respaldan en RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los tres escenarios de código pasan, el hash de 7200 fotogramas no cambió, la suite del motor y la
del port están en verde, la línea y el encabezado están documentados, y el APK con el instrumento
está listo para la siguiente sesión.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- La identidad de 160 no se cumple y no se explica en un intento: el porcentaje no es interpretable,
  así que se retira la línea en vez de publicarla con un número engañoso.

---

## Resultado (lo completa el agente después de editar)

Estado: `verificada por agente`; falta la sesión del televisor, que es justo lo que esta propuesta
habilita.

### Predicho vs. real

- **Las líneas se reparten sin dejar ninguna sin explicar**: coincide. El conteo sale de la **misma
decisión** que elige el camino (`razon_camino`), no de una segunda evaluación de la guarda, así que el
instrumento no puede divergir del código. `straight_path_accounts_for_every_line` comprueba que en una
línea sin capas la cuenta recta sube 1 y la suma de las cuatro es 1.
- **Los efectos de color explican el descarte**: coincide.
`colour_effect_sends_the_line_to_the_general_path` pone `BLDCNT` en modo 1 y obtiene
`descartes_mezcla == 1`, `lineas_rectas == 0` y los otros dos a cero.
- **La línea de camino y el encabezado se formatean**: coincide.
`test_contadores_de_camino_formatean_y_porcentuan` fija el formato exacto
(`# ppucamino rectas=1280 mezcla=320 ventana=0 semi=0 (rectas=80%)`), el redondeo (1 de 3 → 33) y el
caso vacío (sin líneas no se inventa porcentaje). El encabezado lleva `ppu=` tomado de
`gba_core::ppu::PPU_CAMINO_VERSION`, no de una cadena escrita a mano en el port.
- **El dibujado no cambia ni un píxel**: coincide. Los tres hashes guardianes siguen intactos con el
contador en el camino caliente: `600 → 31184031261ea2ec`, `2400` y `7200 → b54066dc06f2b76c`, con
1.776.798 / 2.167.398 / 3.208.998 bloques y `fallback_steps: 0`.
- **El coste del instrumento es despreciable**: coincide, medido con un control intacto (abajo).

### Medición del coste del instrumento (host, 7.200 fotogramas, mismo contenido)

| ms por fotograma | sin contadores | con contadores | razón contra `apu` |
| :--- | ---: | ---: | :--- |
| `ppu` | 0,367 | 0,434 | 3,19 → 3,19 |
| `bg` | 0,197 | 0,234 | 1,71 → 1,72 |
| `spr` (sin tocar) | 0,099 | 0,116 | 0,86 → 0,85 |
| `compose` | 0,071 | 0,084 | 0,62 → 0,62 |
| `apu` (sin tocar, control) | 0,115 | 0,136 | — |

Los absolutos suben ~18 % en la segunda corrida, pero sube lo mismo el `apu`, que no se tocó: la
máquina estaba más lenta (la corrida con contadores se hizo justo después de una compilación de
2m41s). Como el contenido es **el mismo** —el hash de 7.200 fotogramas es idéntico—, `apu` sirve de
control y las razones quedan idénticas a tres cifras: el coste de contar no es medible en host. La
suma de las etapas sigue cerrando contra `ppu` (0,234 + 0,116 + 0,084 = 0,434).

### Archivos modificados

- Motor (`../gba-recomp`):
  - `crates/gba-core/src/mem.rs`: cuatro campos nuevos en `GbaPerfCounters`.
  - `crates/gba-core/src/ppu.rs`: `razon_camino` en `compose` (una decisión, un conteo),
    `PPU_CAMINO_VERSION` y dos tests unitarios.
- Port:
  - `src/recomp.rs`: `PpuPathCounters` (formato y porcentaje) y su acumulación por fotograma.
  - `src/runner.rs`: volcado del reparto al leer los contadores del motor.
  - `src/platform.rs`: línea `# ppucamino` junto a `# regiones` y `ppu=` en el encabezado.
  - `src/recomp_tests.rs`: test de formato y porcentajes.
  - `src/runner_tests.rs`: test del cableado `step_frame` → acumulador, por diferencia y bajo el
    candado de acumuladores; mutado a mano (intercambiar `lineas_rectas` y `descartes_mezcla` en
    `add_ppu_path`) el test falla, y el archivo se restauró byte a byte desde memoria.
  - `docs/guias/medir-rendimiento.md`: la línea nueva, el campo del encabezado y cómo leerlos.
  - `docs/referencia/api-modulos-rust.md`: regenerada.
  - `target/blazing-recomp-release.apk`: el APK con el instrumento.

### Discrepancias

- **El criterio del coste pedía ±2 % en el mismo host y no se midió en absoluto.** La máquina cambió
  entre las dos corridas, así que se comprobó con la razón contra `apu` y contenido idéntico; queda
  declarado en vez de presentar una mejora o una pérdida que no se puede sostener.
- **La línea es un acumulado de sesión, no una ventana.** Se eligió así para no tocar `FrameTiming` ni
  `PerfReport`, que son los structs del formato que leen el analizador y los tests; para leer la escena
  actual hay que restar dos líneas consecutivas, y la guía lo dice.

### Fuera de alcance (hallazgos que no se arreglaron)

- **El analizador de telemetría no parsea `# ppucamino`** (impacto: bajo). La línea se lee a mano o
  restando dos consecutivas; añadirle una sección es trabajo de otra propuesta, y solo compensa si el
  dato se usa en cada sesión.
- **El cambio del motor sigue sin commitear** (impacto: medio). El APK lo lleva por el `[patch]`; un
  build que no use la ruta local perdería la vía rápida *y* sus contadores.
- **El motor advierte de un `const FALLBACK` sin usar en `hostclock.rs`** (impacto: bajo). Es código
  del motor upstream sin tocar por este cambio (`hostclock.rs` no aparece modificado en `git status`),
  así que no se corrigió aquí.
- **Nada impide que un contador se quede obsoleto en silencio** (impacto: bajo). Si mañana cambia la
  guarda y no se actualiza `razon_camino`, los tests de reparto seguirían pasando mientras la línea
  miente; hoy la decisión es una sola variable, así que el riesgo es pequeño, pero no hay un test que
  ate el reparto a la salida real del camino recto (el hash ata los píxeles, no el reparto).

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Sesión corta en el Android TV con el APK instrumentado: ¿el encabezado lleva `ppu=recto-1` y aparecen
  líneas `# ppucamino`? ✔ | ✘ | ?
- Las cuatro cuentas de `# ppucamino` suman múltiplos de 160 y el reparto describe la escena: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
