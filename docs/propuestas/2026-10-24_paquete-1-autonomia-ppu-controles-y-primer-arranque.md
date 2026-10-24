# Propuesta: Paquete 1 — Autonomía de compilación, clarificación de PPU, controles y primer arranque

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resuelve el primer bloque de bloqueos y quejas técnicas de `RANT.MD` (quejas 2, 5, 9, 10, 11 y 15): transparenta los requisitos de compilación local pareada frente al repositorio upstream, erradica términos obsoletos ("shim") definiendo con exactitud el comportamiento de `ppu-nativa-off.flag` y su fallback en CPU, documenta los controles reales del visor de PC (`Shift` para turbo y `N` para paso a paso), describe con precisión el flujo de primer arranque en Android con comandos `adb push` (eliminando la promesa del selector SAF descartado), unifica la denominación de la ROM oficial y declara honestamente el estado de la persistencia de guardado.

### 2. Contexto

- Contexto consultado: `RANT.MD`, `README.md`, `Cargo.toml`, `docs/guias/medir-rendimiento.md`, `docs/guias/ejecutar-visor-doble-pc.md`, `docs/guias/compilar-y-empaquetar-apk.md`, `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`, `docs/explicacion/DECISIONES.md` (ADR 4, ADR 22, ADR 42), `docs/explicacion/roadmap-y-versionado.md`, `docs/PENDIENTES.md`, `docs/BACKLOG.md`, `src/bin/ppu_doble.rs`, `src/platform.rs`, `AGENTS.md`.
- Intención: Eliminar la desorientación técnica del lector externo, permitiéndole entender con exactitud qué se necesita para compilar, cómo funciona la conmutación de la PPU, cómo operar los controles interactivos y cómo alimentar de ROM/BIOS al APK sin promesas de selectores inexistentes.
- Restricciones y decisiones previas (máx. 3):
  - Prohibición de rutas locales absolutas en archivos versionados (`AGENTS.md` §7) → implica: todas las rutas y comandos deben ser relativos al repositorio o a rutas estándar de Android (`/sdcard/Download/`) → protegida por: `Verificación de ausencia de rutas locales absolutas`.
  - Integridad de la suite de pruebas sin regresiones (`AGENTS.md` §6) → implica: los 250 tests existentes deben mantenerse al 100% en verde → protegida por: `La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests`.
  - Conformidad Diátaxis sin errores (`validar_docs.py`) → implica: preservar encabezados canónicos y metadatos en las páginas de guías y explicación modificadas → protegida por: `Todas las páginas modificadas cumplen el estándar Diátaxis`.
- Alternativas descartadas:
  - Eliminar por completo el bloque `[patch]` de `Cargo.toml` inmediatamente → descartada porque rompería la compilación local del visor `ppu_doble` (que requiere las extensiones `perf-counters`, `perf-video` y captura LUT vertical presentes en el worktree `../gba-recomp`). En su lugar, se documenta de forma explícita y honesta el requisito de la carpeta hermana en `README.md` y guías, dejando el `[patch]` debidamente contextualizado hasta que los cambios del motor sean publicados upstream.
  - Ocultar que el guardado `.sav` no persiste en disco → descartada porque engañaría a quien intente jugar capítulos largos creyendo que puede cerrar la app sin perder el progreso.
- Ante imprevistos, prioriza: máxima honestidad y exactitud técnica sobre mantener promesas históricas no implementadas.

### 3. Ejemplos

- Caso normal 1 (Conmutación de PPU en `medir-rendimiento.md`):
  - Entrada: El desarrollador lee la sección para comparar A/B en el teléfono.
  - Antes: "Para volver al shim —y comparar A/B en el propio dispositivo— se crea un archivo de marca..." (incompatible con la eliminación de `PpuShim` del ADR 42).
  - Ahora: "Para desactivar la PPU nativa en GPU y conmutar en caliente al rasterizador por software en CPU provisto por `gba-core` —y comparar A/B en el propio dispositivo— se crea el archivo de marca `ppu-nativa-off.flag`...".
- Caso normal 2 (Controles del visor en `README.md`):
  - Entrada: El desarrollador consulta los atajos de teclado para analizar un fotograma.
  - Antes: "`N` / `Shift+N` (avance paso a paso de fotogramas)" (inexistente; `Shift` activa el limitador turbo).
  - Ahora: "`Espacio` (pausa / reanudar), `N` (avance paso a paso de un fotograma estando en pausa), `Shift` (modo turbo sin límite de 60 FPS mientras se mantenga presionado)".
- Caso normal 3 (Primer arranque del APK en `compilar-y-empaquetar-apk.md`):
  - Entrada: El usuario instala el APK en su dispositivo Android y busca cómo suministrar la ROM.
  - Antes: "En el primer arranque, la aplicación mostrará la pantalla de selección o carga de ROM/BIOS..." (promete selector SAF inexistente).
  - Ahora: Se describe la pantalla de diagnóstico que lista los hashes esperados y se proporcionan los comandos exactos `adb push "Fire Emblem (Europe).gba" /sdcard/Download/` y `adb push gba_bios.bin /sdcard/Download/`.
- Caso límite 1 (Transparencia en persistencia de guardado):
  - Entrada: El jugador revisa el `README.md` y `roadmap-y-versionado.md` para saber si puede cerrar el juego.
  - Antes: El roadmap dice "MVP Jugable: ... completamente jugable de principio a fin con controles y guardado" mientras `PENDIENTES.md` declara la persistencia `.sav` pendiente.
  - Ahora: Se declara explícitamente en el estado del port: "Persistencia de partidas (`.sav`): En desarrollo (la SRAM opera en memoria volátil durante la sesión, pero no se sincroniza a archivo persistente en disco al cerrar la app)".
- Caso límite 2 (Relación de nombres y hash de la ROM):
  - Entrada: El usuario posee un volcado con nombre distinto al del README.
  - Antes: Discrepancia entre `Fire Emblem (Europe)` y `1575 - Fire Emblem (E)(Independent).gba` sin explicar si son variantes distintas.
  - Ahora: Se aclara que el nombre canónico es `Fire Emblem (Europe)` y el volcado de referencia No-Intro es `1575 - Fire Emblem (E)(Independent).gba`, pero que el runtime valida exclusivamente el hash SHA-256 (`6f2c3082...`), aceptando cualquier nombre de archivo que lo contenga.

### 4. Pasos

1. Actualizar `docs/guias/medir-rendimiento.md`, `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md` y el ADR 42 en `docs/explicacion/DECISIONES.md` erradicando cualquier uso residual del término "shim", definiendo que `ppu-nativa-off.flag` conmuta en caliente al rasterizador por software en CPU de `gba-core`, y clarificando que lo descartado en el ADR 42 fue la ejecución concurrente continua de ambos motores gráficos en producción.
2. Actualizar `README.md` y `docs/guias/ejecutar-visor-doble-pc.md` documentando la topología de repositorios pareados: explicar que para ejecutar localmente el visor `ppu_doble` se requiere la carpeta hermana `../gba-recomp` vinculada mediante `[patch]` en `Cargo.toml` debido a las extensiones de telemetría y captura de LUT vertical aún no fusionadas en el upstream oficial.
3. Corregir los atajos de teclado en `README.md` sincronizándolos fielmente con `src/bin/ppu_doble.rs` (`Espacio` para pausar, `N` para paso a paso, `Shift` para modo turbo, `L` para alternar referencia, `Tab` para diferencias, `F5` para captura, `Esc` para salir).
4. Actualizar `docs/guias/compilar-y-empaquetar-apk.md` en su sección de primer arranque: erradicar la promesa del selector visual SAF, detallar el comportamiento de la pantalla de diagnóstico ante la ausencia de archivos y documentar los comandos `adb push` para copiar la ROM y BIOS a `/sdcard/Download/` o al directorio interno de la app.
5. Sincronizar en `README.md`, `docs/explicacion/roadmap-y-versionado.md` y `GLOSSARY.md` el estado real de la persistencia de guardado (SRAM volátil en RAM, guardado en disco `.sav` pendiente) y unificar la denominación de la ROM con su hash criptográfico.
6. Sanear los enlaces rotos de los ADRs que intentaban resolver rutas hacia fuera del repositorio (`../../../gba-recomp/...`), reemplazándolos por referencias canónicas relativas al crate o descripciones de módulo.

### 5. Qué más podría verse afectado

- `tests/packaging_tests.rs`: Ejecuta `test_documentos_sin_afirmaciones_caducas` (valida ausencia de frases obsoletas en `DOCS_ESTADO_ACTUAL`), `test_no_local_paths_in_packaging` (ausencia de rutas locales) y `test_propuestas_declaran_estado_valido`.
- El validador Diátaxis (`validar_docs.py`): Revisa las 22 páginas de documentación.

### 6. Suposiciones

- Suposición 1: El desarrollador que ejecuta `cargo run --bin ppu_doble` en PC dispone del clon de `gba-recomp` en `../gba-recomp` tal como lo venía requiriendo el proyecto. → prototipar: no → si es falso: la documentación advertirá claramente la necesidad de clonarlo antes de intentar compilar el visor.
- Suposición 2: Ningún script de CI o test automatizado depende de la frase "volver al shim" (verificado: no existe en tests). → prototipar: no → si es falso: actualizar el test correspondiente.

### 7. Alcance

- Incluye:
  - Corrección de las quejas 2, 5, 9, 10, 11 y 15 de `RANT.MD`.
  - Clarificación del comportamiento de `ppu-nativa-off.flag` y erradicación del término "shim".
  - Documentación honesta de los requisitos de compilación de PC y topología de dependencias.
  - Sincronización exacta de controles del visor interactivo.
  - Documentación del primer arranque real en Android con comandos `adb push`.
  - Transparencia en la denominación de la ROM y el estado del guardado `.sav`.
- No incluye:
  - Mapeo de estados vigentes/sustituidos de los 46 ADRs (reservado para el Paquete 2).
  - Corrección matemática de líneas de `src-c/`, justificación de offsets de `anatomia-motor-fire-emblem-7.md` y reestructuración Diátaxis de `medir-rendimiento.md` (reservados para el Paquete 3).
  - Implementación de código nuevo para el guardado persistente o selector SAF en Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La documentación de la PPU y telemetría no utiliza el término shim para referirse a la ruta de fallback
Caso de uso: Desarrollador consulta la guía de rendimiento para conmutar la PPU | Tipo: normal
  Dado el archivo `docs/guias/medir-rendimiento.md`
  Cuando se busca la palabra "shim" en la sección de conmutación de PPU
  Entonces no existe ninguna ocurrencia de "volver al shim" y se describe explícitamente la conmutación al rasterizador por software en CPU de `gba-core`.
Test: `test_documentos_sin_afirmaciones_caducas` / inspección textual
Estado previo: pasa (se fortalece con validación estricta)

Escenario: Los controles del visor interactivo en README coinciden con la implementación de ppu_doble
Caso de uso: Usuario ejecuta el visor en PC y utiliza atajos de teclado | Tipo: normal
  Dado el archivo `README.md`
  Cuando se consulta la sección de controles del visor
  Entonces documenta `Espacio` para pausar, `N` para paso a paso y `Shift` para modo turbo, erradicando la combinación errónea `Shift+N`.
Test: inspección textual y concordancia con `src/bin/ppu_doble.rs`
Estado previo: falla (hoy `README.md` contiene `Shift+N (avance paso a paso)`)

Escenario: La guía de APK documenta comandos adb push y no promete selector SAF
Caso de uso: Usuario instala el APK y copia los archivos legales | Tipo: normal
  Dado el archivo `docs/guias/compilar-y-empaquetar-apk.md`
  Cuando se consulta la sección de primer arranque
  Entonces contiene los comandos `adb push` hacia `/sdcard/Download/` y no afirma que la app mostrará un selector de archivos visual SAF.
Test: inspección textual
Estado previo: falla (hoy afirma que "mostrará la pantalla de selección o carga de ROM/BIOS")

Escenario: La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests
Caso de uso: Verificación de no regresión | Tipo: regresión
  Dado el árbol de trabajo tras la sincronización documental
  Cuando se ejecuta `cargo test`
  Entonces los 250 tests (172 unitarios y 78 de integración) pasan en verde sin fallos.
Test: `cargo test`
Estado previo: pasa (250 tests en verde)

Escenario: Verificación de ausencia de rutas locales absolutas
Caso de uso: Auditoría de portabilidad y privacidad | Tipo: regresión
  Dado cualquier archivo modificado en este cambio
  Cuando se auditan rutas en el repositorio
  Entonces no se detectan rutas absolutas del entorno local del usuario.
Test: `tests/packaging_tests.rs::test_no_local_paths_in_packaging`
Estado previo: pasa

### 9. Cómo lo comprobaré

- [La documentación de la PPU no usa "volver al shim"] → herramienta: `Select-String -Path "docs/guias/medir-rendimiento.md" -Pattern "volver al shim"` → pasa si: 0 coincidencias → no detecta: erratas en otros archivos no auditados.
- [Controles de visor en README coinciden con código] → herramienta: inspección manual frente a `src/bin/ppu_doble.rs` → pasa si: `Shift` figura como turbo y `N` como paso a paso → no detecta: atajos que no estén implementados en Windows.
- [Guía de APK documenta adb push y no selector SAF] → herramienta: `Select-String -Path "docs/guias/compilar-y-empaquetar-apk.md" -Pattern "adb push"` → pasa si: contiene los comandos exactos de push → no detecta: diferencias de permisos en versiones antiguas de Android.
- [Suite de tests en verde con 250 tests] → herramienta: `cargo test` → pasa si: `test result: ok. 172 passed` en unitarios y `ok. 78 passed` en integración → no detecta: comportamiento visual exclusivo del dispositivo móvil.
- [Ausencia de rutas locales absolutas] → herramienta: `cargo test --test packaging_tests test_no_local_paths_in_packaging` → pasa si: código 0 → no detecta: rutas ofuscadas en binarios.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

### 1. Predicho vs. real

| Predicción | Resultado real | Estado |
| :--- | :--- | :---: |
| Erradicar "volver al shim" y clarificar que `ppu-nativa-off.flag` conmuta en caliente al rasterizador por software en CPU de `gba-core`. | Modificado en [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md), [`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`](../explicacion/hardware-shim-y-eliminacion-de-emulacion.md) y ADR 42 de [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md). 0 ocurrencias residuales. | Conforme |
| Documentar los requisitos del repositorio hermano `../gba-recomp` y dependencias de PC. | Añadida nota de arquitectura y dependencias en [`README.md`](../../README.md) y en los requisitos de [`docs/guias/ejecutar-visor-doble-pc.md`](../guias/ejecutar-visor-doble-pc.md). | Conforme |
| Sincronizar atajos de teclado del visor interactivo en `README.md`. | Corregido en [`README.md`](../../README.md) (`Espacio` pausa, `N` paso a paso, `Shift` turbo). Eliminada la combinación errónea `Shift+N`. | Conforme |
| Documentar el primer arranque real en Android con comandos `adb push` y pantalla informativa. | Sección 3 y 4 actualizadas en [`docs/guias/compilar-y-empaquetar-apk.md`](../guias/compilar-y-empaquetar-apk.md), con rutas canónicas y comandos `adb push`. Eliminada la promesa del selector SAF. | Conforme |
| Declarar con transparencia el estado de la persistencia de guardado y unificar nombres de ROM. | Añadido estado honesto en [`README.md`](../../README.md) y [`docs/explicacion/roadmap-y-versionado.md`](../explicacion/roadmap-y-versionado.md): SRAM en memoria volátil en `0.0.2`, persistencia `.sav` en desarrollo para `0.1.0`. | Conforme |
| Sanear enlaces que apuntaban fuera del repositorio en ADRs 34 a 36. | Reemplazados los enlaces rotos hacia `../../../gba-recomp/...` por referencias textuales al crate en [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md). | Conforme |
| Suite completa de 250 tests de Rust en verde. | `cargo test` ejecutado: 172 tests unitarios y 78 de integración aprobados (250 tests en verde). | Conforme |
| Validación Diátaxis sin errores ni avisos. | `validar_docs.py` ejecutado: 22 páginas revisadas, 0 errores, 0 avisos. | Conforme |

### 2. Qué se respetó del contexto

- Se respetó la prohibición de rutas locales absolutas (`AGENTS.md` §7): verificado con `cargo test --test packaging_tests test_no_local_paths_in_packaging`.
- Se mantuvo la integridad de la suite de pruebas sin debilitar asserts: los 250 tests pasan en verde sin fallos.
- Se respetó el estándar Diátaxis en las páginas de guías y explicaciones modificadas: 0 errores y 0 avisos en `validar_docs.py`.

### 3. Fuera de alcance (hallazgos que no se arreglaron)

- **Mapeo de estados vigentes/sustituidos en los 46 ADRs de `DECISIONES.md` (Bloqueo 3 de `RANT.MD`):** Reservado para el Paquete 2. — impacto: medio
- **Verificabilidad metodológica y direcciones de ROM/labels en `anatomia-motor-fire-emblem-7.md` (Bloqueo 4):** Reservado para el Paquete 3. — impacto: medio
- **Aritmética de líneas de `src-c/` en `arquitectura-c-recompilado.md` (Queja 14):** Reservado para el Paquete 3. — impacto: bajo
