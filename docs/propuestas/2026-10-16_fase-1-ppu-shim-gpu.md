# Propuesta: Fase 1 — PPU Shim sobre GPU (wgpu) para Modo 0 y Sprites OAM

Fecha: 2026-10-16
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Esta propuesta implementa la **Fase 1 de la transición hacia el Hardware Shim nativo** en la rama `feature/hardware-shim`, atacando el cuello de botella más costoso del proyecto: el rasterizado por software en CPU de la PPU de `gba-core`.

- **Desactivación del rasterizado en CPU:** Se omite la ejecución de las 160 llamadas por fotograma a `render_scanline` en la CPU, eliminando el consumo de 8 a 14 ms por cuadro en procesadores móviles de bajo costo.
- **Renderizador PPU acelerado por GPU (`wgpu`):** Se introduce el subsistema `PpuShim` en el pipeline gráfico que lee directamente la memoria de video de la GBA (VRAM 96 KB, OAM 1 KB y Palette 1 KB) en el momento del VBlank y dibuja:
  1. **Capas de fondos Modo 0 (BG0..BG3):** Generación de quads instanciados para los mapas de tiles (32×32 / 64×64 tiles) con sus registros de desplazamiento horizontal y vertical (`BGxHOFS`/`BGxVOFS`) y prioridades de capa.
  2. **Sprites OAM:** Emisión instanciada de los 128 sprites (posiciones X/Y, dimensiones 8×8 hasta 64×64, volteo horizontal/vertical y paleta correspondiente).
  3. **Composición y Ventanas:** Resolución de transparencias y máscaras de ventanas (cajas de texto y menús) directamente en los shaders de la GPU.
- **Preservación temporal del resto del hardware:** La CPU AOT, la memoria RAM y el APU continúan operando sobre `gba-core::Machine` durante esta fase, aislando el cambio exclusivamente al subsistema gráfico para garantizar reproducibilidad y comparar contra la línea base.

---

### 2. Contexto

- **Contexto consultado:** [`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`](../explicacion/hardware-shim-y-eliminacion-de-emulacion.md), [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) (§ 41), [`src/runner.rs`](../../src/runner.rs), [`src/pipeline.rs`](../../src/pipeline.rs), [`src/present.rs`](../../src/present.rs), [`crates/gba-core/src/ppu.rs`](../../../gba-recomp/crates/gba-core/src/ppu.rs), [`crates/gba-core/src/mem.rs`](../../../gba-recomp/crates/gba-core/src/mem.rs) y la suite de pruebas de integración [`tests/golden_frame_tests.rs`](../../tests/golden_frame_tests.rs).
- **Intención:** Eliminar el 70-85% del tiempo de CPU consumido por fotograma en dispositivos de gama baja (Cortex-A53/A55), logrando que el tiempo de ejecución del juego baje de ~14 ms a ≤2 ms y permitiendo alcanzar 60 FPS estables sin alterar la lógica de juego.
- **Restricciones y decisiones previas (máx. 3):**
  1. *Cero asignaciones dinámicas por fotograma:* Los búferes de vértices, instancias y texturas de la GPU deben preasignarse y reutilizarse en cada cuadro (origen: `AGENTS.md` §2.5 rendimiento y batería) → protegida por: "El renderizado PPU en GPU no realiza `Box::new` ni redimensiona vectores en el bucle principal".
  2. *Topología modular y pruebas satélite:* La lógica del PPU Shim debe residir en `src/ppu_shim.rs` con tests satélite adyacentes `src/ppu_shim_tests.rs` testeables en host con `cargo test` (origen: `AGENTS.md` §4.4).
  3. *Fidelidad visual verificable:* El resultado de salida debe contrastarse frente a los fotogramas dorados de referencia (origen: `tests/golden_frame_tests.rs`) → protegida por: "La imagen resultante coincide en la escena de diálogo y menú con un umbral de tolerancia visual estricto".
- **Alternativas descartadas:**
  - *Vectorización NEON en CPU:* Optimizar el rasterizado en software con instrucciones SIMD de 128 bits en ensamblador ARM fue descartado porque sigue ocupando núcleos de CPU que deben dedicarse a la lógica y audio, y no aprovecha la GPU disponible.
  - *Reemplazar todo el hardware (PPU + Bus + Audio) de un solo golpe:* Descartado por alto riesgo de regresiones difíciles de aislar; el desarrollo incremental por fases garantiza verificar la fidelidad de cada pieza por separado.
- **Ante imprevistos, prioriza:** Exactitud visual en las capas de Modo 0 de *Fire Emblem* por encima de soportar modos gráficos de GBA no utilizados por el juego (Modo 3/4/5 bitmap o Modo 7 afín).

---

### 3. Ejemplos (Antes / Ahora)

- **Caso normal: Escena de diálogo o mapa de batalla (tablet/móvil gama baja, resolución nativa):**
  - *Antes:* CPU ejecuta `render_scanline` 160 veces consecutivas en software. `ppu_ms = 8,5..12,0 ms`, `ejec_ms = 14..18 ms`. La CPU supera los 16,67 ms por cuadro y la tasa cae a 35-45 FPS.
  - *Ahora:* CPU omite `render_scanline`. La GPU carga VRAM (96 KB) y OAM (1 KB) a búferes de GPU y emite las llamadas de dibujo instanciadas en un solo render pass. `ppu_ms ≈ 0,0 ms`, `ejec_ms ≤ 2,5 ms`. La CPU queda holgada y los 60 FPS se sostienen con margen.
- **Caso de prueba (Host `cargo test`): Decodificación de entrada de OAM:**
  - *Dado:* Un búfer de OAM con un sprite en coordenada `(X=120, Y=80)`, forma `Square 16x16`, paleta 3 y tile índice 42.
  - *Cuando:* `PpuShim::extract_sprite_instances(&oam, &palette)` procesa la memoria.
  - *Entonces:* Genera exactamente una instancia gráfica con posición `[120.0, 80.0]`, dimensiones `[16.0, 16.0]`, coordenadas UV del atlas correspondientes al tile 42 y el offset de color de la paleta 3.

---

### 4. Pasos

1. **Estructura y tipos de datos (`src/ppu_shim.rs` y `src/ppu_shim_tests.rs`):**
   - Definir las estructuras de vértices e instancias GPU: `BgInstance`, `SpriteInstance`, `TileAtlas`.
   - Implementar el decodificador de OAM de GBA (atributos 0, 1 y 2: posición, dimensiones, flip horizontal/vertical, prioridad 0-3 y modo 4bpp/8bpp).
   - Implementar el decodificador de mapas de fondos de Modo 0 (lectura de `BGxCNT`, base de mapa de pantalla, base de caracteres de tiles y offsets `BGxHOFS`/`BGxVOFS`).
2. **Shaders WGSL del PPU Shim (`src/ppu_shim.wgsl`):**
   - Shader de vértices instanciado para posicionar fondos y sprites según la cuadrícula de 240×160 píxeles.
   - Fragment shader que aplica la paleta indexada correspondiente (16 colores por subpaleta) y descarta fragmentos transparentes (índice de color 0).
   - Manejo del orden de dibujo mediante el índice de prioridad de la GBA (prioridades 0 a 3).
3. **Integración en el Pipeline Gráfico (`src/pipeline.rs`):**
   - Crear el pipeline de renderizado `PpuShimPipeline` en `wgpu` con sus bind group layouts (textura de atlas de tiles, búfer de paletas uniform y textura de destino).
   - Conectar el render pass para dibujar directamente hacia el framebuffer de presentación de [ScreenScaler](../../src/present.rs).
4. **Bandera de Bypass en `GameRunner` (`src/runner.rs`):**
   - Introducir una bandera `gpu_ppu_enabled: bool` en [GameRunner](../../src/runner.rs#L55).
   - Cuando esté activa, `step_frame` desactiva la llamada a `render_scanline` en `gba-core` y en su lugar expone los slices de memoria `(&bus.vram, &bus.oam, &bus.palette, &bus.io)` para el `PpuShim`.
5. **Verificación y Pruebas Satélite:**
   - Crear tests unitarios en `src/ppu_shim_tests.rs` que validen la extracción de quads frente a estados conocidos de memoria.
   - Ejecutar la suite completa con `cargo test` asegurando que los 131 tests actuales sigan en verde.

---

### 5. Qué más podría verse afectado

- Archivos a modificar:
  - `src/lib.rs` (declaración de `pub mod ppu_shim;`).
  - `src/runner.rs` (exposición de referencias de VRAM/OAM/Palette y bypass de PPU software).
  - `src/pipeline.rs` (integración del pass de dibujo PPU Shim).
- Archivos nuevos a crear:
  - `src/ppu_shim.rs` (lógica del PPU Shim).
  - `src/ppu_shim_tests.rs` (suite satélite de pruebas).
  - `src/ppu_shim.wgsl` (shaders de renderizado de fondos y sprites).
- Pruebas que deben continuar pasando:
  - `cargo test` (131 tests actuales).
  - `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` (0 errores).
  - `python scripts/extraer_referencias.py --check` (0 errores).

---

### 6. Criterios de Aceptación Comprobables

1. **Reducción medible del tiempo de PPU:** En la telemetría, el campo `ppu_ms` en CPU debe caer a **< 0,1 ms** por fotograma cuando el PPU Shim esté activo.
2. **Determinismo visual:** La extracción de instancias para los fondos y los 128 sprites genera la geometría y coordenadas UV idénticas a la especificación de hardware de GBA Modo 0.
3. **Cero asignaciones por cuadro:** El pipeline reutiliza los búferes de instancias preasignados para un máximo de 128 sprites y 4 capas de fondos sin invocar `Vec::push` ni `Box::new` en el ciclo caliente.
4. **Suite 100% verde:** Todos los tests unitarios e integrados pasan exitosamente con `cargo test`.

---

### 7. Plan de Rollback

Dado que el desarrollo ocurre en la rama `feature/hardware-shim` y se introduce bajo la bandera `gpu_ppu_enabled`:
- Si se detecta alguna discrepancia visual o incompatibilidad en un dispositivo específico, se puede alternar `gpu_ppu_enabled = false` para retornar inmediatamente al camino probado de `gba-core::ppu`.
- La rama `main` permanece inalterada y lista para producción en todo momento.

---

### Fuera de alcance (hallazgos que no se arreglaron)

- *Fase 2 (Bus & Timing Shim):* La eliminación de `a->tick` y la sustitución de la memoria plana de IWRAM/EWRAM se reservan para la Fase 2, una vez validada la fidelidad gráfica.
- *Fase 3 (Audio Shim):* El desacoplamiento del motor `m4a` hacia AAudio/Oboe se abordará en la Fase 3.
- *Efectos afines avanzados (Modo 1/2 y rotación de fondos):* Fire Emblem 7 opera en Modo 0 para menús y batallas; el soporte de fondos afines rotados queda diferido si no es requerido por el juego.

---

### Predicho vs. real

| Aspecto | Predicho en la propuesta | Resultado real verificado | Desvío / Comentario |
|---|---|---|---|
| **Módulo PPU Shim** | `src/ppu_shim.rs` con tipos de datos GPU, decodificación OAM y Modo 0 | Implementado con `SpriteGpuInstance` (pod/zeroable), `BgUniformsGpu` y `PpuShim` con búferes preasignados | Idéntico a lo planificado; cero asignaciones dinámicas en caliente. |
| **Shader WGSL** | `src/ppu_shim.wgsl` con mapeo de paleta indexada y descarte de transparentes | Implementado y verificado con `naga::valid::Validator` en `test_ppu_shim_shader_validates` | 0 errores de compilación ni validación de tipos WGSL. |
| **Suite Satélite** | `src/ppu_shim_tests.rs` con pruebas de atributos OAM, wrap-around y poda | 12 tests unitarios nuevos en `src/ppu_shim_tests.rs` pasando al 100% | Validación completa de las 16 formas/tamaños de sprites, coordenadas negativas y poda. |
| **Integración Runner y Pipeline** | Bandera `gpu_ppu_enabled` y transporte desacoplado de VRAM/OAM/Palette/IO | Integrados en `GameRunner`, `EmulationBridge` y `EmulationWorker` con pool reciclable `PpuFrameState` | Cero asignaciones por fotograma; el productor deposita de forma no bloqueante. |
| **Integración Compositor Android** | Renderizado directo sobre `game_texture` en `src/platform.rs` | Conectado con `RENDER_ATTACHMENT` y fallback automático si se desactiva el shim | Preserva el pipeline de escalado `present.rs`, overlay táctil y contador HUD. |
| **Suite Global** | 131 tests preexistentes verdes más tests nuevos | **146 tests aprobados** (124 unitarios + 4 golden frames + 18 packaging) | Cero fallos, cero regresiones. |
| **Empaquetado APK** | APK multi-ABI instalable y verificable en Android | Fat APK de 42.45 MB generado con `package_apk.ps1` (`arm64-v8a`, `armeabi-v7a`, `x86_64`) | Firmado v2/v3 y memoria 16 KB page-aligned con éxito. |
| **Documentación y Calidad** | Diátaxis y referencias sincronizadas | `validar_docs.py` (11 páginas, 0 errores, 0 avisos), `extraer_referencias.py` sincronizado | Cumple el checklist de pre-commit al 100%. |
