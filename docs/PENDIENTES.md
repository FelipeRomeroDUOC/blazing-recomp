# Tareas Pendientes y Deuda Técnica — Blazing Recomp

Catastro exhaustivo de tareas pendientes, refactorizaciones y deuda técnica real/accionable para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**, consolidado a partir de las propuestas y la telemetría del proyecto.

> **Criterio de inclusión:** Este documento registra únicamente **tareas accionables** (código por escribir, bugs por resolver o refactors pendientes). Los aprendizajes técnicos, mediciones empíricas y limitaciones intrínsecas de hardware residen en [`docs/BACKLOG.md`](BACKLOG.md) y [`docs/explicacion/DECISIONES.md`](explicacion/DECISIONES.md).

---

## 1. Audio Nativo (Fase 2 / v0.2.0 del Roadmap — Bloque Crítico)

- [ ] **Backend de audio nativo con AAudio / Oboe:** Diseñar e implementar la salida de audio de baja latencia en Android enlazada al buffer circular del APU de GBA.
- [ ] **Eliminar cálculo en vacío del APU:** Condicionar `pump_audio` para que no ejecute 65.536 veces por segundo la síntesis de audio si el backend de salida no está activo.
- [ ] **Gestión de foco de audio (`AudioFocus`):** Pausar o atenuar el audio automáticamente cuando entra una llamada telefónica, cambian los auriculares o la aplicación pasa a segundo plano.
- [ ] **Sincronización de audio y video:** Coordinar el reloj del APU con el framerate nativo a 60 FPS para evitar distorsiones de buffer (*underruns* / *overflows*).

---

## 2. Almacenamiento, Partidas y Distribución (Fase 1 / v0.1.0 MVP y Fase 2 / v0.3.0)

- [ ] **Persistencia de partidas (`.sav`):** Conectar la memoria de guardado (SRAM / Flash de 64K y 128K) de la GBA con el almacenamiento interno de la aplicación (`filesDir`) para guardar y cargar el progreso de la partida en archivos `.sav`.
- [ ] **Selector interactivo de ROM y BIOS en UI:** Crear una interfaz gráfica en la pantalla de error/inicio para que el usuario pueda examinar carpetas y seleccionar sus archivos `.gba` y `.bin` si no están en `Downloads/`.
- [ ] **Importación y exportación de partidas:** Flujo en Android Storage Access Framework (SAF) para respaldar o transferir partidas guardadas hacia y desde el almacenamiento del usuario.
- [ ] **Menú en juego:** Menú superpuesto (*in-game menu*) con opciones de pausa, reinicio de partida y selección de escala de pantalla.

---

## 3. Entrada y Mandos Físicos (Fase 2 / v0.4.0 del Roadmap)

- [ ] **Soporte de Gamepads físicos (Bluetooth y USB-C):** Mapear los botones del mando mediante la API de Gamepad de Android y los eventos `KeyEvent` / `MotionEvent` de `android-activity`.
- [ ] **Auto-desactivación del overlay táctil:** Ocultar dinámicamente los controles virtuales en pantalla al detectar actividad de un mando físico externo conectado.
- [ ] **Configuración y remapeo de botones:** Pantalla de configuración para personalizar la asignación de botones tanto en mandos físicos como en la interfaz táctil.

---

## 4. Recompilación AOT y Rendimiento de CPU (Fase 1 / v0.1.0 MVP)

- [ ] **Cobertura AOT de rutinas de combate en IWRAM:** Capturar el blob de 32 KB de memoria viva de IWRAM (`iwram_dump.bin`) durante el arranque/combate mediante el arnés de ejecución determinista (`examples/ppu_barrido_nativa.rs --replay secuencias/combate.txt`, volcando `0x03000000`–`0x03007FFF`) para suministrarlo a `recomp build`. Esto permitirá calcular los hashes FNV-1a de 64 bits y emitir los bloques protegidos por `a->guard(m, addr, len, hash)`. Tras ello, mapear en [`blazing.labels.toml`](../blazing.labels.toml) las 10 subrutinas reubicadas dinámicamente por el juego identificadas en la telemetría como hotspots (`0x030035C8`, `0x030036CC`, etc.), eliminando caídas al intérprete y alcanzando el 100 % de ejecución AOT nativa (0 intérprete).
- [ ] **Auditar cobertura de etiquetas en runtime:** Clarificar la relación entre las 18.483 funciones canónicas catalogadas en [`blazing.labels.toml`](../blazing.labels.toml) (cosecha preliminar de 17.207 funciones) y los 144.875 bloques básicos emitidos en `src-c/` (`rcg_blocks[]`), garantizando la cobertura AOT completa de todo el código recompilado frente a caídas al intérprete.
- [ ] **Optimización de llamadas de memoria cruzada C-Rust:** Evaluar caminos directos para lecturas/escrituras de IWRAM/EWRAM que eviten el salto por la vtable en funciones críticas.

---

## 5. Renderizado GPU y PPU Nativa (Fase 0 / v0.0.2 y Fase 1 / v0.1.0 MVP)

- [ ] **Desactivación de la sonda de calibración de arranque:** Apagar la constante `CALIBRAR_PRESENTACION` tras concluir los experimentos en TV Stick para eliminar los 3 segundos de mediciones y artefactos en el arranque.
- [ ] **Liberación de memoria en cambio en caliente:** Liberar las texturas de la PPU nativa si el usuario activa el archivo de marca `ppu-nativa-off.flag`.
- [ ] **Poda dinámica de búferes de sprites:** Dimensionar el búfer de instancias de la GPU en función del recuento real de sprites activos en lugar de reservar siempre el tamaño fijo para 128 sprites.
- [ ] **Soporte de ventana OBJ en caminos optimizados:** Ampliar la evaluación de máscaras complejas si alguna escena secundaria lo requiere.

---

## 6. Higiene de Pruebas, Herramientas y Documentación

- [ ] **Eliminación de intermitencia en tests de worker:** Corregir el límite de tiempo de 500 ms en `test_emulation_worker_lifecycle_and_frame_generation` ([`src/pipeline_tests.rs`](../src/pipeline_tests.rs)) para que sea determinista bajo alta concurrencia de CPU.
- [ ] **Saneamiento de métricas en `DECISIONES.md`:** Actualizar cifras desfasadas de recuento de pruebas, tamaño del instalador APK (39 MB vs 42.5 MB) y frecuencia de telemetría.
- [ ] **Histórico continuo en telemetría:** Evitar truncar el archivo de telemetría de rendimiento en cada inicio de la aplicación o añadir rotación de logs para facilitar comparativas A/B entre sesiones.
- [ ] **Enriquecimiento del comparador de telemetría:** Incorporar el peor fotograma aislado (*frame drop spike*) al informe `--comparar` de `scripts/diagnostico/analizar_telemetria.py`.
