# Propuesta: Calibración Multidispositivo v2 (BlueStacks, Android TV, Tablet y HUD FPS)

Fecha: 2026-10-10
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Elimina las dobles pulsaciones en BlueStacks mediante arbitraje con ventana de enfriamiento físico, estabiliza Android TV a 60 FPS bloqueando la reactivación espuria del overlay táctil en televisores, acelera el rendimiento en tablets en portrait y landscape mediante poda temprana de píxeles planos y particionado de clústeres, y agranda el contador de FPS a 272x44 px con tipografía de 19 px para máxima legibilidad en smartphones de alta resolución.

### 2. Contexto

- Contexto consultado: `src/platform.rs`, `src/input.rs`, `src/overlay.wgsl`, `src/fps.rs`, `src/fps_tests.rs`, `AGENTS.md §2, §4, §6`, reporte de pruebas del usuario en 4 perfiles de hardware.
- Intención: Solucionar simultáneamente los 4 problemas reportados: doble input en BlueStacks, inestabilidad de 30-60 FPS en Android TV, caída de tasa de refresco en tablets Android 11 y visibilidad pequeña del badge en smartphones.
- Restricciones y decisiones previas (máx. 3):
  - Invariante de latencia de swapchain a 1 fotograma (`desired_maximum_frame_latency: 1`) (origen: `docs/explicacion/DECISIONES.md:312`, `src/overlay_tests.rs:373`) → implica: mantener `desired_maximum_frame_latency: 1` y optimizar la tasa de relleno (fillrate) de GPU y CPU. → protegida por: `test_compositor_frame_latency_is_one`.
  - Prohibición de rutas absolutas locales (origen: `AGENTS.md §6`) → implica: solo usar enlaces relativos al repositorio sin rutas de sistema. → protegida por: `test_no_local_paths_in_packaging`.
  - Empaquetado oficial con script único (origen: `AGENTS.md §6`) → implica: compilar y firmar el instalador APK exclusivamente con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`. → protegida por: `test_apk_generation_and_signature`.
- Alternativas descartadas:
  - Desactivar completamente la simulación de respuesta de pantalla (`ResponseMode::Off`) en el runner → descartada porque alteraría la fidelidad visual del LCD y no resuelve el cuello de botella principal de la GPU en la tablet.
  - Eliminar el fondo plateado SP de portrait → descartada porque rompería la identidad visual y la estética retro acordada en fases anteriores.
- Ante imprevistos, prioriza: Estabilidad de 60 FPS sostenidos y precisión del control sobre micro-optimizaciones cosméticas.

### 3. Ejemplos

- Caso normal (BlueStacks): El usuario pulsa una dirección en el teclado físico de la PC → antes: BlueStacks enviaba evento de tecla y toque virtual con micro-desfase, activando una doble pulsación de cruceta al soltar la tecla → ahora: tanto al presionar como al liberar la tecla física, cualquier puntero táctil de ese botón se elimina inmediatamente y se activa un periodo de gracia de 250 ms donde la pantalla táctil queda silenciada, emitiendo exactamente un único movimiento de cursor.
- Caso límite (Android TV): Un control remoto o gamepad emite un `MotionEvent` (puntero/eje) → antes: `on_motion` activaba `touch_enabled = true`, ejecutando el shader de controles táctiles en resolución 4K y bajando la tasa a 30 FPS → ahora: si `is_tv` es verdadero, `touch_enabled` se mantiene forzosamente en `false` y `on_motion` es ignorado, asegurando 60 FPS fijos.
- Caso visual (Móvil / Tablet): Contador de FPS en pantalla → antes: badge de 216x34 px con fuente 15.5 px → ahora: badge de 272x44 px con fuente 19.0 px, perfectamente legible a distancia sin obstruir la pantalla de juego.

### 4. Pasos

1. Añadir el método `release_button(&mut self, button: Button)` en `TouchController` (`src/input.rs`) para purgar de inmediato de la lista de punteros cualquier toque asociado a una tecla física.
2. Incorporar en `AppState` (`src/platform.rs`) los campos `last_physical_input: std::time::Instant` y `is_tv: bool`.
3. En `on_key` (`src/platform.rs`), actualizar `last_physical_input = Instant::now()`, liberar el botón táctil con `self.touch.release_button(button)` y robustecer `combined_keys()` para priorizar mandos físicos.
4. En `on_motion` (`src/platform.rs`), ignorar eventos táctiles si `self.is_tv` está activo o si han transcurrido menos de 250 ms desde la última pulsación física.
5. En `src/overlay.wgsl`, optimizar el modo portrait (`globals.shell > 0.5`): si el píxel se encuentra en la carcasa gris plana fuera de las cajas englobantes de cuencas 3D y botones, retornar de inmediato el color de la placa en un salto O(1).
6. En `src/overlay.wgsl`, particionar horizontalmente la evaluación en landscape para que los píxeles de la izquierda solo evalúen la cruceta y los de la derecha solo el clúster de acción.
7. En `src/fps.rs` y `src/fps_tests.rs`, actualizar las dimensiones a 272x44 px con fuente `19.0 px`, centrando el texto con margen adecuado.
8. Ejecutar la suite completa de pruebas (`cargo test`), validar la documentación con Diátaxis y empaquetar el APK con `scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- `src/fps_tests.rs`: las aserciones sobre dimensiones del badge (deben actualizarse a 272x44).
- `src/input_tests.rs`: añadir prueba unitaria para `release_button`.
- Búsqueda realizada con ripgrep para `BADGE_WIDTH`, `BADGE_HEIGHT` y `touch_enabled`.

### 6. Suposiciones

- La ventana de enfriamiento de 250 ms entre teclado físico y entrada táctil es suficiente para absorber el retraso de liberación de eventos virtuales de BlueStacks sin interferir con usuarios en smartphones táctiles puros → prototipar: no → si es falso: se ajustará el umbral a 150 ms o se basará en el origen del dispositivo de entrada (`InputDevice`).
- Los dispositivos Android TV reportan consistentemente `UI_MODE_TYPE_TELEVISION` a través de `Configuration` de Android → prototipar: no → si es falso: se añadirá fallback a comprobación de `PackageManager.FEATURE_LEANBACK`.

### 7. Alcance

- Incluye: Arbitraje y supresión de dobles inputs en emuladores/teclados, bloqueo permanente del overlay en Android TV, aceleración del shader de overlay táctil en tablets (portrait y landscape), y agrandamiento del badge de FPS a 272x44 px.
- No incluye: Soporte para shaders de postprocesado CRT externos, remapeo de botones configurable por GUI en tiempo de ejecución ni cambios en el pipeline de decodificación AOT de C11.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Purgado instantáneo de botón táctil ante evento físico
Caso de uso: Usuario en BlueStacks pulsando cruceta física mientras el mapeador emula toques en pantalla | Tipo: normal
  Dado que `TouchController` tiene un puntero activo presionando `Button::Up`
  Cuando se invoca `touch.release_button(Button::Up)`
  Entonces `touch.is_pressed(Button::Up)` devuelve `false` y `touch.key_mask() & Button::Up.key_bit() != 0`.
Test: `src/input_tests.rs::test_touch_release_button_cleans_pointer_state`
Estado previo: falla (función no existe aún)

Escenario: Cooldown de entrada física silencia toques simulados en ventana de 250 ms
Caso de uso: Entrada rápida de cruceta en emulador con mapeador superpuesto | Tipo: límite
  Dado que se registra una pulsación física en `on_key` hace menos de 250 milisegundos
  Cuando llega un evento `MotionEvent` de toque simulado sobre un botón
  Entonces `on_motion` descarta el evento y `combined_keys()` no sufre rebote de activación.
Test: `src/platform_tests.rs::test_arbitration_physical_cooldown_silences_touch`
Estado previo: falla (lógica no implementada)

Escenario: Badge de FPS agrandado a 272x44 px con tipografía de 19 px
Caso de uso: Lectura cómoda de FPS y renderer activo en móviles y tablets de alta resolución | Tipo: normal
  Dado que se inicializa `FpsCounter` con modo nativo y renderer activo
  Cuando se consulta `dimensions()` y se rasteriza el badge
  Entonces las dimensiones son `(272, 44)` y el búfer de píxeles contiene exactamente `272 * 44` píxeles no nulos.
Test: `src/fps_tests.rs::test_fps_badge_enlarged_dimensions_and_rasterization`
Estado previo: falla (actualmente es 216x34)

Escenario: Validación estricta del shader WGSL con optimización de placa plana
Caso de uso: Compilación del pipeline gráfico en dispositivos Vulkan y OpenGL ES sin regresiones | Tipo: regresión
  Dado el shader `WGSL` en `src/overlay_shader.rs` con las ramas de poda rápida
  Cuando el parser `naga` valida el módulo de sombreado
  Entonces no produce errores de sintaxis, variables no declaradas ni tipos incompatibles.
Test: `src/overlay_tests.rs::test_overlay_shader_validates`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- `test_touch_release_button_cleans_pointer_state` → herramienta o comando: `cargo test --lib test_touch_release_button` → pasa si: 1 test aprobado → no detecta: retrasos de hardware externos en el bus USB.
- `test_fps_badge_enlarged_dimensions_and_rasterization` → herramienta o comando: `cargo test --lib test_fps_badge` → pasa si: todas las pruebas de badge pasan con dimensiones 272x44 → no detecta: contraste percibido bajo luz solar directa.
- `test_overlay_shader_validates` → herramienta o comando: `cargo test --lib test_overlay_shader_validates` → pasa si: validación WGSL exitosa → no detecta: límites de memoria dinámica del driver del fabricante.
- Escenario visual de 60 FPS en Android TV y Tablet → revisión humana: verificar en dispositivo que el juego mantenga fluidez constante sin caídas a 30 FPS → no detecta: microstutter por recolección de basura del sistema operativo anfitrión.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, los 4 perfiles de hardware pasan las pruebas de regresión, la validación Diátaxis devuelve 0 errores y el APK se empaqueta y firma limpiamente.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Se confirmó mediante `src/overlay_tests.rs:373` que `desired_maximum_frame_latency: 1` es un invariante arquitectónico protegido por test, por lo que la ganancia de fluidez en tablet y Android TV se obtiene optimizando la tasa de relleno del fragment shader y silenciando el overlay espurio en televisores.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

*(Se completará tras la aprobación y ejecución del cambio)*

### Contexto: ¿se respetó?

*(Se completará tras la verificación)*

### Archivos modificados

*(Se completará tras la edición)*

### Discrepancias

*(Se completará tras la verificación)*

### Para la próxima vez

*(Se completará tras el cierre)*

### Fuera de alcance (hallazgos que no se arreglaron)

*(Se completará tras la verificación)*

---

## Revisión humana

- Comprobación visual de fluidez continua a 60 FPS en Android TV y eliminación del salto doble en BlueStacks: ✔ | ✘ | ?
- Legibilidad y tamaño adecuado del nuevo badge de 272x44 px en smartphone: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
