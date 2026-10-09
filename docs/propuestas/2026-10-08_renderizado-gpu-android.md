# Propuesta: Renderizado GPU y Presentación en Android con wgpu y gba-recomp

Fecha: 2026-10-08
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Integra el motor de presentación gráfica en GPU ([`screen`](../gba-recomp/crates/screen)) en la aplicación Android (`blazing-recomp`), conectando el ciclo de vida de la ventana (`ANativeWindow`) para renderizar fotogramas de GBA en pantalla con aceleración por hardware (Vulkan / GLES 3.0 vía `wgpu`), escalado 3:2 y simulación de pantalla.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `pack.toml`, `docs/BACKLOG.md`, `Cargo.toml`, `src/lib.rs`, `../gba-recomp/crates/screen/Cargo.toml`, `../gba-recomp/crates/screen/src/present.rs`.
- Intención: Habilitar la presentación gráfica acelerada por GPU en dispositivos móviles Android para proyectar el framebuffer de la GBA (240x160) a 60 FPS conservando el aspecto 3:2 centrado.
- Restricciones y decisiones previas (máx. 3):
  - Integración modular con `screen` sin duplicar código (origen: `docs/BACKLOG.md` Fase 4 y respuesta humana, 2026-10-08) → implica: vincular `screen = { path = "../gba-recomp/crates/screen" }` en `Cargo.toml` → protegida por: Compilación cruzada exitosa del pipeline gráfico para ARM64.
  - Gestión segura del ciclo de vida de `ANativeWindow` (origen: `AGENTS.md` §2) → implica: crear el presenter solo al recibir `InitWindow` y destruirlo en `WindowDestroyed` para evitar desreferenciar punteros huérfanos → protegida por: Manejo de eventos de ventana y enlace con raw-window-handle.
  - Portabilidad absoluta sin rutas locales (origen: regla de usuario en `AGENTS.md`) → implica: manifiestos y dependencias de Cargo estrictamente relativos al repositorio → protegida por: Independencia de rutas absolutas en configuración de Cargo.
- Alternativas descartadas:
  - Renderizado por software directo con `ANativeWindow_lock` / `unlockAndPost` CPU → descartada porque no soporta shaders WGSL de rejilla LCD ni corrección de color por hardware a 60 FPS estables.
  - Backend GLES 2.0 manual vía JNI / GLSurfaceView → descartada porque añade sobrecarga de puente Java/Kotlin y rompe la arquitectura NativeActivity en Rust puro.
- Ante imprevistos, prioriza: estabilidad del ciclo de vida de la ventana sobre opciones avanzadas de postprocesado.

### 3. Ejemplos

- Caso normal: Android entrega la ventana nativa mediante el evento `InitWindow` → antes: se ignora el evento y la aplicación no muestra gráficos en pantalla → ahora: se crea el `Presenter` en GPU (`wgpu`), se configura la superficie con las dimensiones de pantalla y se presenta el cuadro con aspecto 3:2 centrado.
- Caso límite: La pantalla se apaga o la app pasa a segundo plano recibiendo `WindowDestroyed` → antes: no existían recursos gráficos que liberar → ahora: `Presenter` se destruye de inmediato liberando la superficie de `wgpu` de forma segura sin causar fallos de segmentación.

### 4. Pasos

1. Añadir dependencias en `Cargo.toml`: `screen = { path = "../gba-recomp/crates/screen" }` y `raw-window-handle = "0.6"`.
2. Crear un struct envoltorio seguro (`AndroidWindow`) que contenga el `NativeWindow` de `ndk` e implemente `raw_window_handle::HasWindowHandle` y `raw_window_handle::HasDisplayHandle`.
3. Actualizar `AppState` en `src/lib.rs` para contener `presenter: Option<screen::present::Presenter>`, `screen_kind: screen::ScreenKind`, `lut: screen::ColorLut`, `temporal: screen::Temporal`, `grid_params: screen::present::GridParams` y el búfer RGBA de presentación.
4. Extender el bucle de eventos en `android_main` para capturar `MainEvent::InitWindow`, `MainEvent::WindowResized`, `MainEvent::RedrawNeeded` y `MainEvent::WindowDestroyed`, orquestando la inicialización, redibujado y destrucción de `Presenter`.
5. Implementar un método de presentación de cuadro que aplique la simulación temporal (`temporal.compose`), corrección LUT y envío al swapchain del `Presenter`.
6. Validar la compilación cruzada ARM64 con `cargo ndk -t arm64-v8a build` y comprobación de tipos con `cargo check --target aarch64-linux-android`.

### 5. Qué más podría verse afectado

- `src/lib.rs`: bucle de eventos y estructura `AppState`.
- `Cargo.toml`: adición de dependencias del workspace.
- `Cargo.lock`: resolución de dependencias de renderizado.
- Búsqueda realizada: referencias a `AppState` y `android_main` en `src/`.

### 6. Suposiciones

- `wgpu 26` y sus backends Vulkan/GLES inicializan correctamente la superficie sobre `ANativeWindow` en Android a través de `raw-window-handle 0.6` → prototipar: sí → si es falso: adaptar las flags de inicialización del `Instance` o crear la superficie explícitamente → resultado del prototipo: | no detecta: incompatibilidades con dispositivos antiguos sin Vulkan 1.1 o GLES 3.0.
- La invocación a `temporal.compose` y `presenter.present` se ejecuta holgadamente dentro del presupuesto de fotograma de 16.7 ms en procesadores ARM64 → prototipar: no → si es falso: desactivar temporalmente el filtro de rejilla pesada en dispositivos de gama baja → resultado del prototipo: | no detecta: sobrecalentamiento sostenido en sesiones prolongadas.

### 7. Alcance

- Incluye: Integración de `screen` y `raw-window-handle 0.6`, implementación del wrapper `AndroidWindow`, gestión de eventos de ventana (`InitWindow`, `WindowResized`, `WindowDestroyed`, `RedrawNeeded`) en `src/lib.rs`, preparación del búfer de presentación y compilación cruzada ARM64 (`aarch64-linux-android`).
- No incluye: Entrada de controles táctiles en pantalla (Fase 5), motor de audio interactivo AAudio/Oboe (Fase 6), ni menú en juego táctil (Fase 7).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Compilación cruzada exitosa del pipeline gráfico para ARM64
Caso de uso: Compilación del motor nativo con aceleración GPU para Android | Tipo: normal
  Dado que `Cargo.toml` incluye `screen` y `raw-window-handle` y `src/lib.rs` integra el `Presenter` de GPU
  Cuando ejecuto `cargo ndk -t arm64-v8a build`
  Entonces el proceso finaliza con código de salida 0 y genera el archivo `target/aarch64-linux-android/debug/libblazing_recomp.so`
Test: `test_arm64_gpu_build`
Estado previo: falla (no están añadidas las dependencias ni el código del presenter)

Escenario: Manejo de eventos de ventana y enlace con raw-window-handle
Caso de uso: Verificación de tipos del ciclo de vida gráfico y compatibilidad de handles de ventana | Tipo: límite
  Dado que `src/lib.rs` implementa `AndroidWindow` con `HasWindowHandle + HasDisplayHandle` y enlaza `InitWindow` y `WindowDestroyed`
  Cuando ejecuto `cargo check --target aarch64-linux-android`
  Entonces la comprobación de tipos y sintaxis termina con código de salida 0
Test: `test_gpu_lifecycle_types`
Estado previo: falla (código no existe)

Escenario: Independencia de rutas absolutas en configuración de Cargo
Caso de uso: Portabilidad del proyecto gráfico en cualquier máquina o entorno de compilación | Tipo: regresión
  Dado que `Cargo.toml` y el código del crate usan exclusivamente rutas y configuración portables
  Cuando se buscan patrones de rutas locales absolutas en los archivos del crate
  Entonces el número de coincidencias de rutas locales absolutas es exactamente 0
Test: `test_no_local_paths_in_graphics`
Estado previo: pasa (regresión preventiva)

### 9. Cómo lo comprobaré

- Compilación cruzada exitosa del pipeline gráfico para ARM64 → herramienta o comando: `cargo ndk -t arm64-v8a build`
  → pasa si: el comando termina con código 0 y genera `libblazing_recomp.so` en `target/aarch64-linux-android/debug/` → no detecta: fallos de driver gráfico específicos de ciertos GPUs en hardware real.
- Manejo de eventos de ventana y enlace con raw-window-handle → herramienta o comando: `cargo check --target aarch64-linux-android`
  → pasa si: retorna código 0 sin errores de tipos ni advertencias → no detecta: cuelgues durante cambios dinámicos de orientación de pantalla.
- Independencia de rutas absolutas en configuración de Cargo → herramienta o comando: `git grep -i "C:/" -- Cargo.toml src/`
  → pasa si: 0 coincidencias → no detecta: rutas absolutas Unix sin letra de unidad.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

*(Se completará si un prototipo altera el plan)*

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

*(Se completará tras la verificación)*

### Contexto: ¿se respetó?

*(Se completará tras la verificación)*

### Revisión humana

- ninguno: todo se comprobó con tests (`cargo ndk -t arm64-v8a build`, `cargo check` y `git grep`).
