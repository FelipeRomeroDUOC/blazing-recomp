# Propuesta: Corrección de Escala Dimensional, Adaptación a Orientación y Renderizado del Overlay Táctil

Fecha: 2026-10-09
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Corrige el cálculo de escala y posicionamiento del overlay táctil en `src/input.rs`, la actualización reactiva de tamaño ante rotaciones de pantalla en `src/platform.rs`, y el renderizado procedural de glifos y cápsulas en `src/overlay.wgsl` y `src/overlay.rs`, para eliminar los controles en miniatura, evitar que se oculten en modo horizontal y suprimir los bloques pixelados gigantescos que desfiguran los botones en hardware real.

### 2. Contexto

- Contexto consultado: `docs/prototipos/layout_tactil_v2.html`, `src/input.rs`, `src/overlay.rs`, `src/overlay.wgsl`, `src/platform.rs`, capturas de dispositivo móvil real (`media_1791527630015.png`, `media_1791527644205.png`).
- Intención: Que los controles táctiles y la carcasa SP se visualicen a tamaño ergonómico adecuado y con alta fidelidad estética respecto al prototipo interactivo aprobado (`layout_tactil_v2.html`), funcionando fluidamente tanto en orientación vertical (portrait) como horizontal (landscape).
- Restricciones y decisiones previas (máx. 3):
  - Fidelidad dimensional al prototipo aprobado `layout_tactil_v2.html` (origen: solicitud del usuario y prototipo validado) → implica: cruceta de ~43% del ancho de pantalla, clúster AB de ~42% del ancho, centrados en la mitad inferior de la carcasa en portrait y anclados ergonómicamente en landscape sin topes artificiales de 120 px → protegida por: `test_touch_layout_physical_proportions`.
  - Actualización reactiva al cambiar de orientación o redimensionar (origen: reporte de controles no visibles en landscape en dispositivo real) → implica: `TouchController` debe sincronizar sus dimensiones `(w, h)` y recalcular las hitboxes cuando la ventana física cambia de tamaño → protegida por: `test_touch_controller_resizes_dynamically`.
  - Renderizado sin desbordamiento de glifos ni artefactos de mosaico (origen: capturas de hardware real con bloques rotos superpuestos) → implica: el tamaño de celda del glifo debe escalarse a ~12% del radio del botón y su alfa debe multiplicarse estrictamente por la máscara de relleno del botón en WGSL → protegida por: `test_overlay_glyph_within_bounds`.
- Alternativas descartadas:
  - Cargar texturas PNG pre-renderizadas para los botones → descartada para mantener renderizado procedural puro en GPU sin dependencias de I/O ni escalado borroso a distintos DPIs (`AGENTS.md` §2).
  - Forzar la aplicación únicamente en orientación portrait mediante AndroidManifest → descartada porque el usuario solicitó explícitamente soporte de ambos modos (portrait con carcasa SP y landscape translúcido).
- Ante imprevistos, prioriza: ergonomía táctil y estabilidad de renderizado en GPU sobre efectos decorativos secundarios.

### 3. Ejemplos

- Caso normal (Portrait 1080x2400):
  - Antes: la unidad de escala estaba topada a `120.0 px` por un `.clamp(46.0, 120.0)` heredado de CSS; la cruceta medía apenas ~100 px arrinconada en `y = 2220` (junto al borde inferior) y los glifos tenían celdas de `0.42 * radio`, creando bloques desfigurados de ajedrez flotando fuera del botón.
  - Ahora: la cruceta mide ~460 px de diámetro centrada en `y ≈ 1720` en la mitad inferior plateada, el clúster AB mide ~440 px con L/R alineados encima de B/A, y los glifos están nítidamente contenidos dentro de cada botón sin salirse de su perímetro.
- Caso normal (Landscape 2400x1080):
  - Antes: la ventana cambiaba de orientación pero `TouchController` conservaba el layout de portrait con centros en `y = 2220`, quedando todos los controles fuera de la pantalla de 1080 de alto; la pantalla se veía vacía y oscura.
  - Ahora: al rotar o redimensionar, el controlador detecta el cambio de `(w, h)`, recalcula la orientación a Landscape y ubica los controles de forma translúcida en las esquinas inferiores con márgenes seguros respecto a los bordes y la barra de navegación del sistema.
- Caso límite (Dispositivo con notch o ratio ultra-panorámico 21:9 o 19.5:9):
  - Antes: posiciones calculadas solo respecto al borde inferior podían superponerse con barras de navegación por gestos.
  - Ahora: se preserva un margen de guarda inferior y lateral porcentual para garantizar que los botones permanezcan dentro del área segura interactiva.

### 4. Pasos

1. En `src/input.rs`:
   - Almacenar `size: (u32, u32)` en `TouchController` y exponer un getter `size(&self) -> (u32, u32)`.
   - Reformular `layout()` eliminando el tope rígido de `120.0` px.
   - En Portrait:
     - Escalar la cruceta con `dpad_radius = wf * 0.215` (~460 px de diámetro en 1080p).
     - Centrar la fila principal en `cy = hf * 0.72` (centrado en la carcasa inferior).
     - Ubicar la cruceta en `cx = wf * 0.25` y el clúster AB en `cx = wf * 0.75`.
     - Ubicar Select y Start en `cy = hf * 0.91` con separación proporcional.
   - En Landscape:
     - Escalar con base en la altura `hf`: `dpad_radius = hf * 0.20`.
     - Ubicar los controles en las esquinas inferiores: `dpad_cx = hf * 0.32`, `dpad_cy = hf * 0.72`, `right_cx = wf - hf * 0.32`.
     - Ubicar Select y Start en `cy = hf * 0.90` centrados horizontalmente.
2. En `src/overlay.rs`:
   - Ajustar `glyph_scale` en `ElemParams` a `0.13` (para que el glifo 3x5 mida `0.39 * r` de ancho y `0.65 * r` de alto, perfectamente centrado dentro del botón).
   - En modo Landscape, configurar colores neutros translúcidos (gris suave) para los botones de acción en lugar de rojo/verde saturados, respetando la preferencia del usuario.
3. En `src/overlay.wgsl`:
   - Corregir el cálculo y la composición del glifo: el alfa del glifo debe multiplicarse estrictamente por el `fill` del botón (`g.a * fill`), impidiendo cualquier desbordamiento fuera del círculo/cápsula.
   - Suavizar las celdas del glifo para que los bits no formen bloques cuadrados duros de ajedrez, sino glifos nítidos con antialiasing.
   - Dibujar la carcasa y pozos de fondo para que la cruceta y el clúster conserven la estética de pozo circular y cápsula descrita en el prototipo.
4. En `src/platform.rs`:
   - En `redraw()`, verificar si `window.size()` difiere de `touch.size()`, y llamar a `touch.resize(w, h)` automáticamente.
   - En `MainEvent::WindowResized`, invocar el redibujado de la ventana.
5. En `tests/` o en las suites unitarias:
   - Crear pruebas para validar que las proporciones físicas sean acordes a pantallas móviles y que la sincronización reactiva funcione al cambiar el tamaño de ventana.
6. Recompilar la biblioteca nativa y generar el APK con `scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- `src/input.rs`: hitboxes y coordenadas de colisión táctil.
- `src/overlay.rs`: parámetros pasados a la GPU en el uniforme `elems`.
- `src/overlay.wgsl`: shader de fragmento ejecutado en cada fotograma.
- `src/platform.rs`: sincronización entre la ventana nativa y el controlador táctil.
- `target/blazing-recomp-debug.apk`: nuevo paquete APK generado para pruebas en hardware real.

### 6. Suposiciones

- La resolución física informada por `NativeWindow` en Android refleja los píxeles reales de la superficie (e.g. 1080x2400) → prototipar: no → si es falso: normalizar mediante DPI o escala de densidad de display → resultado del prototipo: | no detecta: dispositivos con ratios inusuales menores a 4:3.
- Reducir `glyph_scale` a 0.13 y multiplicar por `fill` en el shader elimina completamente los artefactos visuales de bloques flotantes → prototipar: sí → si es falso: sustituir la tabla de bits por curvas implícitas SDF analíticas para cada letra → resultado del prototipo: | no detecta: pantallas de muy baja densidad (<160 dpi).

### 7. Alcance

- Incluye: corrección de la fórmula de escala geométrica en `src/input.rs`, sincronización de tamaño en `src/platform.rs`, corrección de shaders y colores en `src/overlay.wgsl` y `src/overlay.rs`, tests unitarios de proporciones físicas y regeneración del APK de depuración.
- No incluye: menús interactivos de ajustes en tiempo de ejecución (Fase 6), carga de carátulas personalizadas para el fondo de la carcasa, ni selector SAF de archivos.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Proporciones físicas ergonómicas en Portrait y Landscape
Caso de uso: Escalar controles a porcentajes de pantalla realistas sin topes diminutos de 120 px | Tipo: normal
  Dado que se solicita el layout para una resolución Portrait típica de 1080x2400
  Cuando se calcula la distribución de controles con `layout(1080, 2400, Orientation::Portrait)`
  Entonces el radio de los botones de acción B y A es superior a 60 px físicos y el centro vertical de la fila principal se ubica entre el 65% y el 80% de la altura de la pantalla
Test: `test_touch_layout_physical_proportions`
Estado previo: falla (actualmente radio de A es 74.4 px pero centro está en 92.5% de la altura, pegado al borde inferior)

Escenario: Sincronización reactiva del controlador táctil al rotar la pantalla
Caso de uso: Evitar que los controles se queden con coordenadas de Portrait al pasar a Landscape | Tipo: normal
  Dado que el controlador táctil se inicializa en Portrait (1080x2400)
  Cuando se actualiza el tamaño a Landscape (2400x1080) con `resize`
  Entonces la orientación cambia a `Orientation::Landscape`, `touch.size()` reporta `(2400, 1080)` y todos los botones están situados dentro de los límites verticales de 1080 px
Test: `test_touch_controller_resizes_dynamically`
Estado previo: falla (`touch.size()` no existe actualmente)

Escenario: Confinamiento estricto de glifos dentro del cuerpo del botón
Caso de uso: Eliminar artefactos donde los píxeles del glifo desbordan fuera del círculo del botón | Tipo: regresión
  Dado que los parámetros de dibujo del overlay se empaquetan para un botón
  Cuando se evalúa el shader con `naga` y se valida el factor de escala del glifo
  Entonces el shader compila y valida sin errores y la escala del glifo respecto al radio es inferior o igual a 0.20
Test: `test_overlay_glyph_within_bounds`
Estado previo: falla (actualmente escala es 0.42, desbordando el botón)

### 9. Cómo lo comprobaré

- Proporciones físicas ergonómicas en Portrait y Landscape → herramienta o comando: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_touch_layout_physical_proportions`
  → pasa si: test unitario finaliza con código 0 → no detecta: sensaciones táctiles subjetivas en la mano del usuario.
- Sincronización reactiva del controlador táctil al rotar la pantalla → herramienta o comando: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_touch_controller_resizes_dynamically`
  → pasa si: test unitario finaliza con código 0 → no detecta: latencias del compositor gráfico de Android.
- Confinamiento estricto de glifos dentro del cuerpo del botón → herramienta o comando: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_overlay_glyph_within_bounds`
  → pasa si: test unitario y validación de WGSL con `naga` finalizan con código 0 → no detecta: artefactos de rasterización en GPUs OpenGL ES 2.0 obsoletas.
- Inspección visual en dispositivo real → verificación humana mediante APK generado con `scripts/package_apk.ps1`.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y se genera un nuevo APK probado y listo para instalar.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

## Revisión humana

- (visible) Los controles táctiles en portrait ocupan de manera ergonómica la mitad inferior sin verse diminutos ni arrinconados en la base: `✔ | ✘ | ?`
- (visible) Los botones en landscape aparecen flotantes en las esquinas inferiores con transparencia adecuada y sin desaparecer: `✔ | ✘ | ?`
- (visible) Los glifos (flechas, A, B, L, R, Select, Start) se ven nítidos, centrados y sin mosaicos cuadrados desfigurados fuera de los botones: `✔ | ✘ | ?`
