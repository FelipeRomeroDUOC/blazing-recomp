# Propuesta: Fase 1.1 — Corrección PPU Shim: Offset VRAM Sprites, Paleta OBJ, Mapeo 1D y Pase de Fondos Modo 0

Fecha: 2026-10-16
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Esta propuesta corrige las discrepancias de renderizado visual observadas tras la integración de la Fase 1 en el hardware real (60 FPS estables pero con gráficos distorsionados y fondo negro), alineando la implementación con la especificación de hardware de la Game Boy Advance:

1. **Separación de Memoria VRAM para Sprites (OBJ Charblocks 4 y 5):**
   - Actualmente, `unpack_tiles_4bpp` leía desde el offset 0 de la VRAM (los primeros 32 KB asignados a fondos).
   - Se ajusta la extracción de tiles de sprites para leer exclusivamente desde el offset `0x10000` (64 KB a 96 KB), donde residen los 1024 tiles reales de los sprites de GBA.
2. **Offset de Paleta para Sprites (256 colores superiores):**
   - La paleta de GBA (1 KB, 512 colores) reserva los primeros 256 colores (`0..255`) para fondos y los siguientes 256 colores (`256..511`) para sprites.
   - Se corrige el fragment shader de sprites para indexar `256 + palette_idx * 16` (y `256 + color_idx` en modo 8bpp), eliminando la corrupción de color.
3. **Mapeo Lineal 1D de Sprites (`DISPCNT` bit 6):**
   - *Fire Emblem: The Blazing Blade* utiliza **1D OBJ mapping**. Los sprites multicuadrante (16×16, 32×32, etc.) leen tiles contiguos en memoria lineal ($N, N+1, N+2\dots$).
   - Se implementa el cálculo de coordenadas UV lineal en el shader para que los sprites grandes se ensamblen sin cortes verticales.
4. **Pase de Renderizado de Fondos Modo 0 (`bg_pipeline`):**
   - Se incorpora al pase de renderizado GPU la emisión de las capas de fondos Modo 0 (`BG0` a `BG3`) activadas en `DISPCNT`, dibujando los screen tilemaps con sus registros de desplazamiento `BGxHOFS`/`BGxVOFS` y prioridades, sobre los cuales se superponen los sprites.

---

### 2. Contexto

- **Contexto consultado:** Fotos de ejecución en pantalla de TV (`60 FPS · AOT · OpenGL`), [`src/ppu_shim.rs`](../../src/ppu_shim.rs), [`src/ppu_shim.wgsl`](../../src/ppu_shim.wgsl), [`src/platform.rs`](../../src/platform.rs), especificación de video de GBA (GBATEK - LCD I/O Display Control and VRAM) y [`docs/propuestas/2026-10-16_fase-1-ppu-shim-gpu.md`](2026-10-16_fase-1-ppu-shim-gpu.md).
- **Intención:** Corregir los 4 desajustes específicos identificados en la prueba en vivo para que los menús, textos y personajes de *Fire Emblem* se vean correctamente y con colores fieles, preservando la tasa de 60 FPS estables alcanzada en Android.
- **Restricciones y decisiones previas (máx. 3):**
  1. *Cero asignaciones dinámicas en caliente:* Los búferes de uniformes de fondos y atlas de tiles se mantienen preasignados y reutilizados cuadro a cuadro (origen: `AGENTS.md` §2.5) → protegida por: "No se invocan `Vec::push` ni `Box::new` en el ciclo de dibujo de fondos y sprites".
  2. *Topología modular y pruebas satélite:* Toda corrección algorítmica de UVs, paleta y decodificación debe respaldarse con tests unitarios en host en `src/ppu_shim_tests.rs` (origen: `AGENTS.md` §4.4) → protegida por: "Tests satélite verifican el offset 0x10000 de VRAM, el offset +256 de paleta y el indexado 1D".
  3. *Portabilidad total y privacidad:* Cero rutas locales del sistema de archivos en los archivos modificados (origen: `AGENTS.md` §6) → protegida por: "Auditoría de rutas absolutas limpia antes de commitear".
- **Alternativas descartadas:**
  - *Generar quads independientes de 8×8 por cada sub-tile en CPU:* Descartado porque aumentaría las llamadas y vértices por fotograma; es mucho más eficiente resolver el indexado 1D en el shader instanciado.
  - *Regresar al rasterizado por software en CPU:* Descartado porque la prueba ya demostró que el compositor GPU sostiene 60 FPS estables en Android sin caídas de framerate.
- **Ante imprevistos, prioriza:** Precisión visual de las capas de diálogo y menú sobre efectos de rotación afín no empleados en Modo 0.

---

### 3. Ejemplos (Antes / Ahora)

- **Caso 1: Paleta y VRAM de Sprites (Logo / Personaje):**
  - *Antes:* Sprite lee VRAM en offset 0 (Charblock 0 de fondos) e indexa la paleta de fondos `0..15`. Resultado: fragmentos de mapa con colores azules y rojos saturados.
  - *Ahora:* Sprite lee VRAM en offset `0x10000` (Charblock 4 de sprites) e indexa la paleta de OBJ `256..271`. Resultado: el sprite muestra los gráficos y colores auténticos del personaje/logo.
- **Caso 2: Fondo de Diálogo / Título (Modo 0):**
  - *Antes:* `render_frame` solo ejecutaba `pass.set_pipeline(&self.sprite_pipeline)`. Pantalla en negro detrás de los sprites.
  - *Ahora:* `render_frame` ejecuta primero las capas activas de fondos `BG3`, `BG2`, `BG1`, `BG0` según prioridad y luego los sprites. La pantalla muestra los escenarios y cajas de texto.

---

### 4. Pasos

1. **Corrección de desempaquetado de tiles de sprites (`src/ppu_shim.rs`):**
   - Dividir el atlas o añadir extracción desde `0x10000..0x18000` para los 1024 tiles de OBJ.
   - Ajustar el desempaquetado de tiles de fondos para cubrir los Charblocks 0 a 3 (`0x0000..0x10000`).
2. **Corrección del shader de sprites (`src/ppu_shim.wgsl`):**
   - Desplazar la lectura de paleta con `+ 256u` para colores OBJ.
   - Soportar el mapeo 1D calculando el tile lineal en base a las dimensiones del sprite y `DISPCNT`.
3. **Pase de fondos Modo 0 (`src/ppu_shim.rs` y `src/ppu_shim.wgsl`):**
   - Subir el búfer de tilemaps y uniformes de fondos (`BGxCNT`, `HOFS`, `VOFS`).
   - Enlazar y ejecutar el pipeline de fondos en el `RenderPass` antes de dibujar los sprites.
4. **Validación con tests satélite (`src/ppu_shim_tests.rs`):**
   - Test unitario para desempaquetado de tiles de OBJ en `0x10000`.
   - Test unitario para offset de paleta de sprites `256..511`.
   - Test unitario para mapeo 1D de sprites multicuadrante.
5. **Generación y verificación del APK:**
   - Ejecutar `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.
   - Comprobar que el APK se empaquete y firme limpiamente.

---

### 5. Qué más podría verse afectado

- Archivos a modificar:
  - `src/ppu_shim.rs` (desempaquetado VRAM OBJ/BG, configuración de fondos en pipeline).
  - `src/ppu_shim.wgsl` (offsets de paleta, indexado 1D, pase de fondos).
  - `src/ppu_shim_tests.rs` (nuevas pruebas satélite).
- Pruebas que deben continuar pasando:
  - `cargo test` (146 tests actuales más los nuevos).
  - `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` (0 errores).
  - `python scripts/extraer_referencias.py --check` (0 errores).

---

### 6. Criterios de Aceptación Comprobables

1. **Offsets de VRAM y Paleta:** Los sprites leen exclusivamente desde el rango `0x10000..0x18000` y consultan la paleta a partir del índice 256.
2. **Mapeo 1D correcto:** Sprites de 16×16 y 32×32 no presentan discontinuidades verticales en sus tiles.
3. **Dibujo de Fondos:** Las capas de fondos habilitadas en `DISPCNT` se emiten en el `RenderPass` previo a los sprites.
4. **60 FPS sostenidos:** El APK mantiene los 60 FPS estables sin asignaciones dinámicas por cuadro.
5. **Suite en verde:** `cargo test` aprueba el 100% de las pruebas.

---

### 7. Plan de Rollback

- La bandera `gpu_ppu_enabled` en `EmulationBridge` y `AppState` permite volver al renderizado CPU en una línea si fuera necesario.
- Los cambios quedan contenidos en la rama `feature/hardware-shim`.

---

### Fuera de alcance (hallazgos que no se arreglaron)

- *Modos afines rotados (Modo 1 y 2):* Fire Emblem opera en Modo 0; los modos afines quedan para cuando se requiera cinemáticas rotadas complejas.
- *Fase 2 (Bus & Timing Shim):* La sustitución de `RtApi` y reloj AOT se abordará tras estabilizar la imagen visual.

---

### Predicho vs. real

| Aspecto | Predicho en la propuesta | Resultado real verificado | Desvío / Comentario |
|---|---|---|---|
| **VRAM Sprites** | Extracción desde `0x10000..0x18000` | Implementado en `unpack_obj_tiles_4bpp` y validado por test satélite | 0 desfases; lee exactamente los Charblocks 4 y 5 |
| **Paleta OBJ** | Offset `+256` en shader | Implementado en `fs_sprite` (`256u + ...`) y validado por test de shader Naga | Descarte exacto de índice 0 y rango 256..511 |
| **Mapeo 1D** | Cálculo lineal contiguo de tiles | Resuelto en WGSL con `flags & 32u` y `base_tile + ty * tw + tx` | Sin partición en CPU; cálculo directo en GPU |
| **Pase Fondos** | Dibujo de capas Modo 0 en RenderPass | 4 uniform buffers y 4 bind groups independientes renderizando BG0..BG3 según prioridad | Cero colisiones de uniformes entre capas consecutivas |
| **Suite Global** | ≥ 146 tests en verde | 149 tests aprobados (127 unitarios + 4 golden + 18 packaging) | +3 nuevos tests satélite específicos |
| **APK Android** | Generación limpia con `package_apk.ps1` | `target/blazing-recomp-release.apk` generado (44.51 MB, multi-ABI, 16 KB alineado, firmado v1/v2/v3) | Listo para probar en hardware Android |
