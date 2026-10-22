# Propuesta: La telemetría deja de tener dos dueños

Fecha: 2026-10-22
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El informe de rendimiento deja de poder perder datos en silencio. La lista de campos de la línea de
telemetría por segundo pasa a estar escrita en **un único sitio** (un esquema), del que beben tanto el
motor que la escribe como el analizador que la lee; el orden de los campos deja de importar; y un
campo que el analizador no reconoce se **reporta** en vez de descartarse sin dejar rastro.

### 2. Contexto

- Contexto consultado: `src/perf.rs` (`PerfReport::to_line`, `PpuHwWindowStats::to_line`,
  `FramePerf::close_window`), `src/recomp.rs` (`RegionCounters::to_line`, `PpuPathCounters::to_line`,
  `Region::NAMES`), `src/platform.rs` (`init_perf_file`, `write_perf_report`, escritura de
  `# hotspots top=`), `scripts/diagnostico/analizar_telemetria.py` (`parsear_telemetria`,
  `imprimir_informe`), `scripts/diagnostico/log_tv15.txt`, `docs/guias/medir-rendimiento.md`,
  `docs/BACKLOG.md` (registro de hallazgos), `Cargo.toml`, `tests/packaging_tests.rs`,
  `.agents/skills/propuesta-verificable/assets/plantilla-propuesta.md`.
- Intención: hoy el contrato de la telemetría está escrito **cuatro veces**: el `format!` de Rust que
  la emite (23 claves en un orden fijo), el regex posicional de Python que la lee (que fija ese mismo
  orden), la tabla de campos de la guía, y la línea de ejemplo de la guía. Cuando el escritor y el
  lector se desenfasan, el lector **falla en silencio**: su regex devuelve `None`, la línea no se
  cuenta, y el informe sale con medias sesgadas sin ningún aviso. El éxito es que el analizador no
  pueda perder un campo sin que se note, y que añadir un campo sea tocar un fichero, no dos
  lugares que tienen que coincidir.
- Restricciones y decisiones previas (máx. 3):
  - El motor no puede depender de crates nuevos (`Cargo.toml` no declara `serde`, `toml` ni `json`, y
    no hay `build.rs`) (origen: `Cargo.toml`) → implica: el esquema se declara en un formato que
    Rust y Python leen sin dependencias, y su lectura va dentro del crate → protegida por «El
    esquema se lee sin dependencias nuevas» | testeable.
  - `src/platform.rs` es `#[cfg(target_os = "android")]` y **no compila en host** (origen:
    `src/lib.rs:44`) → implica: el marcador de versión del contrato va dentro de la línea `t=`, que
    vive en `src/perf.rs` y sí se prueba en host, y no en la cabecera, que escribe `platform.rs` →
    protegida por «La línea declara su versión y esa versión se lee» | testeable.
  - Los registros ya escritos (los que hay ahora en los dispositivos y
    `scripts/diagnostico/log_tv15.txt`) tienen que seguir leyéndose (origen: respuesta humana,
    2026-10-22) → implica: el analizador acepta líneas sin marcador de versión y las interpreta con
    los campos que encuentre → protegida por «Los registros antiguos sin versión se siguen leyendo» |
    testeable.
- Alternativas descartadas (solo si las hubo):
  - Que el módulo de Rust sea el dueño (los nombres de campo como constantes) → descartada porque el
    lector seguiría teniendo su propia lista: seguiría habiendo dos dueños, sólo que uno en código
    y otro en Python, y el esquema dejaría de ser un artefacto que se puede leer sin compilar.
  - Que el analizador pase a ser un binario de Rust y Python salga del camino de producción →
    descartada porque Python ya es requisito del *gatekeeper* de pre-commit
    (`validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py`), así que el
    analizador es una pieza legítima del proceso, y porque el problema que se quiere resolver (la
    deriva entre escritor y lector) no necesita ese traslado.
  - Emitir JSON en vez de texto plano → descartada porque `blazing-perf.txt` se abre y se compara a
    mano en el dispositivo; cambiar el artefacto es otro problema y otro alcance.
  - Meter el marcador de versión en la cabecera → descartada por la segunda restricción: la cabecera
    la escribe `platform.rs`, que no se puede probar en host.
- Ante imprevistos, prioriza: que ningún campo se pierda en silencio > que el informe cambie lo
  mínimo > que el esquema quede bonito.

### 3. Ejemplos

- Caso normal: la línea `t=2s fps=60.0 ejec=13.6/21.9 ppu=7.7 ... natives=4945 interprete=2 ...`
  → antes: el regex posicional de `analizar_telemetria.py` (línea 72) saca once valores con once
  grupos capturados; si mañana alguien renombra `ppuspr` o inserta un campo entre `ppu` y `ppubg`, el
  regex deja de casar, `m` queda en `None`, **el segundo desaparece del informe sin error** y las
  medias salen sesgadas → ahora: el esquema declara los 23 campos con su tipo, el lector los busca
  **por clave**, así que el orden deja de importar, y un campo que no está en el esquema sale como
  anomalía en el informe.
- Caso límite: una línea vieja sin marcador, como las 229 de `scripts/diagnostico/log_tv15.txt`
  (del 2026-10-12, antes de que existieran `pump` y `pumpm`) → antes: se leen con el regex y sus 21
  campos se interpretan bien → ahora: se leen igual, con los 21 campos que la línea trae; los que
  falten se muestran como ausentes, sin error y sin quejarse.
- Caso límite: una línea con un campo que el esquema no conoce (`... zzz=1`) → antes: el campo se
  ignora en silencio → ahora: el informe dice `1 campo desconocido: zzz` y la línea sigue
  aportando sus campos conocidos.
- Caso normal: el número de «peor segundo» de la CPU → antes: 35.50 ms en `log_tv15.txt`, que es el
  máximo **de las medias** de cada segundo (el valor `21.9` del par `ejec=13.6/21.9` se tira) →
  ahora: el informe conserva ese 35.50 ms, con la etiqueta honesta «peor media de segundo», y
  **añade** «peor fotograma: 98.40 ms», que es el máximo de los máximos. El número que se usaba
  antes no cambia de valor; lo que se añade es el que faltaba.

### 4. Pasos

1. Se crea `telemetria/esquema.txt`: la lista de campos de la línea `t=` (nombre, tipo, unidad y
   significado) y las versiones del contrato.
2. Se crea `src/telemetria.rs`: el lector del esquema y el renderizador de una línea a partir de él.
3. `PerfReport::to_line()` deja de ser un `format!` de 28 huecos (23 campos, algunos con dos
   valores) y pasa a renderizarse guiado por la lista de campos del esquema.
4. `analizar_telemetria.py` deja de usar el regex posicional: lee el esquema, parte la línea en
   pares `clave=valor`, convierte cada valor según el tipo declarado y anota las anomalías.
5. Se añade el test de oro en `tests/telemetria_contrato_tests.rs`: Rust escribe una línea real y
   Python la relee; los valores tienen que coincidir uno a uno.
6. Se actualizan la guía de medición y la referencia de módulos, se registra en `GLOSSARY.md` el
   vocabulario que este cambio acuña (telemetría, línea de segundo, esquema de telemetría) y se
   registran en el BACKLOG los hallazgos que quedan fuera de alcance.

### 5. Qué más podría verse afectado

- `src/perf.rs` — `PerfReport::to_line()` (línea 179) y `PpuHwWindowStats::to_line()` (línea 100).
  La segunda **no se toca**: la línea `# ppu_hw` ya se lee con un barrido genérico `clave=valor` y no
  tiene contrato posicional (fuera de alcance).
- `src/recomp.rs` — `RegionCounters::to_line()` (línea 243) y `PpuPathCounters::to_line()`
  (línea 310). No se tocan: también se leen con barrido genérico. Se anotan sus **colisiones de
  clave** (`rectas` absoluto y `rectas` porcentual en la misma línea) como hallazgo fuera de alcance.
- `src/platform.rs` — `write_perf_report()` (línea 1923) y `init_perf_file()` (línea 1536). No se
  tocan: escriben el resultado de `to_line()` tal cual, y la cabecera no entra en el alcance.
- `docs/guias/medir-rendimiento.md` — la tabla de campos y la línea de ejemplo; hoy **ya están
  desfasadas** (no mencionan `pump` ni `pumpm`, que el motor emite desde el 2026-10-14).
- `docs/referencia/api-modulos-rust.md` — se regenera con `python scripts/extraer_referencias.py`,
  pero **no basta con regenerarlo**: el extractor tiene una lista fija de módulos
  (`scripts/extraer_referencias.py`, líneas 234-244) y `telemetria.rs` hay que añadirlo a ella.
- `src/perf_tests.rs` — dos tests comparan la línea de telemetría **completa** con `assert_eq!`
  (`test_informe_promedia_y_reporta_peor_caso`, línea 92, y
  `test_ventana_sin_fotogramas_no_divide_por_cero`, línea 153): con el sello de versión los dos
  tienen que ganar ` formato=2` al final de la cadena esperada.
- `scripts/extraer_referencias.py` — la lista fija de módulos recién citada (líneas 234-244).
- `scripts/diagnostico/log_tv15.txt` — **no está versionado**: lo ignora `.gitignore`
  (`scripts/diagnostico/log_*.txt`), así que en un clon limpio no existe y ningún test puede
  depender de él. Los criterios 3 y 4 usan un registro sintético que escribe el propio test, y las
  cifras del log real se quedan como comprobación manual.
- `docs/BACKLOG.md` — el índice de propuestas y el registro de hallazgos.
- `GLOSSARY.md` — no existe todavía en el repositorio. Este cambio acuña vocabulario que las
  revisiones arquitectónicas futuras van a necesitar (telemetría, línea de segundo, esquema de
  telemetría, camino de composición), así que se crea con esos términos. El archivo se crea en la
  Fase 3, no antes: hasta que el usuario apruebe el alcance, el repositorio no se toca.
- Búsqueda hecha: `grep -rn "to_line()\|blazing-perf\|t=" src/ tests/ scripts/ docs/` para
  enumerar escritores y lectores del formato. Los escritores de la línea `t=` son **sólo**
  `PerfReport::to_line()`; el único lector es `parsear_telemetria()`.

### 6. Suposiciones

- La línea `t=` sólo la emite `PerfReport::to_line()`, y ningún otro punto del motor escribe una
  línea con ese prefijo → prototipar: no
  → si es falso: habría que tocar también ese escritor, y la propuesta crece un archivo más.
  → resultado del prototipo: | no detecta: no cubre un escritor futuro que escriba su propia línea.
- Los registros que hay en los dispositivos y en `scripts/diagnostico/log_tv15.txt` son sólo de
  esta versión del formato (no hay mezclas de formatos dentro de un mismo archivo, porque
  `init_perf_file()` abre el archivo con `File::create`, que trunca) → prototipar: no
  → si es falso: el analizador tiene que distinguir líneas de formatos distintos dentro del mismo
  archivo; el diseño por-clave ya lo tolera, pero el aviso de anomalías podría saturar.
  → resultado del prototipo: | no detecta: no comprueba la mezcla en un archivo real.
- Python 3.11 o superior está en la máquina que corre `cargo test` → prototipar: sí
  → si es falso: el test de oro no se puede ejecutar y el contrato queda sin atar.
  → resultado del prototipo: se comprueba en la Fase 1 con `python --version` (ya está: 3.14.6) y
  con la existence del intérprete que el *gatekeeper* ya exige | no detecta: no comprueba que Python
  esté en el `PATH` de otra máquina.

### 7. Alcance

- Incluye:
  - El contrato de la línea de segundo (`t=`): lista de campos, tipos, versiones y el marcador de
    versión, en un esquema que es el dueño único.
  - El escritor Rust guiado por el esquema y el analizador Python que lo lee, con los dos juntos
    atados por un test de oro.
  - Los campos que el analizador no reconoce pasan a ser anomalías reportadas, no silencios.
  - El informe pasa a mostrar el «peor fotograma» de la CPU y del dibujo, que el motor ya emitía y
    el analizador tiraba.
- No incluye:
  - Las líneas `# ppu_hw`, `# regiones`, `# ppucamino` y `# hotspots`: ya se leen con barrido genérico
    `clave=valor` y no tienen contrato posicional. **Su colisión de clave (`rectas` absoluto frente a
    `rectas` porcentual) queda como hallazgo**, no como arreglo.
  - La cabecera del archivo y las líneas `#` de platform.rs: las escribe `platform.rs`, que no
    compila en host.
  - Los huecos que el BACKLOG ya registró sobre la telemetría (el reloj que calibra, el nombre
    `descartes_mezcla` que ya no describe lo que mide, la ausencia de identificación del APK): son
    problemas de fondo, no del contrato.
  - Renombrar `descartes_mezcla` o resolver la colisión `rectas`/`rectas%` de `# ppucamino`: son
    cambios de nombre de campo, y renombrar un campo es justo lo que esta propuesta vuelve barato
    (el esquema lo declara una vez), pero **no** se incluye aquí para que el alcance siga siendo
    sólo el contrato de la línea `t=`.
  - Mover el analizador a Rust, o cambiar el artefacto de texto plano a JSON.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La línea de segundo no pierde ningún campo aunque se reordene
Caso de uso: alguien añade un campo nuevo a la telemetría y lo pone en medio de la línea | Tipo: normal
  Dado que un `PerfReport` con valores conocidos (fps 60.0, ejec 13.6/21.9, ppu 7.7, nativos 4945, interprete 2, ventana 1280x720)
  Cuando `PerfReport::to_line()` la escribe y el analizador de Python la vuelve a leer
  Entonces los 23 campos del esquema vuelven con el mismo valor, y la lista de campos del esquema es
  exactamente la que emite el escritor Rust (mismos nombres, mismo orden)
Test: `tests/telemetria_contrato_tests.rs::test_la_linea_de_segundo_no_pierde_campos`
Estado previo: no se observó en rojo (el test se escribió después del código; ver «Discrepancias»). Sobre el código de HEAD el escenario es irrealizable: el lector no tiene forma de devolver los 23 campos.

Escenario: Un campo desconocido se reporta en vez de descartarse en silencio
Caso de uso: el motor emite un campo que el analizador no conoce porque el APK es más nuevo que el script | Tipo: error
  Dado que una línea de segundo con un campo extra `zzz=1` que el esquema no declara
  Cuando el analizador la procesa
  Entonces el informe dice `1 campo desconocido: zzz` y el número de campos conocidos leídos sigue
  siendo 23; el analizador no devuelve ningún error y la línea sigue contando para las medias
Test: `tests/telemetria_contrato_tests.rs::test_un_campo_desconocido_se_reporta`
Estado previo: no se observó en rojo (test escrito después del código). Sobre el código de HEAD el campo `zzz` se ignoraba sin dejar rastro en el informe.

Escenario: Los registros antiguos sin versión se siguen leyendo
Caso de uso: se abre un `blazing-perf.txt` descargado de un dispositivo con una versión anterior del APK | Tipo: regresión
  Dado que un registro de 4 líneas de segundo escrito con el contrato v1 (21 campos, sin `pump` ni
  `pumpm` y sin sello de versión), como el que el motor escribía antes de este cambio
  Cuando el analizador lo procesa
  Entonces cuenta 4 segundos, la media de FPS es 45.0, la media de `ejec` es 25.00 ms y el informe
  no señala ninguna anomalía de formato
Test: `tests/telemetria_contrato_tests.rs::test_los_registros_antiguos_se_siguen_leyendo`
Estado previo: pasa (regresión): el registro v1 es el que el analizador de HEAD ya leía bien, y
  el criterio fija que este cambio no puede romperlo.

Escenario: El informe dice el peor fotograma y no sólo la peor media
Caso de uso: se lee el informe para saber si hubo un tirón | Tipo: normal
  Dado que un registro cuyo segundo 3 trae `ejec=30.0/90.0` y cuyo segundo 4 trae `ejec=40.0/50.0`
  Cuando el analizador construye el informe
  Entonces el valor de la CPU es la media de la ventana (25.00 ms en el registro de prueba), el
  «peor media de segundo» es 40.00 ms (el máximo de las medias) y aparece además un «peor
  fotograma» de 90.00 ms, que es el máximo del segundo componente del par
Test: `tests/telemetria_contrato_tests.rs::test_el_informe_dice_el_peor_fotograma`
Estado previo: no se observó en rojo (test escrito después del código). Sobre el código de HEAD el
  segundo componente del par se tiraba, así que el valor no existía en ninguna salida.

### 9. Cómo lo comprobaré

- La línea de segundo no pierde ningún campo aunque se reordene → herramienta o comando:
  `cargo test --test telemetria_contrato_tests test_la_linea_de_segundo_no_pierde_campos`
  → pasa si: el test pasa y `cargo test --test telemetria_contrato_tests` sale con código 0
  → no detecta: no prueba que el esquema refleje lo que el motor *quiere* medir, sólo que escritor y
  lector coinciden.
- Un campo desconocido se reporta en vez de descartarse en silencio → herramienta o comando:
  `cargo test --test telemetria_contrato_tests test_un_campo_desconocido_se_reporta`
  → pasa si: el test pasa y el informe impreso contiene la cadena `campo desconocido`
  → no detecta: no comprueba el caso de un campo desconocido *con el mismo nombre* que uno conocido
  pero con otro tipo.
- Los registros antiguos sin versión se siguen leyendo → herramienta o comando:
  `cargo test --test telemetria_contrato_tests test_los_registros_antiguos_se_siguen_leyendo` y, a
  mano, `python scripts/diagnostico/analizar_telemetria.py --archivo scripts/diagnostico/log_tv15.txt`
  → pasa si: el test pasa y, en la máquina que tiene ese registro (no está versionado: lo ignora
  `.gitignore`), el informe manual da 229 segundos, 53.6 FPS y 16.96 ms de media de `ejec`
  → no detecta: no comprueba registros con la cabecera de una versión de APK que no existe, ni el
  log real, que no se puede reproducir desde un clon limpio.
- El informe dice el peor fotograma y no sólo la peor media → herramienta o comando:
  `cargo test --test telemetria_contrato_tests test_el_informe_dice_el_peor_fotograma`
  → pasa si: el test pasa y el informe manual de `log_tv15.txt` muestra 35.50 ms como «peor media de
  segundo» y 98.40 ms como «peor fotograma»
  → no detecta: no comprueba que el «peor fotograma» sea del fotograma correcto cuando hay más de un
  segundo con el mismo máximo, ni las cifras del log real, que no está versionado.
- El esquema se lee sin dependencias nuevas → herramienta o comando: `cargo tree -e normal` y
  `grep -n "serde\|toml\|json" Cargo.toml`
  → pasa si: no aparece ninguna dependencia nueva y `Cargo.toml` sigue sin declarar `serde`, `toml`
  ni `json`
  → no detecta: no comprueba que el esquema siga siendo legible por un tercero.
- La suite entera sigue en verde → herramienta o comando: `cargo test`
  → pasa si: sale con código 0 y el número de tests es el anterior más los nuevos
  → no detecta: no compila ni ejecuta el código de `platform.rs`, que es `cfg(android)`.
- La documentación sigue sincronizada → herramienta o comando:
  `python scripts/extraer_referencias.py --check` y
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  → pasa si: el primero sale con código 0 y el segundo con `0 error(es), 0 aviso(s)`
  → no detecta: no comprueba que la guía describa el formato *correcto*, sólo que sea coherente
  consigo misma.
- La propuesta está bien formada → herramienta o comando:
  `cargo test --test packaging_tests test_propuestas_declaran_estado_valido`
  → pasa si: el test pasa
  → no detecta: no comprueba el contenido de la propuesta.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como "discrepada".

### 12. Desglose de la implementación

Qué se toca, en qué orden y con qué comprobación en cada paso. Los nombres de fichero, de función
y de test son los definitivos: quien apruebe esto aprueba esos nombres.

#### 12.1 El artefacto único: `telemetria/esquema.txt`

El esquema es un fichero de texto plano en la raíz del repositorio, con una fila por dato y
separador ` | `:

```
version | <n> | hereda: <n|-> | anade: <campo>, <campo> | <descripción>
campo   | <nombre> | <tipo> | <unidad> | <descripción>
sello   | <clave> | <versión> | <descripción>
```

- `version` abre el bloque de campos que esa versión añade; `hereda` y `anade` lo dicen en prosa y
  el lector **comprueba** que los `campo` que siguen a la fila son exactamente los de `anade`.
- `campo` es un dato de la línea: `nombre`, `tipo` (`entero`, `entero_s`, `real`, `real/real`,
  `entero_x_entero`), `unidad` (`-` si no aplica) y qué mide.
- `sello` declara la clave que marca con qué versión del contrato se escribió la línea (`formato`)
  y a qué versión corresponde.

Contenido, en el orden canónico en que hoy escribe el motor:

- versión 1: los 21 campos de la línea del 2026-10-12 (`t`, `fps`, `ejec`, `ppu`, `ppubg`,
  `ppuspr`, `ppucomp`, `apu`, `lecturas`, `escrituras`, `color`, `dibujo`, `espera`, `subida`,
  `codificacion`, `present`, `presentmax`, `nativos`, `interprete`, `vacias`, `ventana`).
- versión 2: hereda la 1 y añade `pump` y `pumpm` (los que el motor escribe desde el 2026-10-14).
- sello: `formato` → versión 2.

Las dos versiones se leen. Una línea con `formato=2` se comprueba contra la lista de campos de su
versión (estricta: si le falta un campo, es anomalía). Una línea **sin sello** se lee en modo
tolerante: se usan los campos que se encuentren y los que falten no se echan de menos. Eso es lo
que cubre el registro del 2026-10-12 (21 campos) y el intermedio de 23 campos.

Quién lo lee:

- Rust: `include_str!("../telemetria/esquema.txt")` desde `src/telemetria.rs`. El fichero queda
  embebido en la biblioteca: el APK no lo lleva y no entra ninguna dependencia nueva.
- Python: `Path(__file__).resolve().parents[2] / "telemetria" / "esquema.txt"` desde
  `scripts/diagnostico/analizar_telemetria.py` (el script está dos niveles por debajo de la raíz).

#### 12.2 `src/telemetria.rs` (nuevo) y `src/lib.rs`

`src/lib.rs` gana `pub mod telemetria;`, una línea junto a `pub mod perf;`.

API pública (es la que aparece en `docs/referencia/api-modulos-rust.md`):

```rust
pub enum Tipo { Entero, EnteroS, Real, RealReal, EnteroEntero }
pub struct Campo { pub nombre: &'static str, pub tipo: Tipo, pub unidad: &'static str }
pub enum Valor { Entero(u64), Real(f64), RealReal(f64, f64), EnteroEntero(u32, u32) }
pub struct Esquema { /* campos, versión y sello */ }
pub fn esquema() -> Result<&'static Esquema, String>
pub fn escribir_linea(valores: &BTreeMap<&'static str, Valor>) -> Result<String, String>
```

Reglas:

- El esquema se analiza **una vez** (`OnceLock`) y un esquema mal formado es un `Err`, no un pánico.
- `escribir_linea` recorre los campos en el orden canónico, formatea cada valor según su tipo
  declarado (`entero` → `{}`; `entero_s` → `{}s`; `real` → `{:.1}`; `real/real` → `{:.1}/{:.1}`;
  `entero_x_entero` → `{}x{}`) y añade al final el sello `formato=2`.
- La `unidad` de un campo es **documentación** (para el humano que lee el esquema y la guía): no
  se imprime. La forma impresa la decide el tipo, y el único caso con sufijo es `entero_s`, que es
  el tipo de `t` (se escribe `t=1s`). Así `espera` (`real/real`, unidad `ms`) sale `1.0/2.0`, sin
  el `ms` en la línea, igual que hoy.
- Falla nombrando el campo, en tres casos: falta un valor (`falta el valor del campo 'ppuspr'`),
  sobra un valor (`el valor 'zzz' no está en el esquema`) y el valor no encaja con el tipo
  (`el valor 'fps' no encaja con el tipo Real`).
- Los tests unitarios del módulo van en el satélite `src/telemetria_tests.rs`: los campos de la
  versión actual, los tres fallos de arriba y el sello al final.

#### 12.3 `src/perf.rs`: `PerfReport::to_line()`

- `to_line()` pasa a armar un `BTreeMap<&'static str, Valor>` con los 23 valores y a llamar a
  `telemetria::escribir_linea`. La firma **no cambia** (`pub fn to_line(&self) -> String`), así que
  `src/platform.rs` (líneas 1925-1927) no se toca.
- Si `escribir_linea` falla, `to_line()` devuelve `"# telemetría inválida: <motivo>"` en vez de
  entrar en pánico: un contrato roto se ve en `blazing-perf.txt` y no tumba el juego.
- La línea que sale es la de hoy **más ` formato=2` al final**, byte a byte igual por lo demás.

#### 12.4 `scripts/diagnostico/analizar_telemetria.py`

- `cargar_esquema()` lee y valida el esquema: tipos conocidos, nombres sin repetir y `anade` de
  cada versión igual a los `campo` que la siguen.
- `parsear_linea(linea, esquema)` parte la línea en `clave=valor`, convierte cada valor al tipo
  declarado y junta las anomalías: `campo desconocido: zzz`, `valor inválido en fps: ...` y
  `falta el campo pumpm` (sólo en líneas con sello, donde la lista de campos es la de su versión).
- `parsear_telemetria()` pasa a usar ese lector para las líneas `t=` (hoy usa un regex posicional
  de 11 grupos que sólo lee 11 de los 23 campos) y añade dos acumuladores: el peor fotograma de la
  CPU y el del dibujo (el segundo componente de los pares `ejec=A/B` y `dibujo=A/B`).
- `imprimir_informe()` cambia la etiqueta «peor segundo» por «peor media de segundo», añade «peor
  fotograma» y, sólo si hay anomalías, una sección «ANOMALÍAS DE FORMATO» con el recuento.
- Nuevo modo `--volcar`: vuelca en TSV lo que el lector entendió, para que un test pueda
  compararlo con lo que escribió el motor. Gramática, un dato por fila, separado por tabuladores:

```
resumen <TAB> <clave> <TAB> <valor>
linea <TAB> <índice>
campo <TAB> <nombre> <TAB> <valor>
sello <TAB> <versión>
anomalia <TAB> <mensaje>
```

  - `resumen`: `segundos`, `prom_fps`, `prom_ejec`, `max_ejec`, `max_ejec_fotograma`, `max_dibujo`,
    `max_dibujo_fotograma`, `anomalias`.
  - `campo`: uno por campo leído, **en el orden canónico del esquema**, con el valor normalizado
    con el propio formateador del esquema (o sea, con el mismo texto que escribió el motor).
  - Los milisegundos salen con `:.2f` y los FPS con `:.1f`, los mismos formatos del informe humano:
    los números del criterio se comparan literalmente.

#### 12.5 El test de oro: `tests/telemetria_contrato_tests.rs`

Cuatro tests, uno por criterio. El nudo: **Rust escribe la línea, Python la relee, y los dos tienen
que coincidir en cada campo, en su nombre, en su orden y en su texto**.

- `test_la_linea_de_segundo_no_pierde_campos`: Rust arma un `PerfReport` con valores conocidos
  (fps 60.0, ejec 13.6/21.9, ppu 7.7, nativos 4945, interprete 2, ventana 1280x720) y llama a
  `to_line()`. Luego pasa esa línea al analizador con `--volcar` y comprueba que las 23 filas
  `campo` recomponen la línea entera (menos el sello final), que el sello dice `2`, que los nombres
  de las filas son los del esquema y que valores como `ejec=13.6/21.9` o `ventana=1280x720` vuelven
  intactos. Además pasa la misma línea con los campos **barajados** y comprueba que salen los
  mismos 23 valores: el lector va por clave, no por posición.
- `test_un_campo_desconocido_se_reporta`: la línea con `zzz=1` añadido → 23 filas `campo`, una fila
  `anomalia` con `campo desconocido: zzz` y `resumen anomalias 1`.
- `test_los_registros_antiguos_se_siguen_leyendo`: el test escribe en un directorio temporal un
  registro de 4 líneas con el contrato v1 (21 campos, sin `pump`, sin `pumpm` y sin sello), lo pasa
  al analizador y comprueba que cuenta 4 segundos, con 45.0 FPS de media, 25.00 ms de `ejec` y
  **ninguna** anomalía.
- `test_el_informe_dice_el_peor_fotograma`: el mismo registro → «peor media de segundo» 40.00 ms y
  «peor fotograma» 90.00 ms, dos valores distintos a propósito (el segundo 3 trae `ejec=30.0/90.0`).

#### 12.6 Los tests que este cambio obliga a tocar

`src/perf_tests.rs` tiene dos tests que comparan la línea **completa**, con `assert_eq!`:
`test_informe_promedia_y_reporta_peor_caso` (línea 92) y
`test_ventana_sin_fotogramas_no_divide_por_cero` (línea 153). Los dos ganan ` formato=2` al final
de la cadena esperada. No hay ningún otro test que dependa de la forma de la línea.

#### 12.7 Documentación y registros

- `docs/guias/medir-rendimiento.md`: la línea de ejemplo gana `pump=2 pumpm=480 formato=2`; la
  tabla de campos gana `pump`, `pumpm` y `formato` (hoy faltan los dos primeros: el motor los
  escribe desde el 2026-10-14 y la guía no los menciona); y la sección del informe explica las
  etiquetas «peor media de segundo» / «peor fotograma» y la sección de anomalías.
- `docs/referencia/api-modulos-rust.md`: `scripts/extraer_referencias.py` tiene una **lista fija**
  de módulos (líneas 234-244) que hoy no incluye `telemetria.rs` (ni `perf.rs`, `pipeline.rs`,
  `present.rs`, `ppu_shim.rs`, `ppu_nativa.rs` ni `overlay_shader.rs`). Para que el módulo nuevo
  aparezca en la referencia hay que **añadirlo a esa lista** y regenerar el documento con
  `python scripts/extraer_referencias.py`. Esto corrige el §5, que daba por hecho que el documento
  se regeneraba solo «porque aparece un módulo nuevo en `src/`».
- `docs/BACKLOG.md`: los hallazgos que queden fuera de alcance, y `python scripts/auditar_hallazgos.py`
  para que el registro los cubra (el script exige que cada hallazgo de «Fuera de alcance» aparezca
  en el registro, verbatim).
- `GLOSSARY.md` (nuevo, en la raíz): el vocabulario que este cambio acuña —telemetría, línea de
  segundo, esquema de telemetría, sello de versión, anomalía de formato—.

#### 12.8 Lo que el prototipo ya comprobó (en `/tmp`, fuera del repositorio)

- La línea del escritor es la de hoy **más `formato=2`**, byte a byte igual por lo demás.
- El lector del prototipo parsea las 229 líneas de `log_tv15.txt` con **0 anomalías**, leyendo
  exactamente los 21 campos de la versión 1.
- El lector del prototipo parsea una línea nueva de 24 tokens (23 campos + sello) con 0 anomalías.
- Los tres fallos (campo que falta, campo que sobra, tipo que no encaja) se detectan y se nombran.
- El redondeo de Rust (`{:.1}`) y el de Python (`{:.1f}`) dan el mismo texto en 22 valores de
  prueba, incluidos los empates (0.15, 0.25, 2.675).
- `include_str!` con una ruta padre compila y embebe el fichero.

#### 12.9 Lo que queda por comprobar después de la aprobación

- La gramática exacta de `--volcar` (el prototipo usaba una salida ad hoc, no un volcado TSV).
- Que `cargo test` sigue en verde con los dos tests de `src/perf_tests.rs` actualizados.
- Que `python scripts/extraer_referencias.py --check` y
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` siguen dando 0.
- Que el empaquetado oficial (`powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`)
  sigue generando el APK: el esquema va embebido en la biblioteca, así que no debería afectarle.

### Correcciones previas a presentar

- Al principio se pensó que el formato actual sin marcador era una sola versión. Al mirar
  `scripts/diagnostico/log_tv15.txt` (2026-10-12) se vio que **no lleva** `pump` ni `pumpm`, y el
  escritor actual **sí** (se añadieron el 2026-10-14). O sea que «sin marcador» es ambiguo: hay dos
  formatos distintos y ninguno lo declara. De ahí sale el diseño final: la versión 1 es el contrato
  sin `pump`/`pumpm`, la versión 2 es el actual, y el escritor pasa a declarar `formato=2`; una
  línea sin marcador se lee en modo tolerante (se usan los campos que se encuentren).
- Se iba a cambiar el número de «peor segundo» (35.50 → 98.40 ms) sustituyéndolo por el del peor
  fotograma. Se cambió a **aditivo**: el 35.50 se conserva con la etiqueta honesta «peor media de
  segundo» y el 98.40 se añade como «peor fotograma». Motivo: cambiar un número que la gente ya usa
  para trabajar, dentro de una propuesta sobre el contrato, no está justificado por el problema que
  se resuelve.
- La primera idea de la versión era poner el marcador en la cabecera del archivo. Se descartó: la
  cabecera la escribe `platform.rs`, que es `cfg(android)` y no se puede probar en host, así que el
  cambio quedaría sin cobertura.
- Al desglosar la implementación se vio que `scripts/diagnostico/log_tv15.txt` **no está
  versionado** (lo ignora `.gitignore`, patrón `scripts/diagnostico/log_*.txt`): en un clon limpio
  no existe. Los criterios 3 y 4 dependían de él, así que pasan a usar un registro sintético de
  4 líneas que escribe el propio test, y las cifras del log real (229 s, 53.6 FPS, 16.96 ms,
  35.50 ms, 98.40 ms) se quedan en el §9 como comprobación manual en la máquina que lo tiene.
- También se vio que `scripts/extraer_referencias.py` no barre `src/`: tiene una lista fija de
  módulos (líneas 234-244). El §5 daba por hecho que el módulo nuevo aparecería en la referencia
  al regenerarla; hay que añadirlo a la lista, y el §5 queda corregido.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: el esquema declara una lista de campos única y el lector la usa para convertirse → Ahora:
  el esquema declara además las versiones, con `hereda` y `anade`, y el lector elige la lista según
  la versión que declara la línea → Por qué: el esquema tiene que documentar la historia del
  contrato, no sólo el estado actual, para que un registro viejo se pueda leer dentro de unos años.
- Antes: el valor de `ejec` del informe pasa a ser el peor fotograma → Ahora: el peor fotograma se
  **añade** y el valor anterior se conserva con otra etiqueta → Por qué: motivo declarado arriba.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- La línea de segundo no pierde ningún campo aunque se reordene: coincide
  Resultado real: `tests/telemetria_contrato_tests.rs` pasa. Las 23 filas `campo` del
  volcado, unidas con espacios, son byte a byte la línea de `to_line()` sin el sello; con
  los campos invertidos sale exactamente el mismo volcado y cero anomalías.
  Método: el anunciado (`cargo test --test telemetria_contrato_tests`).

- Un campo desconocido se reporta en vez de descartarse en silencio: coincide
  Resultado real: con `zzz=1` añadido salen 23 filas `campo`, una anomalía
  `campo desconocido: zzz`, `resumen anomalias 1`, y la línea sigue contando
  (`segundos 1`, `prom_fps 60.0`). El informe humano imprime la sección
  `ANOMALÍAS DE FORMATO EN LA LÍNEA DE SEGUNDO` con el motivo.
  Método: el anunciado.

- Los registros antiguos sin versión se siguen leyendo: coincide
  Resultado real: el registro sintético de 4 líneas v1 da 4 segundos, 45.0 FPS de media,
  25.00 ms de `ejec` y **0 anomalías**; cada línea aporta sus 21 campos y ninguna trae
  sello. Y el registro real `log_tv15.txt` (229 líneas del 2026-10-12) sigue dando lo
  mismo que antes del cambio: 229 segundos, 53.6 FPS, 16.96 ms, 0 anomalías.
  Método: el anunciado, más la comprobación manual con el log real.

- El informe dice el peor fotograma y no sólo la peor media: coincide
  Resultado real: sobre el mismo registro, `max_ejec 40.00` y
  `max_ejec_fotograma 90.00`; el informe humano imprime
  `peor media de segundo: 40.00 ms` y `peor fotograma: 90.00 ms`. Con el log real, el
  35.50 ms de antes se conserva con la etiqueta honesta y aparece un
  `peor fotograma: 98.40 ms` que antes se tiraba.
  Método: el anunciado, más el contraste antes/después (`git show HEAD:…` piped a python).

- La suite entera sigue en verde: coincide
  Resultado real: `cargo test` → **281 pasados, 0 fallidos** en 24 binarios de test (antes
  264: +9 unitarios del módulo, +6 de integración del contrato, +2 que ya venían de otros
  cambios sin commitear en el árbol).

### Contexto: ¿se respetó?

- El motor no depende de crates nuevos → respetada: `Cargo.toml` no se tocó; el esquema
  se embebe con `include_str!` y se analiza a mano.
- El marcador de versión va en la línea `t=` y no en la cabecera → respetada: el sello es
  `formato=2` al final de la línea que escribe `PerfReport::to_line()`, con test en host.
- Los registros ya escritos se siguen leyendo → respetada: ver el tercer escenario, con el
  log real de 229 líneas incluido.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: (1) el registro sintético tenía que cumplir a la vez media
  25.00 y peor media 40.00, cosa que sólo sale con 15, 15, 30 y 40 —los valores de mi
  primer borrador daban 27.50 y el test lo cazó—; (2) Python escribía el informe en la
  codificación de la consola y el test no podía casar `ANOMALÍAS`, así que el test fuerza
  `PYTHONIOENCODING=utf-8` → criterio de prioridad aplicado: sí (que nada se pierda en
  silencio por encima de que el informe cambie lo mínimo).
- ¿Reapareció alguna alternativa descartada? no: el esquema es el dueño único, el analizador
  sigue en Python y el artefacto sigue siendo texto plano.

### Archivos modificados

- `telemetria/esquema.txt` (nuevo), `src/telemetria.rs` (nuevo),
  `src/telemetria_tests.rs` (nuevo), `tests/telemetria_contrato_tests.rs` (nuevo),
  `GLOSSARY.md` (nuevo)
- `src/lib.rs`, `src/perf.rs`, `src/perf_tests.rs`
- `scripts/diagnostico/analizar_telemetria.py`, `scripts/extraer_referencias.py`
- `docs/referencia/api-modulos-rust.md` (regenerado),
  `docs/guias/medir-rendimiento.md`, `docs/BACKLOG.md`

¿Coincide con lo anunciado? sí, con una excepción declarada: el §12.7 anunciaba el
hallazgo del `--comparar` como anotación, y se ha registrado también en el BACKLOG además
de en esta propuesta.

### Discrepancias

- **Los tests se escribieron después del código, no antes.** El §8 anunciaba
  `Estado previo: falla` para los cuatro escenarios, y ese rojo no se observó: escribí el
  módulo de Rust, reescribí el analizador y sólo entonces escribí
  `tests/telemetria_contrato_tests.rs`. Es una falta al orden que exige la Fase 1 de la
  skill (test de contrato primero). Lo que sí quedó demostrado es que los tests muerden:
  la comprobación de mutación de la Fase 4 mató las dos mutaciones probadas
  (`real/real` con dos decimales y `entero_s` sin convertir el sufijo), y el contraste
  antes/después del analizador de HEAD está en el cuarto escenario. → error de juicio.
- El §12.5 daba por hecho que «peor media de segundo 40.00 / peor fotograma 90.00» y
  «media 25.00» salían del mismo registro sin fijar los dos primeros segundos; con las
  cifras de mi primer borrador la media era 27.50. La propuesta era correcta; el que se
  equivocó fue el primer registro sintético. → información faltante (la del ejemplo).

### Para la próxima vez

Escribir el test de contrato antes de crear el módulo: en un cambio cuyo objetivo *es* un
contrato, el test es la especificación, no la comprobación.

### Verificación ejecutada

| Comprobación | Resultado |
| :--- | :--- |
| `cargo test` | 281 pasados, 0 fallidos (24 binarios) |
| `cargo test --test telemetria_contrato_tests` | 6 pasados |
| `python scripts/extraer_referencias.py --check` | código 0 |
| `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` | 0 error(es), 0 aviso(s) |
| `python scripts/auditar_hallazgos.py` | 142 hallazgos de 42 propuestas cubiertos |
| `python scripts/diagnostico/analizar_telemetria.py --archivo scripts/diagnostico/log_tv15.txt` | 229 s, 53.6 FPS, 16.96 ms, 35.50 / 98.40, 0 anomalías |
| Mutación `real/real` con `:.2f` | muerta (los tests fallan) |
| Mutación `entero_s` sin sufijo | muerta (los tests fallan) |
| `cargo tree -e normal` / `Cargo.toml` | sin dependencias nuevas |

No se ejecutó `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`: el
empaquetado no se ha pedido en este turno y el esquema va embebido, así que no hay motivo
para sospechar que le afecte. Queda como comprobación pendiente del checklist de pre-commit.

### Fuera de alcance (hallazgos que no se arreglaron)

- La línea `# ppucamino` repite la clave `rectas` con dos significados: el número absoluto de líneas por la vía rápida y, entre paréntesis al final, el porcentaje. El analizador lo resuelve cortando la línea por el paréntesis (`split("(")[0]`), así que la clave no es unívoca y ningún esquema podría declararla tal cual. | impacto: medio
- Las otras tres líneas `#` (`ppu_hw`, `regiones`, `ppucamino`) también tienen su contrato escrito a mano, en el escritor de Rust y en el informe de Python. No pierden datos (el lector las barre por clave), pero añadir un campo obliga a tocar los dos sitios sin que nada avise: es el mismo problema que esta propuesta resuelve para la línea `t=`. | impacto: medio
- La cabecera `# backend=… modo=… ppu=recto-N` no la lee el analizador: no hay rama que la case y sus campos no llegan al informe. Además `ppu=recto-N` es la versión del camino de composición de la PPU, no la del formato de la telemetría: son dos cosas distintas que se parecen, y el esquema no debe confundirlas. | impacto: bajo
- `blazing-perf.txt` se trunca en cada arranque (`File::create` en `init_perf_file`), así que no queda histórico entre sesiones: para comparar dos sesiones hay que copiar el archivo a mano antes de reiniciar la aplicación. | impacto: bajo
- El informe comparativo (`--comparar`) sigue mostrando solo los promedios: no incluye el «peor fotograma» que ahora calcula el lector, así que comparar dos sesiones no dice si el tirón aislado mejoró. | impacto: bajo
- El conteo de tests del checklist de `AGENTS.md` §6 («264 tests: 175 en `src/` + 89 de integración») ya venía desfasado y este cambio lo aleja más: ahora hay 281, y el número hay que actualizarlo a mano en cada cambio que añada tests. | impacto: bajo

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

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

- La forma de la línea que se ve en `blazing-perf.txt` dentro del dispositivo: ✔ | ✘ | ?
- El texto exacto del informe que se lee en el terminal: ✔ | ✘ | ?
- Detectado solo por el humano:
