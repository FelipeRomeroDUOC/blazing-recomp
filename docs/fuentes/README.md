# Fuentes de Verdad Técnica y Especificaciones Externas

Este directorio almacena las **fuentes de verdad técnicas, especificaciones de hardware y documentación de plataforma externa** que fundamentan el desarrollo de **`blazing-recomp`**.

---

## Distinción entre `docs/explicacion/` y `docs/fuentes/`

Para mantener la claridad arquitectónica del proyecto:

* **[`docs/explicacion/`](../explicacion/README.md):** Describe **cómo funciona este proyecto por dentro**: decisiones de diseño de software de *Blazing Recomp*, compromisos de ingeniería (*trade-offs*), lecciones aprendidas sobre nuestro propio código y el registro histórico de decisiones ([ADR 1 al 46](../explicacion/DECISIONES.md)).
* **`docs/fuentes/`:** Describe **cómo funciona el mundo real fuera de nuestro código**: física y silicio de la consola Game Boy Advance, especificaciones de conjuntos de instrucciones (ARM/Thumb), arquitectura de microprocesadores y GPUs móviles (TBDR), y contratos de bajo nivel del sistema operativo Android y el kernel de Linux.

> [!IMPORTANT]
> **Regla Operativa para Agentes de IA:**  
> Este directorio es **estrictamente de solo lectura** para los agentes automatizados, a menos que el desarrollador humano solicite de forma explícita redactar, complementar o añadir una nueva fuente técnica. No forma parte de los cuatro cuadrantes editables del estándar Diátaxis.

---

## Catálogo de Fuentes de Verdad Técnica

| Documento | Dominio Técnico | Contenido Fundamental |
| :--- | :--- | :--- |
| **[`Manual Técnico de Arquitectura Interna, Procesamiento Gráfico, Motor de Sonido y Emulación de la Game Boy Advance.md`](Manual%20Técnico%20de%20Arquitectura%20Interna,%20Procesamiento%20Gráfico,%20Motor%20de%20Sonido%20y%20Emulación%20de%20la%20Game%20Boy%20Advance.md)** | **Hardware GBA** | Arquitectura del ARM7TDMI (ARMv4T), temporización de buses y waitstates (IWRAM, EWRAM, VRAM, ROM), canales DMA 0–3, controlador LCD/PPU, motor de sonido (PSG + DirectSound FIFOs), interrupciones de hardware (`DISPSTAT`/`VCOUNT`), convención ATPCS y patrones de *jump tables* en Thumb. |
| **[`AArch32, Registros y Modelos de Excepción para Desarrollo Nativo y Videojuegos en Android.md`](AArch32,%20Registros%20y%20Modelos%20de%20Excepción%20para%20Desarrollo%20Nativo%20y%20Videojuegos%20en%20Android.md)** | **Arquitectura ARM** | Estados de ejecución ARMv8-A (AArch64 vs. AArch32), conmutación de registros físicos, modelo de niveles de excepción (EL0–EL3), arquitectura TrustZone, convención de llamadas AAPCS64 y optimizaciones SIMD NEON para desarrollo nativo. |
| **[`Arquitectura de GPUs Moviles TBDR y Gestion de Ancho de Banda en Android.md`](Arquitectura%20de%20GPUs%20Moviles%20TBDR%20y%20Gestion%20de%20Ancho%20a%20Banda%20en%20Android.md)** | **GPU y Gráficos Móviles** | Pipeline de dos fases de Tile-Based Deferred Rendering (Binning/Tiling + Raster en SRAM on-chip), cuellos de botella en memoria unificada (UMA) de TV Sticks (Mali-G31/PowerVR), optimización de texturas R8 paletizadas, *premature tile flushes*, y composición por planos directos de `SurfaceView` frente a `TextureView`. |
| **[`Modelo de Audio Nativo de Baja Latencia en Android (AAudio y OpenSL ES).md`](Modelo%20de%20Audio%20Nativo%20de%20Baja%20Latencia%20en%20Android%20(AAudio%20y%20OpenSL%20ES).md)** | **Audio de Bajo Nivel** | Pila de audio en Android, AudioFlinger frente al flujo directo MMAP HAL (`AAUDIO_SHARING_MODE_EXCLUSIVE`), tamaño de ráfagas nativas por hardware (`framesPerBurst`), dimensionamiento de doble búfer, remuestreo de 32.7 kHz a 48 kHz y restricciones de tiempo real en callbacks con colas circulares *lock-free* SPSC. |
| **[`Almacenamiento en Android Moderno y Storage Access Framework.md`](Almacenamiento%20en%20Android%20Moderno%20y%20Storage%20Access%20Framework.md)** | **Almacenamiento y SO** | Aislamiento de *Scoped Storage* (Android 10+ / API 29+), semántica de URIs de contenido virtual de SAF, persistencia indefinida con `takePersistableUriPermission`, extracción de descriptores enteros de archivo POSIX (`int fd`), mapeo en memoria virtual con `mmap()` y persistencia determinista de SRAM/Flash con `fsync()`. |
