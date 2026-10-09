# Propuesta: Ajustes de Fidelidad Visual, Hendiduras 3D y Ergonomía del Overlay

Fecha: 2026-10-09
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Refina el overlay táctil en `src/input.rs`, `src/overlay.rs` y `src/overlay.wgsl` resolviendo los hallazgos de la prueba en hardware:
1. **Portrait:** Carcasa extendida al ancho total de pantalla (`margin_x = 0.0`), reubicación de la cruceta hacia el interior para eliminar el corte lateral, efecto volumétrico 3D (sombra interior superior, bisel exterior y labio inferior iluminado) en las cuencas de la cruceta y clúster AB, y recentrado simétrico del pozo de botones alineado con el eje de rotación de los 4 botones (L, R, B, A).
2. **Landscape:** Botones y cruceta en material gris claro translúcido (`[0.72, 0.75, 0.82]`), aumento de opacidad global a `0.65`, y tipografía/flechas en contraste nítido oscuro (`#1e293b`).
3. **Ambas:** Mayor separación entre botones de acción B y A (`step` a `unit * 1.00`), aumento de dimensiones de hombros L/R (`r * 1.05`), y aumento del tamaño de fuente en botones de acción (`glyph_scale` de `0.13` a `0.28`).

### 2. Contexto

- Contexto consultado: Capturas de prueba en hardware real aportadas por el usuario, `docs/prototipos/layout_tactil_v2.html`, `src/input.rs`, `src/overlay.rs`, `src/overlay.wgsl`.
- Intención: Corregir imperfecciones visuales observadas en dispositivo (corte en borde izquierdo, pozo AB desfasado, invisibilidad en landscape, botones pegados y glifos minúsculos) logrando máxima calidad estética y ergonomía táctil.
- Restricciones y decisiones previas:
  - Inmutabilidad de mapeo KEYINPUT: Los bits de cada botón permanecen inalterados -> protegida por: `test_touch_hitbox_mapping_untouched`.
  - Consistencia geométrica Portrait vs Landscape: La disposición y formas de botones se mantienen idénticas en ambas orientaciones, alterando únicamente el acabado superficial -> protegida por: `test_overlay_orientation_and_press_state`.
  - Sin dependencias de texturas externas para geometría: Todo el modelado 3D de hendiduras se realiza en WGSL mediante funciones SDF continuas -> protegida por: `test_overlay_shader_validates`.
- Alternativas descartadas:
  - Usar imágenes prerenderizadas para las hendiduras -> descartada para evitar artefactos de escala en resoluciones arbitrarias y mantener independencia de assets.
- Ante imprevistos, prioriza: Ergonomía táctil y legibilidad a 60 fps sobre sombras multicapa excesivamente costosas.

### 3. Ejemplos

- Caso normal (Portrait - Hendidura y Carcasa):
  - Antes: Carcasa con margen lateral de 3.5% que recorta la cuenca de la cruceta en el borde izquierdo; cuencas planas sin relieve; cápsula AB desalineada con respecto a los 4 botones.
  - Ahora: Carcasa de ancho completo (100% de pantalla); cuenca de la cruceta con margen holgado respecto al borde; cuencas con bisel exterior iluminado y sombra interior profunda; cápsula AB centrada exactamente en el centroide de L, R, B, A.
- Caso normal (Landscape - Visibilidad):
  - Antes: Material azul marino oscuro casi invisible sobre fondo negro (`rgba(30, 41, 59, 0.45)`); glifos sin contraste.
  - Ahora: Material gris claro escarchado translúcido con opacidad al 65%; texto y flechas oscuras de alto contraste nítidamente legibles.
- Caso normal (Escala y Fuentes):
  - Antes: Botones L/R delgados y pequeños; botones B y A casi rozándose; letras 'A' y 'B' diminutas al 13% del radio.
  - Ahora: Botones B y A holgados y separados; botones L/R con volumen y presencia táctil sólida; letras 'A' y 'B' al 28% del radio, nítidas y centradas.

### 4. Pasos

1. **Ajustes de layout y dimensiones en `src/input.rs`**:
   - Ajustar `dpad_cx` en portrait de `wf * 0.25` a `wf * 0.27` para centrar holgadamente la cuenca.
   - Aumentar separación diagonal `step` de `unit * 0.86` a `unit * 1.00`.
   - Posicionar L y R ortogonalmente al eje inclinado de B y A para formar una cuadrícula rotada perfecta sin cizalladura.
   - Aumentar radio de L y R de `r * 0.85` a `r * 1.02`.
2. **Parámetros y paleta en `src/overlay.rs`**:
   - Actualizar `button_color` en landscape a `[0.72, 0.75, 0.82]`.
   - Aumentar `opacity` en landscape a `0.65` en `build_globals`.
   - Aumentar `glyph_scale` de `0.13` a `0.28`.
   - Ajustar dimensiones de pastilla de L/R (`extent_y` a `a.radius * 0.58` y `rad_x` a `a.radius * 1.15`).
3. **Shader de hendiduras 3D y carcasa en `src/overlay.wgsl`**:
   - Eliminar margen lateral de la carcasa (`margin_x = 0.0`) para ocupar el 100% del ancho de pantalla.
   - Implementar sombreado 3D en la cuenca de la cruceta: bisel exterior claro, sombra profunda interior superior y reborde inferior iluminado.
   - Recentrar la cápsula AB calculando su centroide simétrico con el cuarteto (B, A, L, R) y dotarla de la misma profundidad 3D.
   - Optimizar contraste de glifos en landscape con tinta oscura `#1e293b`.
4. **Verificación y suite de pruebas**:
   - Comprobar tests existentes y nuevos escenarios con `cargo test --lib`.
   - Reempaquetar APK con `scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- `src/input.rs`: hitboxes de botones B, A, L, R y centros de cruceta.
- `src/overlay.rs`: parámetros visuales, opacidad landscape y escala de fuentes.
- `src/overlay.wgsl`: renderizado de fondo, cuencas y contraste.

### 6. Suposiciones

- El aumento del espaciado de B/A (`step = unit * 1.00`) no colisiona con el borde derecho en pantallas estrechas (aspect ratio 18:9 o superior) -> prototipar: no -> si es falso: modular `step` en función del ancho de ventana.

### 7. Alcance

- Incluye: Ancho de carcasa, relieve 3D en hendiduras, recentrado de cápsula AB, colores y opacidad de landscape, tamaño de L/R, espaciado de B/A y escala de tipografía.
- No incluye: Lógica de emulación, bucle de audio, JNI ni menús de configuración.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Ergonomía de clúster de acción y tamaño de hombros
Caso de uso: Separar botones B/A y agrandar L/R | Tipo: normal
  Dado que se solicita el layout táctil en Portrait o Landscape
  Cuando se evalúan las áreas de B, A, L y R
  Entonces la distancia entre los centros de B y A es al menos el 95% de la unidad base, y el radio de L y R es al menos 1.0 veces el radio de B/A
Test: `test_action_buttons_separation_and_shoulder_size`
Estado previo: falla (distancia actual es 0.86 y radio L/R es 0.85)

Escenario: Paleta gris claro translúcida y contraste en Landscape
Caso de uso: Visibilidad de controles sobre fondo de juego | Tipo: normal
  Dado que se inicializa el overlay en Landscape
  Cuando se consultan los parámetros globales y de botones
  Entonces la opacidad es 0.65 y el color base es gris claro translúcido con canales RGB > 0.70
Test: `test_landscape_frosted_light_gray_palette_and_opacity`
Estado previo: falla (color actual es 0.12, 0.16, 0.23 y opacidad es 0.45)

Escenario: Preservación de mapeo KEYINPUT
Caso de uso: Regresión de colisiones y bits de entrada | Tipo: regresión
  Dado que se modifican posiciones y tamaños de hitboxes
  Cuando se consultan las áreas de toque
  Entonces los 10 botones KEYINPUT siguen presentes con sus identificadores correctos
Test: `test_touch_hitbox_mapping_untouched`
Estado previo: pasa

### 9. Cómo lo comprobaré

- Ergonomía y dimensiones de botones → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_action_buttons_separation_and_shoulder_size`
  → pasa si: código 0 → no detecta: preferencias anatómicas de tamaño de pulgar.
- Paleta y opacidad Landscape → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_landscape_frosted_light_gray_palette_and_opacity`
  → pasa si: código 0 → no detecta: variaciones de brillo de paneles OLED en exteriores.
- Regresión de mapeo KEYINPUT → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_touch_hitbox_mapping_untouched`
  → pasa si: código 0.
- Validación del shader Naga → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_overlay_shader_validates`
  → pasa si: código 0.
- Inspección visual en hardware real → Generación de APK con `scripts/package_apk.ps1` e instalación en dispositivo.

### 10. Terminado cuando

Se completan los 4 pasos, pasan los tests automatizados y se compila el APK firmado para prueba en dispositivo.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse y marcar la propuesta como "discrepada".

## Revisión humana

- (visible) La carcasa en portrait cubre el 100% del ancho de pantalla sin recortar la hendidura izquierda: `✔`
- (visible) Las hendiduras de la cruceta y clúster AB presentan profundidad 3D visible (sombra superior y borde iluminado): `✔`
- (visible) Los 4 botones de acción están simétricamente distribuidos y balanceados dentro de su cápsula: `✔`
- (visible) En landscape, los controles son claramente visibles en gris claro translúcido con flechas y texto nítidos: `✔`
- (visible) Los botones B y A tienen separación holgada, botones L/R mayor tamaño y las letras son grandes y legibles: `✔`

---

## Cierre de la Propuesta (Predicho vs. Real)

### Predicho vs. Real

| Aspecto | Predicho en Propuesta | Resultado Real | Estado |
|---|---|---|---|
| **Ancho de Carcasa Portrait** | Carcasa cubre el 100% del ancho de pantalla sin márgenes laterales negros | `in_shell = smoothstep(shell_top - 1.0, shell_top + 1.0, p.y)` cubre de $x=0$ a $x=\text{res.x}$ | **Cumplido** |
| **Cuenca Izquierda Cruceta** | Sin recorte lateral por la carcasa, margen holgado | `dpad_cx = 0.275 * wf`, cuenca contenida con >160 px de margen respecto al borde | **Cumplido** |
| **Profundidad 3D Hendiduras** | Sombra superior interior, bisel exterior y labio inferior iluminado en cruceta y cápsula AB | Implementado con iluminación angular consistente a 135° en ambos pozos | **Cumplido** |
| **Balance Cuarteto de Acción** | Posicionamiento simétrico de (B, A, L, R) sin cizalladura dentro de la cápsula | Eje ortogonal rotado $(ox, oy) = (dy, -dx)$ y centroide simétrico $(ba + lr) / 2$ | **Cumplido** |
| **Paleta y Opacidad Landscape** | Gris claro translúcido escarchado `[0.72, 0.75, 0.82]` con opacidad `0.65` | `test_landscape_frosted_light_gray_palette_and_opacity` en verde | **Cumplido** |
| **Contraste de Glifos** | Tinta carbón oscura `#1e293b` sobre botones claros y flechas blancas al pulsar | Glifos con tinta oscura `vec3(0.12, 0.13, 0.15)` y corrección de blending alpha | **Cumplido** |
| **Separación B/A y Tamaño L/R** | `step = unit * 1.00`, L/R radio `>= r * 1.00` | `test_action_buttons_separation_and_shoulder_size` en verde | **Cumplido** |
| **Tamaño de Fuente Acciones** | Glifos grandes, nítidos y legibles dentro de los botones | Multiplicador `box_size` ampliado a `5.2x` para A/B/L/R y `3.6x` para flechas | **Cumplido** |
| **Suite Automatizada** | 21 tests de unidad e integración pasando | 21 pasados en verde (0 fallos, 0 regresiones) | **Cumplido** |
| **Artefacto Compilado** | APK firmado y alineado a 16 KB | `target/blazing-recomp-debug.apk` generado y verificado | **Cumplido** |

### Fuera de alcance (hallazgos no abordados en esta iteración)
- Ninguno detectado. El alcance se mantuvo estrictamente acotado a los parámetros visuales, de shader y ergonomía acordados.
