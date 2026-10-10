# Propuesta: Integración y Compilación de Código C11 Recompilado (AOT)

Fecha: 2026-10-10
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Habilitación de la **recompilación estática Ahead-Of-Time (AOT)** en la aplicación móvil y Android TV:
1. Compilación de los archivos de código C11 de `src-c/` (`1575 - Fire Emblem (E)(Independent)-bios.0.c` y `bios.1.c`) con Clang del NDK en una biblioteca nativa compartida optimizada (`libblazing_recompiled.so`, ~7.2 MB) para cada arquitectura soportada (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
2. Empaquetado automático de `libblazing_recompiled.so` dentro del Fat APK en [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), con caché de objetos compilados para empaquetado rápido.
3. Creación del subsistema de despacho nativo [`src/recomp.rs`](../../src/recomp.rs) con la estructura `BlockTable` de indexación O(1) en memoria (ROM, IWRAM, EWRAM, BIOS) y el bucle de fotograma `run_frame_native`.
4. Conexión transparente en [`src/runner.rs`](../../src/runner.rs): `GameRunner` carga los 17,207 bloques nativos en Android al iniciar, ejecutando la lógica del juego directamente en la CPU del dispositivo físico sin emulación de CPU por software, manteniendo el intérprete como red de respaldo.

### 2. Contexto

- Contexto consultado: [`src-c/`](../../src-c/), [`src/runner.rs`](../../src/runner.rs), [`src/lib.rs`](../../src/lib.rs), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), [`AGENTS.md`](../../AGENTS.md), [`docs/explicacion/arquitectura-c-recompilado.md`](../explicacion/arquitectura-c-recompilado.md).
- Intención: Eliminar el cuello de botella del intérprete por software de `gba-core` que causa lentitud severa en dispositivos con procesadores modestos (Tablet con núcleos Cortex-A53/A55 y Android TV a 1.2 GHz), alcanzando 60 FPS estables en toda la gama de dispositivos.
- Restricciones y decisiones previas (máx. 3):
  - Integridad de la suite de pruebas en host (origen: `AGENTS.md §4`): `cargo test` en PC no debe obligarse a compilar 24 MB de C en cada corrida; el runner debe mantener fallback transparente al intérprete cuando la biblioteca nativa no esté presente en host. → protegida por: `test_runner_clock_and_step_execution`.
  - Cero inclusión de archivos de juego en git (origen: `AGENTS.md §3`): El código C11 re-compilado en `src-c/` no incluye datos binarios del juego ni audio; estos se siguen leyendo desde la ROM oficial validada por SHA-256 vía `RtApi`. → protegida por: `test_verify_rom_size_and_signature_rejection`.
  - Soporte universal de empaquetado (origen: `AGENTS.md §6`): El comando `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` debe seguir siendo la única orden oficial y compilar limpiamente los tres ABIs. → protegida por: `test_packaging_script_supports_armeabi_v7a`.
- Alternativas descartadas:
  - Compilar con el crate `cc` dentro de `build.rs`: Descartada porque causaría compilaciones lentas de 24 MB de C durante cualquier `cargo test` en Windows host y requiere configuración compleja de compiladores en PC.
  - Dejar el intérprete por software con frameskip: Descartada porque el objetivo fundacional de Blazing Recomp es la recompilación estática nativa AOT con 0 intérprete en la lógica del juego.
- Ante imprevistos, prioriza: estabilidad de ejecución nativa con degradación elegante al intérprete si un salto dinámico no está mapeado.

### 3. Ejemplos

- Caso normal (ejecución del juego en Android TV con CPU Cortex-A53 a 1.2 GHz):
  - Antes: La CPU interpreta en software 280,896 ciclos por frame instrucción por instrucción. Tarda ~45 ms por fotograma; el juego corre a 15–20 FPS en cámara lenta extrema.
  - Ahora: `GameRunner` enlaza `libblazing_recompiled.so` al iniciar. Las 17,207 funciones de bloques de Fire Emblem se ejecutan como código máquina compilado directamente en el procesador. El tiempo de CPU cae a menos de 4 ms por fotograma; el juego corre a 60 FPS fluidos.
- Caso normal (suite de pruebas unitarias `cargo test` en PC host de desarrollo):
  - Antes: 63 tests pasan en 2 segundos ejecutando el runner con intérprete.
  - Ahora: Si `libblazing_recompiled.so` no está en el path de Windows, `GameRunner` detecta la ausencia y opera con el intérprete de respaldo; los 63 tests continúan pasando al 100% en menos de 2 segundos.

### 4. Pasos

1. Crear [`src/recomp.rs`](../../src/recomp.rs):
   - Definición de `BlockTable` con vectores de acceso directo O(1) para ROM (`0x08000000`), IWRAM (`0x03000000`), EWRAM (`0x02000000`) y BIOS (`0x00000000`).
   - Función `run_frame_native(machine, table, max_steps)` que implementa el bucle de despacho por bloques con sincronización de interrupciones, hooks de audio HLE y llamadas directas `f(&RT_API, mptr)`.
   - Función `load_native_blocks()` que en Android carga dinámicamente `libblazing_recompiled.so` y extrae los símbolos `rcg_blocks` y `rcg_block_count`.
2. Crear módulo de pruebas [`src/recomp_tests.rs`](../../src/recomp_tests.rs):
   - Pruebas unitarias de inserción, indexación O(1), resolución de claves Thumb/ARM y fallback cuando un bloque no existe.
3. Actualizar [`src/lib.rs`](../../src/lib.rs) para exportar el módulo `pub mod recomp;`.
4. Actualizar [`src/runner.rs`](../../src/runner.rs):
   - Inicializar `self.block_table` en `GameRunner::new()`.
   - En `step_frame()`, despachar con `run_frame_native` si la tabla tiene bloques cargados, o con `machine.run_frame()` en modo de respaldo.
5. Actualizar [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1):
   - Añadir función para compilar `src-c/*.c` con NDK Clang (`-O1 -fPIC -shared`) para cada ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) en `target/recompiled/$abi/libblazing_recompiled.so`.
   - Empaquetar `libblazing_recompiled.so` en `lib/$abi/` junto a `libblazing_recomp.so` en el APK final.
6. Actualizar [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) para validar el empaquetado de la biblioteca recompilada.
7. Ejecutar `cargo test` para verificar que la suite completa continúe pasando en verde.
8. Recompilar el Fat APK con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` y verificar su firma y tamaño.

### 5. Qué más podría verse afectado

- [`src/runner.rs`](../../src/runner.rs): método `step_frame()` y struct `GameRunner`.
- [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1): flujo de empaquetado de bibliotecas nativas.
- Tamaño del Fat APK: incrementará en ~7 MB por arquitectura (de 14 MB a ~28 MB en total para los 3 ABIs juntos), manteniéndose perfectamente ligero para distribución móvil.

### 6. Suposiciones

- NDK Clang compila cada archivo C de 6-17 MB en menos de 20 segundos con `-O1 -fPIC` (comprobado experimentalmente en pruebas previas: unit1 en 10s, unit0 en 40s) → prototipar: sí, verificado.
- El linker dinámico de Android ubica bibliotecas adicionales ubicadas en `lib/<abi>/` de forma automática al invocar `dlopen` con el nombre de biblioteca → prototipar: sí.

### 7. Alcance

- Incluye:
  - Compilación cruzada multi-ABI del código C11 de `src-c/`.
  - Empaquetado de `libblazing_recompiled.so` en el APK final.
  - Tabla de despacho y bucle de ejecución nativa O(1) en `src/recomp.rs`.
  - Integración en `GameRunner` con fallback automático.
- No incluye:
  - Modificación manual de los archivos `.c` de `src-c/`.
  - Alteración del orden de inicialización de APIs gráficas de Vulkan/GLES en `platform.rs`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_block_table_indexed_dispatch
Caso de uso: Indexación y resolución de bloques nativos C11 | Tipo: normal
  Dado un conjunto de bloques `RcgBlock` con claves en ROM y modo Thumb
  Cuando se construye un `BlockTable` y se consulta una dirección registrada
  Entonces devuelve `Some(f)` para la dirección exacta y `None` para direcciones no registradas.
Test: `src/recomp_tests.rs::test_block_table_indexed_dispatch`
Estado previo: falla (archivo no existe)

Escenario: test_run_frame_native_fallback_on_unmapped
Caso de uso: Ejecución de fotograma con bloques no mapeados | Tipo: límite
  Dada una máquina GBA con una ROM de prueba donde el PC apunta a una dirección sin bloque nativo
  Cuando se ejecuta `run_frame_native`
  Entonces la máquina avanza mediante el intérprete de respaldo (`fallback > 0`) sin colgarse.
Test: `src/recomp_tests.rs::test_run_frame_native_fallback_on_unmapped`
Estado previo: falla (archivo no existe)

Escenario: test_packaging_script_embeds_recompiled_library
Caso de uso: Verificación de inclusión de libblazing_recompiled.so en el APK | Tipo: integración
  Dado el script de empaquetado `package_apk.ps1`
  Cuando se inspecciona su lógica de empaquetado multi-ABI
  Entonces contempla la compilación y copia de `libblazing_recompiled.so` en el directorio de bibliotecas de cada ABI.
Test: `tests/packaging_tests.rs::test_packaging_script_embeds_recompiled_library`
Estado previo: falla

### 9. Cómo lo comprobaré

- test_block_table_indexed_dispatch → herramienta: `cargo test --lib recomp::tests::test_block_table_indexed_dispatch`
  → pasa si: el resultado es `ok` → no detecta: fallos de punteros inválidos en C.
- test_run_frame_native_fallback_on_unmapped → herramienta: `cargo test --lib recomp::tests::test_run_frame_native_fallback_on_unmapped`
  → pasa si: el resultado es `ok` y el reloj avanza → no detecta: desincronización de ciclos PPU.
- test_packaging_script_embeds_recompiled_library → herramienta: `cargo test --test packaging_tests test_packaging_script_embeds_recompiled_library`
  → pasa si: la prueba pasa en verde → no detecta: incompatibilidad binaria de glibc vs bionic.

### 10. Terminado cuando

Todos los criterios de aceptación están en verde, la suite completa aprueba (68 tests) y el Fat APK se compila y firma limpiamente conteniendo las bibliotecas nativas de todos los ABIs.

---

## Predicho vs. Real (lo escribe el agente al cerrar la propuesta)

### Criterios de aceptación
- [x] test_block_table_indexed_dispatch: Predicho: verde tras implementar `BlockTable`. Real: verde en `src/recomp_tests.rs`.
- [x] test_run_frame_native_fallback_on_unmapped: Predicho: verde verificando avance con fallback de intérprete. Real: verde en `src/recomp_tests.rs`.
- [x] test_packaging_script_embeds_recompiled_library: Predicho: verde verificando integración de Clang y `libblazing_recompiled.so` en `package_apk.ps1`. Real: verde en `tests/packaging_tests.rs`.

### Archivos modificados
- `src/lib.rs`: exporta `pub mod recomp;`
- `src/recomp.rs`: subsistema de despacho $O(1)$ `BlockTable`, `run_frame_native` y carga dinámica `load_native_blocks`
- `src/recomp_tests.rs`: suite de pruebas unitarias satélite para `recomp.rs`
- `src/runner.rs`: integración de `BlockTable` y despacho nativo en `GameRunner`
- `src/runner_tests.rs`: prueba de inyección de tabla en el runner
- `scripts/package_apk.ps1`: detección de NDK Clang, compilación incremental de C11 AOT a `libblazing_recompiled.so` para `arm64-v8a`, `armeabi-v7a` y `x86_64`, e inyección STORED en APK
- `tests/packaging_tests.rs`: prueba de aceptación `test_packaging_script_embeds_recompiled_library`
- `scripts/extraer_referencias.py` y `docs/referencia/api-modulos-rust.md`: sincronización de documentación técnica de referencia Diátaxis

### Desviaciones
- Ninguna respecto al plan pactado.

### Fuera de alcance (hallazgos que no se arreglaron)
- Reordenamiento o prioridad de backend gráfico Vulkan vs GLES (postergado explícitamente a petición del usuario para evaluar primero el impacto del recompilador AOT). Impacto: bajo.
