# Mezcla única del fuego sobre el dragón y el cielo en `ppu_nativa.wgsl`

Fecha: 2026-11-06
Nivel: 2
Estado: discrepada

## Para decidir

- **Qué cambia:** En el shader de composición de la GPU (`src/ppu_nativa.wgsl`) y su espejo de pruebas en CPU (`src/ppu_nativa.rs`), eliminamos la premezcla prematura del fuego (`BG3`) con el fondo (`backdrop`) antes de buscar la segunda capa y aseguramos que, cuando el fuego es la primera capa visible en la escena del dragón, el buscador de prioridades siempre localice el cuerpo del dragón (`BG2`) o el cielo (`backdrop`) debajo para aplicar una única mezcla translúcida.
- **Para qué:** Evitar dos fallos simultáneos que causan el parpadeo en Android:
  1. Cuando `BLDCNT` conmuta a modo texto (`0x2861` / `0x3F28`, sin `BG3` como primer objetivo) en una línea de escaneo, el bucle actual cortaba en `BG3` sin buscar `BG2` y tapaba de forma opaca el cuerpo del dragón con fuego mezclado en negro.
  2. Cuando `BLDCNT` sí tiene activo el bit de `BG3` (`0x2448` / `0x3F68`), el shader mezclaba `BG3` dos veces seguidas (primero con `backdrop` en la lectura de capa y después con `segunda` en la unidad de mezcla), alterando los colores reales del hardware (como el cuadro 677).
- **Ejemplo concreto (píxel del dragón cubierto por fuego, `c3 = 0x3420`, `c2 = 0x624F`, `backdrop = 0x0000`):**
  - *Antes:*
    - Con `BLDCNT = 0x2861` (modo texto en Android): el shader ignoraba `c2` (`0x624F`) y devolvía `alpha_blend(0x3420, 0x0000, 16, 8) = 0x3420` (el fuego tapaba y borraba el cuerpo del dragón en ese cuadro).
    - Con `BLDCNT = 0x3F68` y `EVA=16, EVB=6` (cuadro 677) y `backdrop = 0x1084`: el shader premezclaba `c3` con `backdrop` antes de volver a mezclarlo con `c2`, desviándose de la referencia de hardware (`0x5906`).
  - *Ahora:*
    - Con `BLDCNT = 0x3F68` (`EVA=16, EVB=6`): `c3` se mezcla una sola vez con `c2` usando los pesos reales del registro (`alpha_blend(0x3420, 0x624F, 16, 6) = 0x5906`, idéntico al hardware GBA).
    - Con `BLDCNT = 0x2861` (oscilación a modo texto en Android): el bucle sigue buscando la segunda capa, encuentra a `c2` (`BG2`, el dragón) y aplica `alpha_blend(0x3420, 0x624F, 16, 8) = 0x5D27`, manteniendo visible el cuerpo del dragón detrás del fuego sin parpadeo.
- **Pasos desde quien usa la app:**
  1. Abrir el juego en Android y dejar correr la cinemática de introducción hasta la escena del dragón.
  2. Observar que las llamaradas (`BG3`) se mantienen translúcidas sobre el cuerpo del dragón (`BG2`) y sobre el cielo (`backdrop`) sin fogonazos ni cortes opacos cuando aparecen o cambian los subtítulos.
- **Qué podría romperse:** La apariencia del fuego detrás de los subtítulos (`BG0`) si `BG3` actúa como segunda capa debajo del texto. Para evitarlo, cuando `BG3` es la segunda capa (`hay_primera == true`) y `es_fuego_dragon` está activo, se conserva la precomposición de `BG3` con `backdrop` (`16, 8`).
- **Qué no se toca:** No se modifican registros de hardware ni se fuerzan valores fijos en `BLDCNT`/`BLDALPHA` (respetando el ADR 47), no se altera `compone_pixel_jerarquica_dragon` para no romper los tests existentes, ni se tocan `src/recomp.rs` o `src/pipeline.rs`.
- **Qué necesito de ti:** Solo aprobar la propuesta (y revisar visualmente 1 punto en el dispositivo Android tras compilar el APK).

## Detalle para verificar

### Suposiciones

- **S1 (verificada en código, no se prueba antes de editar):** En `src/ppu_nativa.wgsl:671`, cuando `primera_id == 3u` (`BG3`) y `BLDCNT` pierde el bit 3 (`0x2861`), `necesita_segunda` devuelve `false` y el bucle hace `break` sin evaluar `BG2` (`bg_prio2 == 3u`), haciendo que `segunda` quede en `backdrop` y no se ejecute mezcla sobre `BG2`.
  - *Si fuera falsa:* El cuerpo del dragón no desaparecería al cambiar `BLDCNT` a `0x2861`; bastaría con quitar el doble `alpha_blend`.
- **S2 (verificada en código, no se prueba antes de editar):** Ningún test unitario ni de integración busca literalmente la cadena `c3 = alpha_blend(c3, backdrop, 16u, 8u)` dentro de `src/ppu_nativa.wgsl`, por lo que mover esa mezcla de respaldo a la unidad de mezcla final no rompe tests de inspección estática de WGSL.
  - *Si fuera falsa:* Actualizaríamos el test de inspección estática previa autorización.

### Reglas del proyecto que cambian este plan

- **R1 (Fuente: `docs/explicacion/DECISIONES.md` — ADR 47):** Prohibido alterar o sanear heurísticamente los valores de `BLDCNT` o `BLDALPHA` en memoria; el shader debe usar los coeficientes reales `eva` y `evb` del hardware cuando el registro activa el modo alfa, logrando paridad con `gba-core` en el cuadro 677 (`0x5906`). Protegida por `C1`.
- **R2 (Fuente: `AGENTS.md §1` y `src/ppu_nativa.wgsl:658-665`):** Mantener la resiliencia del fuego del dragón (`bg_prio2 == 3 && bg_prio3 <= 1 && (dispcnt & 0x0C00) == 0x0C00`) ante conmutaciones de `BLDCNT` a modo texto (`0x2861`/`0x3F28`), sin borrar `BG2` ni apagar el fuego. Protegida por `C2` y revisión visual (`solo tú` en Android).
- **R3 (Fuente: `AGENTS.md §3`):** No debilitar `assert!` existentes y mantener todos los tests de `src/ppu_nativa_tests.rs` en verde. Protegida por `C3`.

### Comprobaciones

- **C1 — Mezcla única con coeficientes reales de hardware en cuadro 677 (normal):**
  - *Situación real:* En la línea 70, píxel 15 del cuadro 677 de la intro, `BG3` (`0x3420`, prioridad 1) está sobre `BG2` (`0x624F`, prioridad 3) con `BLDCNT = 0x3F68`, `EVA = 16`, `EVB = 6` y `backdrop = 0x1084`.
  - *Dado que* la escena cumple `es_fuego_dragon` y `BLDCNT` tiene activos el bit 3 (1st target `BG3`) y el bit 10 (2nd target `BG2`) en modo 1,
  - *Cuando* se compone el píxel con la lógica del shader (`compone_pixel_fuego_dragon_wgsl`),
  - *Entonces* el color resultante es exactamente `0x5906` (`alpha_blend(0x3420, 0x624F, 16, 6)` en una sola pasada, sin contaminarse con `backdrop = 0x1084`).
  - *Nombre del test:* `test_wgsl_fuego_dragon_mezcla_unica_cuadro_677` en `src/ppu_nativa_tests.rs`.
  - *Comando:* `python scripts/run_tests_summary.py test_wgsl_fuego_dragon_mezcla_unica_cuadro_677`
  - *Qué no cubre:* La ejecución en GPU física móvil.

- **C2 — Preservación del cuerpo del dragón (`BG2`) cuando `BLDCNT` conmuta a modo texto `0x2861` (extremo):**
  - *Situación real:* En Android, una línea de la escena del dragón recibe `BLDCNT = 0x2861` (1st target = `BG0`, sin bit 3 para `BG3`) en un píxel donde no hay texto (`BG0 = TRANSPARENTE`), pero sí hay fuego (`BG3 = 0x3420`, prioridad 1) delante del cuerpo del dragón (`BG2 = 0x624F`, prioridad 3).
  - *Dado que* `es_fuego_dragon` es verdadero y `primera_id == 3` (`BG3`),
  - *Cuando* se evalúa el píxel con `BLDCNT = 0x2861`,
  - *Entonces* el bucle no corta en `p = 1`, localiza a `BG2` (`0x624F`) como segunda capa y devuelve `alpha_blend(0x3420, 0x624F, 16, 8) = 0x5D27` en lugar de tapar al dragón con fuego mezclado con `backdrop` (`0x3420`).
  - *Nombre del test:* `test_wgsl_fuego_dragon_preserva_bg2_con_bldcnt_texto` en `src/ppu_nativa_tests.rs`.
  - *Comando:* `python scripts/run_tests_summary.py test_wgsl_fuego_dragon_preserva_bg2_con_bldcnt_texto`
  - *Qué no cubre:* El ritmo de presentación VSync en pantalla Android.

- **C3 — Validación de compilación Naga WGSL y no regresión en escenas ordinarias y subtítulos (no debe cambiar):**
  - *Situación real:* Píxeles con subtítulos `BG0` sobre fuego `BG3` (`primera_id = 0`, `segunda_id = 3` con `BLDCNT = 0x2861`) y escenas fuera de la cinemática del dragón (`bg_prio2 != 3` o `bg_prio3 > 1`).
  - *Dado que* el shader `src/ppu_nativa.wgsl` es validado por `naga` y existen 21 tests de cinemática en `src/ppu_nativa_tests.rs`,
  - *Cuando* se ejecuta la suite de `ppu_nativa`,
  - *Entonces* `naga::front::wgsl::parse_str` valida el shader sin errores, `BG0` sobre `BG3` mezcla contra `alpha_blend(c3, backdrop, 16, 8)`, y las escenas no-dragón no activan la rama `es_fuego_dragon`.
  - *Nombre del test:* `test_wgsl_fuego_dragon_subtitulo_bg0_sobre_bg3_y_no_regresion` (más la suite `ppu_nativa`).
  - *Comando:* `python scripts/run_tests_summary.py ppu_nativa`
  - *Qué no cubre:* Dispositivos Android físicos sin correr el APK.

### Archivos que se modificarán

- `src/ppu_nativa.wgsl`: Mover `es_fuego_dragon` antes del bucle de prioridades en `fs_comp` (L554–L715), evitar el corte prematuro cuando `es_fuego_dragon && primera_id == 3u`, precomponer `c3` en el bucle únicamente cuando `BG3` es la segunda capa (`hay_primera == true`), y añadir la rama de respaldo `alpha_blend(primera, segunda, 16u, 8u)` al final de la unidad de mezcla cuando `BLDCNT` no mezcle `BG3`.
- `src/ppu_nativa.rs`: Añadir la función espejo `compone_pixel_fuego_dragon_wgsl` que replica exactamente la lógica de `fs_comp` para pruebas deterministas en CPU.
- `src/ppu_nativa_tests.rs`: Añadir los tests `test_wgsl_fuego_dragon_mezcla_unica_cuadro_677`, `test_wgsl_fuego_dragon_preserva_bg2_con_bldcnt_texto` y `test_wgsl_fuego_dragon_subtitulo_bg0_sobre_bg3_y_no_regresion`.

### Evidencia

- `src/ppu_nativa.wgsl:653-684`: Muestra la precomposición actual `c3 = alpha_blend(c3, backdrop, 16u, 8u)` seguida de `if (!necesita_segunda(...)) { corte = true; break; }`.
- `src/ppu_nativa.wgsl:695-715`: Muestra la unidad de mezcla que vuelve a ejecutar `color = alpha_blend(primera, segunda, eva, evb)` cuando `BLDCNT` incluye a `primera_id` y `segunda_id`.
- `src/ppu_nativa.rs:414-504`: Contiene `compone_pixel` y `necesita_segunda` en CPU.
- `docs/explicacion/DECISIONES.md:1200-1227`: ADR 47 sobre composición dinámica en GPU y prohibición de saneamiento forzado de registros.

## Revisión humana

- Comprobar en el dispositivo Android que la escena del dragón en la cinemática de introducción ya no parpadea ni oculta el cuerpo del dragón detrás del fuego: ✘

## Evaluación

### El hecho

- **Qué se suponía o esperaba (`S1` y revisión en dispositivo):** Que el parpadeo en Android durante la escena del dragón ocurría porque `es_fuego_dragon` (`bg_prio2 == 3 && bg_prio3 <= 1`) cortaba el bucle de prioridades al cambiar `BLDCNT` a `0x2861` o aplicaba doble mezcla con `0x3F68`.
- **Qué pasó de verdad:** Al probar el APK en el dispositivo Android y cruzar el nuevo `blazing-perf.txt` contra `telemetria/blazing-perf-48.txt`:
  1. En `rel=0..148` (`frames 757..905`), el hash de salida GPU (`sal`) fue **100% idéntico (`149/149` cuadros)** entre ambas compilaciones: en `rel=0..43` y `rel=81..148` `BLDCNT` ya tiene el bit 3 activo en las 160 líneas (`0x2168` / `0x3F68`, por lo que `en_t1 == true` y el shader anterior tampoco premezclaba con `backdrop`), y en `rel=44..80` (`BLDCNT = 0x2861`) `BG3` tiene prioridad `2` (`bg3=ON(prio=2,bg3)`), por lo que `es_fuego_dragon` (`bg_prio3 <= 1`) vale `false`.
  2. En la escena principal del dragón y la leyenda (`rel=150..2560`, `frames 907..3317`, 40 segundos), las 160 líneas tienen `bg_prio0=2, bg_prio1=3, bg_prio2=3, bg_prio3=3` (`correccion=INACTIVA`), por lo que `es_fuego_dragon` (`bg_prio3 <= 1`) es **`false` en todos los cuadros de la escena** y la rama editada solo llegó a ejecutarse en 1 fotograma de transición (`rel=149` / `frame=906`).
  3. En `rel=150..502` (donde `es_fuego_dragon` es `false` y ambos APKs ejecutan exactamente el mismo código WGSL), los hashes de entrada `hc`, `hv`, `ho`, `hp` y `hl` coinciden al 100% (`601/601`), pero **`sal` (la salida GPU) cambia en 76 de 353 cuadros** entre ambas corridas (incluyendo 31 cuadros donde `hc`, `hv`, `ho`, `hp` y `hl` no cambiaron respecto a `rel-1`, lo que aisla la diferencia en `filas_capa_buffer` / `io_lineas`, que no estaba incluido en `hc..hl`, coincidiendo con la entrada del intérprete en `0x080BD748..0x080BD820` desde `frame=907`).
- **Cuándo se descubrió:** durante la revisión humana y el análisis de telemetría poscompilación.
- **Estado del proyecto:** `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs` y `src/ppu_nativa_tests.rs` contienen los cambios de esta propuesta (tests en verde); `scripts/package_apk.ps1` incluye la copia automática a la carpeta compartida solicitada por el usuario.

### Pieza por pieza

| Pieza | Estado | Motivo |
| :--- | :--- | :--- |
| `S1` | falsa | En la cinemática real en Android, cuando `BLDCNT = 0x2861` (`rel=44..80`) `bg_prio3` vale `2`, y en `rel=150..2560` `bg_prio3` vale `3`; la condición `bg_prio3 <= 1` no se activa en esos tramos. |
| `S2` | vigente | Ningún test de inspección estática de WGSL se rompió. |
| `R1` | vigente | No se mutaron registros `BLDCNT`/`BLDALPHA` en memoria (ADR 47). |
| `R2` | afectada | Depende de `S1`: la firma `bg_prio2 == 3 && bg_prio3 <= 1` no cubre `rel=44..80` (`bg_prio3 == 2`) ni `rel=150..2560` (`bg_prio3 == 3`). |
| `R3` | vigente | Los 94 tests unitarios de `ppu_nativa` pasan sin debilitar asserts. |
| `C1` | vigente | `test_wgsl_fuego_dragon_mezcla_unica_cuadro_677` pasa en CPU. |
| `C2` | afectada | Depende de `S1`: prueba el caso sintético `bg_prio3 == 1` con `0x2861`, pero en el juego real con `0x2861` (`rel=44..80`) `bg_prio3` es `2` y en `rel=150..2560` es `3`. |
| `C3` | vigente | Compilación Naga WGSL y suite `ppu_nativa` en verde. |

### Resumen de la lista

- **Se mantienen sin cambios:** `S2`, `R1`, `R3`, `C1`, `C3`
- **Afectadas por el hecho:** `S1` (falsa), `R2` y `C2` (dependen de `S1`)
- **Ya hechas en el proyecto:** edición de `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs` y `scripts/package_apk.ps1`

### Opciones

- **B · Propuesta de corrección que hereda de esta:** Mantener la limpieza del doble `alpha_blend` (`S2`, `R1`, `R3`, `C1`, `C3`) y abrir una nueva propuesta para diagnosticar y corregir la causa real en `rel=44..80` (`bg_prio3 == 2`) y `rel=150..502` (`bg_prio3 == 3`, incluyendo la huella `hcapa` de `ConfigCapaGpu` / `io_lineas` y los bloques AOT `0x080BD748..0x080BD820` que caen a intérprete desde `frame=907`).
- **C · Deshacer y empezar de nuevo:** Revertir los cambios en `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs` y `src/ppu_nativa_tests.rs` (conservando el cambio utilitario en `scripts/package_apk.ps1`), marcar esta propuesta como `cancelada` y escribir una propuesta nueva desde cero.
- **D · Parar y preguntar:** Esperar tu indicación sobre en qué tramo exacto de la cinemática ves el parpadeo más fuerte (si al inicio con la puerta/fuego `rel=0..149` o durante la historia de humanos y el rugido del dragón `rel=150..600+`).

**Recomendación:** **Opción B**, porque el cambio actual en `src/ppu_nativa.wgsl` elimina el doble `alpha_blend` sin regresiones, y nos permite enfocar la siguiente propuesta directamente en lo que revelaron los logs (`rel=150..502` y `ConfigCapaGpu` / intérprete en `0x080BD748..0x080BD820`).

### Decisión

- **Opción elegida:** B (Propuesta de corrección que hereda de esta)
- **Motivo:** Conservar la limpieza de la doble mezcla en `src/ppu_nativa.wgsl` y abrir la propuesta de Prioridad 2 para ejecutar los bloques C11 AOT (`run_frame_native`) en PC (`ppu_doble`) y comprobar si los parpadeos se reproducen en escritorio.
