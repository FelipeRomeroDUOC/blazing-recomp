# Arquitectura de GPUs Móviles TBDR y Gestión de Ancho de Banda en Android

## 1. Fundamentos de la Arquitectura TBDR (*Tile-Based Deferred Rendering*)

En el desarrollo de emuladores, puertos nativos de videojuegos y aplicaciones gráficas de alto rendimiento para Android, la arquitectura de la Unidad de Procesamiento Gráfico (GPU) impone restricciones radicales que difieren sustancialmente del modelo tradicional de computadoras de escritorio.

Las GPUs de escritorio convencionales (NVIDIA GeForce, AMD Radeon) operan predominantemente bajo la arquitectura de **Modo Inmediato (*Immediate Mode Rendering*, IMR)**. En el modelo IMR, las primitivas geométricas (triángulos) se procesan secuencialmente a través de las etapas de vértices, rasterización y sombreado de fragmentos, escribiendo inmediatamente cada píxel resultante de vuelta a la memoria de video dedicada (VRAM) a través de un bus de memoria de gran ancho de banda (de 256 a 384 bits con memorias GDDR6/HBM, superando los 500–1000 GB/s) con un consumo eléctrico de 75 a 450 vatios.

Por el contrario, los sistemas en chip (*System-on-Chip*, SoC) móviles y dispositivos embebidos (smartphones, consolas portátiles y dispositivos Android TV / TV Sticks) están sujetos a un presupuesto térmico y de consumo estricto (generalmente entre 1.5 y 5 vatios para el SoC completo). Debido a que el acceso a la memoria principal dinámica fuera del chip (*off-chip DRAM*) es la operación de hardware que mayor energía disipa por bit transferido, las GPUs móviles adoptan la arquitectura de **Renderizado Diferido Basado en Mosaicos (*Tile-Based Deferred Rendering*, TBDR)** o sus variantes por bloques (*Tile-Based Rendering*). Este diseño es el estándar en arquitecturas como:
- **ARM Mali:** Series Midgard, Bifrost (ej. Mali-G31, Mali-G52 en dispositivos de entrada y TV Sticks) y Valhall.
- **Imagination Technologies PowerVR:** Series Rogue y sucesoras.
- **Qualcomm Adreno:** Mediante la tecnología de renderizado *FlexRender* (capaz de operar tanto en modo directo por tiles como en IMR).

```
+-------------------------------------------------------------------------------+
|                       ARQUITECTURA IMR (Immediate Mode)                       |
|                                                                               |
|  [Vértices] ---> [Rasterizado] ---> [Fragment Shader] ---> [DRAM / VRAM]      |
|                                                              (Alto ancho de   |
|                                                                banda externo) |
+-------------------------------------------------------------------------------+

+-------------------------------------------------------------------------------+
|                         ARQUITECTURA TBDR (Tile-Based)                        |
|                                                                               |
|  FASE 1 (Geometría):                                                          |
|  [Vértices] ---> [Binning / Tiling] ---> [Parameter Buffer (DRAM)]            |
|                                                                               |
|  FASE 2 (Fragmentos diferidos por Tile en SRAM on-chip):                      |
|                                    +---------------------------------------+  |
|                                    | Tile Cache (SRAM on-chip, 16x16 px)   |  |
|                                    |  - Rasterizado local                  |  |
|  [Parameter Buffer] -------------> |  - Oclusión HSR / FPK                 |  |
|                                    |  - Fragment Shading                   |  |
|                                    |  - Blending / Mezcla sin DRAM         |  |
|                                    +---------------------------------------+  |
|                                                        |                      |
|                                                        v                      |
|                                              [Frame Buffer en DRAM]           |
|                                           (1 sola escritura final por tile)   |
+-------------------------------------------------------------------------------+
```

---

## 2. El Pipeline de Dos Fases en TBDR

El principio operativo de una GPU TBDR consiste en desacoplar la carga geométrica de la generación de píxeles, subdividiendo el *render target* (la pantalla o búfer de cuadro) en una cuadrícula regular de celdas o teselas (*tiles*), normalmente de 16×16 o 32×32 píxeles. El pipeline divide su procesamiento en dos fases asíncronas consecutivas:

### Fase 1: Geometría y Clasificación por Teselas (*Binning / Tiling Pass*)
1. La GPU ejecuta los *Vertex Shaders* para transformar todas las primitivas geométricas del cuadro desde el espacio local al espacio de pantalla coordinado.
2. Un bloque de hardware dedicado (*Tiling Engine* o *Binner*) evalúa la posición de cada triángulo transformado y determina qué teselas (*tiles*) intersecta su superficie.
3. Se genera una lista de comandos por tesela en una región de la memoria del sistema conocida como **Búfer de Parámetros (*Parameter Buffer*)** o lista de primitivas de escena (*Polygon List*). Este búfer reside en DRAM externa, pero almacena exclusivamente estructuras ligeras de índices y vértices transformados, sin procesar ningún fragmento o píxel.

### Fase 2: Rasterización Diferida y Procesamiento Local en SRAM (*Fragment Pass*)
1. Una vez completada la lista de parámetros para todo el cuadro (o cuando el búfer de parámetros alcanza su umbral de asignación), la GPU procesa las teselas secuencial o paralelamente mediante sus núcleos de cómputo gráfico (*Shader Cores*).
2. Para cada tesela:
   - Se leen las primitivas asignadas a esa tesela concreta desde el *Parameter Buffer*.
   - Toda la rasterización, pruebas de profundidad (*Z-Buffer*) y sombreado de fragmentos se ejecutan **exclusivamente dentro de una memoria estática ultrarrápida integrada en el silicio (*on-chip SRAM Tile Buffer*)**.
   - **Eliminación de Superficies Ocultas (*Hidden Surface Removal, HSR* / *Forward Pixel Kill, FPK*):** Las GPUs TBDR determinan qué fragmentos son visibles antes de invocar el *Fragment Shader*. Los fragmentos que quedan ocluidos por geometría opaca posterior son descartados de inmediato sin ejecutar cálculos de sombreado ni lecturas/escrituras de texturas, lo que reduce la tasa de sobregiro (*overdraw*) a cero teórico en escenas opacas.
   - **Operaciones de Mezcla (*Alpha Blending*):** Las operaciones de transparencia y composición se realizan directamente sobre la memoria SRAM on-chip a velocidad de ciclo de reloj del núcleo, sin ningún costo de lectura o escritura contra la DRAM externa.
3. Al finalizar el renderizado de la tesela de 16×16 píxeles, el bloque resultante de colores se escribe de forma contigua en el búfer de pantalla final en la DRAM del sistema (*Tile Writeback*), típicamente aplicando algoritmos de compresión de búfer sin pérdidas como AFBC (*ARM Frame Buffer Compression*).

---

## 3. Arquitectura de Memoria Unificada (UMA) y Cuellos de Botella de Ancho de Banda

En plataformas móviles y dispositivos de bajo costo (como los SoCs Amlogic S905X2, S905Y2 o Allwinner habituales en TV Sticks y cajas multimedia con Android), no existe memoria de video dedicada (VRAM). La CPU, la GPU, la Unidad de Procesamiento de Video (VPU / decodificador de hardware H.264/HEVC) y el controlador de pantalla (*Display Engine*) comparten un único bus de memoria DRAM del sistema, conformando una **Arquitectura de Memoria Unificada (*Unified Memory Architecture*, UMA)**.

### Capacidad Teórica frente a Ancho de Banda Real
En un dispositivo típico de entrada (ejemplo con memoria LPDDR4 de 32 bits a 1600 MHz):

$$\text{Ancho de Banda Máximo Teórico} = \frac{32 \text{ bits}}{8 \text{ bits/byte}} \times 1600 \text{ MHz} \times 2 \text{ (DDR)} = 12.8 \text{ GB/s}$$

Sin embargo, en dispositivos de gama económica (especialmente TV Sticks con buses recortados a 16 bits de ancho o frecuencias de reloj de 1200–1333 MHz), el ancho de banda efectivo disponible oscila entre **4.8 GB/s y 6.4 GB/s**.

Este canal limitado de datos debe ser disputado concurrentemente por:
1. El sistema operativo Android y los procesos en segundo plano de la CPU.
2. El decodificador de video por hardware (VPU) si existe contenido multimedia en reproducción.
3. El controlador del display, que debe refrescar la pantalla a 1080p o 4K a 60 Hz sin interrupción para evitar desconexiones HDMI.
4. La GPU ejecutando la lógica gráfica del juego y la composición de la interfaz del sistema.

Cualquier sobreconsumo de ancho de banda por parte de la GPU degrada inmediatamente el rendimiento global, elevando la latencia de acceso a memoria para la CPU y provocando caídas de fotogramas (*frame drops*).

---

## 4. Transferencias de Texturas (*Texture Uploads*) y Penalizaciones de Sincronización

En la emulación o recompilación de sistemas retro como Game Boy Advance, el motor de ejecución genera cuadros o líneas de escaneo en memoria principal (CPU) que deben ser transmitidos a la GPU para su escalado, filtrado y presentación en pantalla. En una arquitectura TBDR con UMA, el método empleado para transferir estos datos define la diferencia entre un rendimiento fluido a 60 FPS o un colapso del tiempo de cuadro.

### El Peligro del *Tile Flush* y las Llamadas Bloqueantes
En una GPU IMR de escritorio, llamar a funciones como `glTexSubImage2D` copia datos a un búfer intermedio del driver de forma relativamente transparente. En una GPU TBDR:
- Si se emite una orden de actualización de textura mientras la GPU está procesando la Fase 2 (rasterizado diferido de fragmentos) o si se introducen barreras de sincronización inadecuadas, el hardware se ve obligado a pausar la ejecución de los núcleos de sombreado, vaciar parcialmente las teselas incompletas hacia DRAM (*premature tile flush*), invalidar las cachés de textura y esperar a que el bus UMA complete la copia.
- **Subida por Scanlines frente a Subida por Frame Completo:** Realizar 160 transferencias de textura individuales por cuadro (una tras cada línea de escaneo de la GBA) genera 160 cambios de estado y comandos de sincronización por fotograma. En un procesador con GPU Mali-G31, esta sobrecarga de llamadas al driver gráfico (*driver overhead*) puede consumir entre 4.0 ms y 7.0 ms adicionales por cuadro únicamente en la capa del despachador de comandos de OpenGL ES o Vulkan. La consolidación de la imagen en un único búfer de subida sincronizado con la fase de VBlank erradica este desperdicio.

### Impacto del Formato de Píxel en el Tráfico del Bus
La selección del formato de textura interno determina directamente el volumen de bytes transferidos a través del bus UMA:

| Formato de Textura | Bytes por Píxel | Ancho de Banda a 240×160 (60 FPS) | Ancho de Banda a 1080p (60 FPS) | Características en TBDR |
| :--- | :---: | :---: | :---: | :--- |
| **RGBA8888** | 4 | $9.216 \text{ MB/s}$ | $497.66 \text{ MB/s}$ | Carga 4 componentes completas por texel; satura innecesariamente las líneas de caché de textura si la fuente es indexada. |
| **RGB565** | 2 | $4.608 \text{ MB/s}$ | $248.83 \text{ MB/s}$ | Ahorro del 50% de ancho de banda; pérdida leve de precisión cromática si no se usa dithering. |
| **R8 / Paletizado** | 1 | $2.304 \text{ MB/s}$ | $124.41 \text{ MB/s}$ | **Óptimo para GBA:** Reduce el tráfico en un 75% frente a RGBA8888. El índice de paleta de 8 bits se sube en crudo y el fragment shader resuelve el color final en SRAM local mediante un lookup a una mini-textura o búfer UBO de paleta. |

Al adoptar texturas de luminancia o canal único (R8) con resolución de paleta diferida en el fragment shader, el desempaquetado de color ocurre **dentro de la memoria SRAM on-chip de la tesela**, eliminando la necesidad de transportar bytes de color RGBA a través del saturado bus DRAM externo.

```
Subida Tradicional RGBA (Ineficiente en UMA):
[CPU: Scanlines] --(4 bytes/px en bus UMA)--> [DRAM] ---> [GPU Texture Cache]

Subida Indexada R8 + Fragment Lookup (Óptima en TBDR):
[CPU: Índices]   --(1 byte/px en bus UMA)--> [DRAM] ---> [GPU Texture Cache]
                                                                |
                                                 Lookup en SRAM on-chip (16x16)
                                                 usando paleta en búfer local
                                                 (Cero consumo de bus UMA extra)
```

---

## 5. Composición de Pantalla en Android: SurfaceView frente a TextureView

El subsistema gráfico de Android gestiona la presentación de ventanas mediante dos componentes clave: el servidor de composición **SurfaceFlinger** y el módulo de hardware **Hardware Composer (HWC)**. La elección del componente de interfaz de usuario donde se dibuja el juego influye determinantemente en la carga de la GPU TBDR.

### TextureView y la Sobrecarga de Renderizado en Cascada
- `TextureView` opera como un elemento integrado en el árbol de vistas convencional de Android (`android.view.View`).
- La superficie de renderizado se expone como una textura OpenGL ES gestionada por el renderizador de UI de la aplicación.
- **Impacto en TBDR:**
  1. La aplicación dibuja el contenido del juego en el búfer de `TextureView`.
  2. Al procesar el cuadro de la interfaz, el motor de UI de Android ejecuta un **segundo pase de renderizado completo**, leyendo la textura del juego como una primitiva más dentro de la jerarquía de vistas y mezclándola con los botones o decoraciones del sistema.
  3. Finalmente, `SurfaceFlinger` vuelve a intervenir para componer el resultado.
  4. Este flujo fuerza a una GPU móvil de bajo costo a procesar un ciclo redundante de escritura a DRAM y posterior lectura en teselas, duplicando el consumo de ancho de banda y añadiendo entre 1 y 2 cuadros de latencia (*input lag*).

### SurfaceView y los Planos de Hardware Directos (*Hardware Overlays*)
- `SurfaceView` provee una superficie de dibujo independiente (`Surface`) perforada directamente a través de la jerarquía de ventanas de la aplicación.
- El ciclo de dibujo de la aplicación renderiza directamente sobre los búferes nativos de esta superficie sin pasar por el árbol de vistas de Android.
- **Integración con Hardware Composer (HWC):**
  - El componente HWC de los chips móviles identifica que la superficie de `SurfaceView` es un plano de video o gráficos independiente y delega su escalado y presentación directamente al procesador de pantalla (*Display Engine* del SoC).
  - La composición ocurre en el momento del escaneo físico hacia el panel HDMI o la pantalla, **sin consumir ciclos de cómputo en la GPU TBDR ni lecturas adicionales en memoria DRAM**.
  - Este mecanismo reduce a cero la sobrecarga de composición de capas del sistema operativo.

### Optimización del Formato de Ventana: `RGBX_8888` frente a `RGBA_8888`
En pantallas completas de televisores o modos de juego sin ventanas translúcidas, configurar la ventana con el formato `WINDOW_FORMAT_RGBX_8888` (u omitir el canal alfa mediante `pixel_format: Opaque`) informa al compositor HWC de que no existe transparencia entre la superficie del juego y el plano posterior. Esto permite a la GPU desactivar las operaciones de lectura del búfer de fondo y optimizar el empaquetado de memoria en el controlador del display.

---

## 6. Sincronización Vertical, Choreographer y Doble/Triple Búfer

El modelo de ejecución de Android coordina la presentación gráfica mediante señales de sincronización vertical (*VSync*), emitidas a intervalos regulares (16.66 ms a 60 Hz).

### El Latido del Choreographer
En el entorno de Android, el hilo de la interfaz de usuario se sincroniza a través de la clase `Choreographer`, la cual emite pulsos de *VSync* para tres propósitos secuenciales:
1. `VSYNC-app`: Indica a la aplicación el momento exacto para comenzar a computar la lógica del cuadro y emitir comandos de dibujo.
2. `VSYNC-sf`: Indica al `SurfaceFlinger` que recolecte los búferes completados para componer la escena.
3. `VSYNC-display`: Intervalo en que el controlador de pantalla intercambia el puntero al nuevo fotograma en el escaneo vertical.

### Retardo Estructural Inherente en TBDR
Debido a la naturaleza en dos fases (Geometría diferida &rarr; Sombreado de fragmentos), la mayoría de los controladores gráficos TBDR implementan un diseño de pipeline superpuesto:
- En el cuadro $N$, la GPU ejecuta la Fase 1 (Geometría y *Tiling*) del cuadro actual mientras procesa en paralelo la Fase 2 (Rasterizado en SRAM por teselas) del cuadro anterior $N-1$.
- Como consecuencia técnica, intentar implementar un bucle sincrónico cerrado donde la CPU espera inmediatamente a que la GPU termine de dibujar el cuadro actual (bloqueo mediante `glFinish()` o esperas activas de *fences*) provoca un parón total de la canalización (*pipeline stall*). La GPU vacía sus colas de teselas y pasa a estado inactivo, destruyendo el solapamiento y haciendo imposible mantener los 60 FPS en procesadores modestos.

```
Flujo Ideal de Pipeline Solapado (TBDR a 60 FPS):

Fotograma N:
CPU / Lógica:    |-- Frame N (Recomp) --|
GPU Fase 1:                             |-- Binning Frame N --|
GPU Fase 2:                                                   |-- Raster Frame N (SRAM) --|

Fotograma N+1:
CPU / Lógica:                           |-- Frame N+1 (Recomp) --|
GPU Fase 1:                                                      |-- Binning Frame N+1 --|
GPU Fase 2:                                                                               |-- Raster Frame N+1 --|
```

### Prohibición del *Busy-Waiting* y Sincronización en Espacio de Usuario
En arquitecturas UMA, los bucles de espera activa en la CPU (tales como `while (!frame_ready) {}` o `spinlocks` que leen continuamente variables atómicas en memoria compartida) son sumamente perjudiciales:
1. La CPU satura las líneas de caché L2/L3 y el controlador de memoria ejecutando transacciones inútiles de lectura de memoria a máxima velocidad de reloj.
2. Esta saturación degrada el ancho de banda disponible para las etapas de escritura de teselas de la GPU y para las transferencias del decodificador de video.
3. Se produce una elevación brusca de la temperatura del chip, lo que desencadena los mecanismos de estrangulamiento térmico (*thermal throttling*) del sistema operativo, reduciendo la frecuencia del procesador de 1.8 GHz a menos de 1.0 GHz y degradando irreversiblemente la tasa de fotogramas.

La sincronización entre el runtime nativo del juego y la presentación de pantalla debe delegarse siempre a primitivas de bloqueo del sistema operativo (`pthread_cond_wait`, barreras atómicas sincronizadas o la cola de eventos nativa `ANativeWindow_lock / unlockAndPost`), permitiendo que los núcleos de la CPU entren en estados de bajo consumo (*wait for event*, `WFE` / `WFI`) mientras la GPU y el motor de pantalla procesan sus fases en paralelo.
