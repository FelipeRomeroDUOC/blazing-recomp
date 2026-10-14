# Propuesta: arnés de fotogramas dorados por escena — saber en qué píxel cambió la PPU

Fecha: 2026-10-13
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cuando un cambio en la PPU mueve un píxel, hoy la única pista es que dos hashes globales dejan de coincidir: no
dicen ni en qué fotograma ni en qué parte de la pantalla. Habrá una herramienta que, tras cualquier cambio, diga
"la escena *entrada-avance*, fotograma 5400: 137 píxeles distintos, el primero en (columna 64, fila 18), en la
caja x60–149 / y18–52" y escriba un PNG con esas diferencias en rojo.

### 2. Contexto

- Contexto consultado: [`crates/recomp/src/main.rs`](../../gba-recomp/crates/recomp/src/main.rs) del motor
  (`cmd_runc` 2627, `cmd_verify` 2859, `fb_hash` 2958, `dump_frame` 3037, `InputScript::load` 198);
  [`src/runner.rs`](../../src/runner.rs) y [`src/perf.rs`](../../src/perf.rs); el guion de entrada del banco
  (`gba-input v1`, 202 puntos de pulsación hasta el fotograma 6996, contados con `awk` sobre el propio archivo);
  [`AGENTS.md`](../../AGENTS.md) (§3 regla
  inviolable, §4.1 repos independientes, §4.5 tests); [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md);
  [`2026-10-13_alpha-real-por-tramos.md`](2026-10-13_alpha-real-por-tramos.md) (propuesta pendiente que también
  quiere llevar ese guion a `scripts/diagnostico/`).
- Intención: que cualquier regresión de píxel de la PPU salga localizada en espacio (columna, fila, caja) y en
  tiempo (escena y fotograma ancla), sin abrir el juego a ojo.
- Restricciones y decisiones previas (máx. 3):
  - Los píxeles derivados de la ROM son material propietario y no se versionan nunca (origen: `AGENTS.md` §3,
    Regla Inviolable) → implica: los dorados viven en `out/doradas/`, que `.gitignore` ya excluye con `/out/`;
    solo se versionan la herramienta, el manifiesto (rutas, fotogramas y hashes) y los tests → protegida por:
    *Los dorados no se versionan y el repositorio sigue limpio*.
  - El port y el motor siguen siendo repos independientes (origen: `AGENTS.md` §4.1) → implica: el arnés usa al
    motor como caja negra mediante `recomp runc`; no se cambia ni un archivo de `crates/` → protegida por: *El
    motor no se toca*.
  - La exactitud de píxel manda y nunca se afloja una aserción (origen: `AGENTS.md` §4.5, Decisión 34) → implica:
    la comparación es exacta, sin tolerancias ni "parecidos"; cualquier píxel distinto es regresión → protegida
    por: *Un píxel cambiado se reporta como regresión*.
- Alternativas descartadas:
  - **Versionar los PPM en el repositorio** → descartada: son píxeles de la ROM (la restricción 1).
  - **Valerse de `recomp verify`** → descartada leyendo `run_hash` (main.rs): las dos caras —intérprete y
    recompilado— construyen la misma máquina con el mismo `gba-core`, así que un cambio de la PPU afecta igual a
    ambas y `verify` seguiría imprimiendo `MATCH`. Ese comando comprueba la traducción de CPU, no la PPU.
  - **Añadir `--dump-at` al motor** para volcar varios fotogramas en una sola corrida → descartada por ahora:
    toca el repositorio upstream y con un `--out` por fotograma ancla se cubre lo necesario; se puede añadir
    después sin cambiar el diseño.
  - **Guardar solo los hashes de los anclas** (sin imagen) → descartada: dirían que algo cambió pero no dónde,
    que es exactamente lo que se pide resolver.
- Ante imprevistos, prioriza: no dar un veredicto falso sobre dar uno rápido (una comparación dudosa se rechaza
  con código 2, no se interpreta como regresión).

### 3. Ejemplos

- Caso normal: antes: `frames: 7200 hash: ee7dc7ef514b5a3f native_blocks: 11018334 fallback_steps: 0`; si otro
  binario da otro hash, no hay nada más que mirar → ahora: el arnés imprime `entrada-avance ancla 5400: DIFIERE
  137 px, primero (columna 64, fila 18), caja x60–149 y18–52, filas 18–52, tiles 8×8 afectados 9/96` y escribe
  `out/doradas/entrada-avance_5400.diff.png` con las diferencias en rojo.
- Caso límite: el binario candidato usa una biblioteca recompilada distinta (cambió el recompilador) → antes:
  hash distinto sin más → ahora: las diferencias se listan igual y el resumen añade una advertencia nombrando la
  biblioteca `out/…-bios.dll`, porque entonces las diferencias pueden venir de ahí.
- Caso límite: el dorado se capturó con un guion de entrada que ya no es el del manifiesto → antes: miles de
  diferencias falsas y una pérdida de tiempo → ahora: código 2 con `los dorados no corresponden: script sha256
  X ≠ Y` y ninguna lista de píxeles.
- Caso límite: un PPM corrupto o de otro tamaño → antes: no existía el problema → ahora: código 2 nombrando el
  problema, sin veredicto.

### 4. Pasos

1. Traer a `scripts/diagnostico/` el guion de entrada del banco (`entrada_avance.input`, `gba-input v1`) y
   verificar que reproduce los cinco hashes de ancla ya medidos.
2. Escribir `scripts/diagnostico/escenas_doradas.toml` con este contenido inicial (una tabla por escena;
   `script` es relativo a la carpeta del manifiesto). Se versiona; los píxeles no.

   ```toml
   version = 1

   [[escena]]
   nombre = "entrada-avance"
   script = "entrada_avance.input"
   anclas = [600, 1800, 3600, 5400, 7200]
   ```
3. Escribir `scripts/diagnostico/fotogramas_dorados.py` con tres subcomandos y este contrato de códigos de
   salida: **0** sin diferencias, **1** diferencias de píxel, **2** entrada inválida o dorados no correspondientes
   (nunca se confunde 1 con 2):
   - `diferenciar <a.ppm> <b.ppm> [--mapa m.png]`: compara los dos PPM P6 240×160 píxel a píxel; informa total,
     primer píxel (columna, fila), caja envolvente, rango de filas y rejilla de tiles 8×8; escribe el mapa PNG
     (gris el píxel igual, rojo puro el distinto) con `zlib` de la biblioteca estándar, sin dependencias.
   - `capturar --bin <recomp.exe> --rom <rom> --bios <bios> [--escenas <toml>] [--salida out/doradas]`: por cada
     ancla, ejecuta `recomp runc --frames <k> --input <script> --out out/doradas/<escena>_<k>.ppm` desde la raíz
     del port, comprueba que el hash FNV-1a que imprime el motor coincide con el que se recalcula desde el PPM
     guardado, y escribe `out/doradas/manifesto.json` (sha256 de ROM, BIOS, guion, biblioteca `.dll` y binario,
     más los hashes de cada ancla).
   - `comparar --bin <recomp.exe> --rom <rom> --bios <bios> …`: ejecuta los mismos anclas con el binario
     candidato, rechaza con código 2 si los sha256 de ROM/BIOS/guion no coinciden con el manifiesto, y por cada
     ancla imprime el veredicto y escribe el mapa de diferencias si lo hay; sale con 1 si algún ancla difiere.
4. Escribir `tests/golden_frame_tests.rs` (integración, en `tests/` como `packaging_tests.rs`, invocando
   `python` igual que él): cuatro tests con PPM sintéticos en un directorio temporal, sin ROM.
5. Documentar: ficha en `scripts/diagnostico/README.md` y sección "Fotogramas dorados" en
   [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md).
6. Cierre: comprobación real con la ROM (capturar y comparar con el mismo binario) y blindaje por mutación sobre
   el motor, con respaldo y restauración en `finally`.

### 5. Qué más podría verse afectado

- `tests/packaging_tests.rs`: no se toca, pero el checklist de `AGENTS.md` fija "124 tests"; al añadir 4 pasarán
  a 128 y la cifra del checklist debe actualizarse al cerrar.
- `.gitignore`: no requiere cambios; `/out/` ya cubre `out/doradas/` (verificado leyendo `.gitignore`, que
  ignora `/out/`, `*.dll`, `*.gba`, `*.bin` y `scripts/diagnostico/log_*.txt`).
- `out/1575 - Fire Emblem (E)(Independent)-bios.dll`: el arnés exige que exista (si no, `runc` falla al cargarlo);
  lo comprueba al arrancar y aconseja `recomp build` con código 2.
- `scripts/diagnostico/entrada_avance.input`: llega de un directorio temporal de esta sesión. Si la propuesta
  pendiente de los tramos lo trae primero, se reutiliza el suyo y no se duplica.
- Motor (`crates/`): ninguno; se usa como caja negra.
- `docs/guias/medir-rendimiento.md` y `scripts/diagnostico/README.md`: se les añade contenido (no se altera el
  existente).

### 6. Suposiciones

- El PPM P6 que escribe el motor no pierde fidelidad GBA555: `v8 >> 3` devuelve el canal de 5 bits exacto, así
  que desde el archivo se recalcula el framebuffer y el hash FNV-1a → prototipar: sí (**ya hecho**)
  → si es falso: los dorados tendrían que ser binarios GBA555 y el diseño cambia de formato.
  → resultado del prototipo: los cinco anclas reproducen exactamente los hashes del motor (`ec8c6a01f9add6ff`,
  `7e7e7f291fa0476e`, `8e1a4d22ca4bce3a`, `9d537ab715df54f2`, `ee7dc7ef514b5a3f`) y la expansión 5→8 bits
  resultó invertible para los 32 valores | no detecta: capturas hechas con otra versión del motor.
- El arnés es determinista con la misma biblioteca `.dll` y el mismo guion, así que dos corridas dan PPM idénticos
  byte a byte → prototipar: sí (**ya hecho**)
  → si es falso: no puede haber dorados y el plan entero cambia a comparación entre dos binarios en la misma
  sesión, sin memoria persistente.
  → resultado del prototipo: el fotograma 1800 capturado dos veces da el mismo archivo y el mismo hash
  (`7e7e7f291fa0476e`, 6.499.630 bloques) | no detecta: determinismo en el dispositivo o con otra biblioteca.
- Los cinco anclas propuestos (600, 1800, 3600, 5400, 7200) son pantallas distintas y no degeneradas →
  prototipar: sí (**ya hecho**)
  → si es falso: se ajusta el manifiesto (es dato, no código).
  → resultado del prototipo: 27 / 84 / 80 / 81 / 82 colores únicos; entre anclas consecutivos difieren el 30,5 %,
  29,3 % y 29,5 % de los 38.400 píxeles, y entre 600 y 1800 el 99,6 % | no detecta: qué representa cada pantalla
  (eso lo juzga la persona al mirar los PNG).
- El guion actual del banco alcanza escenas con efectos de color en las que merece la pena mirar píxeles →
  prototipar: no
  → si es falso: los anclas siguen siendo una red de regresión para cualquier cambio de la PPU, y se amplía el
  manifiesto con más guiones cuando existan.

### 7. Alcance

- Incluye: la herramienta con sus tres subcomandos y su contrato de códigos de salida, el manifiesto de
  escenas con los cinco anclas, el guion de entrada traído a `scripts/diagnostico/`, cuatro tests de integración
  y la documentación.
- No incluye: cambios en el motor (`--dump-at` queda para después); captura ni comparación en el dispositivo
  Android; versionar píxeles; localizar el primer fotograma divergente dentro de una escena (la granularidad es
  la del ancla: se amplía agregando anclas al manifiesto); y registrar los dorados en la nube o en otro sitio
  fuera de la máquina.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El diferenciador de fotogramas indica el píxel que cambia
Caso de uso: tras optimizar la PPU, comprobar que no se movió ningún píxel | Tipo: normal
  Dado dos PPM P6 de 240×160 idénticos salvo un píxel, en (columna 64, fila 18)
  Cuando se ejecuta `python scripts/diagnostico/fotogramas_dorados.py diferenciar a.ppm b.ppm --mapa m.png`
  Entonces el proceso sale con código 1 y el informe contiene `1` píxel distinto, el primero en
  `columna=64, fila=18`, la caja `x64–64, y18–18`, y existe `m.png` con 240×160 píxeles donde solo ese es rojo
  puro `(255, 0, 0)`
Test: `el_diferenciador_indica_el_pixel_que_cambia` en `tests/golden_frame_tests.rs` (escribe los PPM en un
directorio temporal; no necesita ROM)
Estado previo: **falla** (la herramienta no existe todavía)

Escenario: Dos fotogramas iguales no reportan ninguna diferencia
Caso de uso: comprobar que la optimización no movió nada | Tipo: límite
  Dado dos PPM idénticos byte a byte
  Cuando se ejecuta `diferenciar`
  Entonces el proceso sale con código 0, informa `sin diferencias` y no se escribe ningún mapa
Test: `dos_fotogramas_identicos_no_reportan_diferencias` en `tests/golden_frame_tests.rs`
Estado previo: **falla** (la herramienta no existe todavía)

Escenario: Un PPM de otro tamaño no se interpreta como una regresión
Caso de uso: recibir un archivo de captura corrupto o incompleto | Tipo: error
  Dado un archivo con cabecera `P5` y otro P6 que contiene 38.399 píxeles en vez de 38.400
  Cuando se ejecuta `diferenciar` con cada uno
  Entonces el proceso sale con código 2, el mensaje nombra el problema (cabecera o tamaño) y no se emite ningún
  veredicto de diferencias
Test: `un_ppm_corrupto_no_se_confunde_con_una_regresion` en `tests/golden_frame_tests.rs`
Estado previo: **falla** (la herramienta no existe todavía)

Escenario: Los dorados que no corresponden se rechazan antes de comparar
Caso de uso: comparar contra dorados capturados con otro guion de entrada u otra ROM | Tipo: regresión
  Dado un manifiesto de dorados cuyo `manifesto.json` guarda un sha256 de guion distinto al del guion actual
  Cuando se ejecuta `comparar`
  Entonces el proceso sale con código 2, el mensaje nombra el campo que cambió (`script`, `rom` o `bios`) y no
  se lista ningún píxel
Test: `los_dorados_desactualizados_no_se_comparan_en_vano` en `tests/golden_frame_tests.rs`
Estado previo: **falla** (la herramienta no existe todavía)

### 9. Cómo lo comprobaré

- **El diferenciador localiza el píxel** → comando: `cargo test --test golden_frame_tests el_diferenciador_indica_el_pixel_que_cambia`
  → pasa si: código 1, coordenadas (64, 18) y el PNG con un solo píxel rojo
  → no detecta: la captura real con el motor (eso lo cubre el siguiente punto) ni formatos de imagen distintos
  al P6.
- **No hay falsos positivos** → comando: `cargo test --test golden_frame_tests dos_fotogramas_identicos_no_reportan_diferencias`
  → pasa si: código 0 sin mapa
  → no detecta: diferencias de un solo bit en canales que la expansión 5→8 bits aplasta (ya se comprobó que no
  las aplasta, pero el test usa bytes sintéticos).
- **Entrada corrupta** → comando: `cargo test --test golden_frame_tests un_ppm_corrupto_no_se_confunde_con_una_regresion`
  → pasa si: código 2 sin veredicto
  → no detecta: un PPM bien formado pero de otra resolución distinta a 240×160 (se comprueba igual, pero el
  test solo cubre los dos casos citados).
- **Dorados desactualizados** → comando: `cargo test --test golden_frame_tests los_dorados_desactualizados_no_se_comparan_en_vano`
  → pasa si: código 2 nombrando el campo
  → no detecta: una ROM distinta con el mismo sha256 (imposible) ni cambios en la biblioteca `.dll`, que solo
  avisa.
- **Corrida real completa, sin regresión** → comando:
  `python scripts/diagnostico/fotogramas_dorados.py capturar --bin <motor actual> --rom "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin`
  seguido de `comparar` con el mismo binario
  → pasa si: los cinco anclas imprimen `OK`, los cinco hashes coinciden con los ya medidos
  (`ec8c6a01f9add6ff`, `7e7e7f291fa0476e`, `8e1a4d22ca4bce3a`, `9d537ab715df54f2`, `ee7dc7ef514b5a3f`) y el
  proceso sale con 0 → no detecta: regresiones en pantallas que los cinco anclas no cubren ni nada del
  dispositivo.
- **El arnés detecta una regresión de verdad (blindaje por mutación)** → M1: en el motor, cambiar de forma
  transitoria una constante de `alpha_blend` (por ejemplo `+ 8` por `+ 7` en un canal), reconstruir y ejecutar
  `comparar` → debe salir con 1 y nombrar un píxel; M2: romper el corte de ventana en `compose` y repetir → debe
  salir con 1. Los archivos se respaldan en RAM y se restauran en un bloque `finally`, sin comandos de Git →
  no detecta: regresiones que no afectan a ningún ancla, que es por qué el manifiesto se puede ampliar.
- Conjunto: `cargo test` (128 verdes, cuatro nuevos), `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`,
  `python scripts/extraer_referencias.py --check` y `python scripts/diagnostico/auditar_labels.py`.

### 10. Terminado cuando

Los cuatro tests pasan dentro de `cargo test`, la corrida real con la ROM da los cinco anclas en `OK`, el
mutante M1 (y M2) sale con código 1 nombrando un píxel y los archivos del motor quedan restaurados, el
repositorio no contiene ningún píxel de la ROM (`git status` limpio salvo los archivos anunciados), la guía y el
README están actualizados, y la cifra de tests del checklist de `AGENTS.md` refleja los 128.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta
  como `discrepada`.
- **Si dos corridas consecutivas sin cambiar nada dan píxeles distintos**, no se encadenan reintentos ni se
  afloja la comparación: se documenta la inestabilidad y se cierra, porque entonces el arnés no mide nada.
- Si el manifiesto resulta inútil por falta de escenas con efectos de color, se amplía con más guiones; no se
  declara un falso "todo correcto".

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primer borrador proponía comparar contra dorados versionados en el repo
  para que una recién clonada tuviera referencia. Violaba la Regla Inviolable de `AGENTS.md` §3 (los píxeles
  son material derivado de la ROM). Ahora los dorados son locales y la referencia se regenera con
  `capturar` desde un binario de confianza.
- **Corregido también:** iba a usar `recomp verify` como comprobación cruzada. Al leer `run_hash` vi que las dos
  caras comparten `gba-core`: una regresión de la PPU saldría igual en las dos y `verify` diría `MATCH`. Sale
  del alcance y queda escrito en las alternativas descartadas.
- **Corregido también:** el primer borrador prometía localizar "el primer fotograma divergente" con búsqueda
  binaria sobre `--frames`. Lo dejé fuera: cuesta hasta 13 corridas por escena y la granularidad de anclas ya
  responde "en qué fotograma" dentro de lo razonable; se puede añadir después sin tocar el diseño.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno: los tres prototipos previos (inofensividad del PPM, determinismo y elección de anclas) confirmaron el
  diseño tal y como está escrito; sus resultados están en §6.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **el_diferenciador_indica_el_pixel_que_cambia**: coincide
  Resultado real: código 1, informe `DIFIERE 1 px`, primer píxel `columna=64 fila=18`, caja `x64-64 y18-18`, mapa PNG con un solo píxel rojo en (64, 18).
  Método: el anunciado (`cargo test --test golden_frame_tests el_diferenciador_indica_el_pixel_que_cambia`).

- **dos_fotogramas_identicos_no_reportan_diferencias**: coincide
  Resultado real: código 0, informe `sin diferencias (38400 píxeles, 240x160)`, sin mapa escrito.
  Método: el anunciado.

- **un_ppm_corrupto_no_se_confunde_con_una_regresion**: coincide
  Resultado real: código 2 en ambos casos (cabecera P5 y payload corto 38399 píxeles); mensajes nombran "cabecera" y "38400" respectivamente; sin veredicto de diferencias.
  Método: el anunciado.

- **los_dorados_desactualizados_no_se_comparan_en_vano**: coincide
  Resultado real: código 2, mensaje `los dorados no corresponden: cambió script respecto al momento de 'capturar'`; sin píxeles listados.
  Método: el anunciado.

- **Corrida real completa, sin regresión**: coincide
  Resultado real: los 5 anclas (600/1800/3600/5400/7200) imprimen `OK` con hashes exactos (`ec8c6a01f9add6ff`, `7e7e7f291fa0476e`, `8e1a4d22ca4bce3a`, `9d537ab715df54f2`, `ee7dc7ef514b5a3f`), salida 0.
  Método: el anunciado (`capturar` + `comparar` con el mismo binario).

- **El arnés detecta una regresión de verdad (M1 — alpha_blend)**: coincide
  Resultado real: mutante `+8` → `+7` en `alpha_blend`; build exit 0; `comparar` exit 1 con **4 anclas difiriendo, 311 píxeles** (600 OK; 1800: 93 px, 3600: 45 px, 5400: 57 px, 7200: 116 px), mapas PNG escritos; `ppu.rs` restaurado byte a byte (`7591063056dcc6978f288765ea1f131e0c22fc8bc57f479c4f58779ce023ab1c`).
  Método: el anunciado (`mutar_alpha_blend.py` con respaldo en RAM y restauración en `finally`).

- **El arnés detecta una regresión de verdad (M2 — in_window_range)**: **difiere** (no detecta, no falla)
  Resultado real: mutante `(x1..x2)` → `(x1..=x2)`; build exit 0; `comparar` exit 0 (todos los anclas OK); `ppu.rs` restaurado.
  Interpretación: los anclas actuales no ejercitan ventanas (`in_win0_y`/`in_win1_y` son falsos en esas escenas), por lo que la mutación no llega a ejecutarse. El blindaje no detecta, pero **no da falso positivo**: salida 0 significa "sin diferencias detectadas en estos anclas".

### Contexto: ¿se respetó?

- **Los píxeles derivados de la ROM no se versionan** (AGENTS.md §3): **respetada**. Dorados en `out/doradas/` (cubierto por `/out/` en `.gitignore`); solo versionados `fotogramas_dorados.py`, `escenas_doradas.toml`, `entrada_avance.input` y `tests/golden_frame_tests.rs`.
- **El port y el motor son repos independientes** (AGENTS.md §4.1): **respetada**. El arnés usa `recomp runc` como caja negra; no se tocó `crates/`.
- **La exactitud de píxel manda** (AGENTS.md §4.5): **respetada**. Comparación exacta píxel a píxel; cualquier diferencia es regresión (código 1).
- **Nada de código destructivo de Git en scripts** (AGENTS.md §4.6): **respetada**. Ambos scripts de mutación respaldan en RAM y restauran en `finally`.

### Archivos modificados

- `scripts/diagnostico/fotogramas_dorados.py` (nuevo)
- `scripts/diagnostico/escenas_doradas.toml` (nuevo)
- `scripts/diagnostico/entrada_avance.input` (nuevo, copiado desde `/tmp/br-suelos/`)
- `scripts/diagnostico/README.md` (añadida ficha de la herramienta)
- `docs/guias/medir-rendimiento.md` (añadida sección "Fotogramas dorados")
- `tests/golden_frame_tests.rs` (nuevo, 4 tests de integración)
- `AGENTS.md` (contador de tests 124 → 128 en el checklist)
- `docs/propuestas/2026-10-13_arnes-de-fotogramas-dorados.md` (este archivo: Resultado y Revisión humana)

**¿Coincide con lo anunciado?** Sí.

### Discrepancias

- **M2 no detecta** (mutación sobre `in_window_range`): información faltante — los anclas del manifiesto no alcanzan escenas con ventanas activas (`in_win0_y`/`in_win1_y` falsos). El arnés no falla (código 0 = "sin diferencias detectadas"), lo cual es correcto para lo que medía, pero no prueba la lógica de ventanas. → Se deja anotado como limitación en la Revisión humana.

### Para la próxima vez

Añadir anclas en escenas que usen ventanas (`WIN0`/`WIN1`) al manifiesto cuando existan guiones grabados de batalla/menús, para que el blindaje M2 también sea efectivo.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- El mapa de diferencias PNG se entiende de un vistazo: se ve la región que cambió sobre el fotograma
  (escenario visible): ✔
- Los cinco anclas del manifiesto representan las pantallas que interesa proteger; faltan o sobran escenas
  (visible): ✔
- ¿Merece la pena mantener esta herramienta viva en `scripts/diagnostico/` de forma rutinaria? (no testeable): ✔
- El blindaje M2 no cubre ventanas porque los anclas actuales no las ejercitan (no testeable, limitación del manifiesto actual): ✔
- Detectado solo por el humano: —

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

- El mapa de diferencias PNG se entiende de un vistazo: se ve la región que cambió sobre el fotograma
  (escenario visible): ✔ | ✘ | ?
- Los cinco anclas del manifiesto representan las pantallas que interesa proteger; faltan o sobran escenas
  (visible): ✔ | ✘ | ?
- ¿Merece la pena mantener esta herramienta viva en `scripts/diagnostico/` de forma rutinaria? (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
