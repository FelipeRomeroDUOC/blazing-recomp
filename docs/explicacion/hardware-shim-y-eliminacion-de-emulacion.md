---
tipo: explicacion
verificado: si
---

# Arquitectura del Hardware Shim y Eliminación de la Emulación

Este documento explica los fundamentos técnicos, la justificación de rendimiento y la arquitectura del **Hardware Shim** concebido para reemplazar el subsistema de hardware emulado (`gba-core`) por una capa nativa orientada a GPU en Android, desarrollada en la rama `feature/hardware-shim`.

---

## Contexto

El port nativo de *Fire Emblem: The Blazing Blade* en **`blazing-recomp`** utiliza un modelo híbrido:
1. **CPU Nativa AOT:** El código de instrucciones ARM7TDMI del cartucho original se recompila estáticamente a C11 (`src-c/`), ejecutándose directamente en los núcleos de CPU física del dispositivo (ARM64-v8a en Android).
2. **Hardware Emulado por Software (`gba-core`):** El comportamiento del bus de memoria, la PPU (gráficos), el APU (audio), los controladores DMA y los temporizadores se emula por software en Rust.

### La problemática en dispositivos de gama baja

A pesar de que el código aritmético y de control corre de forma nativa en la CPU, los dispositivos Android de gama de entrada (procesadores con núcleos Cortex-A53 o Cortex-A55) experimentan dificultades para sostener 60 FPS estables, cayendo a tasas de 30-45 FPS en escenas con múltiples capas o ventanas activas.

Por el contrario, los emuladores comerciales maduros de GBA (como *mGBA* o *MyBoy*) alcanzan 60 FPS sin caídas en estos mismos dispositivos económicos. Las investigaciones y perfiles de telemetría del proyecto (`docs/propuestas/2026-10-10_medicion-rendimiento-por-fase.md` y `docs/explicacion/DECISIONES.md` §§ 29-39) arrojaron los siguientes diagnósticos clave:

* **Rasterizado por software en CPU:** El módulo `ppu.rs` de `gba-core` contiene más de 2.100 líneas de rasterizado que componen cada una de las 160 líneas horizontales visibles de la pantalla píxel a píxel en CPU (`render_scanline`), decodificando tiles de 4/8 bpp, calculando prioridades de 128 sprites de la OAM, ventanas de recorte y mezclas alfa. En CPUs móviles de baja frecuencia, esta sola fase consume entre 8 y 14 ms por fotograma de un presupuesto total de 16,67 ms.
* **Sobrecarga de granularidad en la simulación:** En un emulador cycle-accurate, la CPU se interrumpe periódicamente cada 1.232 ciclos para procesar eventos de línea de video (HBlank), lo que fragmenta la ejecución del código C recompilado.
* **La paradoja de los emuladores maduros:** Los emuladores de alto rendimiento logran su velocidad sacrificando precisión cycle-accurate mediante *frame-based rendering* (componer el fotograma completo de golpe) y aceleración vectorial SIMD/NEON de 128 bits en ensamblador. En un proyecto cuyo objetivo es un port nativo definitivo, continuar emulando el silicio de la consola por software resulta redundante cuando el smartphone posee una GPU moderna y múltiples núcleos.

---

## Idea central

La solución adoptada consiste en **eliminar la emulación del hardware de GBA** e implementar un **Hardware Shim nativo**. 

En lugar de que el código C11 interactúe con un bus simulado que rasteriza por software en la CPU, el Shim proporciona una implementación directa de la interfaz `RtApi` que redirige el dibujo de fondos y sprites a la GPU del dispositivo a través de `wgpu` (Vulkan / OpenGL ES), almacena la memoria en arreglos contiguos planos sin decodificación de mirrors y sincroniza el tiempo directamente con la tasa de refresco del panel.

```text
                           MODELO ACTUAL vs. HARDWARE SHIM
 
   [ ACTUAL: CPU Nativa + Hardware Emulado ]
   Código C11 (Lógica FE7) ──► RtApi ──► gba-core (MemMap)
                                              │
                                              ▼ (160 scanlines en CPU)
                                         ppu.rs (Software Rasterizer)
                                              │
                                              ▼ Framebuffer 240×160 RGB555
                                         pipeline.rs ──► GPU (Blit simple)

   [ OBJETIVO: Hardware Shim Nativo ]
   Código C11 (Lógica FE7) ──► RtApi ──► Hardware Shim (Memoria Plana)
                                              │
                                              ▼ (En VBlank: Cero rasterizado CPU)
                                         GPU wgpu / Vulkan
                                         (Draw Calls instanciadas de Tiles y Sprites)
```

---

## Anatomía del Sistema: Recompilación y la Interfaz `RtApi`

El código generado por el recompilador en `src-c/` no tiene conocimiento directo de `gba-core`. Se comunica exclusivamente mediante la tabla de punteros de función `RtApi` (`crates/gba-core/src/capi.rs`):

```rust
#[repr(C)]
pub struct RtApi {
    pub interp_one: extern "C" fn(*mut c_void) -> u32,
    pub read8:      extern "C" fn(*mut c_void, u32) -> u32,
    pub read16:     extern "C" fn(*mut c_void, u32) -> u32,
    pub read32:     extern "C" fn(*mut c_void, u32) -> u32,
    pub write8:     extern "C" fn(*mut c_void, u32, u32),
    pub write16:    extern "C" fn(*mut c_void, u32, u32),
    pub write32:    extern "C" fn(*mut c_void, u32, u32),
    pub tick:       extern "C" fn(*mut c_void, u32),
    pub guard:      extern "C" fn(*mut c_void, u32, u32, u64) -> u32,
    pub chain_gate: extern "C" fn(*mut c_void) -> u32,
}
```

El parámetro opaco `*mut c_void` que recibe cada bloque C es un puntero a la estructura `Machine`, cuyo primer elemento es el estado de la CPU (`uint32_t c[17]`: registros `r0..r15` más `CPSR`). 

Por lo tanto, para reemplazar `gba-core`, basta con que el nuevo componente nativo instancie una estructura con la misma disposición de registros de CPU al inicio y provea su propia tabla de funciones `RtApi`.

---

## Pilares Arquitectónicos del Hardware Shim

### 1. PPU Shim: De Rasterizado en CPU a Quads en GPU (`wgpu`)
* **Naturaleza gráfica de Fire Emblem:** El juego opera sobre todo en **Modo 0** de la GBA (cuatro capas de texto/fondos con tiles de 8×8 píxeles y paletas indexadas de 4 u 8 bits por píxel, más 128 sprites en OAM), y en los menús usa el **Modo 1**, cuyo fondo BG2 es afín (una transformación por línea, no un Modo 7). No utiliza deformaciones raster por línea de escaneo durante HBlank.
* **Mecanismo del Shim:**
  1. VRAM (96 KB), OAM (1 KB) y Palette RAM (1 KB) se alojan como arreglos contiguos en memoria RAM.
  2. Las escrituras del juego en estas regiones marcan banderas de actualización (`vram_dirty`, `oam_dirty`).
  3. Al concluir la lógica del fotograma (señal de VBlank):
     - La VRAM se transfiere a un atlas de texturas en GPU.
     - Las capas de fondos (mapa del terreno, interfaz, menús) y los 128 sprites de la OAM se emiten como mallas de cuadriláteros (*quads*) instanciados mediante un pipeline de renderizado en `wgpu`.
     - El ordenamiento de capas y las ventanas de recorte se resuelven mediante el depth buffer / stencil de la GPU o en un fragment shader de una sola pasada.
  4. **Impacto:** El tiempo de CPU dedicado a gráficos cae de ~10 ms a menos de **0,2 ms**.

### 2. Memoria Plana y Erradicación del Bus Lento
* En la consola original, la memoria presenta regiones espejadas (*mirrors*) y estados de espera (*waitstates*).
* En el Shim:
  - **IWRAM** (32 KB en `0x03xxxxxx`) y **EWRAM** (256 KB en `0x02xxxxxx`) se mapean en memoria directa indexada con máscaras fijas (`addr & 0x7FFF` y `addr & 0x3FFFF`).
  - Las lecturas y escrituras de memoria ejecutan accesos contiguos sin validaciones de periféricos no utilizados.

### 3. Sincronización y Retiro del Reloj de Cuarzo
* En la consola real, `a->tick` avanza el tiempo para coordinar accesos de bus con los ciclos del cristal de cuarzo de 16,78 MHz.
* Como se demostró en la Decisión 39 (`docs/explicacion/DECISIONES.md`), la lógica de *Fire Emblem* solo pasa una fracción menor de ciclos por `a->tick`, pasando la mayor parte del fotograma en reposo (*halted*) esperando la interrupción de refresco vertical (`VBlankIntrWait`).
* En el Shim:
  - `a->tick` se convierte en una función trivial (cero costo de simulación).
  - La llamada del BIOS `SWI 0x05` (`VBlankIntrWait`) se intercepta directamente para suspender el hilo nativo hasta que el presentador de pantalla (`wgpu` y el `Choreographer` de Android) emita la señal del próximo fotograma, garantizando sincronización perfecta a 60 FPS y consumo mínimo de batería.

### 4. Audio Shim
* *Fire Emblem* utiliza el motor de sonido *Music Player 2000* (M4A). 
* En lugar de emular el sintetizador de hardware PSG y los canales DMA de audio que vuelcan en registros FIFO ciclo a ciclo, el Shim intercepta las rutinas de síntesis del motor de sonido para canalizar los búferes de audio PCM generados directamente a la API de audio de baja latencia de Android (AAudio u Oboe).

---

## Hoja de Ruta de Implementación (Rama `feature/hardware-shim`)

Para preservar la estabilidad del proyecto y permitir validación continua frente a la versión base de `main`, el desarrollo se estructura en tres fases incrementales:

1. **Fase 1: PPU Shim sobre GPU (`wgpu`)**
   - Mantener temporalmente la gestión de memoria y CPU actual, pero desviar el rasterizado de `ppu.rs` hacia un renderizador de fondos y sprites acelerado por GPU.
   - Meta: Recuperar los 8-14 ms de CPU por fotograma y verificar fidelidad visual exacta con los fotogramas de prueba (*golden frames*).

   **Continuación: la PPU nativa.** El shim compone por capas y no implementaba la *unidad de mezcla* de la GBA (alfa, aclarar, oscurecer), así que su imagen no coincidía con la PPU de referencia: en el cuadro 5300 de la secuencia de prueba la diferencia era de 2879 píxeles en las doce líneas sin sprites afines. El módulo `src/ppu_nativa.rs` (`src/ppu_nativa.wgsl`) rehace la composición **por píxel en la GPU**, con la aritmética exacta de la unidad de mezcla de la referencia (`crates/gba-core/src/ppu.rs`): verde en un carril de 6 bits, acumulación con redondeo a media hacia arriba y las condiciones de mezcla de `compose()` (listas del 1.er y 2.º objetivo, máscara de efectos de la ventana, sprite semitransparente).

   La diferencia entre el shim y la PPU nativa no era una diferencia de *estilo*: era un **error de modelo**. La PPU nativa no compone las capas una encima de otra: las evalúa por píxel, elige las dos capas más altas según la prioridad de `BGxCNT` y aplica la mezcla, que es exactamente lo que hace el hardware. Por eso el resultado no se parece a la referencia, **es** la referencia: contra la PPU de software alimentada con el mismo estado de fin de cuadro, la PPU nativa da **0 diferencias de 38400 píxeles** en los dos cuadros de aceptación (5300, con mezcla alfa, y 7500, sin efectos). El detalle, con las mediciones, está en [`docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`](../propuestas/2026-10-19_ppu-nativa-fase-1.md).

   La PPU nativa se dibujó en principio dejando que el vértice leyera la lista de sprites desde un *storage buffer*. Eso funciona en un equipo de escritorio con Vulkan, pero **exige al dispositivo una capacidad que el estándar de OpenGL ES no garantiza**: `DownlevelFlags::VERTEX_STORAGE`, es decir, *storage buffers* visibles desde la etapa de vértices. Un dispositivo sin Vulkan acaba en el backend de OpenGL ES, ese backend puede no dar esa capacidad, y entonces la creación del layout de enlaces aborta la app con un error de validación. La solución no es pedir más capacidad al dispositivo, sino **no usarla**: la lista de sprites completa sigue en un *storage buffer* que solo lee el fragmento (donde hacen falta la paleta, la mezcla y los parámetros de la transformación afín), y la geometría que el vértice necesita —esquina, tamaño y clave de prioridad— viaja en un búfer de instancias como atributos de vértice, que es WebGPU básico. El detalle, con los tres tests que lo blindan, está en [`docs/propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md`](../propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md).

   El fondo deformable de los menús es el **fondo afín BG2 del modo 1**, y la PPU nativa también lo dibuja: `CapaFondoGpu` lleva los ocho registros del afín (`PA`, `PB`, `PC`, `PD`, `BG2X`, `BG2Y`, el wrap y la marca de afín), y el shader muestrea esa capa con la misma aritmética que la referencia (`render_affine_bg` de `crates/gba-core/src/ppu.rs`): punto fijo 20.8, ancla de la línea `ref + línea·(PB, PD)`, paso por píxel `PA` (horizontal) y `PC` (vertical), vuelta al mapa con el módulo de su tamaño cuando el wrap está activo y transparencia fuera del mapa cuando no lo está. Con el fondo afín dibujado, el cuadro 900 de la secuencia `escena3` (el fondo girando de los menús) da **0 diferencias de 38400 píxeles** contra la PPU de software, donde antes había 5222. El detalle, con las mediciones y los casos límite, está en [`docs/propuestas/2026-10-21_fondo-afin-bg2-modo-1.md`](../propuestas/2026-10-21_fondo-afin-bg2-modo-1.md).

   El interruptor `PPU_NATIVA_ACTIVA` (`src/platform.rs`) está **apagado** por defecto: la ruta de producción sigue siendo la del shim, y encenderlo es una decisión que requiere ver la imagen en un dispositivo.
2. **Fase 2: Bus & Timing Shim**
   - Implementar el gestor de memoria plana (IWRAM, EWRAM, VRAM, OAM, Palette, I/O) y sustituir la tabla `RtApi` completa.
   - Reemplazar la simulación de ciclo a ciclo de `a->tick` por avance por fotograma gobernado por VSync.
3. **Fase 3: Audio Shim y Desvinculación de `gba-core`**
   - Conectar el generador PCM de audio a AAudio/Oboe.
   - Retirar completamente la dependencia de `gba-core` en `Cargo.toml`.

---

## Alternativas y compromisos

* **Alternativa descartada: Continuar optimizando la PPU por software en `gba-core`.**
  Las Decisiones 30, 35 y 36 optimizaron el desempaquetado de tiles y la resolución de mezclas alfa por subtramos, logrando mejoras apreciables en escritorio, pero en núcleos móviles lentos de 32/64 bits la barrera matemática de rasterizar 38.400 píxeles por software sigue agotando el presupuesto de fotograma.
* **Alternativa descartada: Decompilación manual completa a C desde cero.**
  Aunque el proyecto comunitario `fireemblem7decomp` existe, adaptar todo el código a un motor moderno desde cero requeriría años de desarrollo, perdiendo los 144.875 bloques C11 plenamente funcionales y testeados que `recomp build` ya generó.
* **Compromiso de aislamiento:** Todo el desarrollo se realiza en la rama `feature/hardware-shim`, manteniendo `main` como línea base funcional garantizada en todo momento.
