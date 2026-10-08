# Backlog del Proyecto — Blazing Recomp

Hoja de ruta, tareas pendientes y seguimiento de hitos para el port nativo de **Fire Emblem: The Blazing Blade** en **Android**.

---

## Estado del Proyecto

- **Fase Actual:** Fase 1 — Configuración base y verificación del motor.
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
- [x] 🟩 Redactar [`docs/ARQUITECTURA_C.md`](ARQUITECTURA_C.md) documentando el modelo de ejecución, registros `c[0..16]`, `RtApi` y análisis de bloques reales de FE7.
- [x] 🟩 Crear y validar [`blazing.labels.toml`](../blazing.labels.toml) enriquecido con símbolos base de FE7 (`Reset_Handler`, `Battle_MainLoop`, `Unit_InitStats`, etc.).
- [x] 🟩 Generar documentación de referencia técnica Diátaxis en [`docs/referencia/`](referencia/README.md) ([`CONFIGURACION_PACK.md`](referencia/CONFIGURACION_PACK.md) y [`FORMATO_LABELS.md`](referencia/FORMATO_LABELS.md)).


---

### Fase 3: Arquitectura y Runtime Android (NDK & Rust)
- [ ] ⬜ Diseñar estructura del proyecto Android (`android/` o crate nativo con `android-activity`).
- [ ] ⬜ Configurar `Cargo.toml` para compilar biblioteca dinámica (`cdylib` / `.so`) para `aarch64-linux-android`.
- [ ] ⬜ Implementar ciclo de vida de aplicación Android (`on_create`, `on_resume`, `on_pause`, `on_destroy`).
- [ ] ⬜ Probar compilación con `cargo ndk -t arm64-v8a build`.

---

### Fase 4: Renderizado y Presentación Gráfica
- [ ] ⬜ Adaptar backend de renderizado de `screen` / `wgpu` a `ANativeWindow` de Android (Vulkan / OpenGL ES 3.0).
- [ ] ⬜ Implementar escalado de aspecto fijo (3:2) con bandas laterales limpias y escalado de píxeles nítido (*integer scaling*).
- [ ] ⬜ Adaptar shaders de simulación LCD y paleta de color (LUT) para GPUs móviles (Adreno / Mali).
- [ ] ⬜ Probar sincronización vertical (VSync / Choreographer a 60 FPS estables).

---

### Fase 5: Sistema de Entrada y Controles Táctiles
- [ ] ⬜ Diseñar superposición táctil (*touch overlay*) en pantalla:
  - D-Pad virtual (Cruceta direccional).
  - Botones A, B, L, R, Start, Select.
  - Botón de acceso rápido al menú.
- [ ] ⬜ Mapear eventos táctiles Android (`AMotionEvent`) a las teclas de la GBA (`KEYINPUT` en `gba-core`).
- [ ] ⬜ Soporte para mandos físicos Bluetooth / USB (Gamepads Xbox, PlayStation, 8BitDo).
- [ ] ⬜ Respuesta háptica (vibración sutil al pulsar botones táctiles).

---

### Fase 6: Audio y Motor de Sonido
- [ ] ⬜ Integrar salida de audio nativa de Android mediante AAudio / Oboe de baja latencia.
- [ ] ⬜ Conectar el motor de audio M4A/MP2K de *Fire Emblem* con el *shadow mixer* de alta fidelidad.
- [ ] ⬜ Gestión de foco de audio (pausar al recibir llamadas o pasar la app a segundo plano).

---

### Fase 7: Almacenamiento, Partidas y Distribución
- [ ] ⬜ Sistema de guardado y persistencia de partidas (`.sav`) en el almacenamiento privado de la app (`Scoped Storage`).
- [ ] ⬜ Flujo de primera ejecución:
  - Selector de archivos para que el usuario elija su ROM y BIOS.
  - Validación de hashes SHA-256 antes de permitir iniciar el juego.
- [ ] ⬜ Menú en juego (pausa, configuración de controles, audio y filtros visuales).
- [ ] ⬜ Generación de APK firmado para instalación directa en dispositivos Android.
