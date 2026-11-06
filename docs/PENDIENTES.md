# Tareas Pendientes y Deuda Técnica — Blazing Recomp

Catastro exhaustivo de tareas pendientes, refactorizaciones y deuda técnica real/accionable para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**, consolidado a partir de las propuestas y la telemetría del proyecto.

> **Criterio de inclusión:** Este documento registra únicamente **tareas accionables** (código por escribir, bugs por resolver o refactors pendientes). Los aprendizajes técnicos, mediciones empíricas y limitaciones intrínsecas de hardware residen en [`docs/BACKLOG.md`](BACKLOG.md) y [`docs/explicacion/DECISIONES.md`](explicacion/DECISIONES.md).

---

## 1. Hito MVP Jugable & Showcase (Fase 1 / v0.1.0 del Roadmap — Frente Crítico)

- [x] **Persistencia de partidas (`.sav`):** Resuelto en la propuesta [`2026-10-28_persistencia-guardado-sav.md`](propuestas/2026-10-28_persistencia-guardado-sav.md). Conectada la memoria de guardado (SRAM / Flash de 64K y 128K) de la GBA con el almacenamiento interno de la aplicación (`filesDir`) con reemplazo atómico `.sav.tmp` y autoguardado táctico continuo (datos de suspensión y semilla PRNG) con debounce de 1,5 segundos.
- [ ] **Exportación e importación de partidas (`.sav`) vía SAF:** Implementar integración con el Storage Access Framework de Android para permitir al usuario exportar e importar sus partidas `.sav` desde/hacia almacenamiento compartido externo o emuladores de PC sin depender de ADB ni privilegios root.
- [ ] **Soporte de mandos físicos (Bluetooth y USB-C):** Mapear los botones del mando mediante la API de Gamepad de Android y los eventos `KeyEvent` / `MotionEvent` de `android-activity`.
- [ ] **Auto-desactivación del overlay táctil:** Ocultar dinámicamente los controles virtuales en pantalla al detectar actividad de un mando físico externo conectado.
- [ ] **Menú nativo y remapeo de botones:** Menú superpuesto en pantalla (*in-game menu*) con opciones de pausa, reinicio de partida y pantalla de configuración para personalizar la asignación de botones (táctiles y físicos).
- [ ] **Selector interactivo de ROM y BIOS en UI:** Interfaz gráfica en la pantalla de error/inicio para examinar carpetas y seleccionar archivos `.gba` y `.bin` sin depender de rutas fijas o ADB.
- [ ] **Feature Insignia: Widescreen adaptativo (16:9 y 21:9):** Extender el viewport horizontal en el mapa táctico (Modo 0) manipulando la composición en [`NativePpu`](../src/ppu_nativa.rs) para proyectar más columnas de tiles del mapa en lugar de mostrar bandas negras.
- [ ] **Eliminar espera activa (*busy-waiting*) en el bucle principal:** Reemplazar el sondeo continuo en bucles cerrados de sincronización entre el runner de emulación y el pipeline de renderizado por coordinación determinista con `Condvar` o eventos VSync de `Choreographer`, reduciendo el calentamiento y consumo de batería en Android.

---

## 2. Audio Nativo y Rendimiento (Consolidado en Fase 0 / v0.0.3)

- [x] **Backend de audio nativo con AAudio NDK:** Resuelto en las propuestas [`2026-10-25_motor-audio-nativo-aaudio.md`](propuestas/2026-10-25_motor-audio-nativo-aaudio.md) y [`2026-10-25_audio-mejorado-mp2k-hle.md`](propuestas/2026-10-25_audio-mejorado-mp2k-hle.md) (commit `fb26fe0`). Implementado el backend de baja latencia en `src/audio.rs` con resampler de 65.536 Hz a 48.000 Hz, control dinámico de deriva (DRC), buffer circular SPSC acotado y mezclador flotante MP2K HLE en 32 bits con fallback a audio fiel.
- [x] **Gestión de ciclo de vida y foco de audio (`AudioFocus`):** Pausa y reanudación automática enlazada a las transiciones de `android-activity` y el pipeline de emulación.
- [ ] **Eliminar cálculo en vacío del APU:** Condicionar `pump_audio` para que no ejecute 65.536 veces por segundo la síntesis de audio si el backend de salida no está activo.

---

## 3. Plataforma de Mods y Plugins (Fase 2 / v0.2.0 - v0.4.0)

- [ ] **Arquitectura de plugins / inyección en Rust y C:** Diseñar un sistema de hooks para interceptar llamadas nativas del juego (eventos de combate, cálculo de daño, condiciones de victoria) sin alterar el código C11 base.
- [ ] **Sustitución de assets (texturas y retratos HD):** Pipeline para reemplazar baldosas y retratos por texturas de mayor resolución aprovechando el renderizado en GPU.
- [ ] **Samples de audio HD:** Interfaz en el mezclador MP2K HLE flotante para cargar muestras de audio de mayor resolución (*soundfonts* / muestras PCM limpias).
- [ ] **Shaders LCD/CRT avanzados:** Catálogo de shaders WGSL seleccionables por el usuario (rejilla de píxeles GBA SP, corrección de color y curvatura CRT).
- [ ] **Guardados de estado instantáneos (Savestates) y rebobinado:** Serialización de la memoria completa (IWRAM/EWRAM/VRAM/registros) para congelar y reanudar partidas instantáneamente.

---

## 4. Ecosistema de la Trilogía GBA y Hackroms (Fase 3 / v0.5.0 - v0.8.0)

- [ ] **Soporte de parches de hackroms (IPS / UPS / BPS):** Aplicación de parches en memoria al vuelo al cargar la ROM base.
- [ ] **Compatibilidad con Fire Emblem 6 (The Binding Blade):** Cosecha de etiquetas AOT y adaptación del runtime para FE6.
- [ ] **Compatibilidad con Fire Emblem 8 (The Sacred Stones):** Cosecha de etiquetas AOT y adaptación del runtime para FE8 (soporte del mapa de mundo y split de rutas).
- [ ] **Optimización para pantallas plegables y tablets:** Disposición ergonómica del overlay táctil para relaciones de aspecto 4:3 y pantallas divididas.

---

## 5. Recompilación AOT y Rendimiento de CPU

- [x] **Cobertura AOT de rutinas de combate en IWRAM:** Resuelto en la propuesta [`2026-10-24_cobertura-aot-combate-iwram.md`](propuestas/2026-10-24_cobertura-aot-combate-iwram.md). Catalogadas las 14 subrutinas críticas de combate en IWRAM en [`blazing.labels.toml`](../blazing.labels.toml) y regeneradas las unidades C11 con guardas FNV-1a, eliminando las caídas al intérprete en secuencias de combate.
- [ ] **Auditar cobertura de etiquetas en runtime:** Clarificar la relación entre las 18.483 funciones canónicas catalogadas en [`blazing.labels.toml`](../blazing.labels.toml) y los 144.875 bloques básicos emitidos en `src-c/` (`rcg_blocks[]`), garantizando la cobertura AOT completa de todo el código recompilado frente a caídas al intérprete.
- [ ] **Fase 2 de eliminación de emulación: Bus plano contiguo sin vtable `m.bus` (ADR 41):** Migrar de `m.bus` polimórfico a un layout contiguo `FlatBus` en memoria con punteros directos a EWRAM, IWRAM, VRAM, OAM y PALRAM, eliminando la indirección de vtable y permitiendo al compilador C11 AOT optimizar accesos de memoria como punteros planos.
- [ ] **Aislamiento de la feature `perf-counters` en APK de producción:** Condicionar la compilación de telemetría atómica a flags de depuración para evitar la penalización medida de hasta el 10,4% de CPU en compilaciones de lanzamiento (*release*).

---

## 6. Renderizado GPU y PPU Nativa

- [x] **Desactivación de la sonda de calibración de arranque:** Resuelto en [`docs/propuestas/2026-10-25_fix-congelamiento-arranque-y-doble-pantalla-calibracion.md`](propuestas/2026-10-25_fix-congelamiento-arranque-y-doble-pantalla-calibracion.md). Apagada la constante `CALIBRAR_PRESENTACION` a `false`.
- [x] **Eliminación definitiva del parpadeo en la cinemática del dragón:** Resuelto en las propuestas de sincronía AOT `2026-11-06_carga-aot-windows-y-paridad-ppu-doble` y `2026-11-06_paso-fino-scanlines-visibles-y-chain-gate-raster`. Eliminados tanto el desfasaje de HBlank/VCount IRQ en los bloques C11 como la doble mezcla de fuego; paridad 100% confirmada visualmente en PC con `ppu_doble` (0 parpadeos).
- [ ] **Resolución de divergencias en cinemática del dragón (cuadros 150 y 677):** Corregir la desalineación de 1 píxel en el scroll senoidal vertical ($Y=100..159$) en cuadro 150 y el nivel de brillo BLDALPHA más claro en GPU que en CPU en cuadro 677 por aplicación de tinte previo al blend jerárquico.
- [ ] **Desaliasing de la clave de tramo de 16 bits en la GPU (sprite vs fondo):** Expandir la clave de ordenación `span_key` en `src/ppu_nativa.rs` a 32 bits o desacoplar capas para evitar colisiones de prioridad entre OBJ y BG bajo ventanas de exclusión y efectos de mezcla complejos.
- [ ] **Liberación de memoria en cambio en caliente:** Liberar las texturas de la PPU nativa si el usuario activa el archivo de marca `ppu-nativa-off.flag`.
- [ ] **Poda dinámica de búferes de sprites:** Dimensionar el búfer de instancias de la GPU en función del recuento real de sprites activos en lugar de reservar siempre el tamaño fijo para 128 sprites.
- [ ] **Soporte de ventana OBJ en caminos optimizados:** Ampliar la evaluación de máscaras complejas si alguna escena secundaria lo requiere.

---

## 7. Higiene de Pruebas, Herramientas y Documentación

- [ ] **Hermetismo de dependencias: Desvinculación del `[patch]` local hacia `../gba-recomp`:** Consolidar los submódulos o dependencias nativas de `gba-core` para que el proyecto pueda compilarse de forma autónoma y reproducible sin requerir un clon simultáneo en el directorio padre.
- [ ] **Campaña de validación física en hardware ("Esperando el televisor" — 96 propuestas):** Ejecutar la verificación visual humana directa en dispositivo físico / pantalla de televisión para las 96 propuestas cerradas por agente que requieren confirmación estética final (colorimetría, nitidez y sincronización VSync).
- [ ] **Eliminación de intermitencia en tests de worker:** Corregir el límite de tiempo de 500 ms en `test_emulation_worker_lifecycle_and_frame_generation` ([`src/pipeline_tests.rs`](../src/pipeline_tests.rs)) para que sea determinista bajo alta concurrencia de CPU.
- [ ] **Saneamiento de métricas en `DECISIONES.md`:** Actualizar cifras desfasadas de recuento de pruebas, tamaño del instalador APK (39 MB vs 42.5 MB) y frecuencia de telemetría.
- [ ] **Histórico continuo en telemetría:** Evitar truncar el archivo de telemetría de rendimiento en cada inicio de la aplicación o añadir rotación de logs para facilitar comparativas A/B entre sesiones.
- [ ] **Enriquecimiento del comparador de telemetría:** Incorporar el peor fotograma aislado (*frame drop spike*) al informe `--comparar` de `scripts/diagnostico/analizar_telemetria.py`.
- [ ] **Refinamiento de herramientas locales de visualización e inspección en PC:** Ampliar las herramientas de diagnóstico en PC para facilitar el acceso directo a la visualización y al estado del juego, permitiendo sustentar hipótesis con evidencia objetiva antes de modificar código.
