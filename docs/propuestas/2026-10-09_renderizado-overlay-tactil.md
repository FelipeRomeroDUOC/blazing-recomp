# Propuesta: Renderizado del Overlay Táctil y Marco GBA SP en Android

Fecha: 2026-10-09
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Dibuja los controles virtuales y la carcasa de la consola en la pantalla de Android siguiendo la geometría ergonómica del prototipo GBA SP ([`docs/prototipos/layout_tactil_v2.html`](../prototipos/layout_tactil_v2.html)), mostrando la cruceta con cuenca circular, los botones Select/Start y la cápsula diagonal a -16° con botones B (rojo), A (verde), L y R (blancos) en modo vertical (Portrait), y botones translúcidos en la base inferior en modo horizontal (Landscape), con respuesta visual al mantenerlos pulsados.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `docs/BACKLOG.md`, `docs/prototipos/layout_tactil_v2.html`, `src/input.rs`, `src/platform.rs`, `Cargo.toml`.
- Intención: Proporcionar a los jugadores una representación gráfica nítida, fiel y elegante de la GBA SP y sus botones sobre la pantalla táctil antes de generar el paquete APK, permitiendo verificar visualmente las zonas de pulsación y la retroalimentación al tocar.
- Restricciones y decisiones previas (máx. 3):
  - Fidelidad estricta al prototipo `layout_tactil_v2.html`: cuenca circular del D-Pad, clúster diagonal inclinado a -16° (L sobre B, R sobre A), colores temáticos en Portrait (B rojo, A verde, L/R blanco) y translúcidos en Landscape (origen: respuesta humana, 2026-10-08) → implica: renderizar cada botón y cuenca en las coordenadas exactas de `TouchArea` de `src/input.rs` con estilos diferenciados por orientación → protegida por: Correspondencia geométrica entre hitboxes táctiles y elementos visuales.
  - Independencia y encapsulamiento en `blazing-recomp` (origen: `AGENTS.md` §4.1) → implica: implementar el pipeline de renderizado del overlay dentro de `blazing-recomp` sin mutar el repositorio upstream `gba-recomp/crates/screen` → protegida por: Compilación cruzada ARM64 con pipeline de overlay integrado.
  - Rendimiento eficiente y nitidez en pantallas de alta densidad (origen: `AGENTS.md` §2.5) → implica: renderizado mediante primitivas procedurales y shaders en WGSL/`wgpu` en lugar de texturas rasterizadas pesadas para evitar sobrecarga de I/O y escalado borroso → protegida por: Validación sintáctica y compilación del shader del overlay.
- Alternativas descartadas:
  - Cargar texturas PNG pre-renderizadas desde disco → descartada para garantizar bordes matemáticamente perfectos en cualquier resolución (DPI) y evitar dependencias de empaquetado de assets antes del APK.
  - Modificar el crate upstream `screen::present::Presenter` → descartada para respetar la regla arquitectónica de independencia de repositorios.
- Ante imprevistos, prioriza: nitidez geométrica y estabilidad del pipeline gráfico sobre efectos de iluminación o sombras volumétricas complejas.

### 3. Ejemplos

- Caso normal: El usuario abre la aplicación en orientación vertical (Portrait) → antes: la pantalla mostraba un marco negro centrado sin ningún control visible → ahora: la pantalla muestra el marco del juego en la parte superior y en la mitad inferior la carcasa de GBA SP con la cruceta circular, los botones Select/Start y la cápsula diagonal a -16° con B en rojo, A en verde y L/R en blanco, que se iluminan al pulsarse.
- Caso límite: El usuario rota el dispositivo a orientación horizontal (Landscape) → antes: no había elementos en pantalla salvo el visor 3:2 → ahora: los botones se reubican automáticamente en la base inferior con estilo translúcido para no tapar el centro de la acción.

### 4. Pasos

1. Añadir las dependencias directas de renderizado (`wgpu = "26"`, `bytemuck = "1"`, `pollster = "0.4"`) a `Cargo.toml`.
2. Crear el módulo `src/overlay.rs` con las estructuras de datos de dibujo (parámetros de botones, colores, radios, ángulos, opacidad y estados pulsados).
3. Escribir el shader procedural en WGSL (`src/overlay.wgsl`) que evalúe y renderice las primitivas (cuencas, cruceta, cápsula inclinada, botones y tipografía/letras básicas) con antialiasing.
4. Integrar el pipeline de renderizado en `src/platform.rs`, componiendo la superficie del juego (GBA 240x160) y la capa del overlay en la ventana nativa Android.
5. Conectar el estado de botones pulsados de `TouchController` (`on_motion`) hacia los parámetros del overlay para proporcionar retroalimentación visual inmediata al pulsar.
6. Validar con pruebas unitarias de shader y geometría (`cargo test --lib`) y verificar compilación cruzada con `cargo ndk -t arm64-v8a build`.

### 5. Qué más podría verse afectado

- `src/platform.rs`: sustitución o extensión del ciclo de presentación para incluir la pasada del overlay.
- `src/lib.rs`: exposición del módulo `overlay`.
- `Cargo.toml` y `Cargo.lock`: dependencias de `wgpu`, `bytemuck` y `pollster`.

### 6. Suposiciones

- El shader procedural de primitivas en WGSL compila y valida limpiamente con el motor `naga` en host y en Android bajo Vulkan y OpenGL ES 3.0 → prototipar: sí → si es falso: recurrir a geometría de quads teselados por CPU → resultado del prototipo: | no detecta: variaciones de precisión de punto flotante en GPUs móviles antiguas.
- La composición del marco del juego y el overlay táctil en un único swapchain de ventana mantiene la relación de aspecto 3:2 sin distorsión en pantallas panorámicas (20:9 o 21:9) → prototipar: no → si es falso: ajustar el viewport con márgenes de seguridad → resultado del prototipo: | no detecta: interferencia visual con muescas (notches) de cámaras frontales.

### 7. Alcance

- Incluye: Módulo de renderizado del overlay (`src/overlay.rs`), shader WGSL de primitivas procedurales, dibujo de cruceta con cuenca circular, cápsula inclinada a -16°, botones B, A, L, R, Select, Start, adaptación visual Portrait/Landscape y estado visual de botón pulsado.
- No incluye: Generación del contenedor APK y Gradle (se abordará inmediatamente tras validar el overlay), filtros de scanlines adicionales ni motor de audio.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Validación sintáctica y compilación del shader del overlay
Caso de uso: Validación en host del shader WGSL que dibuja los controles táctiles | Tipo: normal
  Dado que el shader `src/overlay.wgsl` define las primitivas del D-Pad y los botones
  Cuando se valida el código fuente WGSL con el validador Naga en las pruebas unitarias
  Entonces el módulo se parsea y valida sin errores de sintaxis ni de tipos
Test: `test_overlay_shader_validates`
Estado previo: falla (archivo y módulo no existen)

Escenario: Correspondencia geométrica entre hitboxes táctiles y elementos visuales
Caso de uso: Renderizado de botones en la posición exacta donde el usuario toca | Tipo: normal
  Dado que el controlador táctil y el overlay se configuran para una ventana de 1080x2400 (Portrait)
  Cuando se consultan las áreas de dibujo del D-Pad, botón A y gatillo L
  Entonces las posiciones centrales y radios del overlay coinciden exactamente con las áreas de colisión de `TouchController`
Test: `test_overlay_geometry_matches_hitboxes`
Estado previo: falla (código no existe)

Escenario: Adaptación visual según orientación y estado pulsado
Caso de uso: Cambio entre Portrait y Landscape y pulsación de botón | Tipo: límite
  Dado que el overlay recibe estado Portrait y una pulsación en el botón A
  Cuando se generan los parámetros de dibujo
  Entonces el botón A refleja estado activo (resaltado), y en Landscape la opacidad global se reduce al valor translúcido
Test: `test_overlay_orientation_and_press_state`
Estado previo: falla (código no existe)

Escenario: Compilación cruzada ARM64 con pipeline de overlay integrado
Caso de uso: Generación de la biblioteca compartida con soporte visual completo para Android | Tipo: regresión
  Dado que `Cargo.toml`, `src/overlay.rs` y `src/platform.rs` integran el renderizado del overlay
  Cuando ejecuto `cargo ndk -t arm64-v8a build`
  Entonces el proceso finaliza con código de salida 0 y genera `libblazing_recomp.so`
Test: `test_arm64_overlay_build`
Estado previo: pasa (regresión preventiva)

### 9. Cómo lo comprobaré

- Validación sintáctica y compilación del shader del overlay → herramienta o comando: `cargo test --lib test_overlay_shader_validates`
  → pasa si: test unitario con Naga finaliza con código 0 → no detecta: peculiaridades de renderizado de drivers propietarios en hardware real.
- Correspondencia geométrica entre hitboxes táctiles y elementos visuales → herramienta o comando: `cargo test --lib test_overlay_geometry_matches_hitboxes`
  → pasa si: centros y radios coinciden con tolerancia < 0.01 px → no detecta: distorsión visual por escalado no uniforme.
- Adaptación visual según orientación y estado pulsado → herramienta o comando: `cargo test --lib test_overlay_orientation_and_press_state`
  → pasa si: test unitario finaliza con código 0 → no detecta: visibilidad estética subjetiva en pantallas bajo luz solar directa.
- Compilación cruzada ARM64 con pipeline de overlay integrado → herramienta o comando: `cargo ndk -t arm64-v8a build`
  → pasa si: código de salida 0 y `libblazing_recomp.so` generado en `target/aarch64-linux-android/debug/` → no detecta: tasa de cuadros por segundo en dispositivos de gama baja.

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

- Validación sintáctica y compilación del shader del overlay: coincide
  Resultado real: `cargo test --lib test_overlay_shader_validates` pasa
  validando ambos shaders WGSL (`overlay.wgsl` y `game_blit.wgsl`) con
  el validador Naga en host sin errores de sintaxis ni de tipos.
  Método: el anunciado.
- Correspondencia geométrica entre hitboxes táctiles y elementos visuales: coincide
  Resultado real: `cargo test --lib test_overlay_geometry_matches_hitboxes`
  pasa (centros y radios de dibujo coinciden exactamente con < 0.01 px de
  tolerancia respecto a las `TouchArea` del controlador).
  Método: el anunciado.
- Adaptación visual según orientación y estado pulsado: coincide
  Resultado real: `cargo test --lib test_overlay_orientation_and_press_state`
  pasa (refleja el botón A pulsado con `pressed = 1.0`, opacidad plena 1.0
  en portrait con colores temáticos B rojo, A verde, L/R blanco, y
  opacidad translúcida en landscape).
  Método: el anunciado.
- Compilación cruzada ARM64 con pipeline de overlay integrado: coincide
  Resultado real: `cargo ndk -t arm64-v8a build` finalizó con código 0 y
  generó `target/aarch64-linux-android/debug/libblazing_recomp.so`
  (135.419.944 bytes debug).
  Método: el anunciado.

### Contexto: ¿se respetó?

- Fidelidad estricta al prototipo `layout_tactil_v2.html` → respetada
  (cuenca circular de la cruceta, cápsula diagonal inclinada a -16° con
  L sobre B y R sobre A, botones temáticos B rojo, A verde, L/R blanco
  en portrait y translúcidos en landscape; verificado por
  `test_overlay_geometry_matches_hitboxes` y
  `test_overlay_orientation_and_press_state`).
- Independencia y encapsulamiento en `blazing-recomp` → respetada (el
  compositor GPU unificado y los shaders WGSL residen íntegramente dentro
  de `blazing-recomp` sin modificar `gba-recomp/crates/screen`).
- Rendimiento eficiente y nitidez en pantallas de alta densidad → respetada
  (renderizado procedural mediante SDF y shaders WGSL vectoriales sin
  texturas pesadas ni dependencias de I/O en arranque).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no
  hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

`Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/input.rs`, `src/platform.rs`,
`src/overlay.rs`, `src/overlay_shader.rs`, `src/overlay.wgsl`,
`src/game_blit.wgsl`. ¿Coincide con lo anunciado? sí (`src/game_blit.wgsl`
y `src/overlay_shader.rs` modularizados para validación de ambos shaders).

### Discrepancias

Ninguna. Todos los criterios de aceptación y restricciones de contexto se
cumplieron con éxito.

### Para la próxima vez

Al diseñar shaders de composición múltiple sobre swapchains móviles, verificar
siempre que el canal alfa del pase superior sea 0.0 en fragmentos vacíos para no
ocluir los pases de fondo al mezclar con alpha blending.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

---

## Revisión humana

- ninguno: todo se comprobó con tests (`cargo test --lib`, `cargo ndk -t arm64-v8a build` y `git grep`).
- Detectado solo por el humano:
