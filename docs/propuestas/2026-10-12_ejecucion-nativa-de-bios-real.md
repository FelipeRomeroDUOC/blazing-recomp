# Propuesta: Ejecución nativa de la BIOS real (quitar el desvío al intérprete)

Fecha: 2026-10-12
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El juego deja de gastar el intérprete en la BIOS: la BIOS real del usuario se ejecuta
con los bloques AOT que el APK ya incluye, y solo las direcciones de BIOS sin bloque
siguen pasando por el intérprete como red de seguridad.

### 2. Contexto

- Contexto consultado: [`src/recomp.rs`](../../src/recomp.rs),
  [`src/recomp_tests.rs`](../../src/recomp_tests.rs), [`src/runner.rs`](../../src/runner.rs),
  [`docs/propuestas/2026-10-10_optimizacion-compatibilidad-multidispositivo.md`](2026-10-10_optimizacion-compatibilidad-multidispositivo.md),
  la última línea `# regiones` de `scripts/diagnostico/log_tv15.txt` y, del repositorio del motor,
  `crates/recomp/src/main.rs` y `crates/gba-core/src/mem.rs` en el rev fijado.
- Intención: quitar el coste de CPU que hoy no aporta nada, sin perder la capacidad de arrancar
  con la BIOS real del usuario. El éxito se ve como el juego arrancando igual y la fase `ejec`
  de la telemetría más baja.
- Restricciones y decisiones previas (máx. 3):
  - El flag que marca "la CPU está ejecutando dentro de la BIOS" debe seguir actualizándose en
    cada despacho (origen: `crates/gba-core/src/mem.rs:181-184` del rev fijado: las lecturas de
    la región 0 devuelven bytes reales solo mientras está puesto) → implica: no borrar la
    asignación de `pc_in_bios`; solo se quita el desvío que va detrás → protegida por: *El flag
    pc_in_bios se mantiene en el despacho*.
  - El intérprete es la red de seguridad para direcciones sin bloque (origen: `AGENTS.md §4.2`) →
    implica: no añadir un modo de fallo duro ni dejar sin ejecutar una dirección → protegida por:
    *Una dirección de BIOS sin bloque sigue cayendo al intérprete*.
  - El motor no desvía la BIOS al intérprete (origen: rev fijado
    `de4edf59b872d887046d6a3b005e2df551b6d44c`, `crates/recomp/src/main.rs:2258-2260`: asigna el
    flag y sigue al despacho nativo) → implica: replicar esa conducta, no inventar una nueva →
    protegida por: *El despacho AOT ejecuta un bloque de BIOS en modo BIOS real*.
- Alternativas descartadas:
  - Dejarlo como está → cuesta 53.553.425 pasos de intérprete, el 99,985% del total de la sesión
    medida (origen: última línea `# regiones` de `scripts/diagnostico/log_tv15.txt`), y el
    comentario que lo justifica no coincide con el motor.
  - Desviar solo durante el arranque (por ejemplo los primeros N fotogramas) → el manejador de
    interrupción de la BIOS se ejecuta en cada fotograma, no solo al arrancar, así que el coste
    seguiría ahí; además exigiría una constante sin ninguna medición que la respalde.
  - Fallo duro para la BIOS (abortar si una dirección de BIOS no tiene bloque, como el modo
    `NO_INTERP` del motor) → choca con la segunda restricción.
- Ante imprevistos, prioriza: que el juego siga arrancando en el dispositivo sobre el ahorro de CPU.

### 3. Ejemplos

- **Caso normal** (el PC está en una dirección de BIOS, como `0x00001228` en la telemetría del TV):
  antes: se interpreta instrucción por instrucción (53.553.425 pasos en esa sesión);
  ahora: se despacha el bloque AOT `b_00001228_a`, que el paquete ya contiene
  (comprobado: `grep -rho -E "b_00001228_[at]" src-c/` imprimía ese símbolo).
- **Caso límite** (el PC cae en una dirección de BIOS **sin** bloque, por ejemplo `0x0000_2000`):
  antes: intérprete; ahora: intérprete igual, y la dirección queda registrada en el ranking de
  hotspots, que es la vía por la que se descubre la etiqueta que falta.

### 4. Pasos

1. [`src/recomp.rs`](../../src/recomp.rs), dentro de `run_frame_native`: la guarda `if m.bus.real_bios`
   conserva la asignación del flag y pierde el desvío; el comentario que atribuye el desvío a una
   "especificación upstream" se sustituye por el contrato real.

   ```rust
   // Antes
   if m.bus.real_bios {
       m.bus.pc_in_bios = key < 0x4000;
       if key < 0x4000 { record_hotspot(key); m.step(); fallback += 1; continue; }
   }

   // Ahora
   if m.bus.real_bios {
       m.bus.pc_in_bios = key < 0x4000;   // solo el flag; la BIOS sigue al despacho
   }
   ```

2. [`src/recomp_tests.rs`](../../src/recomp_tests.rs): tres tests nuevos (normal, límite y regresión),
   los tres tomando el candado `bloquear_acumuladores()` que ya usan sus vecinos —los acumuladores
   globales son del proceso y `cargo test` ejecuta en paralelo— y la sustitución de
   `test_run_frame_native_preserves_real_bios_stepping`, cuyas dos aserciones
   (`native == 0` y "el bloque nativo de la BIOS no debe ejecutarse") codifican justamente la
   conducta que se retira. No se debilita ni se borra sin más: el invariante que sí importa
   (el flag `pc_in_bios`) pasa a tener su propio test, y la aserción invertida es la del test normal.
3. Nada en `docs/`: se buscó la premisa en los cuadrantes de Diátaxis
   (`grep -rn -i "0x4000|BIOS.*interprete" docs/explicacion/` → sin resultados) y no se toca ningún
   comentario `///`, así que `docs/referencia/api-modulos-rust.md` no hay que regenerarlo.

### 5. Qué más podría verse afectado

Búsqueda hecha: `grep -rn "run_frame_native" src/ tests/ --include=*.rs` y
`grep -rn "real_bios" src/ tests/ --include=*.rs`

- [`src/runner.rs:126`](../../src/runner.rs) — único llamador de producción (`step_frame`). No cambia.
- [`src/recomp_tests.rs:37`](../../src/recomp_tests.rs) — `test_run_frame_native_fallback_on_unmapped`
  usa `Machine::new` (sin BIOS real), así que no le afecta.
- [`src/recomp_tests.rs:71`](../../src/recomp_tests.rs) — el test que se sustituye.
- [`src/recomp.rs:144`](../../src/recomp.rs) — la otra rama que consulta `real_bios`: la entrega de
  interrupciones. No se toca; es la máquina de excepciones, no una lectura de instrucción, y sigue
  yendo por `m.step()`.
- [`src/platform.rs:2126-2129`](../../src/platform.rs) — lee los contadores y escribe la línea
  `# regiones`. No cambia, pero los valores de la fila `bios` cambian de forma (hoy `53.553.425 / 0`).
- `scripts/diagnostico/analizar_telemetria.py` — imprime el desglose regional. No cambia.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Los bloques de BIOS del paquete cubren todas las direcciones que este juego ejecuta en la BIOS**,
  así que el fallback de BIOS bajará a casi cero en el dispositivo. → prototipar: sí
  → si es falso: quedará visible como pasos de intérprete en la fila `bios` de `# regiones` y se
  cosecharán las etiquetas que falten (trabajo aparte; este cambio no lo empeora).
  → resultado del prototipo: el arnés del motor con BIOS real completó 7.200 fotogramas con
  `fallback_steps: 0` (comando en el apartado 9). El contador real del dispositivo queda por ver.
- **Quitar el desvío no reintroduce el congelamiento en el logo** que motivó ponerlo
  (origen del síntoma: `2026-10-10_optimizacion-compatibilidad-multidispositivo.md:46`). → prototipar: sí
  → si es falso: se revierte el paso 1 y se abre una propuesta para mantener el valor de protección
  de lectura de la BIOS (`bios_open`, que el motor hoy solo refresca desde el intérprete) en el
  despacho nativo; es un hueco del motor, no del port.
  → resultado del prototipo: el mismo arnés, que se comporta como el motor (flag en el despacho,
  BIOS nativa), completó 7.200 fotogramas con `fallback_steps: 0` y con la imagen cambiando entre
  los fotogramas 600, 2.400 y 6.600 (hashes distintos; si estuviera congelado serían iguales).
- **Nadie depende de que las direcciones de BIOS cubiertas aparezcan en `# hotspots top=`.** → prototipar: no
  → si es falso: se registran por la vía del despacho nativo, que es una línea.

### 7. Alcance

- **Incluye:**
  - La guarda de `run_frame_native` en [`src/recomp.rs`](../../src/recomp.rs) y su comentario.
  - Los tres tests de [`src/recomp_tests.rs`](../../src/recomp_tests.rs) y la sustitución del test
    que protege la conducta retirada.
- **No incluye:**
  - Refrescar `bios_open` en el despacho nativo dentro de `gba-core` (repositorio del motor).
  - La reproducibilidad del rev fijado y la agregación del analizador de telemetría: son hallazgos de
    la auditoría de premisas y van en propuestas propias.
  - Cambiar el formato de la telemetría, el límite de hotspots o la cadencia de volcado.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El despacho AOT ejecuta un bloque de BIOS en modo BIOS real**
Caso de uso: arranque con la BIOS real del usuario | Tipo: normal
  Dado un `Machine` con BIOS real y una tabla con un bloque en `0x0000_0080` que anota su invocación
  Cuando se llama a `run_frame_native` con el PC en `0x0000_0080` y `max_steps = 1`
  Entonces el bloque fue invocado, `native == 1` y `fallback == 0`
Test: `src/recomp_tests.rs::test_run_frame_native_despacha_bloque_aot_de_bios_en_modo_bios_real`
Estado previo: **falla** (se espera `native == 0`, `fallback == 1`)

**Escenario (límite): Una dirección de BIOS sin bloque sigue cayendo al intérprete**
Caso de uso: dirección de BIOS que el paquete no cubre | Tipo: límite
  Dado un `Machine` con BIOS real y una tabla con un único bloque en `0x0000_0080`
  Cuando se llama a `run_frame_native` con el PC en `0x0000_2000` y `max_steps = 1`
  Entonces `native == 0`, `fallback == 1` y el contador de región `bios` suma 1 paso al intérprete
Test: `src/recomp_tests.rs::test_run_frame_native_direccion_de_bios_sin_bloque_cae_al_interprete`
Estado previo: **pasa (regresión)**

**Escenario (regresión): El flag pc_in_bios se mantiene en el despacho**
Caso de uso: lecturas de la región 0 desde fuera de la BIOS | Tipo: regresión
  Dado un `Machine` con BIOS real
  Cuando se ejecuta un fotograma con el PC en `0x0000_0080` (BIOS) y otro con el PC en `0x0800_0100` (ROM)
  Entonces `bus.pc_in_bios` es `true` en el primero y `false` en el segundo
Test: `src/recomp_tests.rs::test_run_frame_native_mantiene_pc_in_bios_en_el_despacho`
Estado previo: **pasa (regresión)**

**Escenario (visible): El APK arranca con la BIOS real y deja de interpretar la BIOS**
Caso de uso: sesión real en el Android TV | Tipo: normal
  Dado el APK instalado con el `gba_bios.bin` del usuario en el dispositivo
  Cuando se juega de 30 a 60 s en una escena con movimiento
  Entonces el juego pasa del logo sin congelarse y la fila `bios` de `# regiones` muestra pocos
  pasos de intérprete y miles de bloques nativos (hoy muestra `53.553.425 / 0`)
Test: revisión humana | Estado previo: —

### 9. Cómo lo comprobaré

- **El despacho AOT ejecuta un bloque de BIOS** → comando:
  `cargo test --lib recomp::tests::test_run_frame_native_despacha_bloque_aot_de_bios_en_modo_bios_real`
  → pasa si: termina en `ok` con las tres aserciones
  → no detecta: que el `.so` del APK tenga ese bloque en el dispositivo; eso lo cubre el conteo de
  símbolos de `src-c/` y, al final, la telemetría real.
- **Una dirección de BIOS sin bloque cae al intérprete** → comando:
  `cargo test --lib recomp::tests::test_run_frame_native_direccion_de_bios_sin_bloque_cae_al_interprete`
  → pasa si: termina en `ok`
  → no detecta: qué direcciones concretas quedan sin bloque en el juego completo.
- **El flag pc_in_bios se mantiene** → comando:
  `cargo test --lib recomp::tests::test_run_frame_native_mantiene_pc_in_bios_en_el_despacho`
  → pasa si: termina en `ok` con las dos aserciones
  → no detecta: el otro valor de protección de lectura (`bios_open`), que este cambio no refresca y
  que queda fuera de alcance.
- **La premisa del cambio, medida en el motor** → comando:
  `../gba-recomp/gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime `fallback_steps: 0` y sale con código 0 (hoy imprime
  `frames: 7200 hash: b54066dc06f2b76c native_blocks: 3110974 fallback_steps: 0`)
  → no detecta: que *el port* se comporte igual: ese comando ejecuta el arnés del motor, no
  `run_frame_native`. Tampoco detecta por sí solo que el juego haya pasado el logo, porque un juego
  detenido también completa fotogramas; por eso se comparan los hashes de 600, 2.400 y 6.600
  fotogramas, que hoy son distintos entre sí.
- **Nada más se rompe** → comando: `cargo test`
  → pasa si: 118 tests en verde, 0 fallos (base medida hoy: 105 unitarios + 13 de integración)
  → no detecta: comportamiento en Android; para eso está el escenario visible.
- **Escenario "(visible)"** → revisión humana: instalar el APK con el `gba_bios.bin` del usuario,
  jugar de 30 a 60 s y leer la fila `bios` de `# regiones` en `blazing-perf.txt`
  → no detecta: nada más; el resto queda cubierto por los tests.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — borrar la asignación de `pc_in_bios` → debe morir *El flag pc_in_bios se mantiene en el despacho*.
M2 — devolver el desvío al intérprete → debe morir *El despacho AOT ejecuta un bloque de BIOS*.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Se cumplen los tres escenarios de código, la suite completa sigue en 118 verdes y el escenario
visible queda confirmado por ti.

### 11. Parar si

2 intentos fallidos seguidos con el mismo escenario. Detenerse, explicar qué se probó y marcar la
propuesta como `discrepada`.

### Correcciones previas a presentar

- El comentario del código atribuye a una "especificación upstream" una conducta que el motor no
  tiene: en el rev fijado, `crates/recomp/src/main.rs:2258-2260` asigna el flag y sigue al despacho
  nativo. Se corrige el comentario, no solo la conducta (era el origen de la premisa).
- `AGENTS.md` (checklist de pre-commit) dice 111 tests; hoy `cargo test` mide 105 unitarios + 13 de
  integración = 118. Se usa la medición de hoy como base.
- La propuesta histórica `2026-10-10_optimizacion-compatibilidad-multidispositivo.md:127` afirma que
  el desvío es "idéntico a la implementación oficial de upstream". Un registro histórico no se
  reescribe: la corrección queda registrada aquí.
- `recomp runc` informa `loaded 17207 translated blocks` mientras `src-c/` contiene 144.875 símbolos.
  No investigado (fuera de alcance).

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El despacho AOT ejecuta un bloque de BIOS en modo BIOS real**: coincide.
  Resultado real: en rojo antes de la edición, `panicked at src/recomp_tests.rs:95: el bloque AOT de
  la BIOS debe ejecutarse` (el desvío dejaba `native == 0` y `fallback == 1`); en verde después, con
  `native == 1` y `fallback == 0`.
  Método: el anunciado (`cargo test --lib recomp::tests::test_run_frame_native_despacha_bloque_aot_de_bios_en_modo_bios_real`).
- **Una dirección de BIOS sin bloque sigue cayendo al intérprete**: coincide. Verde antes y después
  (`native == 0`, `fallback == 1`, y la región `bios` suma 1 paso).
  Método: el anunciado.
- **El flag pc_in_bios se mantiene en el despacho**: coincide. Verde antes y después, con las dos
  aserciones (`true` dentro de la BIOS, `false` en ROM).
  Método: el anunciado.
- **El APK arranca con la BIOS real y deja de interpretar la BIOS**: no pudo comprobarse.
  Necesita el dispositivo; queda en la revisión humana de abajo.
- **La premisa, medida en el motor**: coincide. `frames: 7200 hash: b54066dc06f2b76c native_blocks:
  3110974 fallback_steps: 0`, código de salida 0.
- **Nada más se rompe**: coincide con la base, difiere del número anunciado. Real: 107 unitarios +
  13 de integración = **120 en verde, 0 fallos**. La propuesta decía 118; ver Discrepancias.
- **Blindaje por mutación** (previsto en el apartado 9): M1 (borrar el flag) mata *El flag pc_in_bios
  se mantiene en el despacho*; M2 (devolver el desvío al intérprete) mata *El despacho AOT ejecuta un
  bloque de BIOS en modo BIOS real*; M3 (flag siempre verdadero) mata también *El flag pc_in_bios se
  mantiene*, por su mitad de ROM. Los tres mutantes compilaban (0 errores `error[E]`, no solo ausencia
  de líneas de test), así que las muertes son de comportamiento y no de compilación.
  `src/recomp.rs` se respaldó y se restauró desde memoria RAM en un bloque `finally`, con la huella
  sha256 idéntica antes y después; sin ningún comando de Git.

### Contexto: ¿se respetó?

- El flag debe seguir actualizándose en cada despacho → respetada: la asignación queda en el mismo
  punto del bucle y su test la cubre (muere con M1 y con M3).
- El intérprete es la red de seguridad para direcciones sin bloque → respetada: el escenario límite
  pasa antes y después.
- El motor no desvía la BIOS al intérprete → respetada: la guarda quedó reducida a la única línea
  que el motor usa en ese punto.
- ¿Se tocó algo que alguna restricción prohibía? no: no se tocó `gba-core` ni la documentación.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

[`src/recomp.rs`](../../src/recomp.rs) (la guarda y su comentario), [`src/recomp_tests.rs`](../../src/recomp_tests.rs)
(tres tests nuevos; se retiró `test_run_frame_native_preserves_real_bios_stepping`) y este archivo.
¿Coincide con lo anunciado? sí: los dos archivos de código ya estaban listados en el apartado 7.

Aviso de higiene: `src/recomp.rs` y `src/recomp_tests.rs` **ya tenían cambios sin commitear** de otra
sesión antes de empezar (la telemetría por región), así que el `git diff` de esos archivos mezcla
ambos trabajos. Hay una interacción real: esa otra sesión había añadido `record_hotspot(key)` **dentro**
del desvío, y el bloque retirado es justamente ese; el registro sigue existiendo en la rama de fallback
para las direcciones sin bloque, y los bloques cubiertos pasan a contarse como nativos por región
(`region_of(key & !1)` en la rama `Some`), así que la telemetría queda completa por las dos vías.

### Discrepancias

- El apartado 9 esperaba 118 tests en verde y el resultado fueron 120. Al redactar conté la base
  medida (105 + 13) pero no sumé el delta que la propia propuesta introduce (−1 test retirado,
  +3 nuevos). → error de juicio. No afecta a ningún criterio de aceptación.

### Para la próxima vez

- Al fijar el número esperado de tests para "nada más se rompe", calcularlo como base + (nuevos − retirados).
- El detector de compilación del script de mutación marcaba cualquier línea con `error:`; el fallo de
  un test también la imprime. Afinarlo a `error[E` antes de leer el resultado como fallo de compilación.

### Fuera de alcance (hallazgos que no se arreglaron)

Registrados durante la preparación de esta propuesta; no se toca ninguno.

- [`docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md:180`](2026-10-12_drain-hotspots-sin-perdida.md) incluye
  un comando con una ruta local absoluta que contiene el nombre de usuario, lo que choca con la regla
  de portabilidad y privacidad de `AGENTS.md §6` (el archivo es de otro hilo y está sin commitear,
  así que no se edita desde aquí). | impacto: bajo
- `recomp runc` informa `loaded 17207 translated blocks` mientras `src-c/` contiene 144.875 símbolos
  (origen: el comando del apartado 9 y `grep -rho -E "b_[0-9a-f]{7,8}_[at]" src-c/ | sort -u | wc -l`).
  No investigado. | impacto: bajo
- [`docs/explicacion/arquitectura-port-android.md:86`](../explicacion/arquitectura-port-android.md) declara
  una latencia de swapchain de 1 fotograma y [`src/platform.rs:294`](../../src/platform.rs) configura 2.
  | impacto: bajo
- `AGENTS.md` (checklist de pre-commit) cita 111 tests; la medición de hoy da 105 unitarios + 13 de
  integración = 118. | impacto: bajo

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- El juego pasa del logo con la BIOS real en el Android TV (escenario visible): ✔ | ✘ | ?
- Valor de la fila `bios` de `# regiones` en el dispositivo tras el cambio: ✔ | ✘ | ?
- Efecto en la tasa de FPS y en la fase `ejec` de la telemetría: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
