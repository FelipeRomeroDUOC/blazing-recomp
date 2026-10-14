# Backlog del Proyecto — Blazing Recomp

Hoja de ruta, tareas pendientes y seguimiento de hitos para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**.

> **Este es el tablero vivo del proyecto:** qué falta, qué se sabe y dónde está anotado.
> No es una página Diátaxis — el validador lo excluye por nombre —, así que no lleva front-matter.
> Los hallazgos que aún no se han arreglado viven en el [registro de hallazgos](#registro-de-hallazgos),
> y su coherencia con las propuestas la comprueba `python scripts/auditar_hallazgos.py`.

**Última actualización:** 2026-10-14

---

## Estado del Proyecto

- **Versión:** `0.0.2` / `versionCode` 2 ([`Cargo.toml`](../Cargo.toml), [`android/AndroidManifest.xml`](../android/AndroidManifest.xml))
- **Hito del [roadmap](explicacion/roadmap-y-versionado.md):** **Fase 0 (Cimientos, `0.0.x`)**, camino a
  `0.1.0` (MVP jugable de principio a fin).
- **ROM Objetivo:** `Fire Emblem (Europe)` — `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS Objetivo:** `gba_bios.bin` — `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Estado de la suite:** 131 tests en verde (109 en los módulos satélite de `src/`, 18 en
  [`tests/packaging_tests.rs`](../tests/packaging_tests.rs), 4 en
  [`tests/golden_frame_tests.rs`](../tests/golden_frame_tests.rs)). El checklist de pre-commit vive en
  [`AGENTS.md`](../AGENTS.md) §6.

---

## Leyenda de Estados

Los estados de las propuestas, y qué significa cada uno, están especificados en
[`docs/propuestas/README.md`](propuestas/README.md). En resumen: `pendiente` → `aprobada` →
`verificada por agente` → `revisada por humano`, con `discrepada`, `cerrada`, `modificada` y
`descartada` como salidas laterales. Un test
([`test_propuestas_declaran_estado_valido`](../tests/packaging_tests.rs)) impide que una propuesta nueva
llegue sin estado válido.

---

## Áreas de Trabajo y Estado Real

Las áreas son de trabajo, no las fases del roadmap: una misma área puede repartirse entre dos versiones.

### 1. Cimientos, entorno y empaquetado — ✅ completo

- Repositorio, [`AGENTS.md`](../AGENTS.md) y [skills](../.agents/skills/) del proyecto.
- Entorno: NDK `30.0.16248370`, Rust `1.99.0` con los targets Android, `cargo-ndk` v4.1.2.
- Crate nativo Android ([`2026-10-08_crate-nativo-android.md`](propuestas/2026-10-08_crate-nativo-android.md))
  y arnés de empaquetado Fat APK sin Gradle
  ([`2026-10-09_arnes-empaquetado-apk.md`](propuestas/2026-10-09_arnes-empaquetado-apk.md)).
- Empaquetado oficial: `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
  (tres ABIs, `aapt2`, alineación a 16 KB, firma v1/v2/v3).

### 2. Recompilación AOT y cobertura — ✅ completo, con deuda abierta

- Pipeline AOT a C11 y despacho nativo
  ([`2026-10-10_integracion-codigo-c-recompilado-aot.md`](propuestas/2026-10-10_integracion-codigo-c-recompilado-aot.md)).
- Cobertura por cosecha de labels en 5 oleadas (11-oct) hasta **144.875 bloques** traducidos.
- Ejecución con BIOS real ([`2026-10-12_ejecucion-nativa-de-bios-real.md`](propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)).
- **Deuda:** `recomp runc` carga 17.207 bloques y `src-c/` tiene 144.875 símbolos (hallazgo registrado);
  y el intérprete de respaldo todavía ejecuta funciones de combate sin recompilar.

### 3. Renderizado, presentación y overlay táctil — 🟨 casi completo

- Compositor GPU `wgpu` (Vulkan y OpenGL ES 3.0), escalado de aspecto 3:2 con píxeles nítidos,
  shaders LCD, overlay táctil procedural por SDF y pantalla de diagnóstico de emergencia.
- Triple buffering y latencia de swapchain a 2 fotogramas
  ([`2026-10-10_triple-buffering-swapchain-latencia-2.md`](propuestas/2026-10-10_triple-buffering-swapchain-latencia-2.md)).
- **Abierto:** la calibración de presentación está **activa** por el experimento
  [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md),
  que espera una sesión en el televisor. El techo de `present` en tablet (51,4 ms) solo bajó con
  resolución reducida (2,9 ms), descartada por nitidez.

### 4. Entrada y controles — 🟨 falta lo físico

- Táctil de Fase 5.1 y 5.2 completos: D-Pad, clúster GBA SP, háptica, geometría adaptativa por
  orientación y overlay dibujado por shaders.
- **Falta:** mandos físicos Bluetooth / USB-C y auto-desactivación del táctil.

### 5. Rendimiento del fotograma — 🟨 el frente activo

- PPU: desglose por partes, camino especializado por tramos, alpha real y efectos inertes
  (propuestas del 11 al 13-oct). Telemetría sin coste en el APK y contadores de camino.
- **Tres palancas cerradas con medida:** el acceso a memoria sin llamada indirecta
  ([`discrepada`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md), ≈3 %), los registros
  del juego en la CPU ([`discrepada`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md),
  −0,9 %) y el reloj por instrucción
  ([`verificada`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md), 0,13-0,41 %).
- **Lo que sigue vivo, por prioridad:** los **6,00 ms de despacho y lógica de juego** por fotograma
  (el bloque más alto, sin tocar) y la instrumentación que viaja en el APK (**10,4 %**). Ambos, con su
  origen, en el registro de abajo.

### 6. Audio — ⬜ sin empezar

- Salida nativa AAudio / Oboe de baja latencia, motor M4A/MP2K acoplado al *shadow mixer* y gestión de
  foco de audio. Hoy el APU se calcula a 65.536 Hz sin salida de audio (hallazgo registrado).

### 7. Almacenamiento, partidas y distribución — 🟨 a medias

- Ingestión de ROM/BIOS con verificación SHA-256 desde `Downloads/`
  ([`2026-10-10_ingestion-rom-bios-downloads.md`](propuestas/2026-10-10_ingestion-rom-bios-downloads.md))
  y almacenamiento compatible con Android 16.
- **Falta:** persistencia de partidas (`.sav`), menú en juego (pausa, controles, audio, filtros).

---

## Prioridad: lo alto que sigue vivo

Curado a mano a partir del [registro](#registro-de-hallazgos); el registro es la fuente, esto es el orden.

1. **6,00 ms de despacho y lógica de juego por fotograma** — el bloque más grande, por delante de
   cualquier etapa de la PPU. Origen:
   [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md).
2. **Solo el 5-17 % de los ciclos del fotograma pasan por el código nativo** — el resto lo avanza el
   *halt*; es el hueco más grande del presupuesto y el único camino de reloj sin instrumentar. Origen:
   [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md).
3. **La instrumentación que viaja en el APK cuesta 10,4 %** (`perf-counters` activo en
   [`Cargo.toml`](../Cargo.toml)). Origen:
   [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md).
4. **La validación en el dispositivo queda pendiente** en todas las propuestas de PPU y presentación.
5. **El cambio del motor sigue sin commitear** — el APK lo lleva por `[patch]`, así que un clon limpio
   no reproduce el APK (registrado en dos propuestas).
6. **La espera activa del bucle principal** contradice la regla «sin busy-waiting» de
   [`AGENTS.md`](../AGENTS.md) §2.5.
7. **Un test intermitente** (`test_emulation_worker_lifecycle_and_frame_generation`, 8 de cada 10
   corridas falla ya en HEAD): el «131 en verde» depende de la suerte.

---

## Índice de Propuestas

58 propuestas en [`docs/propuestas/`](propuestas/). Se listan por estado; la ficha de cada una tiene su
propio `Resultado` (predicho vs. real) y su `Revisión humana`.

**58 = 31 `verificada por agente` + 11 `cerrada` + 7 `aprobada` + 3 `pendiente` + 3 `discrepada` +
3 `revisada por humano`.**

### Pendientes (3) — sin ejecutar

- [`2026-10-10_calibracion-multidispositivo-v2.md`](propuestas/2026-10-10_calibracion-multidispositivo-v2.md)
- [`2026-10-10_optimizacion-compatibilidad-multidispositivo.md`](propuestas/2026-10-10_optimizacion-compatibilidad-multidispositivo.md)
- [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)

### Aprobadas sin cerrar (7) — alcance pactado, trabajo o medida pendientes

- [`2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`](propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md)
- [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md)
- [`2026-10-13_alpha-real-por-tramos.md`](propuestas/2026-10-13_alpha-real-por-tramos.md)
- [`2026-10-13_contadores-de-camino-de-la-ppu.md`](propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)
- [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)
- [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)
- [`2026-10-14_saneamiento-de-la-documentacion.md`](propuestas/2026-10-14_saneamiento-de-la-documentacion.md)

### Discrepadas (3) — la medida contradijo la premisa

- [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md) — el acceso directo al bus no rinde ≈3 %.
- [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md) — la sonda del port mide otro reloj del que se quería medir.
- [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md) — localizar los registros no ahorra: el volcado de frontera lo devuelve.

### Revisada por humano (3)

- [`2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md`](propuestas/2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md)
- [`2026-10-09_optimizacion-latencia-y-fillrate-gpu-modesta.md`](propuestas/2026-10-09_optimizacion-latencia-y-fillrate-gpu-modesta.md)
- [`2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md`](propuestas/2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md)

### Cerradas (11)

- [`2026-10-09_ajustes-fidelidad-ergonomia-overlay.md`](propuestas/2026-10-09_ajustes-fidelidad-ergonomia-overlay.md)
- [`2026-10-09_auditoria-tests-de-mutacion.md`](propuestas/2026-10-09_auditoria-tests-de-mutacion.md)
- [`2026-10-09_separacion-cuencas-y-orientacion-overlay.md`](propuestas/2026-10-09_separacion-cuencas-y-orientacion-overlay.md)
- [`2026-10-09_tests-casos-limite-y-estres.md`](propuestas/2026-10-09_tests-casos-limite-y-estres.md)
- [`2026-10-09_tests-robustez-y-cobertura-funciones-criticas.md`](propuestas/2026-10-09_tests-robustez-y-cobertura-funciones-criticas.md)
- [`2026-10-10_contador-fps-pantalla.md`](propuestas/2026-10-10_contador-fps-pantalla.md)
- [`2026-10-10_fix-pantalla-negra-y-almacenamiento-android16.md`](propuestas/2026-10-10_fix-pantalla-negra-y-almacenamiento-android16.md)
- [`2026-10-10_ingestion-rom-bios-downloads.md`](propuestas/2026-10-10_ingestion-rom-bios-downloads.md)
- [`2026-10-10_integracion-codigo-c-recompilado-aot.md`](propuestas/2026-10-10_integracion-codigo-c-recompilado-aot.md)
- [`2026-10-10_pacing-fotogramas-y-pipeline-video-gba.md`](propuestas/2026-10-10_pacing-fotogramas-y-pipeline-video-gba.md)
- [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)

### Verificadas por agente (31) — con lo medido dentro; varias esperan el televisor

- [`2026-10-08_controles-tactiles-haptica.md`](propuestas/2026-10-08_controles-tactiles-haptica.md)
- [`2026-10-08_crate-nativo-android.md`](propuestas/2026-10-08_crate-nativo-android.md)
- [`2026-10-08_renderizado-gpu-android.md`](propuestas/2026-10-08_renderizado-gpu-android.md)
- [`2026-10-09_arnes-empaquetado-apk.md`](propuestas/2026-10-09_arnes-empaquetado-apk.md)
- [`2026-10-09_correccion-escala-y-renderizado-overlay.md`](propuestas/2026-10-09_correccion-escala-y-renderizado-overlay.md)
- [`2026-10-09_fidelidad-visual-overlay-prototipo.md`](propuestas/2026-10-09_fidelidad-visual-overlay-prototipo.md)
- [`2026-10-09_manejo-errores-y-compatibilidad-emulador.md`](propuestas/2026-10-09_manejo-errores-y-compatibilidad-emulador.md)
- [`2026-10-09_renderizado-overlay-tactil.md`](propuestas/2026-10-09_renderizado-overlay-tactil.md)
- [`2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md`](propuestas/2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md)
- [`2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md`](propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md)
- [`2026-10-10_medicion-rendimiento-por-fase.md`](propuestas/2026-10-10_medicion-rendimiento-por-fase.md)
- [`2026-10-10_separacion-hilos-emulacion-y-renderizado.md`](propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md)
- [`2026-10-10_triple-buffering-swapchain-latencia-2.md`](propuestas/2026-10-10_triple-buffering-swapchain-latencia-2.md)
- [`2026-10-11_cobertura-aot-exhaustiva-juego-completo.md`](propuestas/2026-10-11_cobertura-aot-exhaustiva-juego-completo.md)
- [`2026-10-11_cobertura-aot-hotspots-segunda-oleada.md`](propuestas/2026-10-11_cobertura-aot-hotspots-segunda-oleada.md)
- [`2026-10-11_cobertura-aot-hotspots-vram-y-entidades.md`](propuestas/2026-10-11_cobertura-aot-hotspots-vram-y-entidades.md)
- [`2026-10-11_cobertura-aot-ia-eventos-y-cursor-tvbox.md`](propuestas/2026-10-11_cobertura-aot-ia-eventos-y-cursor-tvbox.md)
- [`2026-10-11_cobertura-aot-jump-tables-y-scheduler-tvbox.md`](propuestas/2026-10-11_cobertura-aot-jump-tables-y-scheduler-tvbox.md)
- [`2026-10-11_cosecha-exhaustiva-leafs-y-switch-tables-run12.md`](propuestas/2026-10-11_cosecha-exhaustiva-leafs-y-switch-tables-run12.md)
- [`2026-10-11_cosecha-jump-tables-leafs-aot.md`](propuestas/2026-10-11_cosecha-jump-tables-leafs-aot.md)
- [`2026-10-11_desglose-ppu.md`](propuestas/2026-10-11_desglose-ppu.md)
- [`2026-10-11_optimizacion-tvbox-escalado-y-hotspots-aot.md`](propuestas/2026-10-11_optimizacion-tvbox-escalado-y-hotspots-aot.md)
- [`2026-10-11_sembrado-masivo-aot-y-verificacion-telemetria.md`](propuestas/2026-10-11_sembrado-masivo-aot-y-verificacion-telemetria.md)
- [`2026-10-11_suite-diagnostico-y-analisis-aot.md`](propuestas/2026-10-11_suite-diagnostico-y-analisis-aot.md)
- [`2026-10-12_compresion-librerias-nativas-apk.md`](propuestas/2026-10-12_compresion-librerias-nativas-apk.md)
- [`2026-10-12_drain-hotspots-sin-perdida.md`](propuestas/2026-10-12_drain-hotspots-sin-perdida.md)
- [`2026-10-12_ejecucion-nativa-de-bios-real.md`](propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)
- [`2026-10-12_nombre-apk-segun-perfil.md`](propuestas/2026-10-12_nombre-apk-segun-perfil.md)
- [`2026-10-13_arnes-de-fotogramas-dorados.md`](propuestas/2026-10-13_arnes-de-fotogramas-dorados.md)
- [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)
- [`2026-10-14_medir-la-frontera-entre-bloques.md`](propuestas/2026-10-14_medir-la-frontera-entre-bloques.md)

### Esperando el televisor: 28 propuestas con `Revisión humana` pendiente

Todas piden lo mismo: una sesión en el dispositivo. Es el cuello de botella del proyecto ahora mismo.

- [`2026-10-09_ajustes-fidelidad-ergonomia-overlay.md`](propuestas/2026-10-09_ajustes-fidelidad-ergonomia-overlay.md)
- [`2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md`](propuestas/2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md)
- [`2026-10-09_correccion-escala-y-renderizado-overlay.md`](propuestas/2026-10-09_correccion-escala-y-renderizado-overlay.md)
- [`2026-10-09_fidelidad-visual-overlay-prototipo.md`](propuestas/2026-10-09_fidelidad-visual-overlay-prototipo.md)
- [`2026-10-09_manejo-errores-y-compatibilidad-emulador.md`](propuestas/2026-10-09_manejo-errores-y-compatibilidad-emulador.md)
- [`2026-10-09_separacion-cuencas-y-orientacion-overlay.md`](propuestas/2026-10-09_separacion-cuencas-y-orientacion-overlay.md)
- [`2026-10-10_calibracion-multidispositivo-v2.md`](propuestas/2026-10-10_calibracion-multidispositivo-v2.md)
- [`2026-10-10_contador-fps-pantalla.md`](propuestas/2026-10-10_contador-fps-pantalla.md)
- [`2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md`](propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md)
- [`2026-10-10_medicion-rendimiento-por-fase.md`](propuestas/2026-10-10_medicion-rendimiento-por-fase.md)
- [`2026-10-10_separacion-hilos-emulacion-y-renderizado.md`](propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md)
- [`2026-10-12_compresion-librerias-nativas-apk.md`](propuestas/2026-10-12_compresion-librerias-nativas-apk.md)
- [`2026-10-12_drain-hotspots-sin-perdida.md`](propuestas/2026-10-12_drain-hotspots-sin-perdida.md)
- [`2026-10-12_ejecucion-nativa-de-bios-real.md`](propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)
- [`2026-10-12_nombre-apk-segun-perfil.md`](propuestas/2026-10-12_nombre-apk-segun-perfil.md)
- [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md)
- [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md)
- [`2026-10-13_alpha-real-por-tramos.md`](propuestas/2026-10-13_alpha-real-por-tramos.md)
- [`2026-10-13_arnes-de-fotogramas-dorados.md`](propuestas/2026-10-13_arnes-de-fotogramas-dorados.md)
- [`2026-10-13_contadores-de-camino-de-la-ppu.md`](propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)
- [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)
- [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)
- [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)
- [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)
- [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md)
- [`2026-10-14_medir-la-frontera-entre-bloques.md`](propuestas/2026-10-14_medir-la-frontera-entre-bloques.md)
- [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)
- [`2026-10-14_saneamiento-de-la-documentacion.md`](propuestas/2026-10-14_saneamiento-de-la-documentacion.md)

---

## Registro de Hallazgos

#### [`2026-10-08_controles-tactiles-haptica.md`](propuestas/2026-10-08_controles-tactiles-haptica.md)

- Renderizado visual con shaders/texturas de los botones y paleta de colores GBA SP (Fase 5.2) — impacto: bajo
- Calibración de `VibrationEffect` para Android 8.0+ (API 26+) en sustitución del método deprecado `Vibrator.vibrate(long)`. — impacto: bajo

#### [`2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md`](propuestas/2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md)

- Banner gráfico personalizado para Leanback de 320x180 px. — impacto: bajo

#### [`2026-10-09_manejo-errores-y-compatibilidad-emulador.md`](propuestas/2026-10-09_manejo-errores-y-compatibilidad-emulador.md)

- Selector interactivo de archivos de ROM mediante interfaz gráfica integrada en la pantalla de error. — impacto: bajo

#### [`2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md`](propuestas/2026-10-10_fix-carga-aot-dladdr-y-retorno-vulkan.md)

- Elevación de `desired_maximum_frame_latency` a 2 (postergado para una propuesta independiente de swapchain tras validar primero el impacto del recompilador nativo en el dispositivo). Impacto: bajo. — impacto: sin declarar

#### [`2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md`](propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md)

- `pipeline::tests::test_emulation_worker_lifecycle_and_frame_generation` es intermitente por límite de reloj (500 ms) bajo carga concurrente; es **pre-existente** (el HEAD sin cambios falla 8/10 corridas; el árbol con cambios, 2/5). No se modificó su aserción. — impacto: medio
- `wgpu-hal` no expone `preTransform`: la rotación de retrato la ejecuta la plataforma en cada fotograma y no es corregible desde el port sin parchear el hal. — impacto: medio
- La cabecera `modo=` de `blazing-perf.txt` puede etiquetar `INT` aunque haya tabla nativa, si el primer informe (t=1 s) llega antes del primer fotograma. — impacto: bajo
- La referencia técnica generada (`docs/referencia/api-modulos-rust.md`) cubre solo una lista curada de módulos en `scripts/extraer_referencias.py`; el nuevo `src/present.rs` no aparece en ella aunque `--check` pasa. — impacto: bajo
- La sonda de escala 0,5× solo **mide**: bajar la resolución del swapchain de forma permanente está descartado por nitidez en `docs/propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`; si el dato mostrara una ganancia decisiva habría que reabrir esa decisión aparte. — impacto: bajo

#### [`2026-10-10_integracion-codigo-c-recompilado-aot.md`](propuestas/2026-10-10_integracion-codigo-c-recompilado-aot.md)

- Reordenamiento o prioridad de backend gráfico Vulkan vs GLES (postergado explícitamente a petición del usuario para evaluar primero el impacto del recompilador AOT). Impacto: bajo. — impacto: sin declarar

#### [`2026-10-10_medicion-rendimiento-por-fase.md`](propuestas/2026-10-10_medicion-rendimiento-por-fase.md)

- La espera activa del bucle principal (`poll_events` con `Duration::ZERO`, `src/platform.rs` líneas 1567-1575) contradice la regla "sin busy-waiting" de `AGENTS.md` §2.5. — impacto: medio
- Cada acceso a memoria del código recompilado llama de vuelta a Rust (`a->read32`, `a->write32` en el C generado por `crates/recomp-core/src/emit.rs` del fork) — impacto: medio
- El audio se calcula 65.536 veces por segundo aunque la app no tiene salida de audio (`pump_audio` en `crates/gba-core/src/mem.rs` del fork) — impacto: bajo
- `AGENTS.md` §6 indica 63 tests; el código tiene 77. — impacto: bajo
- `scripts/extraer_referencias.py --check` y `validar_docs.py` dependen de que exista `src-c/`, que no se versiona; en un clon limpio el validador da 1 error. — impacto: bajo

#### [`2026-10-10_separacion-hilos-emulacion-y-renderizado.md`](propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md)

- Los shaders en `overlay.wgsl` continúan calculando la carcasa procedural en resoluciones altas (optimización independiente de GPU) — impacto: medio
- El intérprete de respaldo aún ejecuta instrucciones en funciones de combate no recompiladas (cobertura de labels en `blazing.labels.toml`) — impacto: medio

#### [`2026-10-11_cosecha-jump-tables-leafs-aot.md`](propuestas/2026-10-11_cosecha-jump-tables-leafs-aot.md)

- **Guarda defensiva `rcg_block_count` en `src/recomp.rs`:** El límite previo de `100_000` bloques habría rechazado la biblioteca de 103,831 bloques en tiempo de ejecución. Se ajustó preventivamente a `500_000` preservando compatibilidad y seguridad (impacto: bajo, corregido como parte del soporte de escala). — impacto: bajo

#### [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md)

- **La instrumentación que viaja en el APK** (`Cargo.toml:12` activa `perf-counters`) cuesta 10,4 % del fotograma en host, medido con 6 pares intercalados. Es el sospechoso que queda en pie y abre la propuesta nueva. — impacto: alto
- El arnés en host dispersa ±13 % entre corridas idénticas: ningún A/B de menos de ~5 % es interpretable sin pares intercalados. — impacto: medio
- No hay `cc`/`gcc`/`clang` en esta máquina: el host compila el C con TinyCC, que puede subestimar el peso relativo de un salto indirecto frente a un `clang -O2` en ARM64. — impacto: medio
- El camino rápido de ROM queda revertido y medido (≤6 %, dentro del ruido): no descartado para siempre, con el diff reproducible en el apéndice. — impacto: bajo
- Las 8 emisiones de **escritura** y las lecturas de EWRAM/IWRAM siguen por la vtable, sin prototipar: el prototipo cubrió solo la lectura de ROM. — impacto: bajo
- El `.dll` (174 MB, 12 unidades) se forja entero en cada prueba: cada A/B paga minutos de compilación antes de medir 8 segundos. — impacto: bajo

#### [`2026-10-12_compresion-librerias-nativas-apk.md`](propuestas/2026-10-12_compresion-librerias-nativas-apk.md)

- **Ruta absoluta local versionable en un archivo que no es de esta propuesta:** `docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md` (línea 180) contiene `C:/Users/<usuario>/Downloads/...`, lo que viola la regla de portabilidad y privacidad de `AGENTS.md` §6 y bloquearía el punto 5 del checklist de pre-commit. El archivo ya existía antes de esta tarea y no lo he tocado. — impacto: medio
- El repositorio no fija la alineación de páginas de 16 KB para las bibliotecas nativas (`max-page-size`), que Android 15 con páginas de 16 KB puede requerir. — impacto: medio
- El código C11 recompilado pesa ~62 MB por ABI y viaja tres veces: es el 92 % del APK. — impacto: medio
- El Escenario 1 fija "exactamente 6 entradas `lib/**/*.so`", que es el contrato del APK fat. Un empaquetado con `-AllAbis:$false` deja 2 entradas y haría fallar ese test aunque el APK sea correcto para su propósito. — impacto: bajo
- Las opciones del script de empaquetado (`-AllAbis`, `-SkipBuild`, `-NoStrip`, `-Debug`) no están documentadas en `docs/`. — impacto: bajo
- `docs/explicacion/DECISIONES.md` (ADR 15) afirma que el APK pesa 39 MB, cifra que no corresponde al estado actual (186.6 MiB). — impacto: bajo

#### [`2026-10-12_drain-hotspots-sin-perdida.md`](propuestas/2026-10-12_drain-hotspots-sin-perdida.md)

- `nativos` por región cuenta bloques y `interprete` cuenta instrucciones: no se puede derivar un porcentaje de cobertura exacto sin contar instrucciones nativas. — impacto: medio
- [`docs/explicacion/DECISIONES.md:502`](../../docs/explicacion/DECISIONES.md) afirma que el acumulador filtra "exclusivamente direcciones del cartucho (ROM_REGION_START..ROM_REGION_END)", pero [`src/recomp.rs:201`](../../src/recomp.rs) incluye también IWRAM, EWRAM y BIOS. — impacto: bajo
- [`docs/explicacion/DECISIONES.md:503`](../../docs/explicacion/DECISIONES.md) afirma un volcado "cada 5 segundos", pero [`src/platform.rs:2113`](../../src/platform.rs) vuelca cada 2 segundos. — impacto: bajo
- El orden entre direcciones con el mismo conteo no está definido (`sort_unstable_by`). — impacto: bajo

#### [`2026-10-12_ejecucion-nativa-de-bios-real.md`](propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)

- [`docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md:180`](2026-10-12_drain-hotspots-sin-perdida.md) incluye un comando con una ruta local absoluta que contiene el nombre de usuario, lo que choca con la regla de portabilidad y privacidad de `AGENTS.md §6` (el archivo es de otro hilo y está sin commitear, así que no se edita desde aquí). — impacto: bajo
- `recomp runc` informa `loaded 17207 translated blocks` mientras `src-c/` contiene 144.875 símbolos (origen: el comando del apartado 9 y `grep -rho -E "b_[0-9a-f]{7,8}_[at]" src-c/ | sort -u | wc -l`). No investigado. — impacto: bajo
- [`docs/explicacion/arquitectura-port-android.md:86`](../explicacion/arquitectura-port-android.md) declara una latencia de swapchain de 1 fotograma y [`src/platform.rs:294`](../../src/platform.rs) configura 2. — impacto: bajo
- `AGENTS.md` (checklist de pre-commit) cita 111 tests; la medición de hoy da 105 unitarios + 13 de integración = 118. — impacto: bajo

#### [`2026-10-12_nombre-apk-segun-perfil.md`](propuestas/2026-10-12_nombre-apk-segun-perfil.md)

- Los dos tests de estructura del APK saltan en silencio cuando el archivo no existe, así que una pérdida de cobertura se ve igual que un éxito. — impacto: bajo
- Tras el primer empaquetado con el nombre nuevo, el artefacto viejo `target/blazing-recomp-debug.apk` (02:25) queda huérfano: nada lo borra automáticamente. — impacto: bajo
- `apksigner` sigue informando `Verified using v1 scheme (JAR signing): false` pese a que el script pasa `--v1-signing-enabled true`, tanto antes como después de este cambio. Es preexistente y contradice el checklist de `AGENTS.md §6`, que pide los tres esquemas. — impacto: bajo

#### [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md)

- **6,00 ms de despacho y lógica de juego** (impacto: alto). Está sin tocar y ya es el bloque más grande del fotograma, por delante de cualquier etapa de la PPU (`comp` 2,96 ms). Es el tema de la próxima propuesta, no de esta. — impacto: alto
- **La telemetría sigue sin identificar el reloj que calibra** (impacto: bajo-medio). El encabezado publica `reloj=1384ns` o `1891ns`, pero ese número mide el `Instant::now()` del **port** ([`src/perf.rs:305`](../../src/perf.rs#L305)), no el contador del sistema que el motor usa desde el paso 1. Por eso la diferencia 1891 → 1384 ns **no** es evidencia de esta propuesta: es el mismo API medido en días distintos. Sin cambiar esto, ninguna sesión futura podrá comprobar el coste del reloj del motor desde la telemetría. — impacto: bajo-medio
- **Pico de presentación de 505,3 ms** (impacto: medio). La sesión 17 tiene un fotograma en t=78 s con `dibujo=13.7/507.0` y `presentmax=505.3`; la 16 nunca pasó de 20,2 ms. Es una congelación visible de medio segundo y, con una sola aparición, no se puede atribuir ni descartar como regresión de esta propuesta. Necesita otra sesión para saber si es sistemático. — impacto: medio
- **`docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md` versiona una ruta absoluta local** (impacto: bajo, higiene y privacidad). Su apartado de comprobación cita el log de medición con el nombre de la carpeta de descargas del equipo delante, contra la regla de `AGENTS.md` §6. Se deja como está por no reescribir el registro de otra propuesta; el arreglo es quedarse con el nombre del archivo. — impacto: bajo
- **7.483 pasos de intérprete en IWRAM** (impacto: bajo). El hotspot superior es `0x030035C8` (código copiado a IWRAM en tiempo de ejecución); es el 0,0 % de los eventos de despacho —126 pasos en ROM en 167 s— y no explica ninguna caída. — impacto: bajo
- **Evidencia fuera del repositorio** (impacto: bajo). Los registros de las sesiones 16 y 17 que sustentan la tabla no están versionados, así que sus números no se pueden reproducir con [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) desde un clon limpio. El precedente del repositorio es `scripts/diagnostico/log_tv15.txt`; snapshotearlos crea archivos, así que es un cambio aparte. — impacto: bajo

#### [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md)

- `perf_clock.rs` documenta ~30 ns para su camino genérico y la medición lo deja en ~140 ns; ese camino lo usan `armeabi-v7a` y `x86_64`, que [`package_apk.ps1`](../../scripts/package_apk.ps1) construye. — impacto: medio
- El coste de `CNTVCT_EL0` en el dispositivo sigue sin medir: la sonda `reloj=` del port mide `Instant::now()`, no el reloj del motor. — impacto: medio
- El arnés de host se cronometra a sí mismo: toda comparación A/B del proyecto arrastra ~8-10 % de reloj. — impacto: medio

#### [`2026-10-13_alpha-real-por-tramos.md`](propuestas/2026-10-13_alpha-real-por-tramos.md)

- **Los modos 2 y 3 (brillo y oscuridad) buscan la segunda capa en cada píxel sin necesitarla.** El desvío de `blend_mode == 0` ya aplaza la búsqueda a los píxeles con sprite translúcido, pero en los modos 2 y 3 el bucle general llama a `top_two_layers` 240 veces por línea aunque el resultado solo dependa de la capa de arriba (el propio test del motor lo deja escrito: `oscurecido_real_no_busca_la_segunda_capa` exige 0 búsquedas en el camino especializado y >0 en el general). Los destellos de pantalla del juego caen aquí. **Impacto: medio.**. — impacto: medio
- **El coste del reloj no es estable entre ejecuciones.** La misma familia de binarios declara 1.514 ns (sesión 19), 1.504 (20) y **2.073** (21). Si la instrumentación toma una marca de tiempo por línea o por fase, esa diferencia son décimas de milisegundo por fotograma dentro de `comp`, justo la partida que se acaba de ganar. Es la pregunta de la propuesta [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md), que sigue pendiente. **Impacto: medio.**. — impacto: medio
- **`analizar_telemetria.py` no lee `# ppucamino`.** El desglose por caminos de composición se analiza a mano. **Impacto: bajo** (arrastrado de las dos propuestas anteriores). — impacto: bajo
- **`descartes_mezcla` ya no describe lo que su nombre dice.** Con el camino por tramos, una línea contada en `descartes_mezcla` ya no es «una línea descartada de la vía rápida»: es «una línea con alpha real que necesita la segunda capa», y ahora se compone por tramos. El §8 obliga a seguir contándola ahí y el §4.4 prohíbe reclasificar la telemetría en esta ronda, así que el campo (y su texto en `# ppucamino`) se lee mal hasta que una propuesta de telemetría lo renombre o añada una cuenta propia. **Impacto: bajo.**. — impacto: bajo
- **La telemetría no identifica el APK que la emite.** La cabecera trae backend, modo, `reloj` y la versión de pipeline (`ppu=recto-2`), pero ninguna marca del binario, así que atribuir una sesión a un APK depende de la hora del archivo. Un campo con la versión de la aplicación o el hash corto del `.so` cerraría esa duda. **Impacto: bajo.**. — impacto: bajo

#### [`2026-10-13_contadores-de-camino-de-la-ppu.md`](propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)

- **El cambio del motor sigue sin commitear** (impacto: medio). El APK lo lleva por el `[patch]`; un build que no use la ruta local perdería la vía rápida *y* sus contadores. — impacto: medio
- **El analizador de telemetría no parsea `# ppucamino`** (impacto: bajo). La línea se lee a mano o restando dos consecutivas; añadirle una sección es trabajo de otra propuesta, y solo compensa si el dato se usa en cada sesión. — impacto: bajo
- **El motor advierte de un `const FALLBACK` sin usar en `hostclock.rs`** (impacto: bajo). Es código del motor upstream sin tocar por este cambio (`hostclock.rs` no aparece modificado en `git status`), así que no se corrigió aquí. — impacto: bajo
- **Nada impide que un contador se quede obsoleto en silencio** (impacto: bajo). Si mañana cambia la guarda y no se actualiza `razon_camino`, los tests de reparto seguirían pasando mientras la línea miente; hoy la decisión es una sola variable, así que el riesgo es pequeño, pero no hay un test que ate el reparto a la salida real del camino recto (el hash ata los píxeles, no el reparto). — impacto: bajo

#### [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)

- Optimización del modo alpha real (modo 1) con ambas capas presentes (complejidad O(N) por búsqueda de segunda capa, impacto medio en escenas con semitransparencias reales). — impacto: medio
- Parseo automático de la línea `# ppucamino` dentro de `scripts/diagnostico/analizar_telemetria.py` (anotado previamente como fuera de alcance). — impacto: sin declarar

#### [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)

- **La validación en el dispositivo queda pendiente** (impacto: alto, y no es un defecto): es la medida que decide el apartado 8 y se hace en el televisor. — impacto: alto
- **Los búferes por línea se reinicializan enteros** (impacto: bajo-medio). `render_scanline` llena 1.920 bytes por línea (`[[TRANSPARENT; 240]; 4]`) y las cuatro funciones de dibujado ya escriben la línea completa, así que el relleno es trabajo repetido (~0,02-0,03 ms por fotograma, contado dentro de los fondos). Quitarlo necesita los búferes en `MemMap` y partir los préstamos. — impacto: bajo-medio
- **El cambio del motor está sin commitear** (impacto: medio). El APK lo lleva por el `[patch]`, pero cualquier build que no use la ruta local compilaría el motor fijado y perdería la mejora. — impacto: medio
- **El orden de bytes de las ventanas no está documentado en el motor** (impacto: bajo). `WIN0H` empaqueta X1 (izquierda) en el byte alto y X2 (derecha) en el bajo, y el rango es `x1 <= x < x2`, con vuelta cuando X1 > X2. La única constancia es el propio `in_window_range`: mi primer test lo escribió al revés y falló, así que el motor estaba bien y el error era del test. Un comentario junto a la lectura de los registros lo ahorraría. — impacto: bajo

#### [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)

- **Solo el 5-17 % de los ciclos del fotograma pasan por `a->tick`**; el resto lo avanza `skip_to_next_event` durante el *halt*. Es el hueco más grande del presupuesto del fotograma y el único camino de reloj sin instrumentar. — impacto: alto
- **`fallback_steps: 0` no significa «el juego no se detiene»**: `run_frame_native` llama a `m.step()` en el estado *halted* sin contarlo. Un contador de fotogramas dormidos sería barato y aclararía el reparto. — impacto: medio
- **Los shells de `interp_one` (BIOS) avanzan el reloj sin pasar por `rt_tick`**, así que ningún contador de este tipo ve el camino del intérprete. — impacto: medio
- **El despacho indirecto por la tabla `RtApi` no cuesta en este host** (−0,01 ns frente a una llamada directa): la diferencia de 2,8× que midió [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md) era el *trabajo* del accesor, no el salto indirecto. En el ARM del televisor el predictor puede portarse distinto y conviene confirmarlo allí. — impacto: medio
- La media **dinámica** de ciclos por petición (6,55-6,59) es menor que la **estática** del censo (9,22): las escenas calientes son más densas en accesos al bus, que son los que fuerzan el volcado. — impacto: bajo

#### [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md)

- Los grupos de cadena con **muchas salidas** (1.518 puertas y 5.256 `goto` en 089f2d69) hacen que cualquier esquema de volcado por `return` explote: si algún día se retoma, el volcado debe ser **parcial** (solo los registros vivos) y decidido por el analizador de flujo. — impacto: medio
- **El registro del juego en memoria cuesta el 31,1 % de las instrucciones generadas** (media de 8 grupos reales a `-O1`). Es un hecho nuevo y sólido del perfil del juego, aunque localizar no lo aproveche. — impacto: bajo
- La forma «full» (volcado antes de cada llamada de bus) queda medida y descartada: +73,2 % de instrucciones. — impacto: bajo
- El host no tiene compilador nativo (`cc`/`gcc`/`clang`), así que el micro-banco *en ejecución* del escenario 2 no se pudo correr: la fase A se resolvió con censo estático de ensamblado, más barato y ya decisivo. — impacto: bajo

#### [`2026-10-14_medir-la-frontera-entre-bloques.md`](propuestas/2026-10-14_medir-la-frontera-entre-bloques.md)

- **`a->tick` se emite prácticamente una vez por instrucción de juego** (32,1 % de todas las llamadas del C generado; ~4× las vueltas al despachador). Es una llamada indirecta por instrucción para sumar ciclos. Es el candidato con más volumen de todo el camino de llamadas y merece su propia propuesta. — impacto: alto
- **La puerta de bucle devuelve «sigue» casi siempre** (0 % en el arranque; 1,2 % en el avance): su llamada indirecta por vuelta es casi puro coste, aunque el recuento es bajo (~90-263 por fotograma). — impacto: bajo
- **La frontera queda descartada como palanca** con número: 1-2 % del fotograma como techo. — impacto: bajo

#### [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)

- `present` **escala con los píxeles** en la medición de tablet (51,4 ms → 2,9 ms a media resolución): la resolución es la única palanca medida y es una decisión de nitidez del usuario, no técnica. — impacto: medio
- La espera de `present` podría solaparse con la emulación en otro hilo; no se aborda aquí. — impacto: medio
- La cabecera de telemetría ya registra el modo real del swapchain: el barrido no es imprescindible para saber el modo, solo para saber si la configuración ayudaría. — impacto: bajo

#### [`2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md`](propuestas/2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md)

- `docs/explicacion/arquitectura-c-recompilado.md` conserva cifras caducas fuera de las dos líneas del alcance: «más de 115.000 instrucciones descubiertas» y una tabla de unidades de traducción que describe 2-3 archivos cuando `src-c/` tiene 12. — impacto: medio
- El `README.md` raíz sigue describiendo el flujo `recomp.exe play` y una carpeta `mods/` que no existe (verificado con `ls`), y sólo menciona Android una vez — impacto: bajo
- `pack.toml` declara `version = "0.1.0"` mientras la app es `0.0.2` (`Cargo.toml`, `AndroidManifest.xml`) — impacto: bajo
- El número de tests en la documentación puede volver a quedar obsoleto en silencio: ningún test lo deriva del árbol, sólo se prohíben las frases viejas. Un guardián que cuente `#[test]` y lo compare sería la solución, pero excede los criterios de esta propuesta — impacto: bajo

## Checklist de Pre-Commit

Los 5 criterios obligatorios antes de un `commit`/`push` están en [`AGENTS.md`](../AGENTS.md) §6. Aquí
solo el resumen: suite en verde, validación Diátaxis sin errores, referencias de API sincronizadas,
auditoría de labels Aprobada y auditoría de rutas absolutas limpia.
