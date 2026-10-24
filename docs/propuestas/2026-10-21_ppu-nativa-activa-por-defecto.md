# Propuesta: La PPU nativa queda activa por defecto y se apaga con un archivo nuevo

Fecha: 2026-10-21
Nivel: 2
Estado: verificada por agente (todo lo comprobable sin el dispositivo; queda la revisión humana en el móvil)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El juego sale del port dibujado por la **PPU nativa** (composición por píxel en la GPU, con la
aritmética de mezcla de la GBA) sin que haya que tocar nada: hasta ahora hacía falta crear el
archivo `ppu-nativa.flag` en el almacenamiento del móvil, y sin él la app pintaba con el *shim*.
A partir de ahora la PPU nativa es la ruta por defecto y, para volver al *shim* y comparar A/B en
el propio dispositivo, basta con crear el archivo **nuevo** `ppu-nativa-off.flag`: la app se da la
vuelta en caliente, una vez por segundo, sin reiniciar.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs` (`NOMBRE_BANDERA`, `bandera_activada`),
  `src/platform.rs` (arranque, `reevaluar_ppu_nativa`, cabecera de telemetría, construcción perezosa
  del pipeline), `src/ppu_nativa_tests.rs` (los cuatro tests de la marca),
  `scripts/diagnostico/mutar_ppu_nativa_bandera.py` (las tres mutaciones apuntan al texto viejo),
  `docs/guias/medir-rendimiento.md` (sección «Probar la PPU nativa en el dispositivo»),
  `docs/BACKLOG.md`, `docs/propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md`,
  `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`, y el código de `wgpu` 26.0.1 del registro de
  Cargo (`src/backend/wgpu_core.rs`: `handle_error`, `default_error_handler`, `pop_error_scope`).
- Intención: poder **medir en el dispositivo** la PPU nativa antes de corregir el motor hermano
  (`docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md`, hoy `pendiente`). La PPU nativa
  compone desde VRAM en la GPU, así que el defecto del camino rápido del software no se ve en
  pantalla y las dos rutas se pueden medir por separado. El éxito es que un APK recién instalado
  ya dibuje con la PPU nativa y que se pueda volver al *shim* creando un archivo.
- Restricciones y decisiones previas (máx. 3):
  - La imagen de la PPU nativa no cambia (origen: `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`,
    «verificada por agente») → implica: no se toca `src/ppu_nativa.wgsl` ni la aritmética de
    `src/ppu_nativa.rs` → protegida por «La imagen de la PPU nativa no cambia» | testeable.
  - El interruptor se decide en runtime con un archivo, no con una constante de compilación (origen:
    `docs/propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md`, «verificada por agente») → implica:
    el cambio de ruta se puede hacer sin reiniciar la app → protegida por «La marca de apagado se
    puede crear y borrar con la app en marcha» | testeable.
  - Un fallo de la GPU no puede matar la app (origen: `AGENTS.md` §2 y el blindaje de
    `src/error_screen.rs`, que existe porque `wgpu` entra en pánico ante un error de validación) →
    implica: la construcción de la PPU nativa va dentro de un ámbito de error de `wgpu`, con vuelta
    al *shim* si el dispositivo la rechaza → protegida por «Un dispositivo que rechaza la PPU nativa
    sigue jugando con el shim» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Dejar el interruptor como está (encender con `ppu-nativa.flag`) → descartada porque obliga a
    crear un archivo antes de cada prueba y porque el archivo de encendido y el de apagado no
    pueden convivir sin una regla ambigua.
  - Invertir el significado del archivo que ya existe (`ppu-nativa.flag` pasa a apagar) → descartada
    porque rompe un nombre ya documentado; un archivo nuevo lo dice todo sin ambigüedad.
  - Quitar el interruptor y dejar la PPU nativa como única ruta → descartada porque sin vuelta atrás
    no se puede comparar A/B en el dispositivo, que es justo lo que se quiere medir.
- Ante imprevistos, prioriza: que la app **no se rompa** en un dispositivo que no soporte la PPU
  nativa > que la PPU nativa esté activa por defecto > que el *shim* siga siendo una salida limpia.

### 3. Ejemplos

- Caso normal: APK recién instalado, sin ningún archivo en el almacenamiento → antes: la app dibuja
  con el *shim* y la telemetría trae `nativa=0` → ahora: la app dibuja con la PPU nativa y la
  telemetría trae `nativa=60` sobre `gpu=60`.
- Caso normal: con la app en marcha se crea `ppu-nativa-off.flag` en el directorio externo de la app
  → antes: ese archivo no existe en el protocolo y no cambia nada → ahora: en el siguiente tick de
  un segundo la telemetría pasa a `nativa=0` y la imagen vuelve a ser la del *shim*.
- Caso límite: el archivo antiguo `ppu-nativa.flag` de pruebas anteriores sigue en el
  almacenamiento → antes: encendía la PPU nativa → ahora: no hace nada (ya está encendida por
  defecto); para apagarla hay que crear `ppu-nativa-off.flag`.
- Caso límite: un dispositivo cuya GPU rechaza un recurso de la PPU nativa → antes: solo podía
  pasar si el usuario había creado la marca, y entonces la app moría con un pánico de `wgpu` → ahora:
  la app avisa en el Logcat y sigue con el *shim*.

### 4. Pasos

1. `src/ppu_nativa.rs`: `NOMBRE_BANDERA` y `bandera_activada` se sustituyen por
   `NOMBRE_BANDERA_APAGADO` (`ppu-nativa-off.flag`), `apagada_por_marca` y `ppu_nativa_activada`
   (que devuelve `true` salvo que exista la marca de apagado). El archivo antiguo deja de leerse.
2. `src/ppu_nativa.rs`: se añade `MotorDeErrores` (una Implementación real sobre `wgpu::Device` y
   otra de mentira para las pruebas) y `construir_con_guardian`, que abre un ámbito de error por
   cada filtro de `wgpu` (`Validation`, `OutOfMemory` e `Internal`), ejecuta la construcción y
   cierra los ámbitos; si alguno recogió un error, lo devuelve en vez de dejar que `wgpu` entre en
   pánico. `PpuNativaPipeline::new` pasa a devolver `Result<Self, String>` y usa el guardián.
3. `src/platform.rs`: el arranque y la reevaluación de una vez por segundo usan
   `ppu_nativa_activada`; si la construcción de la PPU nativa falla, se anota el fallo en el Logcat
   y el *shim* dibuja el resto de la sesión (sin reintentar).
4. `src/ppu_nativa_tests.rs`: los cuatro tests de la marca se reescriben con la semántica nueva y se
   añade uno para el ámbito de error.
5. `scripts/diagnostico/mutar_ppu_nativa_bandera.py`: las mutaciones apuntan al texto nuevo.
6. `docs/guias/medir-rendimiento.md`: la sección de la PPU nativa describe el estado por defecto, el
   archivo de apagado y las rutas de búsqueda.
7. `docs/BACKLOG.md` (estado de la PPU nativa, ítem 5 y registro de hallazgos) y la ficha de la
   propuesta del 2026-10-20, que queda anotada como sustituida en su interruptor.
8. `Cargo.toml`: se declaran los tres ejemplos que faltaban (`ppu_banda_nativa`, `ppu_barrido_nativa`,
   `ppu_modo_cuadro`). Es la reparación de la línea de base roja, declarada aparte en §7.
9. Se ejecuta la suite, el validador de documentación, la auditoría de referencias y el script de
   mutación; se reempaqueta el APK con el comando oficial.

### 5. Qué más podría verse afectado

- `src/platform.rs`: los cuatro puntos que tocaban la marca —el arranque (línea ~2121), la cabecera
  de telemetría (línea ~1546), la reevaluación de una vez por segundo (línea ~1790) y la construcción
  perezosa del pipeline (línea ~944)—, más el texto de los mensajes del Logcat.
- `src/ppu_nativa.rs`: la constante y la función de la marca, y el punto donde se construye el
  pipeline.
- `src/ppu_nativa_tests.rs`: los cuatro tests de la marca, más el test del guardián.
- `examples/ppu_banda_nativa.rs`, `examples/ppu_barrido_nativa.rs` y `examples/ppu_nativa_compara.rs`:
  los tres llaman a `PpuNativaPipeline::new`, así que hay que adapted al nuevo `Result`.
- `examples/ppu_proto.rs` y `examples/ppu_frente_a_frente.rs`: usan el *shim*, no la PPU nativa;
  no les afecta el cambio de firma (se confirma al compilar).
- `scripts/diagnostico/mutar_ppu_nativa_bandera.py`: las tres mutaciones que sustituyen el texto
  literal de `bandera_activada` y de `NOMBRE_BANDERA`, y el test que se ejecuta al final para
  comprobar la restauración.
- `docs/guias/medir-rendimiento.md`: la sección «Probar la PPU nativa en el dispositivo» (estado por
  defecto, nombre del archivo, rutas, mensajes del Logcat y comandos de `adb`).
- `docs/BACKLOG.md`: el estado de la PPU nativa, el ítem 5 («el shim sigue siendo la ruta de
  producción»), el índice de propuestas (lo regenera `scripts/actualizar_indice_propuestas.py`) y el
  registro de hallazgos.
- `docs/propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md`: su ficha, para que se vea que su
  interruptor cambió de sentido.
- `tests/packaging_tests.rs`: comprueba el vocabulario de la línea `Estado:` de todas las propuestas
  (esta propuesta entra en ese recuento).
- `Cargo.toml`: los bloques `[[example]]` de los tres ejemplos sin declarar.
- Búsqueda hecha: `grep -rn "bandera_activada\|NOMBRE_BANDERA\|ppu-nativa.flag\|ppu_nativa=" src/
  tests/ scripts/ docs/` para no dejar referencias colgando al nombre antiguo.

### 6. Suposiciones

- `wgpu` 26 deja el error de validación en el ámbito de error en vez de entrar en pánico →
  prototipar: sí
  → si es falsa: no habría forma de que un dispositivo que rechaza la PPU nativa sobreviva, y
  activar por defecto sería<'_dejar la app rota en todos los dispositivos salvo en el de pruebas_.
  → resultado del prototipo: confirmado leyendo `wgpu-26.0.1/src/backend/wgpu_core.rs`:
  `ErrorSinkRaw::handle_error` (líneas 621-652) busca el ámbito más interno **cuyo filtro coincida**
  con el del error y lo guarda ahí; si no hay ningún ámbito que coincida, llama al manejador no
  capturado y, si no lo hay, `default_error_handler` hace `panic!("wgpu error: …")` (línea 655).
  `create_render_pipeline` (línea 1332) reporta por esa misma vía. `pop_error_scope` (línea 1633)
  saca el último ámbito abierto y devuelve su error en un futuro ya listo
  (`Box::pin(ready(scope.error))`), así que no hace falta bombear el dispositivo: basta con
  `pollster::block_on`, que ya es dependencia del port. Como el filtro del error decide el ámbito
  que lo captura, hay que abrir **uno por filtro** y cerrar los tres.
- Todos los dispositivos objetivo admiten la PPU nativa → prototipar: no
  → si es falsa: el guardián del paso 3 devuelve el juego al *shim* y no hay caída; la imagen será
  la del *shim*, que es lo que hay hoy.
  → resultado del prototipo: no detectado: no hay aquí un dispositivo con la GPU de destino.
- El tirón de construir el pipeline en el primer fotograma es aceptable → prototipar: no
  → si es falsa: el primer fotograma tarda más de lo normal (compilación del shader y de cuatro
  pipelines). Medible en el dispositivo con la línea `t=` de la telemetría.
  → resultado del prototipo: no detectado: depende del dispositivo.

### 7. Alcance

- Incluye: la semántica del interruptor (PPU nativa activa por defecto, archivo nuevo para
  apagarla), el guardián de error de `wgpu` con vuelta al *shim*, los tests de la marca y del
  guardián, el script de mutación, la guía de medición, el BACKLOG, la ficha de la propuesta
  anterior y el reempaquetado del APK.
- Incluye (declarado aparte, no es parte de la PPU): la reparación de la línea de base de
  `cargo test`, que hoy está **roja** porque los ejemplos `ppu_banda_nativa`, `ppu_barrido_nativa` y
  `ppu_modo_cuadro` no están declarados en `Cargo.toml` y `ppu_banda_nativa` usa
  `gba_core::ppu::forzar_camino_general`, que solo existe con la feature `perf-video`. Sin esto no
  se puede exigir la suite en verde.
- No incluye: corregir el motor hermano (queda en `docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md`,
  `pendiente`), tocar `src/ppu_nativa.wgsl` o la aritmética de la PPU nativa, construir el pipeline
  de forma anticipada en lugar de perezosa, liberar la memoria de vídeo de la PPU nativa cuando se
  apaga, ni medir el coste en FPS de la PPU nativa (no se espera que cambie: el rasterizado por
  software sigue corriendo igual).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Sin ningún archivo, la PPU nativa está activa
Caso de uso: el APK recién instalado dibuja con la PPU nativa sin que nadie toque nada | Tipo: normal
  Dado que el almacenamiento del dispositivo no tiene ni `ppu-nativa.flag` ni `ppu-nativa-off.flag`
  Cuando la app decide qué ruta de dibujo usa
  Entonces la PPU nativa está activa
Test: `cargo test --lib ppu_nativa::tests::sin_marca_la_ppu_nativa_esta_activa`
Estado previo: falla (hoy, sin ningún archivo, la PPU nativa está apagada)

Escenario: La marca de apagado desactiva la PPU nativa y su retirada la vuelve a activar
Caso de uso: se puede volver al shim en caliente para comparar A/B | Tipo: normal
  Dado que la app está en marcha y la PPU nativa está activa
  Cuando se crea el archivo `ppu-nativa-off.flag` en uno de los directorios de telemetría
  Entonces la PPU nativa está apagada, y al borrar ese archivo vuelve a estar activa
Test: `cargo test --lib ppu_nativa::tests::` (los dos tests de la marca de apagado)
Estado previo: falla (hoy el archivo `ppu-nativa-off.flag` no existe en el protocolo)

Escenario: Un dispositivo que rechaza la PPU nativa sigue jugando con el shim
Caso de uso: una GPU que no admite algún recurso de la PPU nativa no mata la app | Tipo: error
  Dado que la construcción de la PPU nativa falla con un error de la GPU
  Cuando el compositor la construye
  Entonces la construcción devuelve el error en vez de entrar en pánico, y la app sigue dibujando
  con el shim
Test: `cargo test --lib ppu_nativa::tests::el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias`
Estado previo: falla (no compila: `construir_con_guardian` todavía no existe)

Escenario: La imagen de la PPU nativa no cambia
Caso de uso: el juego se ve igual que antes del cambio | Tipo: regresión
  Dado que la fase 1 reproduce la imagen de la PPU de referencia píxel a píxel
  Cuando se vuelve a renderizar el cuadro de aceptación y se compara con la referencia
  Entonces las diferencias son 0 de 38.400
Test: `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide`
(con `FE_ROM` apuntando a la ROM y el informe de `examples/ppu_nativa_compara.rs` regenerado)
Estado previo: pasa (regresión)

Escenario: La suite entera del port sigue verde
Caso de uso: el cambio no rompe nada de lo demás | Tipo: regresión
  Dado que la línea de base de `cargo test` está roja por los ejemplos sin declarar
  Cuando se declara los tres ejemplos en `Cargo.toml` y se ejecuta la suite
  Entonces `cargo test` termina en verde, sin errores ni pruebas rojas
Test: `cargo test`
Estado previo: falla (hoy `cargo test --no-run` no compila el ejemplo `ppu_banda_nativa`)

### 9. Cómo lo comprobaré

- Sin ningún archivo, la PPU nativa está activa → herramienta o comando: `cargo test --lib
  ppu_nativa::tests::sin_marca_la_ppu_nativa_esta_activa` → pasa si: el test pasa → no detecta: que
  la app de verdad lo tere con el mismo estado (eso lo comprueba el Logcat en el dispositivo).
- La marca de apagado desactiva y su retirada reactiva → herramienta o comando: `cargo test --lib
  ppu_nativa::tests::` → pasa si: los dos tests de la marca pasan → no detecta: que el archivo se
  encuentre en el directorio que la app busca de verdad en el dispositivo (eso lo comprueba el
  Logcat).
- Un dispositivo que rechaza la PPU nativa sigue jugando con el shim → herramienta o comando:
  `cargo test --lib ppu_nativa::tests::el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias`
  y `python scripts/diagnostico/mutar_ppu_nativa_bandera.py` → pasa si: el test pasa y la mutación
  que quita la apertura del ámbito hace que el test falle → no detecta: que `wgpu` meta el error en
  el ámbito y no entre en pánico (eso es comportamiento de la biblioteca, comprobado leyendo su
  código, no con un test) ni una GPU que acepta la construcción pero falla después al ejecutar (el
  guardián solo cubre la construcción).
- La imagen de la PPU nativa no cambia → herramienta o comando: `cargo test --release --test
  ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide` → pasa si: 0 diferencias de 38.400 →
  no detecta: las escenas que el cuadro de aceptación no ejercita (modos de fondo 3-5 y fondos
  afines, ya anotadas en el BACKLOG).
- La suite entera sigue verde → herramienta o comando: `cargo test` → pasa si: 0 errores y 0 pruebas
  rojas → no detecta: el comportamiento en el dispositivo, que no se ejecuta en el host.
- La documentación sigue siendo válida → herramienta o comando: `python
  .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py
  --check` → pasa si: ambos salen con código 0 → no detecta: que la guía describa mal el
  procedimiento en el dispositivo (eso lo comprueba la persona al leer el Logcat).
- El APK sale bien empaquetado → herramienta o comando: `powershell -ExecutionPolicy Bypass -File
  scripts/package_apk.ps1` → pasa si: el script termina sin error y deja el APK con los tres ABIs →
  no detecta: que la app funcione en el dispositivo (revisión humana).
- La imagen en el dispositivo (visible) → revisión humana: en el Logcat, `PPU nativa: ACTIVADA por
  defecto`; en `blazing-perf.txt`, `nativa=60` sobre `gpu=60`; y al crear `ppu-nativa-off.flag`,
  `nativa=0` un segundo después → no detecta: si la imagen que se ve es la correcta (eso lo ve la
  persona).

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el APK está reempaquetado con el comando oficial.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- La primera idea era dejar el interruptor como estaba y simplemente cambiar el valor por defecto
  dentro de la función. Se descarta: con un solo archivo no se puede expresar «activa» y «apagada» a
  la vez, y el nombre `bandera_activada` dejaría de describir lo que hace. De ahí el archivo nuevo
  con sentido explícito.
- La primera idea era construír el pipeline de la PPU nativa en el arranque del compositor, para
  que el primer fotograma no sufra el tirón de la compilación del shader. Se descarta en esta
  propuesta: cambia el orden de vida de la GPU y no hace falta para la prueba; queda anotado como
  mejora posterior.
- La primera idea de la redacción era «la PPU nativa se apaga con `ppu-nativa-off.flag`» sin más. Se
  completa con la regla de precedencia: si existen los dos archivos, el de apagado manda, porque el
  archivo antiguo es un residuo de las pruebas anteriores y no debe impedir apagar la PPU nativa.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: «la construcción de la PPU nativa se hace dentro de un ámbito de error de `wgpu`» → Ahora:
  igual, y además el ámbito cubre los tres filtros (`Validation`, `OutOfMemory` e `Internal`), no
  solo `Validation` → Por qué: `handle_error` clasifica el error en uno de esos tres filtros y  solo lo guarda en un ámbito con el filtro que coincide; con un solo filtro, un error de memoria
  agotada seguiría entrando en pánico.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Sin ningún archivo, la PPU nativa está activa: coincide
  Resultado real: `cargo test --lib ppu_nativa::tests::sin_marca_la_ppu_nativa_esta_activa` pasa: sin
  ningún archivo en los directorios de búsqueda, `ppu_nativa_activada` devuelve `true`. El nombre del
  archivo de apagado queda fijado por `el_nombre_de_la_marca_de_apagado_es_el_documentado`
  (`ppu-nativa-off.flag`).
  Método: el anunciado
- La marca de apagado desactiva la PPU nativa y su retirada la vuelve a activar: coincide
  Resultado real: pasan `la_marca_de_apagado_apaga_la_ppu_nativa` (con el archivo, apagada) y
  `quitar_la_marca_de_apagado_vuelve_a_encender_la_ppu_nativa` (al borrarlo, encendida).
  Método: el anunciado
- Un dispositivo que rechaza la PPU nativa sigue jugando con el shim: coincide en la parte comprobable
  Resultado real: el guardián abre un ámbito de error por cada filtro de `wgpu` (`Validation`,
  `OutOfMemory` e `Internal`), ejecuta la construcción y devuelve `Err` con el mensaje de la GPU en vez
  de dejar el pipeline a medias; lo comprueban
  `el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias`,
  `el_guardian_deja_pasar_una_construccion_sin_error` y
  `el_guardian_informa_de_todos_los_errores_recogidos`, y el script de mutación mata tanto la mutación
  que quita el ámbito como la que ignora el error. En `src/platform.rs` la construcción va dentro de ese
  guardián: si falla, se anota en el Logcat y el *shim* dibuja el resto de la sesión. Lo que no se puede
  comprobar aquí es que una GPU real rechace la construcción: no hay un dispositivo de destino en esta
  máquina.
  Método: el anunciado para el guardián; la parte del dispositivo queda para la revisión humana
- La imagen de la PPU nativa no cambia: coincide
  Resultado real: los informes de `examples/ppu_nativa_compara` se regeneraron con el código de hoy:
  cuadro 5300 → «diferencias frente a la referencia de estado de fin de cuadro: 0 de 38400» (control del
  arnés: 0) y cuadro 7500 → 0 de 38400, con las dos referencias coincidentes (0 diferencias). El test
  `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide` pasa con `FE_ROM`
  apuntando a la ROM y los informes en `volcados/`.
  Método: el anunciado
- La suite entera del port sigue verde: coincide
  Resultado real: `cargo test` → 264 tests en verde (175 en los módulos de `src/`, 89 de integración en
  `tests/`), 0 fallos, salida 0. Antes de esta propuesta la línea de base estaba **roja**: `cargo test
  --no-run` fallaba con código 101 porque `examples/ppu_banda_nativa.rs` usa
  `gba_core::ppu::forzar_camino_general`, que solo existe con la feature `perf-video` del motor base.
  Método: el anunciado
- El APK sale bien empaquetado: coincide
  Resultado real: `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` termina con código 0
  y deja `target/blazing-recomp-release.apk` (42,52 MB) con los tres ABIs (`arm64-v8a`, `armeabi-v7a` y
  `x86_64`), alineado a 16 KB y firmado (v2 y v3 verificados por `apksigner`).
  Método: el anunciado

### Contexto: ¿se respetó?

- La imagen de la PPU nativa no cambia → respetada: no se tocaron `src/ppu_nativa.wgsl` ni la aritmética de
  la PPU; los dos cuadros de aceptación siguen dando 0 diferencias de 38.400.
- El interruptor se decide en runtime con un archivo → respetada: `src/platform.rs` reevalúa la marca cada
  segundo; crear o borrar `ppu-nativa-off.flag` cambia la ruta de dibujo sin reiniciar la app.
- Un fallo de la GPU no puede matar la app → respetada **en la construcción**: el guardián devuelve el
  error y el compositor cae al *shim*. No cubre un error de `wgpu` al ejecutar un fotograma (queda
  anotado como hallazgo fuera de alcance).
- ¿Se tocó algo que alguna restricción prohibía? no: el shader, la aritmética de la PPU y el motor
  hermano (`../gba-recomp`) quedaron intactos.
- Imprevistos que surgieron: (a) la línea de base de `cargo test` estaba roja —la propuesta la daba por
  buena—; (b) las dos primeras mutaciones del script de mutación estaban invertidas y sobrevivían;
  (c) la suite tiene 264 tests, no los 261 que decía `AGENTS.md` → criterio de prioridad aplicado: sí
  (que la app no se rompa > PPU nativa por defecto > *shim* como salida limpia): la reparación de la
  línea de base se metió dentro de esta propuesta porque, sin ella, el criterio «la suite sigue verde» no
  era comprobable.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/platform.rs`
- `examples/ppu_banda_nativa.rs`, `examples/ppu_barrido_nativa.rs`, `examples/ppu_nativa_compara.rs`
- `scripts/diagnostico/mutar_ppu_nativa_bandera.py`
- `Cargo.toml` (los tres ejemplos sin declarar)
- `docs/guias/medir-rendimiento.md`, `docs/BACKLOG.md`, `AGENTS.md`
- `docs/propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md`,
  `docs/propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md` (esta ficha) y
  `docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md` (solo el marcador de «Fuera de alcance» de la
  plantilla)
- `volcados/ppu-nativa-5300.txt` y `volcados/ppu-nativa-7500.txt` (informes regenerados)

¿Coincide con lo anunciado? sí, salvo `AGENTS.md` y los dos informes de `volcados/`, que no estaban en la
lista del §5: el primero por el recuento de la suite y los segundos por la regresión de imagen.

### Discrepancias

- El guardián de `wgpu` solo cubre la construcción del pipeline: un error de la GPU durante la ejecución
  de un fotograma sigue entrando en pánico → omisión (el alcance pactado era la construcción).
- La PPU nativa, ya activa por defecto, se construye de forma perezosa: el primer fotograma paga la
  compilación del shader y de los cuatro pipelines → omisión (se descartó a propósito en «Correcciones
  previas a presentar»).

### Para la próxima vez

- Comprobar la línea de base de `cargo test` **antes** de escribir la propuesta: estaba roja y la
  propuesta la daba por buena, así que el criterio «la suite sigue verde» no era comprobable.

### Fuera de alcance (hallazgos que no se arreglaron)

- La marca antigua `ppu-nativa.flag` **deja de leerse**: si sigue en el almacenamiento de algún
  dispositivo, ya no hace nada (la PPU nativa está activa por defecto). Para apagarla hay que crear
  `ppu-nativa-off.flag`. | impacto: bajo
- El guardián de errores de `wgpu` solo cubre la **construcción** del pipeline: un error de validación
  que aparezca más tarde, al ejecutar un fotograma, seguiría entrando en pánico. | impacto: medio
- Al retirar la marca de apagado, el pipeline de la PPU nativa sigue construido y con sus texturas
  reservadas en memoria de vídeo hasta que la app se reinicia: se deja de usar, pero no se libera. |
  impacto: bajo

---

## Discrepancia (solo si el estado es «discrepada»)

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

- La imagen en el dispositivo (visible): el Logcat dice `PPU nativa: ACTIVADA por defecto`, `blazing-perf.txt` trae `nativa=60` sobre `gpu=60`, y al crear `ppu-nativa-off.flag` pasa a `nativa=0` un segundo después: ✔ | ✘ | ?
- Un dispositivo cuya GPU rechaza la construcción de la PPU nativa: la app avisa en el Logcat y sigue dibujando con el shim, sin morir: ✔ | ✘ | ?
- El APK recién instalado en el móvil arranca y dibuja con la PPU nativa sin ningún archivo previo: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
