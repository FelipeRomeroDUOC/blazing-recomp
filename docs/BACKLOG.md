# Backlog del Proyecto — Blazing Recomp

Hoja de ruta, tareas pendientes y seguimiento de hitos para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**.

---

## Estado del Proyecto

- **Fase Actual:** Fase 5 — Sistema de entrada y controles táctiles.
- **ROM Objetivo:** `Fire Emblem (Europe)` (`6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`)
- **BIOS Objetivo:** `gba_bios.bin` (`fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`)

---

## Leyenda de Estados

- 🟩 **Completado:** Tarea finalizada y verificada.
- 🟨 **En Progreso:** Trabajo activo en curso.
- ⬜ **Pendiente:** Planificado para próximas iteraciones.
- 🛑 **Bloqueado:** Requiere resolución de dependencia previa.

---

## Fases y Tareas

### Fase 1: Entorno de Desarrollo y Configuración Base
- [x] 🟩 Inicializar repositorio `blazing-recomp` con Git y `.gitignore` seguro.
- [x] 🟩 Redactar [`AGENTS.md`](../AGENTS.md) con arquitectura, contexto y plataforma objetivo (Android).
- [x] 🟩 Integrar y documentar skills del repositorio ([`.agents/skills/`](../.agents/skills/)).
- [x] 🟩 Instalar Android NDK (`30.0.16248370`) y configurar `ANDROID_HOME` / `ANDROID_NDK_HOME`.
- [x] 🟩 Instalar Rust (`rustc`/`cargo` 1.99.0) y añadir targets Android (`aarch64-linux-android`, `x86_64-linux-android`).
- [x] 🟩 Instalar `cargo-ndk` v4.1.2 para compilación cruzada.
- [x] 🟩 Realizar primera compilación de prueba en PC para generar código C11 de *Fire Emblem* y verificar pipeline.

---

### Fase 2: Mapeo y Cobertura de Código (Modelo Híbrido con Intérprete)
- [x] 🟩 Decisión arquitectónica: Adoptar modelo híbrido (99%+ C recompilado estático con intérprete como red de seguridad `interpreter = true` en `pack.toml`).
- [x] 🟩 Redactar [`docs/explicacion/arquitectura-c-recompilado.md`](explicacion/arquitectura-c-recompilado.md) documentando el modelo de ejecución, registros `c[0..16]`, `RtApi` y análisis de bloques reales de FE7.
- [x] 🟩 Crear y validar [`blazing.labels.toml`](../blazing.labels.toml) enriquecido con símbolos base de FE7 (`Reset_Handler`, `Battle_MainLoop`, `Unit_InitStats`, etc.).
- [x] 🟩 Generar documentación de referencia técnica Diátaxis en [`docs/referencia/`](referencia/README.md) ([`CONFIGURACION_PACK.md`](referencia/CONFIGURACION_PACK.md) y [`FORMATO_LABELS.md`](referencia/FORMATO_LABELS.md)).

---

### Fase 3: Arquitectura y Runtime Android (NDK & Rust)
- [x] 🟩 Diseñar estructura del proyecto Android (crate nativo en Rust con `android-activity`).
- [x] 🟩 Configurar `Cargo.toml` para compilar biblioteca dinámica (`cdylib` / `.so`) para `aarch64-linux-android` e integrar `gba-core`.
- [x] 🟩 Implementar ciclo de vida de aplicación Android (`android_main`, `Resume`, `Pause`, `Destroy`, suspensión limpia de memoria).
- [x] 🟩 Probar compilación con `cargo ndk -t arm64-v8a build` (generó `libblazing_recomp.so` ELF ARM64 de 29.5 MB con éxito).

---

### Fase 4: Renderizado y Presentación Gráfica
- [x] 🟩 Adaptar backend de renderizado de `screen` / `wgpu` a `ANativeWindow` de Android (Vulkan / OpenGL ES 3.0).
- [x] 🟩 Implementar escalado de aspecto fijo (3:2) con bandas laterales limpias y escalado de píxeles nítido (*integer scaling*).
- [x] 🟩 Integrar shaders de simulación LCD y paleta de color (LUT) para GPUs móviles (Adreno / Mali).
- [x] 🟩 Configurar sincronización vertical (VSync activo con pacing de presentación desacoplado a 60 FPS).
- [x] 🟩 Conectar pipeline de vídeo de GBA (`Machine` PPU -> `Temporal` -> `ColorLut` -> `Compositor`) mediante módulo puro `GameRunner` y bucle a 60 FPS con suspensión pasiva de batería.
- [x] 🟩 Probar compilación con `cargo ndk -t arm64-v8a build` (generó `libblazing_recomp.so` ELF ARM64 de 134.3 MB con éxito).

---

### Fase 5: Sistema de Entrada y Controles Táctiles
- [x] 🟩 Fase 5.1: Mapeo táctil ergonómico y respuesta háptica ([`docs/propuestas/2026-10-08_controles-tactiles-haptica.md`](propuestas/2026-10-08_controles-tactiles-haptica.md)):
  - D-Pad virtual con cuenca circular y zona muerta central.
  - Clúster ergonómico diagonal GBA SP: L y R sobre B y A inclinados a 16° (eje B→A paralelo).
  - Botones Select y Start centrados en la base.
  - Geometría adaptativa según orientación (Portrait maximizado en mitad inferior, Landscape en franja base).
  - Opción de desactivación manual (`touch_enabled`).
  - Mapeo de eventos táctiles Android (`MotionEvent`) a la máscara `bus.keys` de `gba-core` (`KEYINPUT`).
  - Respuesta háptica por micro-vibración (~15 ms) vía JNI sobre `Vibrator` con tolerancia a fallos.
- [x] 🟩 Fase 5.2: Renderizado visual del overlay en pantalla ([`docs/propuestas/2026-10-09_renderizado-overlay-tactil.md`](propuestas/2026-10-09_renderizado-overlay-tactil.md)):
  - Compositor GPU unificado en `wgpu` (marco de juego 3:2 + overlay táctil en un solo swapchain).
  - Shaders WGSL procedurales SDF sin texturas rasterizadas (bordes nítidos a cualquier DPI).
  - Colores temáticos GBA SP en Portrait (B rojo, A verde, L/R blanco, carcasa plateada en mitad inferior).
  - Modo Landscape translúcido para visibilidad de la acción de juego.
  - Retroalimentación visual inmediata al pulsar botones (resaltado y anillo activo).
- [ ] ⬜ Fase 5.3: Soporte para mandos físicos Bluetooth / USB (Gamepads Xbox, PlayStation, 8BitDo) y auto-desactivación táctil.

---

### Fase 6: Audio y Motor de Sonido
- [ ] ⬜ Integrar salida de audio nativa de Android mediante AAudio / Oboe de baja latencia.
- [ ] ⬜ Conectar el motor de audio M4A/MP2K de *Fire Emblem* con el *shadow mixer* de alta fidelidad.
- [ ] ⬜ Gestión de foco de audio (pausar al recibir llamadas o pasar la app a segundo plano).

---

### Fase 7: Almacenamiento, Partidas y Distribución
- [ ] ⬜ Sistema de guardado y persistencia de partidas (`.sav`) en el almacenamiento privado de la app (`Scoped Storage`).
- [x] 🟩 Detección automática amigable en carpeta `Downloads/` y validación criptográfica de hashes SHA-256 de ROM y BIOS ([`docs/propuestas/2026-10-10_ingestion-rom-bios-downloads.md`](propuestas/2026-10-10_ingestion-rom-bios-downloads.md)).
- [ ] ⬜ Menú en juego (pausa, configuración de controles, audio y filtros visuales).
- [x] 🟩 Arnés de empaquetado y generación de APK firmado para instalación directa en dispositivos Android ([`docs/propuestas/2026-10-09_arnes-empaquetado-apk.md`](propuestas/2026-10-09_arnes-empaquetado-apk.md)).
