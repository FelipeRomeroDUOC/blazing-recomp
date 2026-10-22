# Propuesta: Encender la PPU nativa en el dispositivo con un archivo de marca, sin recompilar

Fecha: 2026-10-20
Nivel: 2
Estado: verificada por agente (todo lo comprobable sin el dispositivo; queda la revisión humana en el móvil)
> **Interruptor sustituido el 2026-10-21.** La marca `ppu-nativa.flag` de esta propuesta **ya no se
> lee**: la PPU nativa pasó a ser la ruta de dibujo **por defecto** y el apagado se hace con el
> archivo nuevo `ppu-nativa-off.flag`. Ver
> [`2026-10-21_ppu-nativa-activa-por-defecto.md`](2026-10-21_ppu-nativa-activa-por-defecto.md).
> El resto del registro de esta propuesta se conserva como está.

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Un archivo de marca (`ppu-nativa.flag`) dejado en el almacenamiento del dispositivo enciende la
**PPU nativa** por software-GPU mientras la app está en marcha, y quitarla la vuelve a apagar. Hoy
ese interruptor es una constante de compilación: encenderlo obliga a recompilar y empaquetar el
APK entero, y un fallo visual obliga a repetir ese ciclo. Con la marca, la prueba en el
dispositivo se hace sin recompilar y se puede volver atrás en caliente.

El APK de prueba que sale de esta propuesta lleva los tres ABIs y la PPU nativa **apagada por
defecto**: sin la marca, la app se comporta exactamente igual que hoy.

### 2. Contexto

- Contexto consultado: `AGENTS.md`; `src/platform.rs` (constante `PPU_NATIVA_ACTIVA`, `Compositor::new`
  y `Compositor::render`, `AppState`, `init_perf_file`, bucle principal, `muestrear_ppu_hw`);
  `src/ppu_nativa.rs` (`PpuNativaPipeline::new` y `render_frame`); `src/perf.rs`
  (`audio_timing_flag_enabled`, `resolve_perf_candidates`, `PpuHwFrameSample`, `PpuHwWindowStats`,
  `to_line`, `record_ppu_hw`, `close_window`); `src/perf_tests.rs`; `src/storage.rs`
  (`resolve_search_directories`); `scripts/package_apk.ps1`; `scripts/diagnostico/analizar_telemetria.py`;
  `android/AndroidManifest.xml`; `docs/guias/medir-rendimiento.md`;
  `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`.
- Intención: poder mirar la imagen que produce la PPU nativa **en el dispositivo real**, y poder
  corregir lo que se vea sin un ciclo de compilación por cada ajuste. El éxito es que la persona
  pueda poner y quitar la marca y ver el cambio, y que la telemetría diga con certeza si quien
  dibujó el cuadro fue la PPU nativa o el shim.
- Restricciones y decisiones previas (máx. 3):
  - El interruptor de la PPU nativa debe estar **apagado por defecto** (origen: propuesta
    `2026-10-19_ppu-nativa-fase-1.md`, fase 1) → implica: un APK recién instalado se comporta
    igual que hoy, sin marca y sin coste → protegida por: «Sin marca, la PPU nativa no se enciende».
  - El interruptor se decide con un **archivo en el almacenamiento**, no con una constante de
    compilación (origen: respuesta humana, 2026-10-20) → implica: el motor relee la marca cada
    segundo desde los directorios donde ya busca la telemetría → protegida por: «La marca enciende
    la PPU nativa en caliente».
  - El APK de prueba debe llevar los **tres ABIs** (origen: respuesta humana, 2026-10-20) →
    implica: el empaquetado se hace con el comando oficial `scripts/package_apk.ps1` →
    protegida por: no testeable (revisión del artefacto).
- Alternativas descartadas (solo si las hubo):
  - Constante de compilación → descartada porque obliga a recompilar por cada ajuste y no deja
    volver atrás sin otro ciclo completo.
  - Variable de entorno → descartada porque en Android no hay un shell estable donde el usuario
    la fije sin `run-as`, y el APK de producción no se depura desde un shell.
  - Menú de depuración dentro de la app → descartada porque la app es una `NativeActivity` sin UI
    propia; darle una es otro proyecto, no una variante de este interruptor.
- Ante imprevistos, prioriza: que un APK sin marca se comporte **exactamente** como hoy > que la
  telemetría distinga nativa de shim.

### 3. Ejemplos

- Caso normal: el usuario deja `ppu-nativa.flag` en el almacenamiento → antes: la PPU nativa nunca
  llega a construirse (constante de compilación apagada) → ahora: en el siguiente tick de un
  segundo, el compositor construye el pipeline nativo y el cuadro lo dibuja él.
- Caso límite: el usuario borra la marca con la app en marcha → antes: no había conmutación en
  caliente → ahora: en el siguiente tick el control vuelve al shim; el pipeline nativo queda
  construido en memoria pero deja de usarse.
- Caso límite: la marca está en un directorio que la app no puede leer → antes: nada que hacer →
  ahora: la PPU nativa sigue apagada y el Logcat dice qué directorios se han mirado.

### 4. Pasos

1. `src/ppu_nativa.rs` gana el nombre de la marca y una función pura que dice si está activa,
   siguiendo el patrón ya existente de `perf::audio_timing_flag_enabled`.
2. `src/platform.rs` deja de depender de la constante de compilación: el estado de la app reevalúa
   la marca una vez por segundo y se lo pasa al compositor, que construye el pipeline nativo de
   forma **perezosa** la primera vez que la marca está activa (así, sin marca, no se reserva ni un
   byte de GPU).
3. `src/perf.rs` y `src/perf_tests.rs`: la línea de telemetría `# ppu_hw` gana el campo `nativa=N`,
   con los fotogramas que dibujó la PPU nativa en esa ventana, para que la persona pueda
   distinguir «dibujó la GPU» de «dibujó la PPU nativa».
4. `scripts/diagnostico/analizar_telemetria.py` suma ese campo a su informe, para que el análisis
   de una captura de dispositivo lo muestre.
5. Tests nuevos en rojo antes de tocar la producción, y después la implementación.
6. Documentación sincronizada: guía de medición en dispositivo, referencia de API regenerada y
   BACKLOG.
7. Empaquetado del APK de prueba con el comando oficial de tres ABIs.

### 5. Qué más podría verse afectado

- `src/platform.rs`: `Compositor::render` (firma), `AppState` (campo nuevo), `init_perf_file`
  (encabezado de telemetría), `muestrear_ppu_hw` (firma), `DrawPhases` (campo nuevo).
- `src/perf.rs`: `PpuHwFrameSample`, `PpuHwWindowStats` (campos nuevos), `to_line`, `record_ppu_hw`,
  `FramePerf` (contador nuevo) y las dos construcciones de `PpuHwWindowStats` en `close_window`.
- `src/perf_tests.rs`: la aserción de la línea `# ppu_hw` (línea 295) cambia porque el formato gana
  un campo.
- `src/ppu_nativa.rs` y `src/ppu_nativa_tests.rs`: función y tests nuevos.
- `scripts/diagnostico/analizar_telemetria.py`: el informe de `ppu_hw` suma `nativa_total`.
- `docs/referencia/api-modulos-rust.md`: se regenera (desaparece la constante `PPU_NATIVA_ACTIVA` y
  aparecen la constante del nombre de la marca y la función nueva).
- `docs/guias/medir-rendimiento.md`: se documenta la marca y el campo `nativa=`.
- `docs/BACKLOG.md`: el punto «prueba en dispositivo» de la fase 1 queda con su procedimiento.
- `tests/packaging_tests.rs`: comprueba la ficha de estado de las propuestas; la nueva la declara.
- Búsqueda hecha: `grep -rn "PPU_NATIVA_ACTIVA\|audio_timing_flag_enabled\|ppu_hw" src/ tests/ scripts/ docs/`
  para no dejar referencias colgando al interruptor viejo ni al formato viejo de la telemetría.

### 6. Suposiciones

- La app puede leer un archivo de marca en `/sdcard/Download` sin permiso especial → prototipar: no
  → si es falso: la guía de medición indica como ruta fiable el directorio externo propio de la
  app, que nunca pide permisos; la función de la marca mira **todos** los directorios candidatos,
  así que basta con ponerla en el que funcione.
- Reevaluar la marca una vez por segundo es lo bastante rápido para el uso previsto → prototipar: no
  → si es falsa: bajar el tick a 250 ms no cuesta más que un `stat` por segundo.
- El pipeline de la PPU nativa se puede construir en el dispositivo real → prototipar: sí
  → si es falso: el Logcat registrationa el fallo de construcción y la app sigue con el shim.

### 7. Alcance

- Incluye: la marca en runtime, la construcción perezosa del pipeline nativo, el campo de
  telemetría `nativa=`, los tests, la documentación y el APK de prueba de tres ABIs.
- No incluye: borrar el shim (paso 8 de la fase 1), sustituir el rasterizado por software (el motor
  sigue rasterizando en CPU: el FPS **no** mejora por esto), medir rendimiento, ni la revisión
  humana en el dispositivo.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Sin marca, la PPU nativa no se enciende
Caso de uso: la persona instala el APK de prueba y juega sin tocar nada | Tipo: regresión
  Dado que un directorio de telemetría existe y **no** contiene `ppu-nativa.flag`
  Cuando se consulta si la PPU nativa está activa
  Entonces la respuesta es `false`
Test: `src/ppu_nativa_tests.rs::test_sin_marca_la_ppu_nativa_no_se_enciende`
Estado previo: falla

Escenario: La marca enciende la PPU nativa en caliente
Caso de uso: la persona deja la marca y quiere el cambio sin reinstalar | Tipo: normal
  Dado que un directorio de telemetría contiene un archivo `ppu-nativa.flag` de tamaño 0
  Cuando se consulta si la PPU nativa está activa
  Entonces la respuesta es `true`
Test: `src/ppu_nativa_tests.rs::test_la_marca_enciende_la_ppu_nativa`
Estado previo: falla

Escenario: Quitar la marca apaga la PPU nativa en caliente
Caso de uso: la persona ve algo raro y quiere volver atrás sin recompilar | Tipo: límite
  Dado que un directorio de telemetría contiene `ppu-nativa.flag`
  Cuando se borra ese archivo y se vuelve a consultar
  Entonces la respuesta pasa a `false`
Test: `src/ppu_nativa_tests.rs::test_quitar_la_marca_apaga_la_ppu_nativa`
Estado previo: falla

Escenario: La telemetría distingue los fotogramas de la PPU nativa
Caso de uso: la persona lee `blazing-perf.txt` y quiere saber quién dibujó | Tipo: normal
  Dado que en una ventana de un segundo se registran 30 fotogramas con la PPU nativa y 30 con el shim
  Cuando se cierra la ventana
  Entonces la línea `# ppu_hw` dice `nativa=30` y `gpu=60`
Test: `src/perf_tests.rs::test_ppu_hw_distingue_los_fotogramas_de_la_ppu_nativa`
Estado previo: falla

Escenario: La imagen del juego en el dispositivo (visible)
Caso de uso: la persona ve el juego en el móvil | Tipo: normal
  Dado que la marca está activa y el juego está en una escena con mezcla alfa
  Cuando se mira la pantalla
  Entonces la imagen sale con la mezcla correcta, sin el defecto que el shim producía
Test: revisión humana en el dispositivo
Estado previo: no pudo comprobarse

### 9. Cómo lo comprobaré

- Sin marca, la PPU nativa no se enciende → herramienta: `cargo test --lib ppu_nativa`
  → pasa si: el test pasa y la función devuelve `false` sin marca → no detecta: que el cableado en
  `platform.rs` pase la marca al compositor (eso lo cubre la revisión humana del artefacto).
- La marca enciende la PPU nativa en caliente → herramienta: `cargo test --lib ppu_nativa`
  → pasa si: el test pasa y la función devuelve `true` con el archivo presente → no detecta: que en
  el dispositivo el directorio elegido sea el que la app lee.
- Quitar la marca apaga la PPU nativa en caliente → herramienta: `cargo test --lib ppu_nativa`
  → pasa si: el test pasa tras borrar el archivo → no detecta: la latencia real del cambio (un
  tick de un segundo).
- La telemetría distingue los fotogramas de la PPU nativa → herramienta:
  `cargo test --lib perf` → pasa si: el test pasa y la línea contiene `nativa=30` → no detecta: que
  el campo llegue desde el dispositivo.
- La imagen del juego en el dispositivo (visible) → revisión humana: con la marca puesta, mirar
  escenas con mezcla alfa, ventanas y sprites translúcidos, y comprobar en `blazing-perf.txt` que
  la línea `# ppu_hw` trae `nativa≈60` → no detecta: diferencias por debajo del píxel.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, la suite completa está en verde y el APK de tres
ABIs está empaquetado y listo para instalar.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- La primera idea era leer la marca **en cada fotograma** dentro del compositor. Se descarta: un
  `stat` por fotograma es trabajo en el camino caliente, y el estado de la app ya tiene un tick
  por segundo donde cabe la reevaluación.
- La primera idea era **quitar** la constante `PPU_NATIVA_ACTIVA` sin más. Se añade la construcción
  perezosa en su lugar: sin marca, el pipeline nativo no se construye y no se reservan sus texturas,
  así que el APK sin marca tiene exactamente el perfil de memoria de hoy.

### Cambios tras prototipos (solo si el plan cambió)

- (sin prototipos: el plan no cambió respecto a lo que se presenta)

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Sin marca, la PPU nativa no se enciende: coincide
  Resultado real: `sin_marca_la_ppu_nativa_no_se_enciende` pasa. Además, la construcción del
  pipeline es **perezosa**: sin marca no se crea, así que no se reserva ni una textura.
  Método: el anunciado.
- La marca enciende la PPU nativa en caliente: coincide
  Resultado real: `la_marca_enciende_la_ppu_nativa` pasa. Sobre el APK ya empaquetado, la cadena
  `ppu-nativa.flag` está dentro de `lib/arm64-v8a/libblazing_recomp.so`.
  Método: el anunciado.
- Quitar la marca apaga la PPU nativa en caliente: coincide
  Resultado real: `quitar_la_marca_apaga_la_ppu_nativa` pasa (borra el archivo y vuelve a
  consultar).
  Método: el anunciado.
- La telemetría distingue los fotogramas de la PPU nativa: coincide
  Resultado real: `test_ppu_hw_distingue_los_fotogramas_de_la_ppu_nativa` pasa; la línea `# ppu_hw`
  termina en `nativa=N` y el analizador de telemetría lo suma como `nativa_total`.
  Método: el anunciado.
- La imagen del juego en el dispositivo (visible): no pudo comprobarse
  Resultado real: no hay dispositivo en esta máquina; queda para la revisión humana.
  Método: distinto: se entrega el APK de tres ABIs y el procedimiento documentado en la guía.

### Contexto: ¿se respetó?

- El interruptor debe estar apagado por defecto → respetada: sin el archivo, el pipeline de la PPU
  nativa no llega a construirse y el APK se comporta como el de siempre.
- El interruptor se decide con un archivo, no con una constante → respetada: la constante
  `PPU_NATIVA_ACTIVA` **desapareció** de `src/platform.rs`; ahora decide la marca.
- El APK de prueba lleva los tres ABIs → respetada: el APK empaquetado lleva `arm64-v8a`,
  `armeabi-v7a` y `x86_64`, con la biblioteca del port y la del AOT recompilado en cada una.
- No tocar `src-c/` ni el motor upstream → respetada: los únicos ficheros de código tocados están
  en `src/` y `scripts/`.
- ¿Se tocó algo que alguna restricción prohibía? no
- Imprevistos que surgieron: la referencia de API no recoge los módulos que no están en la lista
  curada del generador → criterio de prioridad aplicado: no (queda como hallazgo, no bloquea)
- ¿Reapareció alguna alternativa descartada? no

### Archivos modificados

- `src/ppu_nativa.rs` — la constante `NOMBRE_BANDERA` y la función `bandera_activada`.
- `src/ppu_nativa_tests.rs` — 5 tests (marca ausente, presente, retirada, nombre exacto).
- `src/perf.rs` — `ppu_nativa` en la muestra, `nativa_frames` en las estadísticas y el campo
  `nativa=` en la línea `# ppu_hw`.
- `src/perf_tests.rs` — el test nuevo y el texto esperado de la línea, que ahora lleva un campo más.
- `src/platform.rs` — se elimina `PPU_NATIVA_ACTIVA`; construcción perezosa del pipeline; el
  estado de la app reevalúa la marca al arrancar y una vez por segundo.
- `scripts/diagnostico/analizar_telemetria.py` — `nativa_total` en el informe.
- `scripts/diagnostico/mutar_ppu_nativa_bandera.py` — **no anunciado**: es el script de mutación
  que exige la fase 4, siguiendo el patrón del que ya existía para la fase 1.
- `docs/guias/medir-rendimiento.md` — la marca, la línea `# ppu_hw` y el campo `nativa=`.
- `docs/BACKLOG.md` — el estado de la PPU nativa y el registro de hallazgos.
- `docs/referencia/api-modulos-rust.md` — regenerada.
- `docs/propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md` — esta propuesta.

¿Coincide con lo anunciado? no: el script de mutación es el único fichero no anunciado, y la
referencia de API no contiene lo que se predijo (ver «Discrepancias»).

### Discrepancias

- La referencia de API no recoge `NOMBRE_BANDERA` ni `bandera_activada` → información faltante: se
  había predicho que aparecerían. El generador (`scripts/extraer_referencias.py`) solo recorre una
  lista curada de módulos y `src/ppu_nativa.rs` no está en ella —igual que `src/present.rs`— así
  que `--check` pasa y los dos símbolos no aparecen. Queda como hallazgo de impacto bajo.

### Para la próxima vez

- Antes de predecir qué aparece en la referencia de API, mirar la lista de módulos del generador:
  no cubre todo `src/`, y por eso el `--check` puede pasar con la referencia incompleta.

### Fuera de alcance (hallazgos que no se arreglaron)

- La referencia técnica generada (`docs/referencia/api-modulos-rust.md`) no cubre `src/ppu_nativa.rs` (el generador tiene una lista curada de módulos), así que la constante `NOMBRE_BANDERA` y la función `bandera_activada` no aparecen en ella aunque `--check` pasa. — impacto: bajo
- Al retirar la marca, el pipeline de la PPU nativa sigue construido y con sus texturas reservadas en memoria de vídeo hasta que la app se reinicia: se deja de usar, pero no se libera. — impacto: bajo
- El script de empaquetado anuncia la firma v1, pero `apksigner` no la aplica: con `minSdkVersion 26` la firma v1 queda sin efecto y el verificador la da por ausente. — impacto: bajo

---

## Discrepancia (solo si el estado es «discrepada»)

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana

- La imagen del juego en el dispositivo (visible): ✔ | ✘ | ?
- La ruta fiable para dejar la marca en el dispositivo (permisos de almacenamiento): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
