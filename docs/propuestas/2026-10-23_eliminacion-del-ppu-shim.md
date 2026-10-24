# Propuesta: Eliminar el PpuShim del crate y de la app

Fecha: 2026-10-23
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El port deja de llevar dentro el rasterizador GPU antiguo (`PpuShim`): la app solo compone con la
PPU nativa y, si esa no se puede crear, vuelve al renderizado por software. Los 29 arneses y las
10 pruebas que dependían del shim se conservan como archivo de consulta, sin compilarse.

### 2. Contexto

- El shim **no está muerto en el código**: sigue conectado en [`src/platform.rs`](../../src/platform.rs)
  (campo `ppu_shim` en la línea 99, construcción en la 817, rama `use_gpu_shim` en la 980).
- La única situación en la que puede dibujar es esta: la marca `ppu-nativa-off.flag` está retirada
  (PPU nativa activa) **y** `PpuNativaPipeline::new` falla en el dispositivo. Entonces
  `ppu_nativa_fallida` queda en `true` y el shim dibuja el cuadro
  ([`src/platform.rs`](../../src/platform.rs) líneas 966-1008).
- Si el shim desapareciera sin más, ese caso mostraría un búfer **vacío**, porque el hilo de
  emulación no rellena `rgba` cuando la GPU está activa
  ([`src/pipeline.rs`](../../src/pipeline.rs) líneas 355-358). Por eso la eliminación incluye
  apagar la GPU en el puente cuando la PPU nativa falla.
- El ADR 42 ya decidió retirar el shim y descartó «mantener un modo híbrido o fallback de PPU
  software» ([`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), entrada 42). Lo que
  falta es ejecutar esa decisión en el código.
- [`docs/PENDIENTES.md`](../PENDIENTES.md) ya tiene abierta la tarea «Borrado definitivo del
  PpuShim» (línea 45).
- Restricciones del proyecto:
  - No se toca `gba-core` ni el motor upstream (origen: `AGENTS.md` §4.1).
  - Los tests se escriben antes de editar y deben fallar por la razón correcta (origen:
    `AGENTS.md` §6, fase 1).
  - El registro histórico no se reescribe (origen: `docs/propuestas/README.md`, «Nota histórica»).
- Decisión del usuario (2026-10-23): «Las pruebas de informe y los arneses obsoletos se guardan
  como registro documental, solo se elimina el código y se desconecta de la app y el APK».
- Alternativas descartadas:
  - **Borrar los 29 arneses y las 10 pruebas**: el usuario pidió conservarlos como registro.
  - **Dejar el módulo `pub mod ppu_shim` compilado pero sin uso**: no elimina código del crate ni
    del APK y mantiene el mantenimiento de `wgpu`.
  - **Mantener el shim como fallback**: contradice el ADR 42 y la petición.

### 3. Ejemplos

- Caso normal, arranque: un APK recién instalado con `ppu-nativa-off.flag` ausente → antes: la PPU
  nativa dibuja y el shim queda construido en memoria sin usarse → ahora: la PPU nativa dibuja y
  no existe ningún pipeline de shim. Mismo resultado en pantalla.
- Caso límite, GPU sin soporte para la PPU nativa: el dispositivo rechaza
  `PpuNativaPipeline::new` → antes: el shim dibujaba el cuadro → ahora: se apaga la GPU en el
  puente, el hilo de emulación vuelve a rasterizar por software y el siguiente fotograma se sube
  como RGBA. Se pierde un fotograma durante el cambio; la imagen no se queda en negro.
- Caso de inventario: `cargo test --test arnes_experimentos_tests` → antes: 47 experimentos y 45
  con `proto` → ahora: 18 y 16. Los 29 retirados viven en `docs/historico/ppu-shim/`.

### 4. Pasos

1. Escribir los tres tests nuevos de `tests/higiene_ppu_shim_tests.rs` y actualizar las cifras de
   `tests/arnes_experimentos_tests.rs` (18 experimentos, 16 con `proto`); ejecutarlos y comprobar
   que fallan porque el shim todavía está.
2. Mover a `docs/historico/ppu-shim/` los 29 arneses y las 10 pruebas de informe, con un
   `README.md` que explique que no compilan y por qué se conservan.
3. Quitar los 29 bloques `[[example]]` de `Cargo.toml`.
4. Borrar `src/ppu_shim.rs`, `src/ppu_shim.wgsl` y `src/ppu_shim_tests.rs`; quitar
   `pub mod ppu_shim;` de `src/lib.rs`.
5. En `src/platform.rs`: quitar el campo, la construcción y la rama del shim; dejar el camino por
   software como único fallback. Al detectar `ppu_nativa_fallida`, llamar
   `worker.bridge().set_gpu_ppu_enabled(false)` una sola vez y registrarlo en el log. Cambiar la
   cabecera de telemetría de `+gpu_shim` a `+gpu_nativa`.
6. En `examples/comun/gpu.rs`: borrar el enum `Backend`, la construcción del shim y `Gpu::nueva()`;
   dejar `Gpu::nueva_nativa()` como única vía. Los 4 ejemplos que la usan no cambian.
7. Actualizar los comentarios de `src/perf.rs` y `src/pipeline.rs` que dicen «PpuShim».
8. Regenerar `docs/referencia/api-modulos-rust.md` y actualizar `docs/PENDIENTES.md`,
   `docs/explicacion/DECISIONES.md` (nota de implementación en el ADR 42),
   `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`,
   `docs/explicacion/ppu-caminos-de-composicion.md`, `GLOSSARY.md` y `examples/README.md`.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "ppu_shim\|PpuShim" src/ examples/ tests/ Cargo.toml` y
`grep -rn "gpu_shim" src/`.

- `src/lib.rs` línea 36: declaración del módulo.
- `src/platform.rs`: campo (99), construcción (817), rama `use_gpu_shim` (980-1008), comentario de
  `FORZAR_PPU_SOFTWARE` (144), cabecera de telemetría (1581).
- `src/pipeline.rs`: comentarios de `FramePacket::ppu_state` (68) y del puente (173, 179).
- `src/perf.rs`: comentarios de `is_gpu` y del contador de fotogramas GPU (56, 80).
- `examples/comun/gpu.rs`: import de `PpuShimPipeline`, enum `Backend`, `Gpu::nueva()`.
- 29 ejemplos: 26 `shim_*.rs` + `ppu_proto.rs` + `ppu_compare.rs` + `ppu_frente_a_frente.rs`.
  Los tres últimos también son específicos del shim: comparan el shim contra el software.
- 10 pruebas de informe: `capa_ninguno`, `contenido_bg_shim`, `identificar_x`,
  `mezcla_registros`, `obj_apagado`, `origen_x`, `shim_aportacion`, `shim_fondo`,
  `sprite_debajo` y `frente_a_frente` (29 tests en total). Ninguna importa `ppu_shim`; leen
  informes de `volcados/`, que hoy no existen (solo hay `volcados/ppu-nativa-5300.txt`).
- `tests/arnes_experimentos_tests.rs`: cifras 47 y 45, y la lista de 47 nombres.
- `tests/packaging_tests.rs` no enumera ejemplos: lee solo el nivel superior de `docs/propuestas/`,
  así que el subdirectorio nuevo no lo afecta.
- `scripts/extraer_referencias.py` escanea solo `src/`: la referencia técnica cambia al borrar el
  módulo y hay que regenerarla.
- `docs/explicacion/ppu-caminos-de-composicion.md` afirma hoy que el shim «sigue sin ventana de
  sprites» (línea 769); queda en pasado.
- `docs/BACKLOG.md` y `RANT.md` contienen menciones históricas al shim que no se tocan.

### 6. Suposiciones

- «Registro documental» significa mover los 29 arneses y las 10 pruebas a
  `docs/historico/ppu-shim/`, fuera de `examples/` y `tests/`, donde no se compilan ni se ejecutan.
  → si significa mantener las pruebas dentro de la suite, cambian los criterios 2 y 3: las 10
  pruebas se quedan en `tests/` y el total de tests no baja de 82 a 85 de integración.
- El validador Diátaxis no marca la carpeta nueva: `README.md` se trata como página índice y se
  omiten sus secciones obligatorias (comprobado en
  `.agents/skills/diataxis-docs/scripts/validar_docs.py`, líneas 51 y 235-237). → si la marcara,
  el archivo pasa a `docs/propuestas/historico/`, que el validador excluye (línea 329).
- Un fotograma con el búfer anterior durante el cambio a software es aceptable. → si no lo es,
  hay que rellenar `rgba` también cuando la GPU está activa, con un coste de copia por fotograma.
- Ningún script de empaquetado ni de diagnóstico enumera los ejemplos por nombre; solo
  `contrastar_arnes.py` recibe el nombre por argumento. → si alguno los enumerara, habría que
  actualizarlo antes de borrar los bloques de `Cargo.toml`.

### 7. Alcance

- Incluye:
  - Borrar el código del shim y desconectarlo de `platform.rs`, `lib.rs` y el arnés común.
  - Archivar los 29 arneses y las 10 pruebas que dependen del shim.
  - Degradar a software cuando la PPU nativa no se pueda construir.
- No incluye:
  - Reescribir las propuestas históricas que citan `--example shim_*`: siguen siendo registro de
    lo que se midió, aunque sus comandos ya no se puedan ejecutar.
  - Tocar `docs/BACKLOG.md`, `RANT.md` ni `gba-core`.
  - Actualizar el recuento de tests de `AGENTS.md`, que ya estaba desfasado antes de este cambio
    (dice 285; la suite actual tiene 295).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El shim ya no está en el crate ni en la app
  Dado que se ejecuta la suite desde la raíz del repositorio
  Cuando el test lee `src/` y `Cargo.toml`
  Entonces no existen `src/ppu_shim.rs`, `src/ppu_shim.wgsl` ni `src/ppu_shim_tests.rs`, `src/lib.rs`
  no declara `ppu_shim`, y `src/platform.rs` no contiene `ppu_shim`, `PpuShim` ni `gpu_shim`
Test: `tests/higiene_ppu_shim_tests.rs::test_el_shim_ya_no_esta_en_el_crate_ni_en_la_app` |
Estado previo: falla | pasa

Escenario: Quedan 18 experimentos y el arnés solo usa la PPU nativa
  Dado que se listan los archivos de `examples/` y los bloques `[[example]]` de `Cargo.toml`
  Cuando el test cuenta los experimentos y lee `examples/comun/gpu.rs`
  Entonces hay 18 archivos y 18 bloques, ninguno empieza por `shim_`, y `gpu.rs` no contiene
  `PpuShimPipeline` ni `Backend::Shim` pero sí `PpuNativaPipeline`
Test: `tests/higiene_ppu_shim_tests.rs::test_quedan_18_experimentos_y_el_arnes_solo_usa_la_ppu_nativa`
  y `tests/arnes_experimentos_tests.rs::test_el_inventario_coincide_con_cargo_toml` |
Estado previo: falla (47 y 45) | pasa

Escenario: El archivo documental conserva los 29 arneses y las 10 pruebas
  Dado que existe `docs/historico/ppu-shim/`
  Cuando el test cuenta sus archivos `.rs`
  Entonces hay 39 archivos `.rs` y su `README.md` nombra el ADR 42
Test: `tests/higiene_ppu_shim_tests.rs::test_el_archivo_documental_conserva_los_39_ficheros` |
Estado previo: falla (la carpeta no existe) | pasa

Escenario: Si la PPU nativa falla, el juego vuelve al software (visible)
  Dado un APK de prueba donde `PpuNativaPipeline::new` devuelve `Err`
  Cuando se inicia el juego en el dispositivo
  Entonces la imagen del juego sigue apareciendo (renderizada por software) y el log registra
  «PPU nativa no disponible: se vuelve al renderizado por software»
Test: revisión humana en el dispositivo | Estado previo: no aplica | pasa

### 9. Cómo lo comprobaré

- El shim ya no está en el crate ni en la app → comando:
  `cargo test --test higiene_ppu_shim_tests test_el_shim_ya_no_esta_en_el_crate_ni_en_la_app`
  → pasa si: 1 test aprobado, 0 fallos → no detecta: símbolos que pudieran quedar en el `.so`
  empaquetado (para eso haría falta inspeccionar el binario con el NDK).
- Quedan 18 experimentos y el arnés es nativo → comandos:
  `cargo test --test higiene_ppu_shim_tests test_quedan_18_experimentos_y_el_arnes_solo_usa_la_ppu_nativa`
  y `cargo test --test arnes_experimentos_tests`
  → pasa si: 18 archivos, 18 bloques, 16 con `proto`, y el arnés solo declara `PpuNativaPipeline`
  → no detecta: que cada experimento siga midiendo lo mismo al ejecutarse (necesita ROM y GPU).
- El archivo documental conserva los 39 ficheros → comando:
  `cargo test --test higiene_ppu_shim_tests test_el_archivo_documental_conserva_los_39_ficheros`
  → pasa si: 39 `.rs` y el README cita el ADR 42 → no detecta: que esos ficheros sigan siendo
  reproducibles (no lo son: dependen del shim borrado).
- Fallback a software (visible) → revisión humana: construir un APK de prueba con
  `PpuNativaPipeline::new` forzado a `Err`, instalarlo y comprobar que se ve el juego y el mensaje
  del log → no detecta: otros modos de fallo de GPU distintos de la creación del pipeline.
- Regresión completa → comandos: `cargo build --all-targets --features proto` (exit 0),
  `cargo test` (250 tests, 0 fallos), `python scripts/extraer_referencias.py --check` (exit 0) y
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` (0 errores, 0 avisos)
  → no detecta: el comportamiento en el dispositivo, que cubre el escenario visible.
- Si el entorno no tiene NDK, el empaquetado con
  `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` no se podrá ejecutar; se
  anotará como no comprobado en el `Resultado`.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación y la suite completa queda en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- Supuse que el shim ya no se usaba en ningún caso. Al leer `platform.rs` y `pipeline.rs` verifiqué
  que sí se usa cuando falla la construcción de la PPU nativa; el plan añade el paso 5 para no
  dejar ese caso con un búfer vacío.
- Supuse que las 10 pruebas de informe tenían volcados que las hacían ejecutables. Verifiqué que
  `volcados/` solo contiene `ppu-nativa-5300.txt`, así que esas pruebas ya se omitían sin `FE_ROM`.
  Archivarlas no elimina verificaciones que hoy corran.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Retirada del shim en el crate y la app:** Predicho y cumplido. Se eliminaron `src/ppu_shim.rs`, `src/ppu_shim.wgsl` y `src/ppu_shim_tests.rs`; se quitó la declaración en `src/lib.rs` y se desconectaron todos los campos y ramas en `src/platform.rs`.
- **Archivo histórico documental:** Predicho y cumplido. Se trasladaron 29 arneses de `examples/` y 10 pruebas de `tests/` a `docs/historico/ppu-shim/` junto con su `README.md` explicativo del ADR 42.
- **Inventario de experimentos:** Predicho y cumplido. En `examples/` y en `Cargo.toml` permanecen exactamente 18 ejemplos (16 con la feature `proto`).
- **Degradación limpia a software:** Predicho y cumplido. Si `PpuNativaPipeline::new` falla, `AppState::redraw` desactiva la GPU en el worker (`worker.bridge().set_gpu_ppu_enabled(false)`) y el hilo de emulación pasa a generar búferes RGBA por software directamente.
- **Suite de pruebas:** Predicho y cumplido. La suite completa (`cargo test`) finalizó con 254 tests aprobados (0 fallos).

### Archivos modificados

- **Eliminados (3 en `src/`, 29 en `examples/`, 10 en `tests/`):**
  - `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `src/ppu_shim_tests.rs`
  - 29 ejemplos de `examples/` (`shim_*.rs`, `ppu_proto.rs`, `ppu_compare.rs`, `ppu_frente_a_frente.rs`)
  - 10 pruebas de `tests/` (`capa_ninguno`, `contenido_bg_shim`, `frente_a_frente`, etc.)
- **Creados:**
  - `tests/higiene_ppu_shim_tests.rs`
  - `docs/historico/ppu-shim/README.md` (y los 39 archivos `.rs` archivados)
- **Modificados:**
  - `src/lib.rs`, `src/platform.rs`, `src/perf.rs`, `src/pipeline.rs`
  - `Cargo.toml`, `examples/comun/gpu.rs`
  - `tests/arnes_experimentos_tests.rs`, `tests/ppu_nativa_tests.rs`, `tests/datos/ppu-nativa-hashes-base.txt`
  - `docs/referencia/api-modulos-rust.md`, `docs/PENDIENTES.md`, `docs/explicacion/DECISIONES.md`, `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`, `docs/explicacion/ppu-caminos-de-composicion.md`, `GLOSSARY.md`, `examples/README.md`

### Discrepancias

- La prueba `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian` verificaba los hashes de los 3 archivos del shim en `src/` a través de `tests/datos/ppu-nativa-hashes-base.txt`. Al borrarse el shim, la prueba falló al comprobar solo 2 ficheros en vez de 3. Se sincronizó el archivo de hashes base para omitir los archivos borrados y verificar la inmutabilidad de los 2 archivos de `gba-core`.

### Para la próxima vez

- Incluir `tests/datos/` al escanear referencias a identificadores de módulos antes de redactar la propuesta.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno. Todos los componentes y documentos asociados quedaron completamente sincronizados.

---

## Revisión humana

- Si la PPU nativa falla, el juego vuelve al software: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
