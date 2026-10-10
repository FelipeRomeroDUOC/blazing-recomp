# Propuesta: Optimización de Rendimiento y Compatibilidad Multidispositivo (Móvil, BlueStacks, Tablet y Android TV)

Fecha: 2026-10-10
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resolución integral de las cuatro incidencias reportadas en el parque de dispositivos de prueba:
1. **Móvil (Snapdragon 7 Gen 3):** Reubicación ergonómica del badge de FPS con márgenes adaptativos de *Safe Area* (insets contra esquinas redondeadas y notch de smartphones modernos). Incorporación de indicador de modo de ejecución (`AOT` nativo C11 vs `INT` intérprete) en el badge para telemetría visual inmediata.
2. **BlueStacks (Android 11):** Eliminación del `thread::sleep(1ms)` perjudicial en el bucle principal cuando VSync está habilitado, estabilizando la presentación a 60 FPS exactos y erradicando el doble avance de casilla en la cruceta (doble input por acumulación de steps).
3. **Tablet (Android 11):** Optimización de fillrate de GPU en [`src/overlay.wgsl`](../../src/overlay.wgsl) mediante poda de fragmentos (early discard) de la carcasa SP fuera de las hitboxes activas en portrait, reduciendo drásticamente la carga de sombreado en paneles de alta resolución (2K/WQXGA). Garantía de carga de biblioteca AOT C11 con target NDK API 26 y rutas canónicas.
4. **Android TV:** Aislamiento de la BIOS oficial en [`src/recomp.rs`](../../src/recomp.rs) ejecutando las instrucciones `key < 0x4000` exclusivamente en el intérprete cuando `real_bios` esté activo (siguiendo la especificación upstream de `gba-recomp`), resolviendo el congelamiento en el inicio. Soporte nativo de eventos de control remoto y mando físico (`InputEvent::KeyEvent`).

---

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs), [`src/recomp.rs`](../../src/recomp.rs), [`src/fps.rs`](../../src/fps.rs), [`src/overlay.wgsl`](../../src/overlay.wgsl), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), [`AGENTS.md`](../../AGENTS.md).
- Intención: Brindar una experiencia fluida a 60 FPS estables y controles fiables en todo el espectro de hardware (teléfonos modernos con notch, emuladores Android 11, tablets de gran pantalla y Android TV con mando).
- Restricciones y decisiones previas (máx. 3):
  - Prioridad 0-Interpreter en ROM y fidelidad upstream en BIOS (origen: `AGENTS.md §4.2` y arquitectura `gba-recomp` upstream): El código del juego en ROM (`key >= 0x08000000`), IWRAM y EWRAM se ejecuta con los 17,207 bloques nativos AOT; la BIOS real de 16 KB con verificaciones de cabecera criptográfica se ejecuta en el intérprete paso a paso. → protegida por: `test_run_frame_native_preserves_real_bios_stepping`.
  - Pacing VSync libre de busy-waiting y sleep impreciso (origen: `AGENTS.md §2.5` rendimiento y batería): La sincronización debe coordinarse con el swapchain de GPU (`desired_maximum_frame_latency: 1` con `PresentMode::AutoVsync`/`Fifo`) sin invocar `thread::sleep(1ms)`, evitando caídas de VSync a 30 Hz. → protegida por: `test_pacing_without_drift`.
  - Soporte universal de mandos de Android TV (origen: `AGENTS.md §2.4` mandos físicos): Mapeo bidireccional de `KeyEvent` sin alterar el sistema multitáctil en pantallas táctiles. → protegida por: `test_key_event_mapping_to_gba_buttons`.
- Alternativas descartadas:
  - Bajar la resolución de renderizado global en tablets con un framebuffer intermedio escalado: Descartada inicialmente en favor de optimizar las ramas y bounding boxes del shader procedural, preservando la nitidez nativa del arte vectorial.
  - Eliminar por completo el soporte de BIOS física: Descartada porque `AGENTS.md §3` define el soporte oficial de `gba_bios.bin` (SHA-256 verificado).

---

### 3. Ejemplos

- Caso Móvil (pantalla curva/punch-hole):
  - Antes: El contador de FPS está pegado a 16 px del borde derecho y 14 px del superior, quedando oculto tras el bisel físico curvado de la pantalla OLED.
  - Ahora: El badge se posiciona con un margen seguro adaptativo (48 px en horizontal y 24 px en vertical en landscape; 24 px y 36 px en portrait) y muestra `60 FPS · AOT`, totalmente visible y sin interferir con la cruceta ni botones.
- Caso BlueStacks (Android 11):
  - Antes: `thread::sleep(1ms)` provoca pérdida sistemática del ciclo de VSync a 60 Hz; la app cae a 30 FPS en el swapchain y ejecuta 2 steps lógicos por cada render, provocando que tocar la cruceta mueva el cursor 2 casillas ("doble input").
  - Ahora: Eliminado el sleep artificial; el bucle sincroniza limpiamente a 60 FPS (1 step por render) y cada toque de cruceta avanza exactamente 1 casilla determinista.
- Caso Android TV:
  - Antes: Con `gba_bios.bin` presente, el juego intenta ejecutar bloques recompilados dentro de la BIOS (`key < 0x4000`) y se congela en el logo; además, el control remoto y mandos físicos no responden porque `KeyEvent` no se procesa.
  - Ahora: La BIOS real ejecuta su secuencia de validación en el intérprete y transiciona fluidamente al juego a 60 FPS en código C11 AOT; el control remoto de TV (flechas y botón OK) y mandos de consola controlan el juego a la perfección.
- Caso Tablet (Portrait vs Landscape):
  - Antes: En portrait, el shader evalúa SDFs 3D volumétricos en toda la mitad inferior de una pantalla 2K/WQXGA, hundiendo los FPS a 11.
  - Ahora: El shader poda agresivamente los píxeles que no intersecan las hendiduras activas mediante early reject AABB; la tasa de refresco sube y el badge indica con certeza si la biblioteca AOT está activa.

---

### 4. Pasos

1. **[`src/recomp.rs`](../../src/recomp.rs):**
   - En `run_frame_native`, incorporar la guarda upstream:
     ```rust
     if m.bus.real_bios && key < 0x4000 {
         m.step();
         continue;
     }
     ```
   - Enriquecer `BlockTable` y `load_native_blocks` para exponer si el motor corre en modo nativo AOT (`is_native() == true`).
2. **[`src/platform.rs`](../../src/platform.rs):**
   - En el bucle de eventos, agregar procesamiento de `InputEvent::KeyEvent`: mapear `KeyAction::Down` y `KeyAction::Up` con códigos estándar de Android (`KEYCODE_DPAD_*`, `BUTTON_A`, `BUTTON_B`, `BUTTON_START`, `BUTTON_SELECT`, `ENTER`, `BACK`) a la máscara de teclas de la máquina virtual.
   - En el bucle de sincronización de fotogramas, eliminar el bloque `thread::sleep(Duration::from_millis(1))` que provocaba la caída a 30 FPS en Android 11.
   - En el cálculo de posición del HUD de FPS, aplicar safe-area insets: `margin_x = 48.0` en landscape (o `24.0` en portrait) y `margin_y = 20.0` (o `36.0` en portrait).
3. **[`src/fps.rs`](../../src/fps.rs):**
   - Ampliar el formato del badge a `format!("{fps_val} FPS · {mode}")` (donde `mode` es `AOT` o `INT`), adaptando el ancho del badge a 104x26 px con tipografía JetBrains Mono de `assets/fonts/`.
4. **[`src/overlay.wgsl`](../../src/overlay.wgsl):**
   - Optimizar el sombreador de fragmentos: aplicar poda de cajas envolventes (AABB early exit) en el bloque de la carcasa SP en portrait, evitando cálculos de biseles y cuencas 3D en áreas neutras.
5. **[`scripts/package_apk.ps1`](../../scripts/package_apk.ps1):**
   - Configurar Clang para enlazar con `--target=...26` (`aarch64-linux-android26`, `armv7a-linux-androideabi26`, `x86_64-linux-android26`), alineado con el `minSdkVersion 26` del manifiesto.
6. **Validación y Pruebas:**
   - Crear tests de unidad para el mapeo de `KeyEvent` y el aislamiento de la BIOS real.
   - Ejecutar la suite completa (`cargo test`), validación Diátaxis y extracción de referencias.
   - Generar el APK empaquetado multi-ABI oficial con `scripts/package_apk.ps1`.

---

### 5. Qué más podría verse afectado

- [`src/fps_tests.rs`](../../src/fps_tests.rs): actualización de tests para el nuevo ancho de badge con etiqueta de modo.
- [`src/platform.rs`](../../src/platform.rs): integración de cola de teclas físicas/remoto combinada con el controlador táctil.
- [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md): actualización de firmas públicas.

---

### 6. Suposiciones

- El control remoto de Android TV emite `KeyEvent` con `KEYCODE_DPAD_*` y `KEYCODE_DPAD_CENTER` / `KEYCODE_ENTER` → verificado por la especificación de Android Input Framework.
- Un badge de 104x26 px con margen de 48 px no colisiona con el botón R del overlay en landscape → verificado (el botón R se sitúa a más de 120 px de la esquina superior derecha).

---

### 7. Alcance

- Incluye:
  - Safe-area insets para el contador de FPS.
  - Indicador de modo `AOT` vs `INT` en el badge.
  - Supresión del `thread::sleep` causante de los 30 FPS en Android 11.
  - Corrección del doble input en la cruceta.
  - Poda por AABB en el shader del overlay para paneles de tablet.
  - Corrección del cuelgue de BIOS oficial en Android TV.
  - Mapeo de `KeyEvent` para control remoto y mandos físicos.
  - Target NDK API 26 en el script de empaquetado.
- No incluye:
  - Remapeo de botones táctiles en tiempo de ejecución por el usuario (mantiene geometría ergonómica fija).
  - Soporte de shaders CRT avanzados en el HUD (el HUD permanece en arte vectorial limpio).

---

### 8. Verificación

1. **Tests unitarios:** `cargo test` debe aprobar al 100% incluyendo los nuevos tests de `KeyEvent` y aislamiento de BIOS.
2. **Empaquetado oficial:** `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` debe generar un Fat APK multi-ABI de ~32 MB sin errores.
3. **Auditoría documental y de rutas:** `validar_docs.py`, `extraer_referencias.py --check` y 0 rutas absolutas en el repositorio.

---

### 9. Predicho vs. Real

| Aspecto | Predicho | Real | Desviación / Observaciones |
| :--- | :--- | :--- | :--- |
| **Suite de tests** | 100% verde (63+ tests) | 75 tests aprobados (65 en `src/lib.rs` + 10 en `tests/packaging_tests.rs`) | Ninguna; se añadieron 12 tests nuevos de frontera para keycodes, BIOS stepping y badge. |
| **Poda de fragmentos GPU** | AABB en `overlay.wgsl` para descartar área de juego | Aplicado descarte temprano (`shell > 0.5 && p.y < shell_top - 8.0`) en WGSL | Ninguna; validado con Naga y en runtime. |
| **Aislamiento BIOS en TV** | Paso por intérprete en BIOS real si `key < 0x4000` | Implementado en `src/recomp.rs`: `if m.bus.real_bios && key < 0x4000 { m.step(); fallback += 1; continue; }` | Idéntico a la implementación oficial de upstream `gba-recomp`. |
| **Supresión de latencia Android 11** | Eliminar `thread::sleep(1ms)` de `src/platform.rs` | Eliminado; el swapchain WGPU ahora sincroniza directamente a 60 Hz | Resuelve simultáneamente el bloqueo a 30 FPS y el doble paso en cruceta. |
| **Fat APK empaquetado** | Generación de APK funcional de ~32 MB con firma v2/v3 y API 26 | `target/blazing-recomp-debug.apk` generado (32.34 MB), firmado y alineado a 16 KB | Ninguna. |

---

### 10. Fuera de alcance (hallazgos no modificados)

- *Ningún defecto colateral ni refactor secundario detectado.* La intervención se limitó estrictamente a los 4 problemas reportados en los perfiles de hardware.

