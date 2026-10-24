# Backlog del Proyecto — Blazing Recomp

Hoja de ruta, tareas pendientes y seguimiento de hitos para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**.

> **Este es el tablero vivo del proyecto:** qué falta, qué se sabe y dónde está anotado.
> No es una página Diátaxis — el validador lo excluye por nombre —, así que no lleva front-matter.
> Los hallazgos históricos fuera de alcance viven en el [registro de hallazgos](#registro-de-hallazgos) (y su coherencia con las propuestas la comprueba `python scripts/auditar_hallazgos.py`), mientras que las tareas accionables abiertas se gestionan en [`docs/PENDIENTES.md`](PENDIENTES.md).

**Última actualización:** 2026-10-24

---

## Estado del Proyecto

- **Versión:** `0.0.2` / `versionCode` 2 ([`Cargo.toml`](../Cargo.toml), [`android/AndroidManifest.xml`](../android/AndroidManifest.xml))
- **Hito del [roadmap](explicacion/roadmap-y-versionado.md):** **Fase 0 (Cimientos, `0.0.x`)**, camino a
  `0.1.0` (MVP jugable de principio a fin).
- **ROM Objetivo:** `Fire Emblem (Europe)` — `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS Objetivo:** `gba_bios.bin` — `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Estado de la suite:** 257 tests en verde (172 en los módulos de `src/`, 85 de integración en
  [`tests/`](../tests/)). El checklist de pre-commit vive en
  [`AGENTS.md`](../AGENTS.md#checklist-obligatorio-de-pre-commit-gatekeeper) §7.

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
- **PPU nativa** (fase 1): composición por píxel en la GPU con la unidad de mezcla de la GBA
  ([`2026-10-19_ppu-nativa-fase-1.md`](propuestas/2026-10-19_ppu-nativa-fase-1.md),
  [`verificada por agente`](propuestas/2026-10-19_ppu-nativa-fase-1.md)). Da **0 diferencias de
  38.400 píxeles** frente a la PPU de referencia en los dos cuadros de aceptación. **Es la ruta de
  dibujo por defecto** desde
  [`2026-10-21_ppu-nativa-activa-por-defecto.md`](propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md);
  Conforme al ADR 42, el antiguo `PpuShim` por software fue completamente retirado del árbol activo
  y archivado en [`docs/historico/ppu-shim/`](historico/ppu-shim/README.md). La PPU nativa en GPU (`NativePpu`)
  es la única ruta de renderizado de producción. Ante fallo en la inicialización de GPU o activación
  de la bandera de contingencia `ppu-nativa-off.flag`, el motor conmuta automáticamente al rasterizador
  por software en CPU provisto por `gba-core` (`render_scanline`).
- **Fondo afín del modo 1** (el fondo girando de los menús): la PPU nativa lo dibuja con la
  aritmética de la referencia
  ([`2026-10-21_fondo-afin-bg2-modo-1.md`](propuestas/2026-10-21_fondo-afin-bg2-modo-1.md),
  [`verificada por agente`](propuestas/2026-10-21_fondo-afin-bg2-modo-1.md)). El cuadro 900 de
  `secuencias/escena3.txt` da **0 diferencias de 38.400 píxeles** (antes 5222) y el arnés sintético
  da 0 en los siete casos de wrap y de tamaño de mapa.
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
5. **El camino rápido del alpha pinta píxeles equivocados en una esquina** — la clave de tramo del
   parche local resume la máscara de fondos y la prioridad del sprite en el mismo número de 16
   bits, y la prioridad se pisa con la máscara: cuando un sprite cambia de prioridad justo donde un
   fondo cambia de opacidad, el tramo se pinta con la pareja de capas del primer píxel y salen
   píxeles distintos de los que da el camino general (8 de 240 en la escena sintética medida). Está
   sin arreglar:
   [`2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md).
6. **Eliminación del PpuShim completada (ADR 42)** — El shim fue retirado del árbol activo de código
   y archivado con sus 39 ficheros y arneses en [`docs/historico/ppu-shim/`](historico/ppu-shim/README.md).
   La contingencia ante fallo de GPU o diagnóstico se apoya en el rasterizador por software en CPU de `gba-core`.
7. **El cambio del motor sigue sin commitear** — el APK lo lleva por `[patch]`, así que un clon limpio
   no reproduce el APK (registrado en dos propuestas).
8. **La espera activa del bucle principal** contradice la regla «sin busy-waiting» de
   [`AGENTS.md`](../AGENTS.md) §2.5.
9. **Un test intermitente** (`test_emulation_worker_lifecycle_and_frame_generation`, 8 de cada 10
   corridas falla ya en HEAD): el «131 en verde» depende de la suerte.

---

## Índice de Propuestas

159 propuestas en [`docs/propuestas/`](propuestas/). Se listan por estado; la ficha de cada una tiene su
propio `Resultado` (predicho vs. real) y su `Revisión humana`.

**159 = 95 `verificada por agente` + 40 `cerrada` + 2 `aprobada` + 3 `pendiente` + 10 `discrepada` + 6 `revisada por humano` + 2 `modificada` + 1 `descartada`.**

### Pendientes (3) — sin ejecutar

- [`2026-10-10_calibracion-multidispositivo-v2.md`](propuestas/2026-10-10_calibracion-multidispositivo-v2.md)
- [`2026-10-10_optimizacion-compatibilidad-multidispositivo.md`](propuestas/2026-10-10_optimizacion-compatibilidad-multidispositivo.md)
- [`2026-10-14_saneamiento-de-la-documentacion.md`](propuestas/2026-10-14_saneamiento-de-la-documentacion.md)

### Aprobadas sin cerrar (2) — alcance pactado, trabajo o medida pendientes

- [`2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`](propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md)
- [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)

### Discrepadas (10) — la medida contradijo la premisa

- [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md) — el acceso directo al bus no rinde ≈3 %.
- [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md) — la sonda del port mide otro reloj del que se quería medir.
- [`2026-10-14_bombeo-audio-por-evento.md`](propuestas/2026-10-14_bombeo-audio-por-evento.md)
- [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md) — localizar los registros no ahorra: el volcado de frontera lo devuelve.
- [`2026-10-17_subcamino-de-composicion-con-sprite-translucido.md`](propuestas/2026-10-17_subcamino-de-composicion-con-sprite-translucido.md)
- [`2026-10-18_bg0-710-pixeles-shim-cuadro-1000.md`](propuestas/2026-10-18_bg0-710-pixeles-shim-cuadro-1000.md)
- [`2026-10-18_decode-oam-otro-cuadro-regresion.md`](propuestas/2026-10-18_decode-oam-otro-cuadro-regresion.md)
- [`2026-10-18_fondos-tapan-afines-cuadro-7500.md`](propuestas/2026-10-18_fondos-tapan-afines-cuadro-7500.md)
- [`2026-10-18_sprites-afines-cuadro-7500.md`](propuestas/2026-10-18_sprites-afines-cuadro-7500.md)
- [`2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md`](propuestas/2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md)

### Revisada por humano (6)

- [`2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md`](propuestas/2026-10-09_compatibilidad-android-tv-y-armeabi-v7a.md)
- [`2026-10-09_optimizacion-latencia-y-fillrate-gpu-modesta.md`](propuestas/2026-10-09_optimizacion-latencia-y-fillrate-gpu-modesta.md)
- [`2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md`](propuestas/2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md)
- [`2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md`](propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md)
- [`2026-10-23_eliminar-register-spilling-y-bucle-anidado-fs-comp.md`](propuestas/2026-10-23_eliminar-register-spilling-y-bucle-anidado-fs-comp.md)
- [`2026-10-23_pulido-final-subida-sprites-uniformes-y-paso-capas.md`](propuestas/2026-10-23_pulido-final-subida-sprites-uniformes-y-paso-capas.md)

### Cerradas (40)

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
- [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md)
- [`2026-10-13_alpha-real-por-tramos.md`](propuestas/2026-10-13_alpha-real-por-tramos.md)
- [`2026-10-13_contadores-de-camino-de-la-ppu.md`](propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)
- [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)
- [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)
- [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)
- [`2026-10-14_reloj-barato-en-pump-audio.md`](propuestas/2026-10-14_reloj-barato-en-pump-audio.md)
- [`2026-10-16_arnes-comparacion-shim-vs-software.md`](propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md)
- [`2026-10-16_contar-escrituras-de-registros-por-cuadro.md`](propuestas/2026-10-16_contar-escrituras-de-registros-por-cuadro.md)
- [`2026-10-16_contar-lineas-con-cambio-de-registros.md`](propuestas/2026-10-16_contar-lineas-con-cambio-de-registros.md)
- [`2026-10-17_buscar-estado-faltante-cuadro-1000.md`](propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md)
- [`2026-10-17_capas-por-aportacion-cuadro-1000.md`](propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md)
- [`2026-10-17_captura-por-linea-cuadro-1000.md`](propuestas/2026-10-17_captura-por-linea-cuadro-1000.md)
- [`2026-10-17_comparar-capas-del-cuadro-fijo.md`](propuestas/2026-10-17_comparar-capas-del-cuadro-fijo.md)
- [`2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`](propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md)
- [`2026-10-18_afines-conteo-matriz-7500.md`](propuestas/2026-10-18_afines-conteo-matriz-7500.md)
- [`2026-10-18_buscar-escena-bg0-sprites-decode-oam.md`](propuestas/2026-10-18_buscar-escena-bg0-sprites-decode-oam.md)
- [`2026-10-18_instrumentar-copia-limpia-gba-core.md`](propuestas/2026-10-18_instrumentar-copia-limpia-gba-core.md)
- [`2026-10-18_instrumentar-render-sprites-afines.md`](propuestas/2026-10-18_instrumentar-render-sprites-afines.md)
- [`2026-10-23_eliminacion-del-ppu-shim.md`](propuestas/2026-10-23_eliminacion-del-ppu-shim.md)
- [`2026-10-23_reorganizar-examples-y-archivar-obsoletos.md`](propuestas/2026-10-23_reorganizar-examples-y-archivar-obsoletos.md)
- [`2026-10-23_resolucion-auditoria-rant-md.md`](propuestas/2026-10-23_resolucion-auditoria-rant-md.md)
- [`2026-10-23_resolucion-inconsistencias-rant-md.md`](propuestas/2026-10-23_resolucion-inconsistencias-rant-md.md)
- [`2026-10-23_resolucion-quejas-comprension-y-legibilidad-rant.md`](propuestas/2026-10-23_resolucion-quejas-comprension-y-legibilidad-rant.md)
- [`2026-10-24_paquete-1-autonomia-ppu-controles-y-primer-arranque.md`](propuestas/2026-10-24_paquete-1-autonomia-ppu-controles-y-primer-arranque.md)
- [`2026-10-24_paquete-2-vigencia-y-clasificacion-adrs.md`](propuestas/2026-10-24_paquete-2-vigencia-y-clasificacion-adrs.md)
- [`2026-10-24_paquete-3-rigor-anatomia-src-c-y-diataxis.md`](propuestas/2026-10-24_paquete-3-rigor-anatomia-src-c-y-diataxis.md)
- [`2026-10-24_resolucion-cuarta-auditoria-rant-md.md`](propuestas/2026-10-24_resolucion-cuarta-auditoria-rant-md.md)
- [`2026-10-24_resolucion-segunda-auditoria-rant-md.md`](propuestas/2026-10-24_resolucion-segunda-auditoria-rant-md.md)
- [`2026-10-24_resolucion-tercera-auditoria-rant-md.md`](propuestas/2026-10-24_resolucion-tercera-auditoria-rant-md.md)

### Verificadas por agente (95) — con lo medido dentro; varias esperan el televisor

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
- [`2026-10-16_fase-1-1-correccion-ppu-shim-vram-fondos-y-sprites.md`](propuestas/2026-10-16_fase-1-1-correccion-ppu-shim-vram-fondos-y-sprites.md)
- [`2026-10-16_fase-1-2-intercalado-prioridades-bg-obj-y-descarte-obj-window.md`](propuestas/2026-10-16_fase-1-2-intercalado-prioridades-bg-obj-y-descarte-obj-window.md)
- [`2026-10-16_fase-1-3-soporte-sprites-8bpp-gpu.md`](propuestas/2026-10-16_fase-1-3-soporte-sprites-8bpp-gpu.md)
- [`2026-10-16_fase-1-ppu-shim-gpu.md`](propuestas/2026-10-16_fase-1-ppu-shim-gpu.md)
- [`2026-10-17_camino-rapido-contra-general-cuadro-1000.md`](propuestas/2026-10-17_camino-rapido-contra-general-cuadro-1000.md)
- [`2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md`](propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md)
- [`2026-10-17_capa-debajo-del-sprite-cuadro-1000.md`](propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md)
- [`2026-10-17_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md`](propuestas/2026-10-17_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md)
- [`2026-10-17_contenido-bg2-cuadro-1000.md`](propuestas/2026-10-17_contenido-bg2-cuadro-1000.md)
- [`2026-10-17_diagnostico-afin-y-por-linea.md`](propuestas/2026-10-17_diagnostico-afin-y-por-linea.md)
- [`2026-10-17_frente-a-frente-del-estado-por-linea.md`](propuestas/2026-10-17_frente-a-frente-del-estado-por-linea.md)
- [`2026-10-17_identificar-color-x-del-shim-cuadro-1000.md`](propuestas/2026-10-17_identificar-color-x-del-shim-cuadro-1000.md)
- [`2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`](propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md)
- [`2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md`](propuestas/2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md)
- [`2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md`](propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md)
- [`2026-10-17_shim-contra-aportacion-cuadro-1000.md`](propuestas/2026-10-17_shim-contra-aportacion-cuadro-1000.md)
- [`2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md`](propuestas/2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md)
- [`2026-10-18_afines-conteo-matriz-7500.resultado.md`](propuestas/2026-10-18_afines-conteo-matriz-7500.resultado.md)
- [`2026-10-18_bldcnt-bldalpha-cuadro-1000.md`](propuestas/2026-10-18_bldcnt-bldalpha-cuadro-1000.md)
- [`2026-10-18_bldcnt-por-linea-cuadro-1000.md`](propuestas/2026-10-18_bldcnt-por-linea-cuadro-1000.md)
- [`2026-10-18_buscar-cuadros-con-x-fundido.md`](propuestas/2026-10-18_buscar-cuadros-con-x-fundido.md)
- [`2026-10-18_causa-errores-6000-7500-8000.md`](propuestas/2026-10-18_causa-errores-6000-7500-8000.md)
- [`2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md`](propuestas/2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md)
- [`2026-10-18_direccion-texel-composicion-afines.md`](propuestas/2026-10-18_direccion-texel-composicion-afines.md)
- [`2026-10-18_fundido-bldalpha-otros-cuadros.md`](propuestas/2026-10-18_fundido-bldalpha-otros-cuadros.md)
- [`2026-10-18_medir-working-tree-gba-core.md`](propuestas/2026-10-18_medir-working-tree-gba-core.md)
- [`2026-10-18_pixeles-afines-cuadro-7500.md`](propuestas/2026-10-18_pixeles-afines-cuadro-7500.md)
- [`2026-10-18_referencia-afines-gba-core.md`](propuestas/2026-10-18_referencia-afines-gba-core.md)
- [`2026-10-18_residuo-x-con-correccion-obj-cuadro-1000.md`](propuestas/2026-10-18_residuo-x-con-correccion-obj-cuadro-1000.md)
- [`2026-10-18_revisar-aislamiento-afines.md`](propuestas/2026-10-18_revisar-aislamiento-afines.md)
- [`2026-10-18_revisar-comparacion-shim-afin-pixeles.md`](propuestas/2026-10-18_revisar-comparacion-shim-afin-pixeles.md)
- [`2026-10-18_subida-vram-bg-shim-cuadro-1000.md`](propuestas/2026-10-18_subida-vram-bg-shim-cuadro-1000.md)
- [`2026-10-18_tiles-afines-cuadro-7500.md`](propuestas/2026-10-18_tiles-afines-cuadro-7500.md)
- [`2026-10-19_afin-cuadro-5300.md`](propuestas/2026-10-19_afin-cuadro-5300.md)
- [`2026-10-19_bg1-o-sprites-sin-bg0-5300.md`](propuestas/2026-10-19_bg1-o-sprites-sin-bg0-5300.md)
- [`2026-10-19_buscar-escena-afin-visible.md`](propuestas/2026-10-19_buscar-escena-afin-visible.md)
- [`2026-10-19_causa-diferencias-shim-5300.md`](propuestas/2026-10-19_causa-diferencias-shim-5300.md)
- [`2026-10-19_color-pixeles-afines-vs-fondo.md`](propuestas/2026-10-19_color-pixeles-afines-vs-fondo.md)
- [`2026-10-19_composicion-pixeles-afines.md`](propuestas/2026-10-19_composicion-pixeles-afines.md)
- [`2026-10-19_comprobar-bldcnt-shim-5300.md`](propuestas/2026-10-19_comprobar-bldcnt-shim-5300.md)
- [`2026-10-19_desplazamiento-fondos-lineas-sin-afines-5300.md`](propuestas/2026-10-19_desplazamiento-fondos-lineas-sin-afines-5300.md)
- [`2026-10-19_fondos-modo-lineas-sin-afines-5300.md`](propuestas/2026-10-19_fondos-modo-lineas-sin-afines-5300.md)
- [`2026-10-19_mezcla-encendida-control-shim-5300.md`](propuestas/2026-10-19_mezcla-encendida-control-shim-5300.md)
- [`2026-10-19_opacidad-bg0-ppu-5300.md`](propuestas/2026-10-19_opacidad-bg0-ppu-5300.md)
- [`2026-10-19_por-que-bg1-no-afecta-shim-5300.md`](propuestas/2026-10-19_por-que-bg1-no-afecta-shim-5300.md)
- [`2026-10-19_ppu-nativa-fase-1.md`](propuestas/2026-10-19_ppu-nativa-fase-1.md)
- [`2026-10-19_scroll-por-cuadro-shim-5300.md`](propuestas/2026-10-19_scroll-por-cuadro-shim-5300.md)
- [`2026-10-19_shim-frente-emulador-5300.md`](propuestas/2026-10-19_shim-frente-emulador-5300.md)
- [`2026-10-19_verificar-comparacion-afin-replay.md`](propuestas/2026-10-19_verificar-comparacion-afin-replay.md)
- [`2026-10-20_ppu-nativa-bandera-en-runtime.md`](propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md)
- [`2026-10-21_fondo-afin-bg2-modo-1.md`](propuestas/2026-10-21_fondo-afin-bg2-modo-1.md)
- [`2026-10-21_ppu-nativa-activa-por-defecto.md`](propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md)
- [`2026-10-21_ppu-nativa-sin-storage-en-vertices.md`](propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md)
- [`2026-10-21_ventana-obj-camino-rapido.md`](propuestas/2026-10-21_ventana-obj-camino-rapido.md)
- [`2026-10-22_arnes-comparacion-visual-y-causal.md`](propuestas/2026-10-22_arnes-comparacion-visual-y-causal.md)
- [`2026-10-22_arnes-de-experimentos-comun.md`](propuestas/2026-10-22_arnes-de-experimentos-comun.md)
- [`2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md)
- [`2026-10-22_contrato-de-telemetria.md`](propuestas/2026-10-22_contrato-de-telemetria.md)
- [`2026-10-22_sincronizar-referencia-visor-y-temblor.md`](propuestas/2026-10-22_sincronizar-referencia-visor-y-temblor.md)
- [`2026-10-22_visor-doble-interactivo-ppu.md`](propuestas/2026-10-22_visor-doble-interactivo-ppu.md)
- [`2026-10-23_lut-vertical-en-produccion-y-salto-ppu-cpu.md`](propuestas/2026-10-23_lut-vertical-en-produccion-y-salto-ppu-cpu.md)
- [`2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md`](propuestas/2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md)
- [`2026-10-23_ppu-nativa-paleta-lut-vertical.md`](propuestas/2026-10-23_ppu-nativa-paleta-lut-vertical.md)
- [`2026-10-24_cobertura-aot-combate-iwram.md`](propuestas/2026-10-24_cobertura-aot-combate-iwram.md)

### Modificadas (2) — cambiadas sobre la marcha

- [`2026-10-19_scroll-por-linea-copia-shim-5300.md`](propuestas/2026-10-19_scroll-por-linea-copia-shim-5300.md)
- [`2026-10-19_sin-mezcla-corregida-shim-5300.md`](propuestas/2026-10-19_sin-mezcla-corregida-shim-5300.md)

### Descartadas (1) — la medición las dejó fuera

- [`2026-10-19_sin-mezcla-shim-5300.md`](propuestas/2026-10-19_sin-mezcla-shim-5300.md)

### Esperando el televisor: propuestas con `Revisión humana` pendiente

Son 85 en total. Todas piden lo mismo: una sesión en el dispositivo. Es el cuello de botella del proyecto ahora mismo.

- [`2026-10-09_correccion-escala-y-renderizado-overlay.md`](propuestas/2026-10-09_correccion-escala-y-renderizado-overlay.md)
- [`2026-10-09_fidelidad-visual-overlay-prototipo.md`](propuestas/2026-10-09_fidelidad-visual-overlay-prototipo.md)
- [`2026-10-09_manejo-errores-y-compatibilidad-emulador.md`](propuestas/2026-10-09_manejo-errores-y-compatibilidad-emulador.md)
- [`2026-10-10_calibracion-multidispositivo-v2.md`](propuestas/2026-10-10_calibracion-multidispositivo-v2.md)
- [`2026-10-10_contador-fps-pantalla.md`](propuestas/2026-10-10_contador-fps-pantalla.md)
- [`2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md`](propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md)
- [`2026-10-10_medicion-rendimiento-por-fase.md`](propuestas/2026-10-10_medicion-rendimiento-por-fase.md)
- [`2026-10-10_separacion-hilos-emulacion-y-renderizado.md`](propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md)
- [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md) — el acceso directo al bus no rinde ≈3 %.
- [`2026-10-12_compresion-librerias-nativas-apk.md`](propuestas/2026-10-12_compresion-librerias-nativas-apk.md)
- [`2026-10-12_drain-hotspots-sin-perdida.md`](propuestas/2026-10-12_drain-hotspots-sin-perdida.md)
- [`2026-10-12_ejecucion-nativa-de-bios-real.md`](propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)
- [`2026-10-12_nombre-apk-segun-perfil.md`](propuestas/2026-10-12_nombre-apk-segun-perfil.md)
- [`2026-10-12_ppu-60fps-estable.md`](propuestas/2026-10-12_ppu-60fps-estable.md)
- [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md) — la sonda del port mide otro reloj del que se quería medir.
- [`2026-10-13_alpha-real-por-tramos.md`](propuestas/2026-10-13_alpha-real-por-tramos.md)
- [`2026-10-13_arnes-de-fotogramas-dorados.md`](propuestas/2026-10-13_arnes-de-fotogramas-dorados.md)
- [`2026-10-13_contadores-de-camino-de-la-ppu.md`](propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)
- [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)
- [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)
- [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md)
- [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)
- [`2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md`](propuestas/2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md)
- [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](propuestas/2026-10-14_los-registros-del-juego-viven-en-la-cpu.md) — localizar los registros no ahorra: el volcado de frontera lo devuelve.
- [`2026-10-14_medir-la-frontera-entre-bloques.md`](propuestas/2026-10-14_medir-la-frontera-entre-bloques.md)
- [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)
- [`2026-10-14_reloj-barato-en-pump-audio.md`](propuestas/2026-10-14_reloj-barato-en-pump-audio.md)
- [`2026-10-14_saneamiento-de-la-documentacion.md`](propuestas/2026-10-14_saneamiento-de-la-documentacion.md)
- [`2026-10-16_arnes-comparacion-shim-vs-software.md`](propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md)
- [`2026-10-16_contar-escrituras-de-registros-por-cuadro.md`](propuestas/2026-10-16_contar-escrituras-de-registros-por-cuadro.md)
- [`2026-10-16_contar-lineas-con-cambio-de-registros.md`](propuestas/2026-10-16_contar-lineas-con-cambio-de-registros.md)
- [`2026-10-17_buscar-estado-faltante-cuadro-1000.md`](propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md)
- [`2026-10-17_camino-rapido-contra-general-cuadro-1000.md`](propuestas/2026-10-17_camino-rapido-contra-general-cuadro-1000.md)
- [`2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md`](propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md)
- [`2026-10-17_capa-debajo-del-sprite-cuadro-1000.md`](propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md)
- [`2026-10-17_capas-por-aportacion-cuadro-1000.md`](propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md)
- [`2026-10-17_captura-por-linea-cuadro-1000.md`](propuestas/2026-10-17_captura-por-linea-cuadro-1000.md)
- [`2026-10-17_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md`](propuestas/2026-10-17_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md)
- [`2026-10-17_contenido-bg2-cuadro-1000.md`](propuestas/2026-10-17_contenido-bg2-cuadro-1000.md)
- [`2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`](propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md)
- [`2026-10-17_frente-a-frente-del-estado-por-linea.md`](propuestas/2026-10-17_frente-a-frente-del-estado-por-linea.md)
- [`2026-10-17_identificar-color-x-del-shim-cuadro-1000.md`](propuestas/2026-10-17_identificar-color-x-del-shim-cuadro-1000.md)
- [`2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`](propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md)
- [`2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md`](propuestas/2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md)
- [`2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md`](propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md)
- [`2026-10-17_shim-contra-aportacion-cuadro-1000.md`](propuestas/2026-10-17_shim-contra-aportacion-cuadro-1000.md)
- [`2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md`](propuestas/2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md)
- [`2026-10-17_subcamino-de-composicion-con-sprite-translucido.md`](propuestas/2026-10-17_subcamino-de-composicion-con-sprite-translucido.md)
- [`2026-10-18_afines-conteo-matriz-7500.md`](propuestas/2026-10-18_afines-conteo-matriz-7500.md)
- [`2026-10-18_bg0-710-pixeles-shim-cuadro-1000.md`](propuestas/2026-10-18_bg0-710-pixeles-shim-cuadro-1000.md)
- [`2026-10-18_bldcnt-bldalpha-cuadro-1000.md`](propuestas/2026-10-18_bldcnt-bldalpha-cuadro-1000.md)
- [`2026-10-18_bldcnt-por-linea-cuadro-1000.md`](propuestas/2026-10-18_bldcnt-por-linea-cuadro-1000.md)
- [`2026-10-18_buscar-cuadros-con-x-fundido.md`](propuestas/2026-10-18_buscar-cuadros-con-x-fundido.md)
- [`2026-10-18_buscar-escena-bg0-sprites-decode-oam.md`](propuestas/2026-10-18_buscar-escena-bg0-sprites-decode-oam.md)
- [`2026-10-18_causa-errores-6000-7500-8000.md`](propuestas/2026-10-18_causa-errores-6000-7500-8000.md)
- [`2026-10-18_decode-oam-otro-cuadro-regresion.md`](propuestas/2026-10-18_decode-oam-otro-cuadro-regresion.md)
- [`2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md`](propuestas/2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md)
- [`2026-10-18_direccion-texel-composicion-afines.md`](propuestas/2026-10-18_direccion-texel-composicion-afines.md)
- [`2026-10-18_fondos-tapan-afines-cuadro-7500.md`](propuestas/2026-10-18_fondos-tapan-afines-cuadro-7500.md)
- [`2026-10-18_fundido-bldalpha-otros-cuadros.md`](propuestas/2026-10-18_fundido-bldalpha-otros-cuadros.md)
- [`2026-10-18_instrumentar-copia-limpia-gba-core.md`](propuestas/2026-10-18_instrumentar-copia-limpia-gba-core.md)
- [`2026-10-18_instrumentar-render-sprites-afines.md`](propuestas/2026-10-18_instrumentar-render-sprites-afines.md)
- [`2026-10-18_pixeles-afines-cuadro-7500.md`](propuestas/2026-10-18_pixeles-afines-cuadro-7500.md)
- [`2026-10-18_referencia-afines-gba-core.md`](propuestas/2026-10-18_referencia-afines-gba-core.md)
- [`2026-10-18_residuo-x-con-correccion-obj-cuadro-1000.md`](propuestas/2026-10-18_residuo-x-con-correccion-obj-cuadro-1000.md)
- [`2026-10-18_sprites-afines-cuadro-7500.md`](propuestas/2026-10-18_sprites-afines-cuadro-7500.md)
- [`2026-10-18_subida-vram-bg-shim-cuadro-1000.md`](propuestas/2026-10-18_subida-vram-bg-shim-cuadro-1000.md)
- [`2026-10-18_tiles-afines-cuadro-7500.md`](propuestas/2026-10-18_tiles-afines-cuadro-7500.md)
- [`2026-10-19_ppu-nativa-fase-1.md`](propuestas/2026-10-19_ppu-nativa-fase-1.md)
- [`2026-10-20_ppu-nativa-bandera-en-runtime.md`](propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md)
- [`2026-10-21_fondo-afin-bg2-modo-1.md`](propuestas/2026-10-21_fondo-afin-bg2-modo-1.md)
- [`2026-10-21_ppu-nativa-activa-por-defecto.md`](propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md)
- [`2026-10-21_ppu-nativa-sin-storage-en-vertices.md`](propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md)
- [`2026-10-21_ventana-obj-camino-rapido.md`](propuestas/2026-10-21_ventana-obj-camino-rapido.md)
- [`2026-10-22_arnes-de-experimentos-comun.md`](propuestas/2026-10-22_arnes-de-experimentos-comun.md)
- [`2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md)
- [`2026-10-22_contrato-de-telemetria.md`](propuestas/2026-10-22_contrato-de-telemetria.md)
- [`2026-10-22_sincronizar-referencia-visor-y-temblor.md`](propuestas/2026-10-22_sincronizar-referencia-visor-y-temblor.md)
- [`2026-10-22_visor-doble-interactivo-ppu.md`](propuestas/2026-10-22_visor-doble-interactivo-ppu.md)
- [`2026-10-23_eliminacion-del-ppu-shim.md`](propuestas/2026-10-23_eliminacion-del-ppu-shim.md)
- [`2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md`](propuestas/2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md)
- [`2026-10-23_ppu-nativa-paleta-lut-vertical.md`](propuestas/2026-10-23_ppu-nativa-paleta-lut-vertical.md)
- [`2026-10-23_pulido-final-subida-sprites-uniformes-y-paso-capas.md`](propuestas/2026-10-23_pulido-final-subida-sprites-uniformes-y-paso-capas.md)
- [`2026-10-24_resolucion-cuarta-auditoria-rant-md.md`](propuestas/2026-10-24_resolucion-cuarta-auditoria-rant-md.md)
- [`2026-10-24_resolucion-tercera-auditoria-rant-md.md`](propuestas/2026-10-24_resolucion-tercera-auditoria-rant-md.md)

---

## Registro de Hallazgos

> **Naturaleza histórica de este registro:** Este apartado conserva de forma acumulativa e inmutable los hallazgos fuera de alcance documentados en la sección `### Fuera de alcance` al cerrar cada propuesta de cambio (auditados automáticamente por `python scripts/auditar_hallazgos.py`). Muchos de estos hallazgos ya han sido resueltos en propuestas posteriores o corresponden a notas técnicas y limitaciones físicas de hardware. Para consultar la lista viva de **tareas pendientes y accionables de desarrollo**, véase [`docs/PENDIENTES.md`](PENDIENTES.md).

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

#### [`2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md`](propuestas/2026-10-14_corregir-afirmaciones-que-el-codigo-contradice.md)

- `docs/explicacion/arquitectura-c-recompilado.md` conserva cifras caducas fuera de las dos líneas del alcance: «más de 115.000 instrucciones descubiertas» y una tabla de unidades de traducción que describe 2-3 archivos cuando `src-c/` tiene 12. Corregirlo necesita una medición fresca de `src-c/`. — impacto: medio
- El `README.md` raíz sigue describiendo el flujo `recomp.exe play` y una carpeta `mods/` que no existe (verificado con `ls`), y sólo menciona Android una vez. — impacto: bajo
- `pack.toml` declara `version = "0.1.0"` mientras la app es `0.0.2` (`Cargo.toml`, `AndroidManifest.xml`) — impacto: bajo
- El número de tests en la documentación puede volver a quedar obsoleto en silencio: ningún test lo deriva del árbol, sólo se prohíben las frases viejas. Un guardián que cuente `#[test]` y lo compare sería la solución, pero excede los criterios de esta propuesta. — impacto: bajo

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

#### [`2026-10-14_reloj-barato-en-pump-audio.md`](propuestas/2026-10-14_reloj-barato-en-pump-audio.md)

- `BlockTable::from_blocks` en `src/recomp.rs` aloja un vector `rom` indexado por byte de hasta ~128 MB en 64 bits con <0,2 % de densidad, penalizando la caché L2/L3 en cada despacho. — impacto: medio
- `render_scanline` y `render_sprites` en `../gba-recomp/crates/gba-core/src/ppu.rs` evalúan coordenadas y lecturas de VRAM de sprites antes de descartar píxeles ya ocluidos por un sprite de mayor prioridad. — impacto: medio
- `EmulationWorker` en `src/pipeline.rs` realiza una copia extra de 153,6 KB por fotograma desde `runner.frame_rgba()` hacia el búfer del pool en lugar de componer directamente en el búfer destino. — impacto: bajo
- `record_hotspot` en `src/recomp.rs` adquiere el `Mutex` global `HOTSPOTS` en cada instrucción ejecutada por el intérprete de respaldo. — impacto: bajo

#### [`2026-10-16_arnes-comparacion-shim-vs-software.md`](propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md)

- **Captura única por cuadro (causa probable de las bandas):** el plan de Fase 1 (paso 4) y `src/pipeline.rs` toman VRAM, OAM, paleta e I/O una sola vez por cuadro, después de `step_frame()`. El shim aplica ese estado a todas las líneas, aunque el juego cambie registros a mitad del cuadro (como ocurre con los efectos de las capturas 5 y 6). Esto contradice la premisa de la Fase 1 y necesita una propuesta propia de estado por línea. — impacto: sin declarar
- **Ventanas y blend:** `src/ppu_shim.wgsl` no tiene ninguna mención a ventanas ni blend en la revisión de hoy. La Fase 1.4 lo tiene previsto. — impacto: sin declarar
- **Solo modo 0:** `decode_io_regs` y `render_frame` solo tratan el modo 0 (comentario en `ppu_shim.rs`). Las escenas en otros modos (por ejemplo, la pantalla de título) no quedan cubiertas. — impacto: sin declarar

#### [`2026-10-16_fase-1-1-correccion-ppu-shim-vram-fondos-y-sprites.md`](propuestas/2026-10-16_fase-1-1-correccion-ppu-shim-vram-fondos-y-sprites.md)

- *Modos afines rotados (Modo 1 y 2):* Fire Emblem opera en Modo 0; los modos afines quedan para cuando se requiera cinemáticas rotadas complejas. — impacto: sin declarar
- *Fase 2 (Bus & Timing Shim):* La sustitución de `RtApi` y reloj AOT se abordará tras estabilizar la imagen visual. — impacto: sin declarar

#### [`2026-10-16_fase-1-2-intercalado-prioridades-bg-obj-y-descarte-obj-window.md`](propuestas/2026-10-16_fase-1-2-intercalado-prioridades-bg-obj-y-descarte-obj-window.md)

- *Soporte completo de máscaras de ventana por hardware (WIN0, WIN1, WINOBJ):* Los efectos de recorte geométrico por ventana se abordarán si una escena de batalla o transición específica lo requiere. — impacto: sin declarar

#### [`2026-10-16_fase-1-ppu-shim-gpu.md`](propuestas/2026-10-16_fase-1-ppu-shim-gpu.md)

- *Fase 2 (Bus & Timing Shim):* La eliminación de `a->tick` y la sustitución de la memoria plana de IWRAM/EWRAM se reservan para la Fase 2, una vez validada la fidelidad gráfica. — impacto: sin declarar
- *Fase 3 (Audio Shim):* El desacoplamiento del motor `m4a` hacia AAudio/Oboe se abordará en la Fase 3. — impacto: sin declarar
- *Efectos afines avanzados (Modo 1/2 y rotación de fondos):* Fire Emblem 7 opera en Modo 0 para menús y batallas; el soporte de fondos afines rotados queda diferido si no es requerido por el juego. — impacto: sin declarar

#### [`2026-10-17_diagnostico-afin-y-por-linea.md`](propuestas/2026-10-17_diagnostico-afin-y-por-linea.md)

- `src/ppu_shim.rs` sigue tomando un único estado por fotograma: con 81 % de los cuadros de la introducción escribiendo E/S a mitad de cuadro, ninguna mejora del shim puede reproducirlos hasta que reciba estado por línea. — impacto: medio. — impacto: medio
- `BgLayerConfig.mosaic` sigue leyendo el bit 6 de `BGnCNT` como mosaico en BG2/BG3, donde ese bit es afín: hoy es inocuo porque el juego no usa BG2/BG3 afines en la introducción (0 de 1.112 cuadros), pero etiqueta mal el campo. — impacto: bajo. — impacto: bajo
- El informe guarda cuántas líneas difieren de la línea 0, no cuáles: una sola escritura en mitad del cuadro y una tabla por línea de barrido producen el mismo número, así que el barrido no distingue ambos casos. — impacto: bajo. — impacto: bajo

#### [`2026-10-17_frente-a-frente-del-estado-por-linea.md`](propuestas/2026-10-17_frente-a-frente-del-estado-por-linea.md)

- La mezcla (`BLDCNT`/`BLDALPHA`/`BLDY`) y las ventanas (`WIN0`/`WIN1`/`OBJWIN`) siguen sin existir en el shim: el experimento las mide, pero no las corrige. — impacto: medio. — impacto: medio
- El residuo de 1,33 % medio en cuadros sin mezcla (picos de 3,3 %) no está explicado: apunta a VRAM que cambia dentro del cuadro o al dibujado de sprites, y necesita su propio experimento. — impacto: bajo. — impacto: bajo
- El ejemplo necesita GPU y tarda unos 5 s por cuadro cuando hay decenas de estados, así que no puede entrar en la suite de CI: las pruebas solo leen el informe ya generado. — impacto: bajo. — impacto: bajo

#### [`2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`](propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md)

- El índice de propuestas de `docs/BACKLOG.md` sigue en 58 fichas y no incluye las propuestas del 2026-10-16 ni las del 2026-10-17, así que su recuento por estado ya no cuadra. — impacto: bajo. — impacto: bajo

#### [`2026-10-17_subcamino-de-composicion-con-sprite-translucido.md`](propuestas/2026-10-17_subcamino-de-composicion-con-sprite-translucido.md)

- La vía por tramos construye sus tramos solo con WIN0 y WIN1 e ignora la ventana de sprites (`obj_window`), que el camino general sí consulta en `control_at`. Con OBJWIN parcial y alpha del modo 1, los píxeles dentro de la máscara usarían el control de fuera. No se observó en este cuadro porque `DISPCNT` = `0x1F00` (sin ventanas); se detectó leyendo el código, no midiendo. — impacto: medio. — impacto: medio
- La instrumentación y el ejemplo viven en `../gba-recomp` sin commit; el motor local queda con tres contadores y dos interruptores nuevos que solo afectan a builds con `perf-video`. — impacto: bajo. — impacto: bajo

#### [`2026-10-19_ppu-nativa-fase-1.md`](propuestas/2026-10-19_ppu-nativa-fase-1.md)

- El `Cargo.toml` versionado fija `gba-core` al rev `de4edf59` y a la vez lo redirige con `[patch]` a `../gba-recomp`; ese rev no tiene sección `[features]`, así que sin la carpeta hermana el proyecto no compila. — impacto: alto
- La captura por línea de `gba-core` (`CapturaLinea`) clona VRAM, IO, paleta y OAM completos por línea visible: unos 99 KB por línea y 15,8 MB por cuadro. — impacto: medio
- El rev fijado `de4edf59` de `gba-recomp` no tiene ni la captura por línea ni la feature `perf-video`: solo viven en el worktree local que trae el `[patch]` de `Cargo.toml`, así que el arnés de comparación no es reproducible con el rev fijado. — impacto: medio
- El runtime toma VRAM, OAM, paleta e I/O una sola vez por cuadro (`src/pipeline.rs`), así que el estado por línea no existe en producción: la fase 1b necesita tocar el motor o llevar su propia ruta de depuración. — impacto: medio
- `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `src/ppu_shim_tests.rs` y `examples/` siguen sin versionar: el shim actual, que el criterio de regresión quiere congelar, todavía no está en el repositorio. — impacto: medio
- Los once volcados `volcados/afin-*-5300.txt` son byte-idénticos (mismo SHA-256), así que diez de los experimentos repetidos no midieron nada nuevo. — impacto: medio
- El validador Diátaxis excluye `docs/propuestas/**` y `docs/BACKLOG.md` de las páginas que revisa, así que un ancla rota en esos ficheros no se detecta: el enlace del README de propuestas apuntaba a un ancla que el generador del índice dejaba obsoleta en cada regeneración y el validador daba 0 avisos. — impacto: medio
- Los dos cuadros de aceptación (5300 y 7500) no usan fondos en modo bitmap (3-5) ni fondos afines, así que las exclusiones de §7 no están cubiertas por ningún criterio: un juego que los use se dibujaría mal con la PPU nativa. La fase 1 solo dibuja fondos de tiles (modos 0 y 1) y sprites (afines incluidos). — impacto: medio
- `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs`, `tests/ppu_nativa_tests.rs`, `tests/datos/`, `examples/` y `scripts/diagnostico/mutar_ppu_nativa.py` siguen **sin versionar**: el módulo nuevo, que la fase 1 entrega, todavía no está en el repositorio. — impacto: medio
- El arnés de comparación depende de `FE_ROM` y de la feature `proto` (y por tanto del worktree local de `gba-recomp`): sin la carpeta hermana, los dos criterios que comparan píxeles no se pueden ejecutar y se omiten con un mensaje. — impacto: medio
- La comparación sin mezcla (shim frente a la PPU con la mezcla apagada) no cubre la segunda capa ni el fondo, porque en la salida sin mezcla solo manda la capa superior: si el criterio normal falla por ahí, el arnés actual no lo distinguiría. — impacto: bajo
- La PPU nativa reserva seis texturas (cuatro de fondos, la de sprites y la de sprites en modo ventana) más los búferes de configuración; en el dispositivo eso es memoria de vídeo que el shim no gastaba, y no se ha medido. — impacto: bajo

#### [`2026-10-20_ppu-nativa-bandera-en-runtime.md`](propuestas/2026-10-20_ppu-nativa-bandera-en-runtime.md)

- La referencia técnica generada (`docs/referencia/api-modulos-rust.md`) no cubre `src/ppu_nativa.rs` (el generador tiene una lista curada de módulos), así que la constante `NOMBRE_BANDERA` y la función `bandera_activada` no aparecen en ella aunque `--check` pasa. — impacto: bajo. — impacto: bajo
- Al retirar la marca, el pipeline de la PPU nativa sigue construido y con sus texturas reservadas en memoria de vídeo hasta que la app se reinicia: se deja de usar, pero no se libera. — impacto: bajo. — impacto: bajo
- El script de empaquetado anuncia la firma v1, pero `apksigner` no la aplica: con `minSdkVersion 26` la firma v1 queda sin efecto y el verificador la da por ausente. — impacto: bajo. — impacto: bajo

#### [`2026-10-21_fondo-afin-bg2-modo-1.md`](propuestas/2026-10-21_fondo-afin-bg2-modo-1.md)

- La PPU nativa compone con el estado de fin de cuadro: en el cuadro 900 el juego escribe `BG2X`/`BG2Y` durante el cuadro, así que frente a la referencia **por línea** quedan 13247 diferencias de 38400 (las dos referencias difieren en eso). — impacto: medio
- El shim solo tiene capas de fondo del modo 0 (scroll, sin transformación): no dibuja el fondo afín BG2, así que con la PPU nativa apagada (archivo de marca `ppu-nativa-off.flag`) los menús con el fondo girando salen sin esa capa. — impacto: medio
- La PPU nativa sigue sin dibujar el modo 2 (BG2 y BG3 afines) ni los modos 3, 4 y 5 (bitmap): el juego no los usa, pero un fondo de bits saldría en negro y el modo 2 no se deformaría. — impacto: medio

#### [`2026-10-21_ppu-nativa-activa-por-defecto.md`](propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md)

- El guardián de errores de `wgpu` solo cubre la **construcción** del pipeline: un error de validación que aparezca más tarde, al ejecutar un fotograma, seguiría entrando en pánico. — impacto: medio
- La marca antigua `ppu-nativa.flag` **deja de leerse**: si sigue en el almacenamiento de algún dispositivo, ya no hace nada (la PPU nativa está activa por defecto). Para apagarla hay que crear `ppu-nativa-off.flag`. — impacto: bajo
- Al retirar la marca de apagado, el pipeline de la PPU nativa sigue construido y con sus texturas reservadas en memoria de vídeo hasta que la app se reinicia: se deja de usar, pero no se libera. — impacto: bajo

#### [`2026-10-21_ppu-nativa-sin-storage-en-vertices.md`](propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md)

- La clave de profundidad que desempata los sprites es `prioridad * 128 + índice de OAM`, y cabe en el rango `[0, 1]` del adjunto de profundidad solo porque `MAX_SPRITES` es 128 y la prioridad máxima es 3 (`3 * 128 + 127 = 511 < 512`): si el máximo de sprites creciera, la clave se saldría del rango y el desempate dejaría de ser correcto. — impacto: bajo
- Los búferes de sprites reservan 128 entradas (8 KB el *storage buffer* y 3 KB el de instancias) aunque en un cuadro normal haya menos de veinte sprites: el tamaño reservado no se ajusta al cuadro y la memoria de vídeo no se devuelve. — impacto: bajo
- La geometría de cada sprite está duplicada: la misma información viaja en el *storage buffer* (que lee el fragmento) y en el búfer de instancias (que lee el vértice), en dos formatos distintos. — impacto: bajo

#### [`2026-10-21_ventana-obj-camino-rapido.md`](propuestas/2026-10-21_ventana-obj-camino-rapido.md)

- El atajo por tramos sigue sin componer la máscara de la ventana de sprites: con el arreglo esas líneas van píxel a píxel, pero se podría componer por tramos con la máscara y recuperar parte del rendimiento. Es la mejora que quedó anotada en el §2 de esta propuesta. — impacto: medio
- `lineas_efecto_alpha` ya no cuenta las líneas con alpha real que además tienen la ventana de sprites, así que el reparto por modo del efecto no cubre esas líneas. — impacto: bajo
- La ventana de sprites sigue sin estar soportada por hardware en el shim de GPU del port (`src/ppu_shim.rs`), que es un camino distinto del de la PPU por software. — impacto: bajo
- Al auditar las rutas absolutas se vio que el hallazgo del BACKLOG que accuse la ruta local `C:/Users/<usuario>/Downloads/...` en `docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md` ya no se cumple: ese archivo no contiene ninguna ruta de usuario. — impacto: bajo

#### [`2026-10-22_arnes-de-experimentos-comun.md`](propuestas/2026-10-22_arnes-de-experimentos-comun.md)

- `ppu_banda_nativa`, `shim_capa_x`, `shim_vram_bg` y otros conservan campos (`Pixel.idx`, `Replica.entrada`) que nunca se leen. Son advertencias anteriores al cambio y arreglarlas obliga a tocar sus `main`. Impacto: bajo. — impacto: bajo
- El recuento de tests del checklist de `AGENTS.md` decía 264 cuando ya había 285. Corregido el 2026-10-22: ahora dice 285 (184 en `src/` + 101 en `tests/`). — impacto: bajo

#### [`2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md)

- El defecto vive en el parche local (`../gba-recomp`), cuyo trabajo sigue sin commitear: el APK lo lleva por `[patch]`, así que un clon limpio compila el upstream fijado, que no tiene camino rápido y, por tanto, no tiene este defecto. — impacto: medio
- La PPU nativa del port (`src/ppu_nativa.rs`) compone siempre por píxel en la GPU: no sufre este defecto, pero tampoco se benefica del atajo por tramos, así que su coste por fotograma es el de la composición completa. — impacto: bajo

#### [`2026-10-22_contrato-de-telemetria.md`](propuestas/2026-10-22_contrato-de-telemetria.md)

- La línea `# ppucamino` repite la clave `rectas` con dos significados: el número absoluto de líneas por la vía rápida y, entre paréntesis al final, el porcentaje. El analizador lo resuelve cortando la línea por el paréntesis (`split("(")[0]`), así que la clave no es unívoca y ningún esquema podría declararla tal cual. — impacto: medio
- Las otras tres líneas `#` (`ppu_hw`, `regiones`, `ppucamino`) también tienen su contrato escrito a mano, en el escritor de Rust y en el informe de Python. No pierden datos (el lector las barre por clave), pero añadir un campo obliga a tocar los dos sitios sin que nada avise: es el mismo problema que esta propuesta resuelve para la línea `t=`. — impacto: medio
- La cabecera `# backend=… modo=… ppu=recto-N` no la lee el analizador: no hay rama que la case y sus campos no llegan al informe. Además `ppu=recto-N` es la versión del camino de composición de la PPU, no la del formato de la telemetría: son dos cosas distintas que se parecen, y el esquema no debe confundirlas. — impacto: bajo
- `blazing-perf.txt` se trunca en cada arranque (`File::create` en `init_perf_file`), así que no queda histórico entre sesiones: para comparar dos sesiones hay que copiar el archivo a mano antes de reiniciar la aplicación. — impacto: bajo
- El informe comparativo (`--comparar`) sigue mostrando solo los promedios: no incluye el «peor fotograma» que ahora calcula el lector, así que comparar dos sesiones no dice si el tirón aislado mejoró. — impacto: bajo
- El conteo de tests del checklist de `AGENTS.md` §6 hay que actualizarlo a mano en cada cambio que añada tests, y así se ha desfasado varias veces (corregido el 2026-10-22 a 285: 184 en `src/` + 101 en `tests/`; el número volverá a desfasarse). — impacto: bajo


#### [2026-10-22_sincronizar-referencia-visor-y-temblor.md](propuestas/2026-10-22_sincronizar-referencia-visor-y-temblor.md)

- Corrección de sincronización de scanline / multiplexado en el shader WGSL para efectos de scroll en línea (impacto medio): el visor ahora detecta y explica con precisión matemática en el `.txt` cada caso de vibración o cartel de unidad, pero la renderización por scanlines en la GPU nativa formará parte de una propuesta de renderizado posterior una vez clasificados los patrones observados. — impacto: medio

#### [2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md](propuestas/2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md)

- `src/ppu_nativa.rs`: la subida de sprites (`sprites_buffer` e instancias) no es diferencial; son pocos bytes, impacto: `bajo`. — impacto: sin declarar
- `src/ppu_nativa.wgsl`: en `fs_comp`, la indexación dinámica `cfg_capa.capas[b]` y `prio[b]` dentro del bucle anidado `4 × 4` sobre una estructura local de `272 bytes` (`ConfigCapa`) fuerza *register spilling* a memoria privada (*stack*) en GPUs móviles como la Mali-G31 (OpenGL ES 3.0 no permite indexar registros con variables no constantes), lo que domina el tiempo de ejecución del fragment shader y bloquea `present` en dos intervalos de VSync (`~31 ms`), impacto: `alto`. — impacto: sin declarar

#### [2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md](propuestas/2026-10-23_ppu-nativa-lut-vertical-y-composicion-directa.md)

- En el motor base `gba-core` (`crates/gba-core/src/ppu.rs`, línea 976), la clave rápida `clave_de_tramo` calcula `mascara | if prioridad != 0xFF { 0x100 | prioridad as u16 } else { 0 }` sin desplazar `prioridad`, por lo que un sprite con `prioridad = 2` activa el bit 1 (`0x0002`), colisionando con el bit de opacidad de `BG1` en `mascara` durante `compose_alpha_tramos` (causa de los 77 píxeles de diferencia en el bocadillo `¡Aquí! ▼` del `cuadro 9254`, donde la PPU nativa dibuja correctamente el borde del globo encima del retrato y la PPU por software de `gba-core` pisa el borde). No se modifica `gba-core` por la restricción de congelación de `tests/datos/ppu-nativa-hashes-base.txt` (`impacto: bajo`). — impacto: bajo

#### [2026-10-24_resolucion-tercera-auditoria-rant-md.md](propuestas/2026-10-24_resolucion-tercera-auditoria-rant-md.md)

- **Captura real del blob binario de IWRAM (32 KB):** La captura del volcado en memoria viva para recompilar las 10 subrutinas de combate (`0x030035C8`, etc.) es un desarrollo técnico sustantivo de instrumentación en el arnés de ejecución determinista registrado en [`docs/PENDIENTES.md`](../PENDIENTES.md). — impacto: medio
- **Selector SAF interactivo en Android:** El reemplazo del acceso directo a `/sdcard/Download/` por un selector gráfico `ACTION_OPEN_DOCUMENT` es una tarea de UI nativa Android registrada en [`docs/PENDIENTES.md`](../PENDIENTES.md). — impacto: bajo

## Checklist de Pre-Commit

Los 5 criterios obligatorios antes de un `commit`/`push` están en [`AGENTS.md`](../AGENTS.md#checklist-obligatorio-de-pre-commit-gatekeeper) §7. Aquí
solo el resumen: suite en verde, validación Diátaxis sin errores, referencias de API sincronizadas,
auditoría de labels Aprobada y auditoría de rutas absolutas limpia.
