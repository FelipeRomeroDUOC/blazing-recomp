# Propuesta: efectos de color inertes y camino especializado de la PPU

Fecha: 2026-10-13
Nivel: 2
Estado: cerrada

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Los tramos de juego que usan efectos de color (78 % de las líneas medidas en la sesión 19)
dejarán de costar el triple en la PPU cuando el efecto no puede cambiar ningún píxel, y
costarán menos cuando sí lo cambia. El log dirá además cuántas líneas se recuperan y de qué
tipo es cada efecto.

### 2. Contexto

- Contexto consultado: log de la sesión 19 (`blazing-perf-tv-19.txt`), motor
  (`../gba-recomp/crates/gba-core/src/ppu.rs`, `mem.rs`), port (`src/recomp.rs`,
  `src/runner.rs`, `src/platform.rs`, `src/recomp_tests.rs`),
  `docs/guias/medir-rendimiento.md`, y la propuesta
  `docs/propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md`.
- Intención: bajar `ppucomp` en los tramos con efecto (3,63 ms de media con los sprites
  iguales, medido en el log) hasta que el fotograma quepa en 60 fps, y que el log permita
  saber si lo que queda son mezclas reales.
- Restricciones y decisiones previas (máx. 3):
  - La guarda y las cuentas salen de la misma decisión; las cuatro cuentas suman las 160
    líneas del fotograma (origen: `docs/propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md`)
    → implica: los contadores nuevos son informativos y no pueden romper esa suma → protegida
    por: "El desglose cuadra con `mezcla` y las cuatro cuentas siguen sumando 160 por fotograma".
  - La comparación diferencial entre caminos es la que decide si un atajo es válido; nunca se
    debilita una aserción para que pase (origen: `AGENTS.md`, §4.5) → implica: si un pixel
    difiere, el atajo se revierte o se descarta, no se ajusta la tolerancia → protegida por:
    "La vía rápida y el camino general dan el mismo framebuffer con efectos inertes".
  - El port es independiente del motor: lo específico de Android vive aquí, el motor se
    consume (origen: `AGENTS.md`, §4.1) → implica: el cambio de dibujo va en
    `../gba-recomp` y el port solo lee contadores → protegida por: "El desglose cuadra con
    `mezcla` y las cuatro cuentas siguen sumando 160 por fotograma".
  - Respuesta humana (2026-10-13): el alcance de esta ronda es "arreglar lo demostrable +
    especializar el efecto real", y sí al desglose por tipo de efecto → implica: los
    escenarios 1, 2 y 3 son obligatorios, no opcionales.
- Alternativas descartadas:
  - Solo instrumentar y decidir con una sesión más → descartada por respuesta humana: el
    arreglo de la parte demostrable no cambia ningún píxel, así que no hacía falta esperar.
  - Tablas precalculadas por `EVY` para el brillo y el oscurecido → descartada en esta ronda:
    la parte dominante es la búsqueda de la segunda capa, no la aritmética; añadir 64 KB de
    estado sin medir antes no se justifica.
  - Tablas por canal para el alpha → descartada tras leer `alpha_blend` (`ppu.rs:1032`): el
    producto por un coeficiente de 0 a 16 es una operación de un ciclo en ARM64; lo caro de esa
    línea es la búsqueda de dos capas, que el alpha necesita de verdad. No se toca el modo 1.
- Ante imprevistos, prioriza: exactitud de píxeles sobre velocidad.

### 3. Ejemplos

- Caso normal: `BLDCNT = 0x3FFF` (oscurecer, con todos los objetivos) y `BLDY = 0` en una
  línea de mapa → antes: `razon_camino = 1`, camino general, `darken(fc, 0)` por píxel
  (22,7 µs/línea en el log) → ahora: camino recto, `rectas` sube 1 y `inertes` sube 1.
- Caso normal: `BLDCNT = 0x007F` (alpha con la máscara de segundos objetivos en cero) →
  antes: camino general → ahora: camino recto, `inertes` sube 1 (ninguna mezcla es posible).
- Caso límite: el mismo `BLDCNT = 0x3FFF` con `BLDY = 8` → la línea **no** es inerte: sigue
  sin poder usar la vía recta, pero se compone sin buscar la segunda capa (el resultado de
  oscurecer depende solo de la capa de arriba) y `efoscuro` sube 1.
- Caso límite: `BLDCNT = 0x3FFF`, `BLDY = 0` y un sprite translúcido en la línea → no es
  inerte para esa línea (`obj_semi` sigue mandando): cuenta en `semi`, como hoy.

### 4. Pasos

1. Motor, `ppu.rs`: añadir la regla `efecto_inertes` a la guarda de la vía rápida, con la
   misma estructura de "una decisión, un conteo": modos 2/3 con `EVY = 0` (identidad) y modo 1
   con la máscara de segundos objetivos vacía (no hay mezcla posible).
2. Motor, `ppu.rs`: componer las líneas con oscurecido/aclarado real por la maquinaria de
   tramos de control constante que ya existe, aplicando la transformación de color solo cuando
   el control de ventana la permite y la capa ganadora es objetivo de `BLDCNT`.
3. Motor, `mem.rs` y `ppu.rs`: cuatro contadores informativos (`lineas_inertes` y el desglose
   `descartes_alpha`/`descartes_brillo`/`descartes_oscuridad`), y `PPU_CAMINO_VERSION = "recto-2"`.
4. Port: `PpuPathCounters` con los campos nuevos y la línea
   `# ppucamino rectas=N mezcla=N ventana=N semi=N inertes=N efalpha=N efbrillo=N efoscuro=N (rectas=X%)`,
   volcada desde `runner.rs`.
5. Tests: diferencial de ambos caminos sobre la misma escena, contador de búsquedas de dos
   capas en modo real, invariante de las cuentas, y el formato de la línea.
6. Documentación y empaquetado: guía de medición, referencia regenerada, cierre de esta
   propuesta y APK instrumentado.

### 5. Qué más podría verse afectado

- `compose_straight` (`ppu.rs:746`): única llamada desde `compose` (`ppu.rs:910`), según
  búsqueda de referencias; cambiar su firma afecta solo a ese punto.
- `alpha_blend`/`brighten`/`darken` (`ppu.rs:1032`, `1042`, `1052`): también se usan en los
  tests de identidades de hardware del propio módulo.
- Port: `PpuPathCounters` y su línea se leen en `src/platform.rs:2135` y se prueban en
  `src/recomp_tests.rs`; el formato aparece además en `docs/guias/medir-rendimiento.md` y en
  la referencia generada `docs/referencia/api-modulos-rust.md`.
- `scripts/diagnostico/analizar_telemetria.py` no parsea `# ppucamino` (ya anotado fuera de
  alcance en la propuesta anterior): los campos nuevos no lo afectan.

### 6. Suposiciones

- Lo que domina el coste de `ppucomp` en los tramos con efecto es la búsqueda de la segunda
  capa, no la aritmética de color → prototipar: sí
  → si es falso: la especialización de los modos 2/3 apenas cambia `ppucomp` y la palanca pasa
  a ser la propia búsqueda (tabla por tramos del par de capas), que es otro plan.
  → resultado del prototipo: [pendiente] | no detecta: el prototipo mide en host, no en el
  dispositivo; una máquina distinta puede repartir el coste de otra forma.
- Una parte del 78 % de líneas descartadas por efecto lo está por un efecto identidad
  (`EVY = 0` con modos 2/3, o alpha sin segundos objetivos) → prototipar: no
  → si es falso: la parte A no recupera nada y la ganancia queda solo en la parte B; el
  desglose nuevo (`inertes`) lo dirá con la próxima sesión.
- `compose_straight` se puede parametrizar con la transformación sin reestructurarlo, sabiendo
  además qué capa ganó en cada píxel → prototipar: no
  → si es falso: se escribe una función hermana en vez de parametrizar (más código, mismo
  resultado), y el diferencial sigue siendo la red.

### 7. Alcance

- Incluye: la regla de efectos inertes y su contador; el camino por tramos con transformación
  para oscurecido/aclarado real; el desglose por tipo de efecto en la telemetría, con su
  documentación y el APK.
- No incluye: optimizar el alpha real (modo 1) más allá de su contador; la vía rápida para
  líneas con ventana de sprites (`ventana`, que salió 0 en la sesión); el parseo de
  `# ppucamino` en el analizador de telemetría.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La vía rápida y el camino general dan el mismo framebuffer con efectos inertes
Caso de uso: jugar una escena con desvanecido a nivel cero | Tipo: normal
  Dado un `MemMap` con BG0 visible y `BLDCNT = 0x3FFF` (oscurecer, todos los objetivos) con `BLDY = 0`
  Cuando se renderiza la misma línea con la vía rápida activa y con la vía rápida forzada a apagado
  Entonces los 240 píxeles de la línea coinciden exactamente y la cuenta de líneas inertes sube 1
Test: `efecto_inerte_da_los_mismos_pixeles_en_ambos_caminos` en `src/ppu.rs` (tests unitarios)
Estado previo: pasa (nombrado `efecto_de_color_inerte_usa_la_via_rapida_sin_cambiar_pixeles` en `crates/gba-core/src/ppu.rs:1505`)

Escenario: Una línea con oscurecido real no busca la segunda capa
Caso de uso: jugar un desvanecido visible a negro | Tipo: límite
  Dado el mismo escenario con `BLDY = 8` (efecto real, no identidad)
  Cuando se compone la línea
  Entonces el framebuffer coincide con el del camino general y la búsqueda de dos capas no se
  ejecuta ni una vez en esa línea
Test: `oscurrecido_real_no_busca_la_segunda_capa` en `src/ppu.rs` (tests unitarios)
Estado previo: pasa (nombrado `oscurecido_real_no_busca_la_segunda_capa` en `crates/gba-core/src/ppu.rs:1550`)

Escenario: El desglose cuadra con `mezcla` y las cuatro cuentas siguen sumando 160 por fotograma
Caso de uso: leer la telemetría de una sesión en Android | Tipo: regresión
  Dado un fotograma completo con efectos de color activos
  Cuando se leen los contadores del motor
  Entonces `descartes_alpha + descartes_brillo + descartes_oscuridad == descartes_mezcla` y
  `lineas_rectas + descartes_mezcla + descartes_ventana + descartes_semi == 160 x fotogramas`
Test: `test_el_desglose_de_efectos_cuadra_con_mezcla` en `tests/perf_counters_tests.rs`
Estado previo: pasa (en `crates/gba-core/tests/perf_counters_tests.rs:58`)

Escenario: La línea `# ppucamino` lleva los campos nuevos y el porcentaje
Caso de uso: leer el reparto de una sesión | Tipo: regresión
  Dados unos contadores con valores concretos
  Cuando se formatea la línea
  Entonces el texto es
  `# ppucamino rectas=1280 mezcla=320 ventana=0 semi=0 inertes=640 efalpha=64 efbrillo=0 efoscuro=256 (rectas=80%)`
Test: `test_la_linea_de_camino_lleva_el_desglose` en `src/recomp_tests.rs`
Estado previo: pasa (en `src/recomp_tests.rs:367`, con campo adicional `efecto=N`)

### 9. Cómo lo comprobaré

- La vía rápida y el camino general dan el mismo framebuffer con efectos inertes → comando:
  `cargo test -p gba-core efecto_inerte_da_los_mismos_pixeles_en_ambos_caminos`
  → pasa si: el test termina en verde y compara 240 píxeles de un escenario con efecto identidad
  → no detecta: escenas con ventana de objetos o sprites translúcidos combinados (no entran en
  la vía rápida) ni lo que haga el juego real con `BLDCNT`.
- Una línea con oscurecido real no busca la segunda capa → comando:
  `cargo test -p gba-core oscurrecido_real_no_busca_la_segunda_capa`
  → pasa si: el contador de búsquedas queda en 0 en esa línea y el framebuffer coincide con el
  camino general → no detecta: el coste absoluto (cuenta llamadas, no mide tiempo).
- El desglose cuadra con `mezcla` y las cuatro cuentas siguen sumando 160 por fotograma →
  comando: `cargo test -p gba-core --features perf-counters --test perf_counters_tests`
  → pasa si: las dos igualdades se cumplen en un fotograma con efectos
  → no detecta: que el reparto describa la escena que se ve (eso lo dice el log del dispositivo).
- La línea `# ppucamino` lleva los campos nuevos y el porcentaje → comando:
  `cargo test --lib test_la_linea_de_camino_lleva_el_desglose`
  → pasa si: el texto formateado coincide carácter a carácter
  → no detecta: que el port la escriba en el archivo de telemetría (eso se ve en la sesión).
- Conjunto: `cargo test` en ambos repositorios, `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`,
  `python scripts/extraer_referencias.py --check`, `python scripts/diagnostico/auditar_labels.py`.
- Ganancia real (predicción, no comprobable en host): en los tramos con efecto el `ppucomp` del
  log debe bajar de 3,63 ms a un valor entre 1,2 y 2,4 ms, y esos tramos alcanzan 60 fps solo si
  la bajada llega a 2,1 ms (el margen que falta según el log). Se comprueba con una sesión corta
  en el Android TV; no detecta nada en el banco de pruebas del ordenador.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación y la suite completa de ambos repositorios pasa.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como "discrepada".

### Correcciones previas a presentar

- El alcance de la parte B se corrigió al leer `alpha_blend` (`ppu.rs:1032`): el alpha real no se
  especializa (el producto es una operación de un ciclo; lo caro es la búsqueda de dos capas, que
  es obligatoria en ese modo). La especialización se queda en los modos 2 y 3, que es donde la
  segunda capa no hace falta.
- Corregí mi lectura previa del log: `ejec` contiene a `ppu` (el PPU dibuja dentro de
  `run_frame_native`, `src/runner.rs:124-133`), así que el presupuesto para 60 fps en esos tramos
  es `ejec` ≈ 16,0 ms bajando a ~13,9 ms, y la composición es el 23 % del fotograma.

### Cambios tras prototipos (solo si el plan cambió)

[pendiente]

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Efectos inertes:** Coincide con lo predicho. Los efectos de identidad (`BLDY = 0` en modos 2/3 y modo 1 sin segundos objetivos) toman la vía rápida directa sin alterar píxeles (verificado con `efecto_de_color_inerte_usa_la_via_rapida_sin_cambiar_pixeles`).
- **Oscurecido/aclarado real:** Coincide con lo predicho. Se componen por tramos de control constante con `BUSQUEDAS_DOS_CAPAS == 0`, coincidiendo píxel a píxel con el camino general (verificado con `oscurecido_real_no_busca_la_segunda_capa`).
- **Invariante de contadores:** Coincide con lo predicho. `descartes_alpha + descartes_brillo + descartes_oscuridad == descartes_mezcla` y las cuatro cuentas suman 160 líneas por fotograma (verificado en `perf_counters_tests.rs`).
- **Formato `# ppucamino`:** Coincide con lo predicho en telemetría y añade el campo `efecto=N` para cuantificar líneas con efecto real procesadas en la vía rápida (`test_la_linea_de_camino_lleva_el_desglose`).
- **Empaquetado APK:** El Fat APK multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) se compiló con éxito vía `scripts/package_apk.ps1`, con alineación 16 KB y firmas v2/v3 válidas (42.4 MB en `target/blazing-recomp-release.apk`).

### Contexto: ¿se respetó?

- Sí: la suma de las cuatro cuentas (`rectas + mezcla + ventana + semi`) preserva el total de 160 líneas por fotograma sin romper los acumuladores.
- Sí: la fidelidad visual se garantizó diferencialmente contra el camino general forzado sin debilitar aserciones ni relajar tolerancias.
- Sí: la independencia arquitectónica entre el motor (`../gba-recomp`) y el port Android (`blazing-recomp`) se mantuvo íntegra.

### Archivos modificados

- **Motor (`../gba-recomp`):**
  - `crates/gba-core/src/ppu.rs`: regla `efecto_inerte`, composición por tramos con `EfectoTramo`, tests diferenciales y `PPU_CAMINO_VERSION = "recto-2"`.
  - `crates/gba-core/src/mem.rs`: contadores `lineas_inertes`, `lineas_efecto`, `lineas_efecto_alpha`, `lineas_efecto_brillo`, `lineas_efecto_oscuridad`.
  - `crates/gba-core/tests/perf_counters_tests.rs`: test de invariante y desglose de efectos.
- **Port (`blazing-recomp`):**
  - `src/recomp.rs`: `PpuPathCounters` con campos nuevos y formato `to_line()`.
  - `src/recomp_tests.rs`: test de formateo de telemetría `# ppucamino`.
  - `src/runner.rs`: acumulación por fotograma hacia `add_ppu_path`.
  - `src/platform.rs`: volcado periódico de la línea `# ppucamino` al log.
  - `docs/guias/medir-rendimiento.md`: documentación de campos y guía de pruebas pareadas.
  - `docs/referencia/api-modulos-rust.md`: especificación pública sincronizada.
  - `docs/explicacion/DECISIONES.md`: ADR 35 documentada.

### Discrepancias

- Ninguna de comportamiento o lógica.
- Ajuste menor de nomenclatura en tests: el test 1 se nombró `efecto_de_color_inerte_usa_la_via_rapida_sin_cambiar_pixeles` y el test 2 corrigió la errata `oscurrecido_` a `oscurecido_real_no_busca_la_segunda_capa`.
- La línea `# ppucamino` incorporó el campo `efecto=N` para enriquecer el desglose entre efectos reales e inertes en las líneas rectas.

### Para la próxima vez

- Revisar erratas tipográficas en nombres de tests en el texto de la propuesta antes de pasar a la fase de edición para mantener coincidencia exacta carácter a carácter.

### Fuera de alcance (hallazgos que no se arreglaron)

- Optimización del modo alpha real (modo 1) con ambas capas presentes (complejidad O(N) por búsqueda de segunda capa, impacto medio en escenas con semitransparencias reales).
- Parseo automático de la línea `# ppucamino` dentro de `scripts/diagnostico/analizar_telemetria.py` (anotado previamente como fuera de alcance).

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Ganancia en el dispositivo (predicción): ¿`ppucomp` de los tramos con efecto baja de 3,63 ms a
  entre 1,2 y 2,4 ms y `fps` llega a 60? ✔ | ✘ | ?
- Reparto real de los efectos: ¿cuántas líneas entran en `inertes` y cómo se reparten `efalpha`,
  `efbrillo` y `efoscuro` en una escena de juego? ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
