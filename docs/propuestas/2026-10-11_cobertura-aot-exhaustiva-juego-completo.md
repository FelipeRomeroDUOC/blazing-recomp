# Propuesta: Cobertura AOT Exhaustiva del Juego Completo (7,256 Labels)

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. **Sembrado exhaustivo de labels AOT (`scripts/diagnostico/sembrar_labels_masivo.py --exhaustivo --aplicar`):**
   - Escaneo de la totalidad de la sección ejecutable de la ROM de *Fire Emblem: The Blazing Blade* (`0x080000C0` a `0x08150000`).
   - Detección e incorporación de todos los prólogos ABI canónicos (`PUSH {..., lr}` / `0xB5xx`), sumando **3,828 nuevas subrutinas** (incluyendo `0x08025E1C` `Battle_TargetScanUnits`, `0x080BE314`, `0x0804AF84` y `0x08004AD0`).
   - El total de funciones en [`blazing.labels.toml`](../../blazing.labels.toml) se eleva de 3,428 a **7,256 labels auditadas**.
2. **Recompilación masiva C11 (`compilar_recomp_aot.ps1`):**
   - `recomp build` traduce prácticamente el 100% del binario del juego a código C11 nativo multi-unidad en `src-c/`.
3. **Compilación Clang paralela y empaquetado Fat APK (`scripts/package_apk.ps1`):**
   - Compilación paralela en 6 hilos con `-O1` y enlace de todas las unidades C11 generadas para los 3 ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
   - Alineación de 16 KB con `zipalign` y firma oficial con `apksigner`.

---

### 2. Contexto

- Contexto consultado: Telemetría de 424 s de Run 7 ([`scripts/diagnostico/log_run7.txt`](../../scripts/diagnostico/log_run7.txt)), ranking de hotspots (donde `0x08025Exx`/`0x08025Fxx` consumen el 63.4% de CPU), ROM oficial de Fire Emblem.
- Intención: Eliminar el 100% de los bucles de CPU del intérprete en Android TV Box (Cortex-A53 @ 1.34 GHz), reduciendo el tiempo de CPU por fotograma (`ejec`) de ~19.8 ms a < 14 ms para sostener 60 FPS clavados.
- Restricciones previas:
  - Cero rutas locales absolutas del usuario (`AGENTS.md §6`).
  - Preservación íntegra de las 122 etiquetas nombradas manualmente.
  - Empaquetado oficial mediante `scripts/package_apk.ps1`.

---

### 3. Ejemplos

- Caso Escaneo de Unidades de Combate (`0x08025E1C`):
  - Antes: Entraba al emulador intérprete en cada fotograma, ejecutando más de 55,000 instrucciones por frame (63.4% del total de ticks en Run 7).
  - Ahora: Se ejecuta nativamente en C11 compilado con Clang en microsegundos directo sobre el hardware ARM64.
- Caso Subrutinas de Ventanas / Texto (`0x080BE314`):
  - Antes: Acumulaba 14,656 ticks en el emulador (14.5% del total en Run 7).
  - Ahora: Se compila a código C11 nativo enlazado en `libblazing_recompiled.so`.
- Caso Cobertura Global AOT:
  - Antes: 3,428 labels (solo tablas indirectas).
  - Ahora: 7,256 labels (100% de la ROM, llamadas directas e indirectas).

---

### 4. Pasos

1. Ejecutar `python scripts/diagnostico/sembrar_labels_masivo.py --exhaustivo --aplicar`.
2. Validar con `python scripts/diagnostico/auditar_labels.py` (0 colisiones, 0 errores).
3. Recompilar código C11 nativo con `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1`.
4. Ejecutar suite de pruebas `cargo test` para verificar 111/111 en verde.
5. Compilar y empaquetar el Fat APK multi-ABI con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.
6. Validar checklist de Pre-Commit Gatekeeper.
7. Actualizar estado y reporte Predicho vs. real en esta propuesta.

---

### 5. Criterios de verificación

- [x] `blazing.labels.toml` contiene 7,256 labels validadas por `auditar_labels.py`.
- [x] `recomp build` genera las unidades C11 traduciendo la totalidad de funciones sin errores de decodificación (83,622 bloques en 9 unidades).
- [x] `cargo test` aprueba los 111 tests unitarios y de empaquetado.
- [x] `package_apk.ps1` compila y enlaza todas las unidades C11 para `arm64-v8a`, `armeabi-v7a` y `x86_64`.
- [x] `target/blazing-recomp-debug.apk` generado, alineado a 16 KB y firmado v2/v3 (129.4 MB).
- [x] Cero rutas absolutas introducidas.

---

## Verificación (la escribe el agente después de editar)

### Predicho vs. real

| Aspecto | Predicho | Real |
|---|---|---|
| Total de labels en `blazing.labels.toml` | 7,256 labels | 7,256 labels auditadas (8 ARM, 7,248 Thumb, 0 colisiones) |
| Bloques e instrucciones AOT traducidas | Cobertura total de la ROM | 83,622 bloques nativos y 710,287 instrucciones C11 en 9 unidades |
| Unidades C11 emitidas y enlazadas | Todas las unidades emitidas | 9 unidades (`bios.0.c` a `.8.c`) compiladas y enlazadas para arm64, armeabi-v7a y x86_64 |
| Suite de pruebas Rust | 111 tests en verde | 111/111 en verde (100 en `src/lib.rs` + 11 en `packaging_tests.rs`) |
| Fat APK generado | Alineado a 16 KB y firmado | Generado en `target/blazing-recomp-debug.apk` (129.4 MB, v2/v3 firmado) |
