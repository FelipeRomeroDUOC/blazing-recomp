# Glosario de Blazing Recomp

Vocabulario técnico y conceptos arquitectónicos propios de este port nativo.
Cada término incluye su definición concreta, el contexto en el que opera y el enlace
al archivo de código fuente, esquema o documento de referencia que lo fundamenta.

---

## Índice por Categorías

- [1. Arquitectura y Recompilación AOT](#1-arquitectura-y-recompilacion-aot)
- [2. Subsistema Gráfico (PPU) y Hardware Shim](#2-subsistema-grafico-ppu-y-hardware-shim)
- [3. Capa Nativa Android y Runtime](#3-capa-nativa-android-y-runtime)
- [4. Telemetría y Diagnóstico](#4-telemetria-y-diagnostico)
- [5. Metodología de Pruebas y Validación](#5-metodologia-de-pruebas-y-validacion)

---

## 1. Arquitectura y Recompilación AOT

### Recompilación estática AOT (*Ahead-Of-Time*)

Proceso de ingeniería inversa y traducción previa que transforma el código máquina ARMv4T y Thumb de la ROM original de Game Boy Advance en código fuente C11 compilable de forma nativa para la CPU de destino (ARM64-v8a en Android). A diferencia de los emuladores tradicionales, no existe un bucle dinámico de interpretación ni compilación JIT durante la partida: las funciones del juego se ejecutan como código nativo de la máquina anfitriona.  
Fuente: [`src-c/`](src-c/), [`blazing.labels.toml`](blazing.labels.toml).

### Intérprete de respaldo (*Fallback Interpreter*)

Emulador de CPU ARMv4T provisto por `gba-core` que se ejecuta únicamente cuando el flujo de control salta a una dirección de memoria que aún no ha sido recompilada en C11 o que no posee una etiqueta asignada en `blazing.labels.toml`. El objetivo arquitectónico del proyecto es la cobertura total (*0 Interpreter*), manteniendo el intérprete solo como red de seguridad.  
Fuente: [`src/telemetria.rs`](src/telemetria.rs), [`crates/armv4t`](https://github.com/JRickey/gba-recomp).

### Hotspot

Punto caliente o dirección de memoria de la ROM en la que el intérprete de respaldo consume un porcentaje apreciable del tiempo total de CPU. Los hotspots se detectan y priorizan mediante el análisis del registro `blazing-perf.txt` para convertirlos en funciones C11 nativas.  
Fuente: [`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py).

### Mapa de etiquetas (*Labels Map*)

Archivo de configuración estructurado en formato TOML (`blazing.labels.toml`, bajo el estándar `gba-labels v2`) que define las fronteras y puntos de entrada de las 18.483 funciones catalogadas del juego: dirección de memoria (`address`), modo de instrucción (`mode = "thumb"` o `"arm"`) y nombre del símbolo exportado (`name`).  
Fuente: [`blazing.labels.toml`](blazing.labels.toml), [`docs/referencia/FORMATO_LABELS.md`](docs/referencia/FORMATO_LABELS.md).

### Unidad de traducción C11

Cada uno de los 12 archivos fuente físicos (`bios.0.c` a `bios.11.c`) generados por el recompilador Ahead-Of-Time en `src-c/`, con un volumen total de 7.256.939 líneas de código (~215,8 MB). El código emitido se particiona automáticamente en cortes con un umbral de ~16 MB en fronteras de bloque para evitar desbordamientos de tabla de símbolos o tiempos excesivos de análisis semántico en los compiladores nativos de destino (Clang y GCC).  
Fuente: [`src-c/`](src-c/), [`docs/explicacion/arquitectura-c-recompilado.md`](docs/explicacion/arquitectura-c-recompilado.md).

### Función canónica

Cada una de las 18.483 subrutinas completas identificadas y catalogadas en `blazing.labels.toml`. Representa una función lógica del juego delimitada arquitectónicamente por su punto de entrada y prólogo canónico (`PUSH {..., lr}`, `0xb5xx` en Thumb) y sus instrucciones de retorno (`POP {..., pc}` o `BX LR`). Cada función canónica agrupa internamente uno o múltiples bloques básicos C11 contiguos.  
Fuente: [`blazing.labels.toml`](blazing.labels.toml), [`docs/referencia/FORMATO_LABELS.md`](docs/referencia/FORMATO_LABELS.md), [`docs/explicacion/arquitectura-c-recompilado.md`](docs/explicacion/arquitectura-c-recompilado.md).

### Bloque básico C11 (`rcg_blocks[]`)

Fragmento elemental de ejecución lineal dentro del arreglo lineal maestro de 144.875 bloques emitidos por el recompilador AOT (`rcg_block_count = 144875` en `src-c/...bios.11.c`). Cada bloque representa una secuencia atómica de instrucciones traducidas a C11 que culmina en una bifurcación, salto condicional, llamada a función o retorno, subordinados jerárquicamente a las funciones canónicas del juego.  
Fuente: [`src-c/`](src-c/), [`docs/explicacion/arquitectura-c-recompilado.md`](docs/explicacion/arquitectura-c-recompilado.md).

### ATPCS e Interworking ARM/Thumb

*ARM-Thumb Procedure Call Standard*: especificación de la ABI que regula el paso de argumentos en registros (`r0` a `r3`), el valor de retorno y la preservación de registros (`r4` a `r11`). El *interworking* es la transición dinámica entre el juego de instrucciones de 32 bits (ARM) y el de 16 bits (Thumb) mediante instrucciones `bx` y `blx`, alternando el modo según el bit menos significativo de la dirección destino ($LSB = 1$ para Thumb, $LSB = 0$ para ARM).  
Fuente: [`docs/fuentes/Manual Técnico de Arquitectura Interna, Procesamiento Gráfico, Motor de Sonido y Emulación de la Game Boy Advance.md`](docs/fuentes/Manual%20Técnico%20de%20Arquitectura%20Interna,%20Procesamiento%20Gráfico,%20Motor%20de%20Sonido%20y%20Emulación%20de%20la%20Game%20Boy%20Advance.md).

### Tabla de saltos (*Jump Table*)

Patrón generado por compiladores de C para bifurcaciones múltiples (`switch-case`), donde se indexa una tabla de punteros y se carga el destino en el contador de programa (`mov pc, rx` o `ldr pc, [base, offset]`). Requiere un tratamiento analítico especial durante la recompilación AOT para que el desensamblador reconozca todos los destinos de salto válidos.  
Fuente: [`scripts/diagnostico/desensamblar_rom.py`](scripts/diagnostico/desensamblar_rom.py).

---

## 2. Subsistema Gráfico (PPU) y Hardware Shim

### Hardware Shim (Histórico)

Capa intermedia de intercepción de bus y memoria implementada originalmente en la rama `feature/hardware-shim` y **retirada de producción conforme al ADR 42** en favor de la `NativePpu` en GPU. En su diseño preliminar, interceptaba las escrituras que el juego realizaba sobre los registros I/O del LCD (`0x04000000`), VRAM (`0x06000000`), OAM (`0x07000000`) y paleta (`0x05000000`). Su código y arneses de experimentación se conservan archivados en [`docs/historico/ppu-shim/`](docs/historico/ppu-shim/README.md).  
Fuente: [`docs/explicacion/DECISIONES.md`](docs/explicacion/DECISIONES.md#42-eliminación-definitiva-de-ppushim-software-en-favor-de-pipeline-gpu-directo-nativeppu), [`docs/historico/ppu-shim/README.md`](docs/historico/ppu-shim/README.md).

### PPU Nativa (`NativePpu`)

Subsistema de renderizado gráfico acelerado por hardware escrito en Rust mediante la API `wgpu` y shaders WGSL. Procesa la geometría de las cuatro capas de fondo (BG0 a BG3) y los sprites (OBJ) directamente en la GPU del smartphone o PC, optimizado para arquitecturas gráficas móviles TBDR.  
Fuente: [`src/ppu_nativa.rs`](src/ppu_nativa.rs).

### PPU de Referencia

Implementación de rasterizado por software provista por el motor upstream `gba-core::ScreenRenderer`. Dibuja los píxeles scanline a scanline en la CPU y actúa como patrón de verdad absoluta para verificar la exactitud visual y matemática de la PPU nativa.  
Fuente: [`examples/comun/mod.rs`](examples/comun/mod.rs).

### Tramo de calibración (*Calibration Span*)

Intervalo de tiempo (típicamente 3 segundos) durante el cual el subsistema de presentación evalúa el comportamiento de un candidato (`PresentMode`, latencia, escala) en la pantalla del dispositivo. La telemetría registra cada tramo con `# tramo` y la selección final con `# elegido`.  
Fuente: [`src/present.rs`](src/present.rs).

### Clave de tramo (*Span Key*)

Identificador compacto de 64 bits que codifica el estado completo de los registros gráficos durante un intervalo de líneas de escaneo uniformes. Permite comparar instantáneamente configuraciones entre líneas y reutilizar pipelines gráficos en GPU sin comprobaciones redundantes.  
Fuente: [`docs/propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](docs/propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md).

### Camino de composición

Uno de los recorridos con los que la PPU compone una línea de pantalla, repartidos por la línea `# ppucamino` de la telemetría: `rectas` (vía rápida, con los registros constantes en el tramo), `mezcla` (efecto de color que necesita las dos capas), `ventana` (ventana de sprites activa) y `semi` (sprite translúcido). Los cuatro suman las líneas dibujadas y cada línea cae en uno solo: el primero que impide usar la vía rápida.  
Fuente: [`src/ppu_nativa.rs`](src/ppu_nativa.rs), [`docs/explicacion/ppu-caminos-de-composicion.md`](docs/explicacion/ppu-caminos-de-composicion.md).

### LUT Vertical de Registros LCD

Búfer uniforme o textura 1D en memoria de vídeo que almacena los valores de los registros I/O del LCD para cada una de las 160 scanlines del fotograma. El shader de fragmentos consulta esta tabla para aplicar desplazamientos finos de scroll (`BGxHOFS`/`BGxVOFS`) y efectos HBlank sin fragmentar la pasada de renderizado en múltiples llamadas de dibujo.  
Fuente: [`src/ppu_nativa.rs`](src/ppu_nativa.rs), [`docs/propuestas/2026-10-23_lut-vertical-en-produccion-y-salto-ppu-cpu.md`](docs/propuestas/2026-10-23_lut-vertical-en-produccion-y-salto-ppu-cpu.md).

### Paleta LUT Vertical

Estructura de datos en memoria GPU que mapea los colores indexados de 15 bits (formato GBA BGR555) a RGBA8 para cada scanline. Soporta modificaciones de paleta en mitad de cuadro generadas por interrupciones de temporizador o DMA sin requerir transferencias masivas de texturas.  
Fuente: [`docs/propuestas/2026-10-23_ppu-nativa-paleta-lut-vertical.md`](docs/propuestas/2026-10-23_ppu-nativa-paleta-lut-vertical.md).

### Subida diferencial (*Dirty Tracking*)

Mecanismo de optimización que monitoriza los rangos de memoria modificados en VRAM, OAM y Paleta mediante máscaras de suciedad o límites mínimos y máximos. Durante el fotograma, solo los bloques de memoria que hayan sido alterados por la CPU se transfieren a los búferes de la GPU, minimizando la contención del bus PCIe e interconectores móviles.  
Fuente: [`src/ppu_nativa.rs`](src/ppu_nativa.rs), [`docs/propuestas/2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md`](docs/propuestas/2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md).

### Ventana OBJ de camino rápido (*OBJ Window Fast Path*)

Optimización condicional del shader de composición que detecta si el bit de ventana de sprites en `DISPCNT` está apagado en todo el fotograma o tramo. Cuando está inactivo, el shader omite el cálculo del stencil de sprites y la lectura del mapa de recorte, reduciendo el consumo de registros escalares y aumentando la tasa de relleno (*fillrate*).  
Fuente: [`docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md`](docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md).

### Fondo afín (Modo 1, BG2)

Modo de vídeo mixto de GBA donde las capas BG0 y BG1 operan en modo texto tradicional, mientras que BG2 opera mediante una matriz de transformación afín 2D ($A, B, C, D$ y centro de referencia $X_0, Y_0$) permitiendo escalado, rotación y cizallamiento. Es intensamente utilizado en *Fire Emblem* para cinemáticas de batalla y transiciones.  
Fuente: [`src/ppu_nativa.rs`](src/ppu_nativa.rs), [`docs/propuestas/2026-10-21_fondo-afin-bg2-modo-1.md`](docs/propuestas/2026-10-21_fondo-afin-bg2-modo-1.md).

### Desplazamiento por línea (*Per-scanline Scroll / HDMA*)

Técnica de programación gráfica donde el controlador DMA de GBA transfiere nuevos valores de desplazamiento horizontal (`BGxHOFS`) o vertical (`BGxVOFS`) hacia los registros I/O en cada periodo de HBlank, generando efectos visuales de ondulación de agua, calor o falsa perspectiva tridimensional.  
Fuente: [`docs/fuentes/Manual Técnico de Arquitectura Interna, Procesamiento Gráfico, Motor de Sonido y Emulación de la Game Boy Advance.md`](docs/fuentes/Manual%20Técnico%20de%20Arquitectura%20Interna,%20Procesamiento%20Gráfico,%20Motor%20de%20Sonido%20y%20Emulación%20de%20la%20Game%20Boy%20Advance.md).

---

## 3. Capa Nativa Android y Runtime

### Capa Nativa (`android-activity`)

Módulo en Rust que interactúa directamente con el NDK de Android a través de `android-activity` bajo el modelo `NativeActivity`. Inicializa el bucle de eventos del sistema operativo, gestiona el ciclo de vida de la aplicación (`onPause`, `onResume`, `onDestroy`) y asocia la superficie de la ventana (`ANativeWindow`) con el contexto gráfico de Vulkan o GLES.  
Fuente: [`src/lib.rs`](src/lib.rs), [`src/platform.rs`](src/platform.rs), [`docs/explicacion/arquitectura-port-android.md`](docs/explicacion/arquitectura-port-android.md).

### Buzón de fotogramas (*Frame Mailbox*)

Mecanismo de comunicación asíncrono y libre de bloqueos (*lock-free*) implementado en `src/pipeline.rs`. Desacopla el hilo que ejecuta la lógica del juego (fijado estrictamente a 60 ticks de GBA por segundo) del hilo de renderizado de la GPU. Aplica una política de reemplazo atómico donde el hilo productor deposita el último fotograma completo sin encolar retrasos ni bloquearse si el hilo gráfico está ocupado.  
Fuente: [`src/pipeline.rs`](src/pipeline.rs), [`docs/propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md`](docs/propuestas/2026-10-10_fix-pipeline-multihilo-mailbox-y-perf-txt.md).

### Pacing de fotogramas y Choreographer

Estrategia de sincronización temporal coordinada con la API `AChoreographer` de Android o el VSync de la pantalla. Alinea el intercambio de búferes gráficos (*swapchain presentation*) exactamente con la frecuencia de refresco del panel (60 Hz o múltiplos adaptativos), eliminando el tartamudeo (*micro-stuttering*) y el desgarro de pantalla (*tearing*).  
Fuente: [`src/present.rs`](src/present.rs), [`docs/fuentes/Arquitectura de GPUs Moviles TBDR y Gestion de Ancho de Banda en Android.md`](docs/fuentes/Arquitectura%20de%20GPUs%20Moviles%20TBDR%20y%20Gestion%20de%20Ancho%20de%20Banda%20en%20Android.md).

### Superposición táctil (*Touch Overlay*)

Capa de interfaz virtual superpuesta en la pantalla táctil que emula el control clásico de Game Boy Advance: cruceta direccional (D-Pad), botones de acción (A, B) y gatillos superiores (L, R, Start, Select), permitiendo jugar ergonómicamente en smartphones sin gamepad físico.  
Fuente: [`src/overlay.rs`](src/overlay.rs).

### Cuencas táctiles y zonas muertas (*Touch Basins & Deadzones*)

Modelo geométrico de detección táctil:
- **Cuenca (*Basin*):** Área convexa o elíptica extendida alrededor de un botón virtual que captura pulsaciones cercanas con tolerancia ergonómica.
- **Zona muerta (*Deadzone*):** Umbral circular central en la cruceta donde no se emite ninguna dirección activa, impidiendo entradas diagonales falsas provocadas por el deslizamiento natural del pulgar.  
Fuente: [`src/overlay.rs`](src/overlay.rs), [`docs/propuestas/2026-10-09_separacion-cuencas-y-orientacion-overlay.md`](docs/propuestas/2026-10-09_separacion-cuencas-y-orientacion-overlay.md).

### Storage Access Framework (SAF) y Scoped Storage

Modelo de seguridad y almacenamiento impuesto en versiones modernas de Android (API 30+). En la arquitectura objetivo del proyecto (`v0.3.0`), SAF gestiona el acceso mediante el selector del sistema (`ACTION_OPEN_DOCUMENT`) para otorgar permisos persistentes a URIs (`content://`) de la ROM y BIOS. En la versión de desarrollo actual (`v0.0.2`), el runtime accede de forma directa a su almacenamiento privado y a `/sdcard/Download/` apoyándose en los permisos declarados en el manifiesto.  
Fuente: [`src/storage.rs`](src/storage.rs), [`docs/fuentes/Almacenamiento en Android Moderno y Storage Access Framework.md`](docs/fuentes/Almacenamiento%20en%20Android%20Moderno%20y%20Storage%20Access%20Framework.md).

### Fat APK

Paquete de distribución de Android (`.apk`) que integra las bibliotecas dinámicas nativas (`.so`) compiladas para los tres ABIs soportados (`arm64-v8a`, `armeabi-v7a`, `x86_64`), empaquetado y firmado mediante herramientas directas del SDK (`aapt2`, `zipalign` con alineación de 16 KB para compatibilidad con Android 15/16 y `apksigner` esquemas v1/v2/v3) sin depender de Gradle.  
Fuente: [`scripts/package_apk.ps1`](scripts/package_apk.ps1), [`docs/guias/compilar-y-empaquetar-apk.md`](docs/guias/compilar-y-empaquetar-apk.md).

---

## 4. Telemetría y Diagnóstico

### Telemetría

Las mediciones que el motor escribe en `blazing-perf.txt` mientras se ejecuta el juego, para diagnosticar el rendimiento sin depurador y sin parar la partida. Es un artefacto de texto plano, pensado para abrirse a mano en el dispositivo y para leerse con `scripts/diagnostico/analizar_telemetria.py`.  
Fuente: [`src/telemetria.rs`](src/telemetria.rs), [`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py).

### Línea de segundo

Una línea `t=<n>s ...` de `blazing-perf.txt`: el resumen de la ventana de **un segundo** de ejecución (FPS, tiempo de CPU, PPU, APU, dibujo y swapchain). No es una línea por fotograma: cada fotograma se acumula y la ventana se cierra al cumplirse el segundo. Su contrato lo declara [`telemetria/esquema.txt`](telemetria/esquema.txt).  
Fuente: [`telemetria/esquema.txt`](telemetria/esquema.txt).

### Esquema de telemetría

El fichero [`telemetria/esquema.txt`](telemetria/esquema.txt): la lista de campos de la línea de segundo, con su tipo, su unidad y la versión del contrato que los introdujo. Es el **dueño único** de esa lista. Lo leen los dos lados del contrato —el escritor ([`src/telemetria.rs`](src/telemetria.rs), que lo embebe con `include_str!`) y el lector ([`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py))— así que añadir un campo es tocar un fichero, no dos sitios que tienen que coincidir.  
Fuente: [`telemetria/esquema.txt`](telemetria/esquema.txt).

### Sello de versión

El campo `formato=<n>` con el que termina cada línea de segundo: dice con qué versión del contrato se escribió. Sin él, «una línea sin los campos nuevos» y «una línea a la que le faltan campos» son indistinguibles. Una línea sin sello se lee en modo tolerante (los campos que falten no se echan de menos); una línea sellada se comprueba contra la lista de campos de **su** versión.  
Fuente: [`telemetria/esquema.txt`](telemetria/esquema.txt).

### Anomalía de formato

Lo que el analizador reporta cuando una línea no encaja con el esquema: un campo que no está declarado, un valor que no encaja con su tipo, un campo que falta en una línea sellada o una versión que el esquema no conoce. No se descarta la línea: se anota el motivo en la sección `ANOMALÍAS DE FORMATO` del informe y la línea sigue contando para las medias. Antes de esto, un desajuste hacía que el regex no casara y el segundo **desaparecía del informe sin ningún aviso**.  
Fuente: [`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py).

### Desglose de fases de fotograma

Métricas desglosadas en la telemetría que cuantifican la duración de cada subfase del ciclo de un frame:
- `ejec`: Tiempo de ejecución de lógica de CPU y subrutinas del juego.
- `ppu`: Tiempo de rasterización por software en CPU provisto por `gba-core` (`ppubg + ppuspr + ppucomp`). Permanece en 0 ms cuando la `NativePpu` en GPU está activa.
- `apu`: Síntesis y mezcla de canales de audio PSG y Direct Sound.
- `dibujo`: Tiempo total de ejecución del compositor (`Compositor::render`, desglosado internamente en `subida + codificacion + present`).
- `present`: Bloqueo esperando la sincronización con el compositor de pantalla (*swapchain*).  
Fuente: [`src/perf.rs`](src/perf.rs), [`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py).

---

## 5. Metodología de Pruebas y Validación

### Visor Doble Interactivo (`ppu_doble`)

Herramienta de desarrollo y validación visual ejecutada en PC ([`src/bin/ppu_doble.rs`](src/bin/ppu_doble.rs)). Renderiza dos vistas simultáneas del juego: a la izquierda, la salida de la PPU de referencia en CPU; a la derecha, la salida generada por la PPU nativa acelerada por GPU (`wgpu`). Permite congelar fotogramas, pausar, conmutar vistas e inspeccionar diferencias gráficas en tiempo real.  
Fuente: [`src/bin/ppu_doble.rs`](src/bin/ppu_doble.rs), [`docs/guias/ejecutar-visor-doble-pc.md`](docs/guias/ejecutar-visor-doble-pc.md).

### Arnés de experimentos (`comun`)

Módulo común de pruebas ([`examples/comun/mod.rs`](examples/comun/mod.rs)) que inicializa la ROM oficial de *Fire Emblem*, la BIOS y el bus de memoria en un estado estrictamente determinista. Permite que múltiples experimentos y benchmarks visuales compartan la misma infraestructura sin duplicar código.  
Fuente: [`examples/comun/mod.rs`](examples/comun/mod.rs), [`examples/README.md`](examples/README.md).

### Replay determinista

Secuencia registrada de entradas de usuario y ticks de reloj capturada de forma idéntica entre sesiones. Permite ejecutar exactamente la misma partida sobre diferentes versiones del motor o ramas de Git para realizar análisis de regresión visual y mediciones de rendimiento reproducibles.  
Fuente: [`examples/ppu_nativa_compara.rs`](examples/ppu_nativa_compara.rs), [`examples/ppu_barrido_nativa.rs`](examples/ppu_barrido_nativa.rs).

### Gatekeeper de Pre-Commit

Conjunto de cinco comprobaciones automáticas e inviolables definidas en [`AGENTS.md`](AGENTS.md) que deben cumplirse antes de autorizar cualquier `git commit`:
1. Suite de pruebas al 100% en verde (`cargo test`).
2. Validación de documentación Diátaxis sin errores ni avisos (`validar_docs.py`).
3. Sincronización estricta de referencias de API de Rust (`extraer_referencias.py --check`).
4. Auditoría de etiquetas de la ROM conforme (`auditar_labels.py`).
5. Cero rutas absolutas locales del usuario o sistema en los archivos del repositorio.  
Fuente: [`AGENTS.md`](AGENTS.md).
