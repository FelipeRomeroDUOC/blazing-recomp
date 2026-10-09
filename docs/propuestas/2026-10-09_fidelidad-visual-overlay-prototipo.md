# Propuesta: Fidelidad Visual del Overlay Android según Prototipo GBA SP (layout_tactil_v2.html)

Fecha: 2026-10-09
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Rediseña el overlay táctil procedural y el compositor GPU en `src/overlay.rs`, `src/overlay.wgsl` y `src/platform.rs` para replicar con máxima fidelidad visual el prototipo interactivo `docs/prototipos/layout_tactil_v2.html`: paleta gris-lavanda uniforme, carcasa plateada con bordes redondeados y márgenes laterales, degradado, brillo y sombra física en botones, pastillas L/R ampliadas con ángulo propio, cruceta en cruz con brazos redondeados (manteniendo hitboxes circulares intactas), tipografía real rasterizada con `fontdue` en atlas R8 (Outfit para botones y JetBrains Mono para etiquetas de sistema), feedback táctil al 92% con cruceta azul al pulsar, y modo landscape con acabado en cristal oscuro translúcido.

### 2. Contexto

- Contexto consultado: `docs/prototipos/layout_tactil_v2.html`, capturas de referencia del prototipo, `src/overlay.rs`, `src/overlay.wgsl`, `src/platform.rs`, `src/input.rs`, `Cargo.toml`.
- Intención: Transformar el overlay táctil en una réplica visual idéntica y de alta gama del prototipo `layout_tactil_v2.html` aprobado por el usuario, sustituyendo los glifos de matriz de bits por tipografía vectorial nítida rasterizada en atlas GPU y dotando a la carcasa y botones de profundidad estética profesional (luces, sombras y degradados).
- Restricciones y decisiones previas (máx. 3):
  - Inmutabilidad de la lógica de entrada (origen: instrucción explícita del usuario) → implica: prohibido modificar la lógica interna de `src/input.rs` ni el mapeo KEYINPUT; las hitboxes circulares de colisión se preservan exactamente iguales → protegida por: `test_touch_hitbox_mapping_untouched`.
  - Tipografía real con fuentes OFL embebidas (origen: instrucción explícita del usuario que sustituye la restricción previa "sin texturas" solo para texto) → implica: incluir archivos de fuente `.ttf` con sus respectivas licencias en `assets/fonts/`, embeberlas con `include_bytes!`, rasterizar glifos requeridos al arrancar con `fontdue` en una textura R8 de un solo canal y enlazarla al bind group del shader → protegida por: `test_font_atlas_contains_required_glyphs`.
  - Estética visual estricta del prototipo (origen: `layout_tactil_v2.html` y capturas de referencia) → implica: botones en paleta gris-lavanda (#a4a9c5 a #7b819f), carcasa plateada flotante con esquinas redondeadas y márgenes laterales en Portrait, cruceta azulada (#5b8def) al pulsar y cristal oscuro translúcido en Landscape → protegida por: inspección visual humana y shaders validados.
- Alternativas descartadas:
  - Continuar refinando la cuadrícula de bits 3×5 → descartada porque no puede reproducir la fidelidad tipográfica de Outfit Bold ni JetBrains Mono requerida por el diseño.
  - Renderizar texto mediante texturas web/DOM externas → descartada para mantener la aplicación como binario nativo autocontenido en Android sin dependencias de WebView.
- Ante imprevistos, prioriza: estabilidad de renderizado en GPU y rendimiento fluido a 60 fps sobre resolución excesiva del atlas de fuentes.

### 3. Ejemplos

- Caso normal (Portrait):
  - Antes: botones con colores rojo, verde y blanco; carcasa ocupando toda la mitad inferior plana sin margen lateral ni esquinas redondeadas; cruceta formada por cuatro círculos flotantes; glifos pixelados en matriz de 3×5; pastillas Select/Start con una 'S' deformada superpuesta.
  - Ahora: carcasa plateada clara con esquinas redondeadas (radio ~28 px) y margen lateral respecto a los bordes de la pantalla; cruceta continua en cruz con brazos redondeados y centro cuadrado; todos los botones en degradado gris-lavanda con borde superior iluminado y sombra inferior; pastillas L y R rotadas y ampliadas; letras 'A', 'B', 'L', 'R' nítidas en tipografía Outfit Bold; etiquetas "SELECT" y "START" en JetBrains Mono situadas encima de pastillas lisas.
- Caso normal (Pulsación):
  - Antes: el botón solo cambiaba de brillo sin escala ni color distintivo en la cruceta.
  - Ahora: al presionar un botón de acción se escala al 92% con sombra comprimida; al presionar cualquier dirección de la cruceta el brazo correspondiente se ilumina en azul eléctrico (#5b8def).
- Caso normal (Landscape):
  - Antes: botones circulares básicos translúcidos en las esquinas.
  - Ahora: estética "glassmorphism" en cristal oscuro translúcido con bordes iluminados sutiles y glifos vectoriales claros.

### 4. Pasos

Los pasos se ejecutarán en este orden estricto, con verificación y aprobación entre fases:

1. **Paleta de color uniforme gris-lavanda**:
   - En `src/overlay.rs`, unificar `button_color` para que todos los botones usen el gradiente/color base gris-lavanda del prototipo (`--btn-light: #a4a9c5`, `--btn-dark: #7b819f`), eliminando los colores rojo, verde y blanco.
2. **Carcasa plateada con esquinas redondeadas y margen lateral**:
   - En `src/overlay.wgsl`, renderizar el cuerpo de la carcasa en portrait mediante un rectángulo redondeado (`sd_round_rect`) con márgenes laterales e inferiores de ~14 px y radio de esquina de ~28 px, con gradiente vertical plateado suave y sombra perimetral.
3. **Efectos de degradado, brillo y sombra en botones**:
   - En `src/overlay.wgsl`, calcular iluminación volumétrica en cada botón: degradado diagonal (145°), bisel superior con luz especular (borde claro de 1.5 px) y sombra proyectada inferior/exterior.
4. **Hombros L y R como pastillas inclinadas orientadas**:
   - Extender `struct Elem` en WGSL y `ElemParams` en Rust para incorporar un campo de rotación/ángulo `angle: f32`.
   - Orientar y agrandar las pastillas L y R en el eje rotado (aproximadamente -30° relativo al eje horizontal) para que coincidan con la cápsula.
5. **Cruceta continua en cruz con brazos redondeados**:
   - En `src/overlay.wgsl`, evaluar la cruceta como la unión SDF de dos rectángulos redondeados cruzados con centro común sobre el pozo hundido, preservando las cuatro hitboxes circulares en `src/input.rs`.
6. **Integración de tipografía real con atlas R8**:
   - Añadir la dependencia `fontdue` en `Cargo.toml`.
   - Incorporar las fuentes libres `Outfit-Bold.ttf` y `JetBrainsMono-Bold.ttf` en `assets/fonts/` junto con sus archivos de licencia OFL.
   - En `src/overlay.rs`, crear un generador de atlas de textura en escala de grises (R8Unorm) que rasterice los glifos `'A'`, `'B'`, `'L'`, `'R'`, `'S'`, `'E'`, `'L'`, `'C'`, `'T'`, `'A'`, `'R'`, `'T'` y flechas.
   - Actualizar el bind group de `src/platform.rs` y el shader `src/overlay.wgsl` para recibir la textura del atlas y su sampler lineal, renderizando el texto nítido mapeado por coordenadas UV.
   - Colocar las etiquetas "SELECT" y "START" sobre las pastillas y dejar las pastillas lisas.
   - Reemplazar el test unitario de bitmaps 3×5 por `test_font_atlas_contains_required_glyphs`.
7. **Feedback de pulsación (escala al 92% y cruceta azul)**:
   - Modificar la evaluación SDF del botón activo multiplicando su radio visual por 0.92 al estar presionado (`el.pressed > 0.5`).
   - Al pulsar una dirección de la cruceta, teñir el brazo activo con azul `#5b8def`.
8. **Modo Landscape en cristal oscuro translúcido**:
   - Configurar en `src/overlay.wgsl` para landscape: fondos de pozos desactivados, botones en material oscuro translúcido `rgba(30, 41, 59, 0.45)` con bisel blanco tenue y glifos de alta luminosidad.

### 5. Qué más podría verse afectado

- `Cargo.toml` y `Cargo.lock`: adición de dependencia `fontdue`.
- `assets/fonts/`: nuevos archivos de fuente y licencias OFL.
- `src/overlay.rs`: estructuras de elementos, generación de atlas de glifos y parámetros uniformes.
- `src/overlay.wgsl`: shader de renderizado con muestreo de textura y primitivas SDF mejoradas.
- `src/platform.rs`: configuración de recursos wgpu (textura de atlas y sampler).
- `src/packaging_tests.rs`: verificación de empaquetado del nuevo APK.

### 6. Suposiciones

- La rasterización de glifos con `fontdue` en tiempo de inicialización de la app tarda menos de 15 ms en hardware móvil → prototipar: sí → si es falso: generar el atlas R8 pre-rasterizado estático en tiempo de compilación con `build.rs` → resultado del prototipo: | no detecta: sistemas operativos sin soporte de memoria dinámica.
- La extensión del struct uniforme `Elem` con un float de ángulo mantiene el alineamiento a múltiplos de 16 bytes requerido por WGSL → prototipar: no → si es falso: empaquetar el ángulo en el vector de parámetros existente → resultado del prototipo: | no detecta: GPUs con restricciones de uniform buffer menores a 1 KB.

### 7. Alcance

- Incluye: diseño visual y shaders de `src/overlay.*`, atlas de texto con `fontdue`, archivos de fuentes OFL en `assets/fonts/`, actualización de pipeline wgpu en `src/platform.rs`, suite de pruebas unitarias y reempaquetado del APK.
- No incluye: modificaciones en la lógica de hitboxes de `src/input.rs`, cambios en los bits KEYINPUT, menús interactivos ni carga de archivos ROM.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Generación y validez del atlas de fuentes tipográficas
Caso de uso: Rasterizar glifos reales en textura R8 sin artefactos | Tipo: normal
  Dado que las fuentes Outfit y JetBrains Mono están embebidas
  Cuando se construye el atlas tipográfico al inicializar el overlay
  Entonces genera una textura R8 no vacía donde cada glifo requerido tiene dimensiones positivas y coordenadas UV válidas entre 0.0 y 1.0
Test: `test_font_atlas_contains_required_glyphs`
Estado previo: falla (módulo de atlas de fuentes aún no existe)

Escenario: Preservación intacta del mapeo de entrada y hitboxes de colisión
Caso de uso: Asegurar que el rediseño estético no altere la lógica de input | Tipo: regresión
  Dado que se solicita el layout de colisiones de `src/input.rs`
  Cuando se consultan las áreas de toque en Portrait y Landscape
  Entonces la cantidad de hitboxes, sus botones asignados y los bits KEYINPUT correspondientes permanecen idénticos a la versión base
Test: `test_touch_hitbox_mapping_untouched`
Estado previo: pasa (regresión preventiva)

Escenario: Validación estricta del shader WGSL con atlas y sampler
Caso de uso: Compilación limpia del pipeline de renderizado en GPU | Tipo: normal
  Dado que `src/overlay.wgsl` incluye el bind group con textura de atlas y sampler
  Cuando se analiza y valida el shader con `naga`
  Entonces no produce errores de sintaxis, tipos ni desalineación de uniforms
Test: `test_overlay_shader_with_font_atlas_validates`
Estado previo: falla (shader actual no incluye bindings de textura ni sampler)

### 9. Cómo lo comprobaré

- Generación y validez del atlas de fuentes tipográficas → herramienta: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_font_atlas_contains_required_glyphs`
  → pasa si: código 0 → no detecta: aberraciones ópticas en tamaños de glifo microscópicos.
- Preservación intacta del mapeo de entrada y hitboxes → herramienta: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_touch_hitbox_mapping_untouched`
  → pasa si: código 0 → no detecta: diferencias ergonómicas físicas entre usuarios.
- Validación del shader WGSL con atlas → herramienta: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_overlay_shader_with_font_atlas_validates`
  → pasa si: código 0 → no detecta: comportamientos anómalos en controladores gráficos propietarios no estándar.
- Inspección visual en hardware real → verificación humana con el nuevo APK generado por `scripts/package_apk.ps1`.

### 10. Terminado cuando

Se completan los 8 puntos del alcance en orden, se cumplen todos los criterios de aceptación y se genera un APK verificable en hardware real.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

## Revisión humana

- (visible) La carcasa en portrait presenta esquinas redondeadas plateadas con márgenes laterales visibles respecto a la pantalla: `✔ | ✘ | ?`
- (visible) Los botones tienen acabado gris-lavanda uniforme con luces, sombras y degradado según el prototipo: `✔ | ✘ | ?`
- (visible) Las letras A, B, L, R y etiquetas SELECT / START muestran tipografía real limpia (Outfit y JetBrains Mono) sin píxeles toscos: `✔ | ✘ | ?`
- (visible) La cruceta se visualiza como una cruz continua con brazos redondeados y se ilumina en azul al pulsar: `✔ | ✘ | ?`
- (visible) En landscape, los controles se visualizan en estilo cristal oscuro translúcido: `✔ | ✘ | ?`
