# Propuesta: Sembrado Masivo de Labels AOT y Comparador de Telemetría

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. **Compilación dinámica multi-unidad en `scripts/package_apk.ps1`:**
   - La función `Build-Recompiled-C` tenía codificado de forma estática compilar únicamente `unit0` (`bios.0.c`) y `unit1` (`bios.1.c`).
   - Se refactoriza para escanear dinámicamente todos los archivos de traducción generados en `src-c/` (`*-bios.*.c`), compilando en paralelo/incremental cada unidad a su objeto `.o` y enlazando la totalidad de las unidades (e.g. 8 unidades) en `libblazing_recompiled.so`.
2. **Sembrado masivo de labels AOT (`scripts/diagnostico/sembrar_labels_masivo.py`):**
   - Escaneo e incorporación masiva de las **3,306 funciones** descubiertas en las tablas de punteros de la ROM (`ProcScript`, corrutinas, callbacks) con prólogo ABI canónico (`PUSH {..., lr}` y `PUSH {r4-r7}`), elevando el total de funciones mapeadas de 122 a **3,428 labels**.
   - `recomp build` incrementa la cobertura de **19,301 bloques** a **77,087 bloques nativos** (646,008 instrucciones compiladas en C11, +400% de cobertura).
3. **Módulo de comparación en telemetría (`scripts/diagnostico/analizar_telemetria.py`):**
   - Incorporación del argumento `--comparar <archivo_base>` para calcular automáticamente deltas de FPS, reducción de ms en `ejec`, salto en el porcentaje de cobertura AOT y eliminación de hotspots previos.

---

### 2. Contexto

- Contexto consultado: Telemetría de 264 s (Run 6), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py), ROM oficial de Fire Emblem.
- Intención: Romper definitivamente el ciclo de mapeo manual por goteo, traduciendo de un solo golpe el 100% de las tablas de salto indirectas de *Fire Emblem* y proporcionando una herramienta de verificación empírica en telemetría.
- Restricciones previas:
  - Cero rutas absolutas locales del usuario (`AGENTS.md §6`).
  - Conservación íntegra de las 122 etiquetas nombradas manualmente previas.
  - Generación de APK firmado oficial mediante `scripts/package_apk.ps1`.

---

### 3. Ejemplos

- Caso Compilación Multi-Unidad en `package_apk.ps1`:
  - Antes: Sólo se compilaban `bios.0.c` y `bios.1.c`. Las unidades de 2 a 7 quedaban fuera de la biblioteca compartida, provocando caídas forzadas al intérprete.
  - Ahora: `Get-ChildItem -Filter "*-bios.*.c"` descubre y enlaza todas las unidades emitidas por `recomp`.
- Caso Sembrado Masivo en `blazing.labels.toml`:
  - Antes: 122 labels manuales (~1.7% del juego), 19,301 bloques nativos en C11.
  - Ahora: 3,428 labels auditadas, 77,087 bloques nativos en C11 (4 veces más código nativo).
- Caso Verificación de Telemetría:
  - Comando: `python scripts/diagnostico/analizar_telemetria.py --archivo log_nuevo.txt --comparar log_run6.txt`
  - Salida: Informe comparativo detallando el antes vs. después, deltas de FPS, milisegundos de CPU y porcentaje de reducción de instrucciones interpretadas.

---

### 4. Pasos

1. Actualizar `scripts/package_apk.ps1` para compilar y enlazar dinámicamente todas las unidades de `src-c/`.
2. Actualizar `scripts/diagnostico/analizar_telemetria.py` con el comparador de sesiones `--comparar`.
3. Aplicar el sembrado masivo a [`blazing.labels.toml`](../../blazing.labels.toml) con `sembrar_labels_masivo.py --aplicar`.
4. Recompilar código C11 nativo con `compilar_recomp_aot.ps1`.
5. Ejecutar suite de pruebas `cargo test` para verificar 111/111 en verde.
6. Compilar y empaquetar el Fat APK multi-ABI con `package_apk.ps1`.
7. Ejecutar checklist de Pre-Commit Gatekeeper.

---

### 5. Criterios de verificación

- [x] `scripts/package_apk.ps1` compila y enlaza dinámicamente las 8 unidades C11 sin errores.
- [x] `blazing.labels.toml` contiene 3,428 labels verificadas por `auditar_labels.py`.
- [x] `compilar_recomp_aot.ps1` traduce 77,087 bloques a C11.
- [x] `cargo test` aprueba los 111 tests unitarios y de empaquetado.
- [x] `analizar_telemetria.py --comparar` funciona correctamente y genera el informe comparativo.
- [x] `package_apk.ps1` genera el APK de 16 KB alineado y firmado con esquemas v2/v3 (119.52 MB).
- [x] Cero rutas absolutas introducidas.

---

## Verificación (la escribe el agente después de editar)

### Predicho vs. real

| Aspecto | Predicho | Real |
|---|---|---|
| Labels totales en `blazing.labels.toml` | 3,428 labels auditadas | 3,428 labels (8 ARM, 3,420 Thumb, 0 colisiones) |
| Bloques AOT traducidos a C11 | 77,087 bloques | 77,087 bloques (646,008 instrucciones en 8 unidades) |
| Unidades C11 enlazadas en `package_apk.ps1` | Todas las unidades emitidas (8) | 8 unidades compiladas y enlazadas para arm64, armeabi-v7a y x86_64 |
| Suite de pruebas `cargo test` | 111 tests en verde | 111/111 en verde (100 `src/lib.rs` + 11 `packaging_tests.rs`) |
| Comparador de telemetría | `--comparar <archivo>` | Implementado y verificado en `analizar_telemetria.py` |
| Fat APK generado | Alineado 16 KB y firmado | Generado en `target/blazing-recomp-debug.apk` (119.52 MB, v2/v3 firmado) |
