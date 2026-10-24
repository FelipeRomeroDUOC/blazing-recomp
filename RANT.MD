# RANT.MD

> Cuarta lectura, 2026-10-24. Perfil: desarrollador con experiencia en descompilación y recompilación de juegos, que llega a este repositorio sin conocerlo y espera aprender cómo funcionan por dentro la consola y el juego. Leo README, portal de docs, las cuatro guías, las seis referencias (la de API solo el índice), los ocho documentos de explicación, el índice de los 46 ADR y los ADR 40-42, el glosario, el changelog, `PENDIENTES`, las primeras 526 líneas del BACKLOG y su cola, los README de `examples/`, `scripts/diagnostico/` y `docs/historico/`, y los comentarios de cabecera de `src/`. No leo la lógica del código.

Esta vez el núcleo se sostiene. Las cifras cierran, el estado de los ADR está resuelto y las guías ya no me mandan a documentos que se contradicen entre sí. El problema se mudó al documento que se presenta como «tablero vivo»: el BACKLOG. Ahí es donde vuelvo a no saber qué es verdad.

---

## Lo que sí funcionó

- **El índice de ADR con estado.** `[Vigente]`, `[Sustituida por…]`, `[Histórica…]` y el bloque `Estado:` de cada entrada resuelven el problema más caro que tenía la documentación. El ADR 41 ya dice «Parcialmente vigente. Marco conceptual rector activo; Fase 1 (PPU) sustituida por ADR 42; Fases 2 y 3 pendientes». Eso es exactamente lo que necesitaba para no leer 1.205 líneas a ciegas.
- **Las cifras de `src-c/` cierran.** La tabla de `arquitectura-c-recompilado.md` suma 7.256.939 líneas (640.265 + 5.803.603 + 813.071), el README repite la misma cifra, `src-c/` tiene 12 unidades y `rcg_block_count = 144875` está donde dice. Las 18.483 etiquetas, las 23 ARM y las 18.460 Thumb también cuadran.
- **El estado de IWRAM ya no se disfraza.** `FORMATO_LABELS.md` y el paso 4 de `mapear-labels-aot.md` dicen sin rodeos que las 18.483 etiquetas son de ROM y que **ninguna** de IWRAM está catalogada. Lo comprobé: 18.483 direcciones, 0 en `0x03xxxxxx`. El 17.207 aparece explicado como cosecha inicial, no como un recorte misterioso.
- **La guía de rendimiento da un inicio rápido de tres pasos.** El paso 0 con `adb pull` y `analizar_telemetria.py --top 10` es lo que un lector nuevo necesita antes de perderse en la tabla de campos.
- **La telemetría tiene un dueño único.** `telemetria/esquema.txt` gobierna el escritor y el lector; `--volcar` y `--comparar` ya están documentados en `comandos-cli.md`; la explicación de `# ppucamino`, `# regiones` y `# ppu_hw` es de las mejores páginas del repositorio.
- **`ppu-caminos-de-composicion.md` avisa en la primera línea de que es histórica** y de que los ejercicios necesitan `../gba-recomp`. Eso evita el malentendido de creer que el rasterizado por software es la ruta de producción.
- **El archivo del shim está bien contado.** `examples/README.md` dice 18 arneses activos, 29 archivados y 10 suites; `docs/historico/ppu-shim/README.md` lista los 39 archivos y explica por qué no se compilan.
- **La guía del APK ya nombra el repositorio hermano en Requisitos** y enseña a copiar ROM y BIOS con `adb push`. Ese era el bloqueo más tonto y ya no está.
- **`api-modulos-rust.md` ya incluye `present.rs` y `ppu_nativa.rs`**, y `--check` pasa. La referencia generada dejó de tener el hueco que tenía.

---

## Quejas de comprensión

### El tablero vivo dice que el shim sigue ahí y que es el fallback

- **Dónde:** `docs/BACKLOG.md`, «Áreas de Trabajo y Estado Real» §3 y «Prioridad: lo alto que sigue vivo» #6.
- **Texto:** «si la GPU del dispositivo rechaza un recurso el guardián de `wgpu` devuelve el juego al shim»; «El shim de la PPU sigue en el repositorio […] borrar `src/ppu_shim.*` (que además están sin versionar, así que el borrado es irreversible)».
- **Qué me pasó:** vengo de leer el ADR 42, el glosario, la guía de rendimiento y el documento del hardware shim, y todos dicen que el `PpuShim` está retirado, archivado en `docs/historico/` y que el fallback es el rasterizador por software de `gba-core`. El BACKLOG dice lo contrario: que sigue en el repositorio y que es a donde vuelve la app. Me quedé un buen rato mirando las dos versiones sin saber cuál manda. Si el BACKLOG es el tablero vivo, esto no es un detalle.
- **Qué esperaba encontrar:** una sola verdad: «el shim se retiró; el fallback es software» o «el shim sigue y es la reserva», con el otro párrafo marcado como resuelto.

### El BACKLOG tiene tres cifras de tests y dos fechas

- **Dónde:** `docs/BACKLOG.md`, cabecera y «Estado del Proyecto»; `AGENTS.md`, checklist de pre-commit.
- **Texto:** «**Última actualización:** 2026-10-20»; «**Estado de la suite:** 250 tests en verde (172 en los módulos de `src/`, 78 de integración en `tests/`)»; y, en el registro, «corregido el 2026-10-22 a 285: 184 en `src/` + 101 en `tests/`».
- **Qué me pasó:** el índice de propuestas lista fichas del 2026-10-24, así que la fecha de cabecera ya no es la del documento. Y al contar con el comando que la propia documentación usa (`cargo test -- --list`) salen **257** tests: 172 del crate y 85 de integración. No sé si el 250 es una foto vieja, si el 285 fue un pico y luego se borraron suites, o si me falta algo. Si el número es el criterio de «suite al 100 % en verde», necesito el número correcto.
- **Qué esperaba encontrar:** «257 a 2026-10-24 (172 + 85)», o «250 a 2026-10-20» con una nota de que la cifra se congela a esa fecha.

### El registro de hallazgos mezcla lo abierto con lo cerrado

- **Dónde:** `docs/BACKLOG.md`, «Registro de Hallazgos».
- **Texto:** «El índice de propuestas de `docs/BACKLOG.md` sigue en 58 fichas»; «`AGENTS.md` §6 indica 63 tests; el código tiene 77»; «`src/ppu_shim.*` […] siguen sin versionar».
- **Qué me pasó:** la cabecera dice que ahí viven «los hallazgos que aún no se han arreglado», así que lo leí como una lista de trabajo. Tres de las entradas que comprobé ya están resueltas: el índice dice 157 fichas, el recuento de tests actual es 257 y los archivos del shim ya no están en el árbol activo (ni en `src/`, ni en `examples/`, ni en `tests/`). No puedo usar el registro para saber qué queda vivo.
- **Qué esperaba encontrar:** cada hallazgo con `Estado: abierto | cerrado`, o un registro histórico separado del tablero.

### La arquitectura dice que la PPU por software se eliminó

- **Dónde:** `docs/explicacion/arquitectura-port-android.md`, «Alternativas y Compromisos» #2.
- **Texto:** «**Cero emulación de PPU en CPU:** Se eliminó la rasterización por software línea a línea (`render_scanline`), asumiendo el compromiso de resolver fondos, sprites y mezclas directamente en shaders de GPU (`wgpu`)».
- **Qué me pasó:** la guía de rendimiento explica cómo conmutar al rasterizador por software con `ppu-nativa-off.flag`; el documento del hardware shim dice que la app vuelve al rasterizador por software ante fallos de GPU; el glosario define la «PPU de Referencia» como rasterizado por software. «Se eliminó» suena a que ya no existe, pero es exactamente el código al que se vuelve. No sé si `render_scanline` sigue en el producto o solo en el motor upstream.
- **Qué esperaba encontrar:** «la ruta de producción no la usa; se conserva como contingencia y diagnóstico», que es lo que dice el ADR 42.

### La guía de rendimiento dice que la app prefiere OpenGL ES

- **Dónde:** `docs/guias/medir-rendimiento.md`, «Líneas de calibración de presentación».
- **Texto:** «Durante el experimento de presentación la aplicación prefiere OpenGL ES, así que **solo deben compararse registros del mismo backend**».
- **Qué me pasó:** el `CHANGELOG` dice «Restauración de Vulkan como backend gráfico preferente: `PREFERIR_OPENGL_ES = false`», y `analisis_rendimiento.md` dice «Vulkan es el backend preferente». La guía me pide comparar logs del mismo backend, pero me dice que el backend por defecto es el contrario del que el resto del repositorio declara. No sé qué voy a ver en el encabezado de mi primer log, y por tanto no sé cómo diseñar el A/B.
- **Qué esperaba encontrar:** «Vulkan es el preferente; OpenGL ES es el respaldo automático», o el valor exacto de la constante si el experimento la cambia.

### La guía de rendimiento dice que la calibración está apagada fuera del experimento

- **Dónde:** `docs/guias/medir-rendimiento.md`, «Líneas de calibración de presentación».
- **Texto:** «está **activa** mientras dure el experimento […] fuera de él queda **desactivada por defecto**».
- **Qué me pasó:** el BACKLOG dice que «la calibración de presentación está **activa** por el experimento» y `PENDIENTES` tiene una tarea abierta para «apagar la constante `CALIBRAR_PRESENTACION` tras concluir los experimentos». Si sigue activa, «fuera de él queda desactivada» no describe el estado actual, y no hay un «fuera de él» que un lector pueda ver. No sé si los primeros 12-15 segundos de mi sesión son calibración o juego normal.
- **Qué esperaba encontrar:** «hoy la constante vale `true`; se apagará al cerrar el experimento», con la fecha o la propuesta que lo decide.

### Dos rangos de IWRAM que no se tocan

- **Dónde:** `docs/guias/mapear-labels-aot.md`, paso 4; `docs/explicacion/anatomia-motor-fire-emblem-7.md` §1; `docs/PENDIENTES.md` §4.
- **Texto:** la guía dice «Cálculos de ataque, evasión y animación de batalla (`0x03002B80`–`0x03003200`)»; la anatomía dice «`0x030035C8` a `0x030036CC`»; PENDIENTES dice «`0x030035C8`, `0x030036CC`, etc.».
- **Qué me pasó:** los rangos no se solapan. Si sigo la guía y desensamblo `0x03002B80–0x03003200`, no encuentro los hotspots que la anatomía y PENDIENTES nombran. Si sigo a la anatomía, no sé por qué la guía da otra dirección. Para alguien que viene de descompilar, esto es el equivalente a que dos mapas del mismo edificio marquen puertas distintas.
- **Qué esperaba encontrar:** un rango canónico, o «son dos bloques distintos: A en … y B en …».

### `IntrMain` está en una dirección que es un puntero

- **Dónde:** `docs/guias/mapear-labels-aot.md`, paso 4.
- **Texto:** «Despacho de interrupciones (`IntrMain`, `0x03007FFC` / `0x030028A0`)».
- **Qué me pasó:** la anatomía explica que `0x03007FFC` es la palabra de la IWRAM que contiene el **vector de usuario**, y que ese vector apunta a `IntrMain`, que está copiado en otra dirección. Si añado una etiqueta en `0x03007FFC`, etiqueto un puntero, no una función. La guía me está dando la dirección del dedo en vez de la de la puerta.
- **Qué esperaba encontrar:** «vector de usuario en `0x03007FFC`; `IntrMain` se copia a …».

### La anatomía promete firmas de llamada en las etiquetas

- **Dónde:** `docs/explicacion/anatomia-motor-fire-emblem-7.md` §7, punto 3.
- **Texto:** «Las 18.483 funciones catalogadas en el archivo de configuración del proyecto asignan identificadores semánticos y firmas de llamada a cada subrutina en ROM».
- **Qué me pasó:** `FORMATO_LABELS.md` lista `name`, `address`, `mode` y `end`; no hay firmas. Busqué las firmas para entender la ABI y no encontré ninguna. Las etiquetas solo nombran puntos de entrada. La frase me hizo perder tiempo y, peor, me hizo desconfiar del resto de la sección.
- **Qué esperaba encontrar:** «nombre, dirección y modo; la firma no está en el mapa».

### El blob de IWRAM no tiene procedimiento

- **Dónde:** `docs/PENDIENTES.md` §4; `docs/guias/mapear-labels-aot.md`, paso 4.
- **Texto:** «Capturar el blob de 32 KB de memoria viva de IWRAM (`iwram_dump.bin`) durante el arranque/combate mediante el arnés de ejecución determinista»; la guía dice que `recomp build` «exige un volcado de memoria viva (*blob* binario de 32 KB capturado con el estado de la RAM inicializada)».
- **Qué me pasó:** quiero hacer la tarea. No hay script, comando, escena, fotograma ni ruta de salida. La guía dice «para mapear funciones normales en ROM, continúa con el Paso 5», pero para IWRAM se detiene en «es una tarea pendiente». No puedo producir `iwram_dump.bin` ni sé cómo se lo doy a `recomp build`. Que esté declarado como pendiente es honesto, pero el itinerario del portal anuncia «mapear subrutinas en ROM (con análisis de limitación en IWRAM)» y el lector que quiere ir más allá se queda en la puerta.
- **Qué esperaba encontrar:** el nombre del arnés, la escena o el fotograma, el comando y el destino del blob.

### El empaquetado no nombra la segunda biblioteca

- **Dónde:** `docs/guias/compilar-y-empaquetar-apk.md`, paso 2; `docs/referencia/comandos-cli.md`, tabla de scripts.
- **Texto:** «Compila las bibliotecas nativas `libblazing_recomp.so` para `arm64-v8a`, `armeabi-v7a` y `x86_64`»; «Incorpora las bibliotecas compartidas `libblazing_recomp.so` en las rutas `lib/<abi>/`».
- **Qué me pasó:** `AGENTS.md` y `arquitectura-c-recompilado.md` dicen que el APK lleva **dos** bibliotecas: el runtime Rust y `libblazing_recompiled.so` con los 144.875 bloques. La guía de empaquetado solo nombra la primera. No sé cómo se compila la C11 ni cómo entra en el APK, así que no puedo verificar que el juego vaya dentro. Para un proyecto de recompilación, esa es la biblioteca que importa.
- **Qué esperaba encontrar:** «compila `libblazing_recomp.so` y `libblazing_recompiled.so` (esta con `compilar_c_paralelo.py`)».

### El índice de explicación da el shim por archivado, pero su documento dice que faltan dos fases

- **Dónde:** `docs/explicacion/README.md`, tabla de documentos; `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`, nota de estado.
- **Texto:** «Diseño conceptual para sustituir el hardware emulado por GPU nativa, completado en Fase 1 (`NativePpu`) y archivado según ADR 42»; el documento dice «Fase 2 (Bus plano y timing por VSync): *Pendiente de implementación*» y «Fase 3 (Audio nativo AAudio / desacople de `gba-core`): *Pendiente de implementación*».
- **Qué me pasó:** «archivado» suena a iniciativa cerrada y a documento que ya no hace falta. El documento dice que solo la Fase 1 está sustituida; las otras dos son futuro. Casi me salto la página, que es justo la que explica el marco conceptual que el BACKLOG da por vigente.
- **Qué esperaba encontrar:** «Fase 1 sustituida; Fases 2-3 pendientes (documento conceptual)».

### `sub_8059E7C` no está en el mapa de etiquetas

- **Dónde:** `docs/explicacion/anatomia-motor-fire-emblem-7.md` §1 y §7.
- **Texto:** «rutinas de interpolación y cálculo de combate en ROM como `sub_8059E7C`»; «las 18.483 funciones […] asignan identificadores semánticos».
- **Qué me pasó:** busqué `sub_8059E7C` en `blazing.labels.toml` y no existe. No es una de las 18.483. El comando del §7 (`--buscar-prologo 0x08059E7C`) puede comprobar que hay un prólogo, pero no que ese sea el nombre. ¿Es un nombre de la descompilación `fireemblem7u` o una etiqueta del proyecto? Para alguien que viene de recompilar, la diferencia importa.
- **Qué esperaba encontrar:** «nombre de la descompilación `fireemblem7u`; no está en el mapa», o el nombre real de la etiqueta.

---

## Quejas de legibilidad

### El checklist de `AGENTS.md` está en §7, pero el BACKLOG manda a §6

- **Dónde:** `docs/BACKLOG.md`, cabecera y «Checklist de Pre-Commit».
- **Texto:** «El checklist de pre-commit vive en `AGENTS.md` §6»; «Los 5 criterios obligatorios […] están en `AGENTS.md` §6».
- **Qué me pasó:** fui a §6 y encontré el pipeline operativo de cinco fases, no el checklist. El checklist está en §7, «Notas y Reglas Personalizadas del Usuario». Tuve que buscar en todo el archivo. Es un enlace interno que manda al sitio equivocado.
- **Qué esperaba encontrar:** `§7` o un enlace directo al encabezado del checklist.

### `FORMATO_LABELS` empieza en 3.1

- **Dónde:** `docs/referencia/FORMATO_LABELS.md`, «Campos y atributos».
- **Texto:** «### 3.1 Cabecera», «### 3.2 Entradas de Funciones (`[[functions]]`)».
- **Qué me pasó:** busqué las secciones 1 y 2 para seguir el orden. No existen: el documento tiene encabezados `##` sin numerar y, de pronto, subsecciones 3.1 y 3.2. Pensé que faltaba una página o que había saltado un fragmento.
- **Qué esperaba encontrar:** subsecciones sin número, o una estructura 1/2/3 completa.

### El README no lista `examples/` ni `telemetria/`

- **Dónde:** `README.md`, «Estructura del Proyecto».
- **Texto:** la lista va de `src/` a `tests/`, `blazing.labels.toml` y `pack.toml`; no aparecen `examples/` ni `telemetria/`.
- **Qué me pasó:** las guías y el glosario hablan de 18 ejemplos y de `telemetria/esquema.txt`; no los encontré desde el README. `examples/README.md` solo se enlaza desde el glosario. Para un lector que viene de recompilación, los arneses de micro-benchmarking son la parte más interesante y están escondidos.
- **Qué esperaba encontrar:** ambos directorios en la estructura, o un enlace a sus README.

### El diagrama del portal no incluye Fuentes ni Histórico

- **Dónde:** `docs/README.md`, «Cuadrantes de Documentación».
- **Texto:** el diagrama Mermaid muestra Guías, Referencia, Explicación, Propuestas y Backlog; la misma página tiene una sección 4 (Fuentes de Verdad Externa) y una sección 6 (Archivo Histórico).
- **Qué me pasó:** usé el diagrama para decidir qué leer y me quedé con la idea de que solo había tres cuadrantes y dos anexos. Fuentes e Histórico aparecen después, sin lugar en el mapa. No supe si eran parte de Diátaxis o material aparte.
- **Qué esperaba encontrar:** los dos nodos en el diagrama, o una nota de que quedan fuera de los cuatro cuadrantes.

### El índice de `api-modulos-rust` se corta a mitad de frase

- **Dónde:** `docs/referencia/api-modulos-rust.md`, «Índice de Módulos».
- **Texto:** «Entrada táctil ergonómica (Fase 5.1, propuesta |»; «Overlay táctil y marco GBA SP (Fase 5.2, propuesta |»; «Módulo del shader del overlay: incluye el WGSL como constante y lo |»; «Plataforma Android (solo compila con `cfg(target_os = \"android\")`): |».
- **Qué me pasó:** leí el índice para elegir un módulo y cuatro descripciones terminan a mitad de frase. La tabla es válida, pero el texto termina justo antes del cierre de la celda. No sé qué propuesta es ni qué sigue después de «y lo». Es una referencia generada: si la tabla no se lee, la referencia no sirve para orientarse.
- **Qué esperaba encontrar:** descripciones completas o un enlace a la sección del módulo; si el generador se corta con un `|`, que lo escape.

### El registro de hallazgos son 200 líneas de viñetas sin estado

- **Dónde:** `docs/BACKLOG.md`, «Registro de Hallazgos».
- **Texto:** «Los hallazgos que aún no se han arreglado viven en el registro de hallazgos», seguido de una lista plana agrupada por propuesta, con «impacto: bajo/medio/alto» al final de cada viñeta.
- **Qué me pasó:** intenté recorrerlo para saber qué queda. A las 200 líneas me rendí. No hay `abierto/cerrado`, no hay responsable, no hay prioridad distinta del sufijo de impacto, y los mismos asuntos (el shim, el commit del motor, los tests) aparecen también en «Prioridad: lo alto que sigue vivo». No sé cuál de las dos listas es la canónica.
- **Qué esperaba encontrar:** una tabla con estado, impacto y enlace a la propuesta, o un filtro por estado; y una sola lista de prioridades.

---

## Suposiciones que tuve que hacer

- **Supuse que `0x030035C8` era el bloque de combate** porque aparece en la anatomía y en PENDIENTES. La guía da otro rango (`0x03002B80–0x03003200`) y no sé si son dos bloques o un error. Seguí con el de la anatomía, pero apostando.
- **Supuse que el primer arranque tendría un selector de archivos** porque `AGENTS.md` dice que la app «debe incluir un flujo inicial donde el usuario selecciona su propia copia». El README y la guía del APK enseñan a copiar por ADB, y PENDIENTES dice que el selector está pendiente. Seguí con ADB.
- **Supuse que el «Registro de Hallazgos» era una lista de tareas** porque la cabecera dice que ahí viven los hallazgos no arreglados. No lo es: mezcla resueltos y abiertos.
- **Supuse que `AGENTS.md §6` era el checklist** porque el BACKLOG lo dice dos veces. No lo es: el checklist está en §7.
- **Supuse que el número de tests era 250** porque lo dicen el BACKLOG y AGENTS. `cargo test -- --list` dice 257. Seguí con el comando, no con la cifra.
- **Supuse que el shim estaba retirado** porque lo dicen el ADR 42, el glosario y el documento del hardware shim. El BACKLOG dice que sigue. Seguí con el ADR, pero sin poder resolver la contradicción.

**Bloqueos:**

- **No pude producir el blob de IWRAM.** No hay comando ni arnés documentado; la guía se detiene en «es una tarea pendiente».
- **No pude verificar el índice de `api-modulos-rust`** más allá de la tabla: el cuerpo es generado y tiene más de 3.000 líneas; no es una página de lectura.
- **No pude leer las 157 propuestas una por una.** Leí el índice y las más recientes.
- **No pude leer las cinco fuentes técnicas completas.** Leí el README del catálogo.

---

## Dónde me rendí

- **`docs/BACKLOG.md`, «Registro de Hallazgos»:** después de unas 200 líneas de viñetas sin estado, dejé de leer. No podía distinguir lo vivo de lo cerrado, y el esfuerzo no parecía rentable.
- **`docs/referencia/api-modulos-rust.md`:** leí el índice y las primeras secciones; el resto es una lista generada de 3.000+ líneas. No es un documento para aprender, sino para consultar.
- **`docs/propuestas/`:** 157 archivos. Leí el índice, las propuestas recientes y las que el BACKLOG enlaza; no todas.
- **`docs/fuentes/`:** cinco manuales largos. Leí el catálogo y las referencias que las guías citan, no los cuerpos completos.
- **`docs/explicacion/DECISIONES.md`:** leí el índice y los ADR 40-42; no los 46 completos.

---

## Los bloqueos más graves

1. **El BACKLOG se contradice con el ADR 42 en el shim y el fallback, y mezcla fechas y cifras de tests.** Es el documento que se presenta como tablero vivo; si ahí no se puede confiar, el lector no sabe qué queda pendiente ni qué es producción.
2. **IWRAM no es ejecutable:** dos rangos incompatibles, `IntrMain` en la dirección de un puntero y ningún procedimiento para capturar el blob. El itinerario anuncia esa ruta y el lector que quiere seguirla se detiene.
3. **La arquitectura dice que la PPU por software se eliminó, mientras las guías la usan como fallback.** El lector no puede separar la ruta de producción de la contingencia, que es justo lo que el ADR 42 quería dejar claro.
4. **La guía de rendimiento contradice al CHANGELOG y al BACKLOG en el backend preferido y en el estado de la calibración.** El protocolo de comparación A/B queda sin base: no sé qué backend ni qué configuración estoy midiendo.
5. **Las etiquetas se presentan como si llevaran firmas de llamada, y no las llevan.** Para un lector que viene de recompilación, la ABI es el mapa; prometerlo y no darlo desarma la anatomía.
