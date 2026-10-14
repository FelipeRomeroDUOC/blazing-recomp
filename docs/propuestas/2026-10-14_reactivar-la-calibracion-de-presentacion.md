# Propuesta: Reactivar la calibración de presentación para medirla en el televisor

Fecha: 2026-10-14
Nivel: 2
Estado: aprobada (el usuario aprobó la vía disciplinada el 2026-10-14)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Al arrancar, la app vuelve a probar unas pocas configuraciones del swapchain (latencia 1/2/3, una
sonda a media resolución y otra sin VSync), mide lo que se bloquea en `present` en cada una, adopta
la mejor solo si mejora de forma medible, y deja el resultado escrito en la telemetría. Es el
**mismo barrido que ya existe**, que hoy está apagado por una decisión medida en una tablet.

### 2. Contexto

- Contexto consultado: el comentario de la propia constante y su uso
  ([`src/platform.rs`](../../src/platform.rs) 137-146 y 1550-1562), la medida de `present` y `dibujo`
  (1000-1069), el modo real que registra la cabecera (753), el mecanismo y sus pruebas
  ([`src/present.rs`](../../src/present.rs), [`src/present_tests.rs`](../../src/present_tests.rs)),
  el guardián ([`src/overlay_tests.rs`](../../src/overlay_tests.rs) 405-421), las propuestas
  [`2026-10-10_triple-buffering-swapchain-latencia-2.md`](2026-10-10_triple-buffering-swapchain-latencia-2.md)
  y [`2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md`](2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md),
  [`analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md),
  [`medir-rendimiento.md`](../guias/medir-rendimiento.md) y los logs
  [`log_tv15.txt`](../../scripts/diagnostico/log_tv15.txt) y `log_run13.txt`.
- Intención: saber si la configuración del swapchain puede bajar los ~7 ms que la telemetría llama
  `present` **en el Android TV**, que es el dispositivo objetivo. El éxito se ve como las líneas
  `# tramo` / `# resultado` / `# elegido` en `blazing-perf.txt`.
- Restricciones y decisiones previas (máx. 3):
  - El barrido está apagado **por decisión** (origen: `platform.rs:137-146`, decisión 2026-10-10) →
    implica: reactivarlo es **revisar** esa decisión, no borrarla; el guardián se actualiza con su
    motivo en vez de eliminarse → protegida por: *El guardián sigue protegiendo la decisión*.
  - La nitidez nativa está decidida (origen: la misma, «se decidió preservar la nitidez nativa») →
    implica: reducir la resolución **no se adopta nunca**; ese tramo queda como sonda → protegida
    por: *La nitidez no se toca*.
  - Los tests no se debilitan en silencio (origen: `AGENTS.md`, §4.5 y §5) → implica: la única
    aserción que cambia es el guardián, y cambia **con el motivo escrito al lado** → protegida por:
    *El guardián sigue protegiendo la decisión*.
- Alternativas descartadas:
  - Cambiar la constante a `true` sin tocar el guardián → descartada: rompería
    `test_decisiones_de_presentacion_declaradas` y contradiría una decisión registrada sin dejar
    rastro.
  - Leer solo la cabecera de un log nuevo (que ya registra el modo real) → descartada **como
    sustituto**, no como primer paso: dice en qué modo está el TV, pero no si la latencia o la escala
    ayudarían. Se usará de todos modos al leer el resultado.
  - Atacar la resolución como valor por defecto → descartada: es la decisión de nitidez del usuario,
    no una decisión técnica.
  - Solapar la espera con la emulación (hilo aparte) → descartada **por ahora**: es un cambio
    arquitectónico grande y aún no sabemos si hay espera que solapar.
- Ante imprevistos, prioriza: la imagen nativa por encima de cualquier ganancia de `present`.

### 3. Ejemplos

- **Caso normal.** Al arrancar:
  antes: log `calibración de presentación desactivada (nitidez nativa por decisión)` y la telemetría
  **sin ninguna línea `# tramo`**.
  ahora (objetivo): la telemetría lleva cuatro o cinco bloques
  `# tramo idx=N lat=… modo=… escala=… [referencia|sonda]`, cada uno con su
  `# resultado idx=N present=…ms fps=…`, y **una sola** línea
  `# elegido lat=… modo=… escala=… (motivo)`.
- **Caso límite.** Si en el TV nada supera a la referencia, el log debe cerrar con
  `# elegido … referencia (sin mejora medible sobre la referencia (X ms))` y la constante vuelve a
  `false` en el mismo cambio de cierre.
- **Caso de guardián.** La aserción
  antes: `platform_src.contains("const CALIBRAR_PRESENTACION: bool = false;")`
  ahora: la misma aserción sobre `= true;`, con el motivo escrito apuntando a esta propuesta.

### 4. Pasos

1. Poner `CALIBRAR_PRESENTACION` a `true` y actualizar su comentario para que describa el
   experimento en curso (medición en el televisor) además del resultado de la tablet, que se
   conserva.
2. Actualizar en el mismo cambio el guardián `test_decisiones_de_presentacion_declaradas`: la
   aserción pasa a exigir `= true;` y su mensaje explica por qué y enlaza esta propuesta, de modo
   que el estado del experimento no pueda derivar en silencio en ninguna de las dos direcciones.
3. Pasar la suite del port (128 verdes) y comprobar por mutación que el guardián mata el cambio
   contrario (poner la constante a `false` debe hacerlo fallar).
4. Empaquetar con el comando oficial y ejecutar una sesión de 5-10 minutos en el televisor leyendo
   la cabecera y las líneas `# tramo` / `# elegido`.
5. Cerrar: si la configuración elegida mejora a la referencia, mantener `true` y registrar la
   decisión nueva; si no, volver a `false` y registrar que el resultado de la tablet también vale en
   el TV.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "CALIBRAR_PRESENTACION" .` y
`grep -rn "present_mode|latency|escala_render|aplicar_present_config" src/`.

- `src/platform.rs` — la constante, su comentario y el arranque del barrido (1557).
- `src/overlay_tests.rs` — `test_decisiones_de_presentacion_declaradas` (405-421), única aserción que
  cambia; `test_render_escala_bufer_y_wiring_de_calibracion` (384-401) exige que los puntos de
  enganche existan y **no se toca**.
- `src/present.rs` y `src/present_tests.rs` — el mecanismo y su suite: **no se tocan**.
- [`analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y
  [`medir-rendimiento.md`](../guias/medir-rendimiento.md) — ambos nombran la constante y describen el
  barrido; si el experimento cambia su estado, hay que sincronizarlos.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) — no nombra la constante (comprobado);
  el APK cambia de contenido, así que el empaquetado se vuelve a correr igual.
- Sin más referencias encontradas.

### 6. Suposiciones

- **El resultado de la tablet no se traslada al televisor**, de modo que repetir el barrido allí no
  es repetir un experimento resuelto. → prototipar: no (exige el dispositivo, y es justo lo que el
  experimento mide) → si es falso: se volverá a `false` y quedará registrado que la decisión de 2026
  también vale en el TV.
  → no detecta: diferencias entre el televisor concreto y otros dispositivos.
- **El guardián actualizado sigue siendo un guardián** y no una etiqueta que se reescribe al gusto.
  → prototipar: sí, por mutación (paso 3): con la constante a `false` el test debe fallar
  → si es falso: el guardián no protege nada y hay que rediseñarlo antes de continuar.
  → no detecta: que alguien cambie constante y test a la vez sin motivo; eso solo lo ve la revisión.
- **El coste del barrido (12-15 s de arranque y artefactos a media resolución) es aceptable para una
  sesión de diagnóstico.** → prototipar: no → si es falso: se acorta `SEGUNDOS_POR_TRAMO` mientras el
  experimento esté abierto.
  → no detecta: el efecto sobre la primera impresión de un usuario real.

### 7. Alcance

- **Incluye:**
  - La constante `CALIBRAR_PRESENTACION`, su comentario y el guardián que la protege (port).
  - La corrida de la suite y el empaquetado con el comando oficial.
  - La sesión de diagnóstico en el televisor y el cierre del experimento.
- **No incluye:**
  - Convertir la sonda a media resolución en candidata adoptable: la nitidez nativa sigue decidida.
  - Tocar `src/present.rs`, `src/present_tests.rs` o el umbral `MARGEN_MEJORA` del módulo.
  - Solapar la espera de presentación con la emulación en otro hilo.
  - Reescribir las propuestas de 2026-10-10: son registro histórico.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El guardián sigue protegiendo la decisión**
Caso de uso: cualquier cambio futuro del barrido | Tipo: regresión
  Dado el árbol con la propuesta aplicada
  Cuando se ejecuta `cargo test test_decisiones_de_presentacion_declaradas` y, además, se pone la
  constante a `false` a mano
  Entonces el test pasa con la propuesta aplicada y **falla** con la constante cambiada en sentido
  contrario, en ambos casos nombrando el motivo en el mensaje de la aserción
Test: `src/overlay_tests.rs::test_decisiones_de_presentacion_declaradas`
Estado previo: **pasa** con la aserción vieja (`= false;`); es el guardián que se actualiza

**Escenario: La suite del port sigue en verde**
Caso de uso: integración del cambio | Tipo: regresión
  Dado el port con la propuesta aplicada
  Cuando se ejecuta `cargo test`
  Entonces salen **128 tests en verde** (109 + 4 + 15) y ningún archivo de `src/present.rs` ni
  `src/present_tests.rs` cambia
Test: `cargo test`
Estado previo: **pasa** (línea base medida hoy: 128 verdes, código de salida 0)

**Escenario: El barrido deja constancia en la telemetría**
Caso de uso: sesión de 5-10 minutos en el Android TV | Tipo: normal
  Dado el APK con el barrido activo y una sesión en el televisor
  Cuando se lee `blazing-perf.txt`
  Entonces aparece al menos un `# tramo` por configuración, un `# resultado` por tramo y **exactamente
  un** `# elegido`, y la cabecera dice el modo real del swapchain
Test: revisión humana + [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)
Estado previo: **no existe** (hoy no hay ninguna línea `# tramo`)

**Escenario (visible): La imagen y la nitidez no cambian**
Caso de uso: jugar en el televisor con el barrido activo | Tipo: regresión
  Dado el APK con el barrido activo
  Cuando se juega la misma escena que en la sesión de referencia
  Entonces el juego se ve con la nitidez nativa de siempre (ninguna sonda se adopta) y el fotograma
  no empeora: la mediana de FPS no baja
Test: revisión humana
Estado previo: **no comprobado** (nunca se ha corrido el barrido en este televisor)

### 9. Cómo lo comprobaré

- **El guardián sigue protegiendo la decisión** → comando:
  `cargo test test_decisiones_de_presentacion_declaradas`, y después, con la constante puesta a
  `false` a mano, el mismo comando
  → pasa si: verde en el primer caso y rojo en el segundo, con el motivo en el mensaje
  → no detecta: que constante y test se cambien a la vez con un motivo inventado.
- **La suite del port sigue en verde** → comando: `cargo test`
  → pasa si: 128 en verde y código de salida 0
  → no detecta: nada del comportamiento en el dispositivo.
- **El barrido deja constancia** → comando: empaquetar con
  `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`, instalar y jugar 5-10 minutos
  → pasa si: la telemetría lleva los `# tramo`, los `# resultado` y un solo `# elegido`
  → no detecta: que la configuración elegida sea buena para otras escenas u otros televisores.
- **La imagen no cambia** → revisión humana: comparar la nitidez y los FPS de la misma escena con la
  sesión de referencia
  → no detecta: micro-tirones que una mediana de FPS no revela.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — poner la constante a `false` sin tocar el guardián → debe morir *El guardián sigue protegiendo
la decisión*.
M2 — eliminar la aserción en vez de actualizarla → debe morir el mismo escenario (y se considera
fallo de implementación, no una solución).
M3 — hacer adoptable la sonda a media resolución → debe morir *La imagen y la nitidez no cambian*.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan, la suite queda en 128 verdes, el guardián actualizado mata el cambio
contrario, la telemetría del televisor lleva el barrido con su veredicto, y el experimento queda
cerrado en un sentido o en otro con su motivo escrito.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- El guardián no puede reescribirse de forma que siga matando el cambio contrario: se para y se
  cierra como `discrepada`, porque sin guardián el cambio no es mantenible.
- El barrido arrastra el arranque más allá de lo tolerable incluso con `SEGUNDOS_POR_TRAMO` reducido:
  se para, se documenta y se recurre a la cabecera del log como único diagnóstico.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi recomendación inicial fue «cambiar la constante a `true`, es
  una línea». Al abrir el archivo apareció el guardián `test_decisiones_de_presentacion_declaradas`
  y la decisión registrada en el comentario de la constante, así que la propuesta se reescribió: ya
  no es un cambio de una línea, es **revisar una decisión** con su guardián. Es también la razón por
  la que esta propuesta es Nivel 2 y no Nivel 1.
- **Corregido también:** atribuí `present` a espera de VSync sin más. El propio comentario de la
  constante lo desmiente en la tablet: bajar a media resolución llevó `present` de 51,4 ms a 2,9 ms,
  así que **escala con los píxeles** y no es solo sincronización. El apartado 2 recoge la corrección y
  el escenario de nitidez la protege.
- **Añadido tras descubrirlo:** la cabecera de telemetría ya registra el modo real del swapchain
  (`present_mode()` devuelve `config.present_mode`), dato que se usará al leer el resultado aunque no
  sustituya al barrido.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno: el plan se ejecutó tal cual (constante, comentario y guardián en el mismo cambio).

---

## Resultado (lo completa el agente después de editar)

`Estado: verificada por agente` en los escenarios de código; los del dispositivo no se intentaron.

El cambio está aplicado y verificado en todo lo que se puede verificar sin el televisor: el guardián
quedó actualizado y **sigue mordiendo**, y la suite sigue en 128 verdes. Los dos escenarios que exigen
el dispositivo quedan pendientes de la sesión.

### Predicho vs. real

- **El guardián sigue protegiendo la decisión**: coincide.
  Resultado real: con la constante activa el guardián pasa (`1 passed`); con la constante puesta a
  `false` a mano y el test sin tocar, **falla** (`panicked at src/overlay_tests.rs:418`, código de
  salida 101); y tras restaurar el archivo desde la copia en RAM, vuelve a pasar. El `sha256` del
  archivo restaurado coincide con el del respaldo.
  Método: el anunciado, con la mutación hecha y deshecha dentro de un bloque `finally`.
- **La suite del port sigue en verde**: coincide.
  Resultado real: 109 + 4 + 15 = **128 verdes**, código de salida 0. `src/present.rs` y
  `src/present_tests.rs` no se tocaron.
  Método: `cargo test`.
- **El barrido deja constancia en la telemetría**: **no se intentó** (exige el televisor).
- **La imagen y la nitidez no cambian**: **no se intentó** (exige el televisor).

### Contexto: ¿se respetó?

- El barrido apagado por decisión → respetada: no se eliminó la decisión, se **revisó** con su motivo
  escrito, y el guardián cambió en el mismo sentido y en el mismo cambio.
- La nitidez nativa decidida → respetada: la sonda a media resolución sigue siendo `Rol::Sonda`; no se
  tocaron `MARGEN_MEJORA`, `ESCALA_REDUCIDA` ni `src/present.rs`.
- Los tests no se debilitan en silencio → respetada: la única aserción que cambió es el guardián, y pasó
  a ser más informativa, no más débil; se comprobó por mutación que sigue matando el cambio contrario.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- [`src/platform.rs`](../../src/platform.rs) — la constante y su comentario.
- [`src/overlay_tests.rs`](../../src/overlay_tests.rs) — el guardián.
- [`analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y
  [`medir-rendimiento.md`](../guias/medir-rendimiento.md) — afirmaban que la calibración estaba
  desactivada; actualizados.
- Esta propuesta.

¿Coincide con lo anunciado? sí, con un añadido: el apartado 5 no había previsto que esos dos documentos
enunciaran el estado de la constante en presente, y se sincronizaron (lo exige la fase 5 del pipeline).

### Discrepancias

- Mi recomendación inicial («es un cambio de una línea») fue **información faltante**: no había leído el
  guardián ni el comentario de la decisión. Corregido antes de editar, y es la razón de que la propuesta
  sea Nivel 2.

### Para la próxima vez

Antes de proponer cambiar el valor de una constante en este repositorio, buscar su nombre en todo el
árbol (`grep -rn`): aquí las decisiones se protegen con tests que leen el fuente.

### Fuera de alcance (hallazgos que no se arreglaron)

- `present` **escala con los píxeles** en la medición de tablet (51,4 ms → 2,9 ms a media resolución): la
  resolución es la única palanca medida y es una decisión de nitidez del usuario, no técnica. | impacto: medio
- La cabecera de telemetría ya registra el modo real del swapchain: el barrido no es imprescindible para
  saber el modo, solo para saber si la configuración ayudaría. | impacto: bajo
- La espera de `present` podría solaparse con la emulación en otro hilo; no se aborda aquí. | impacto: medio

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Escenario «(visible)»: la imagen y la nitidez no cambian en el televisor: ✔ | ✘ | ?
- Escenario *El barrido deja constancia en la telemetría*: exige una sesión en el dispositivo: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
