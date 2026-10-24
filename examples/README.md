# Directorio de Ejemplos y Bancos de Prueba de la PPU

Este directorio contiene los arneses ejecutables de micro-benchmarking, experimentación gráfica y validación de regresión visual de **`blazing-recomp`**.

---

## 1. Arneses Activos de Producción (`NativePpu`)

Los siguientes 18 ejemplos constituyen los bancos de prueba operativos para validar, diagnosticar y perfilar el pipeline de renderizado nativo por GPU (`wgpu`) y la composición de la PPU:

| Ejemplo | Comando de Ejecución | Propósito y Validación |
| :--- | :--- | :--- |
| **`ppu_barrido_nativa`** | `cargo run --example ppu_barrido_nativa` | Recorre los fotogramas de una escena y contrasta la salida de la PPU nativa (GPU) frente al software sin requerir features adicionales. |
| **`ppu_banda_nativa`** | `cargo run --example ppu_banda_nativa --features proto` | Inspecciona diferencias de la PPU nativa en una banda de scanlines forzando el camino general. |
| **`ppu_nativa_afin`** | `cargo run --example ppu_nativa_afin --features proto` | Banco de pruebas sintético para verificar transformaciones afines de fondos (Modos 1 y 2) y sprites con rotación/escalado. |
| **`ppu_nativa_compara`** | `cargo run --example ppu_nativa_compara --features proto` | Compara cuadro a cuadro la salida visual generada por `NativePpu` frente a los cuadros dorados de referencia. |
| **`ppu_nativa_atribuye`** | `cargo run --example ppu_nativa_atribuye --features proto` | Atribuye y desglosa las diferencias de estado por línea entre pases y componentes de la PPU. |
| **`ppu_afin_y_linea`** | `cargo run --example ppu_afin_y_linea --features proto` | Barre un rango de cuadros capturando parámetros afines y desplazamientos por línea. |
| **`ppu_tiempos`** | `cargo run --example ppu_tiempos --features proto` | Mide la latencia de CPU y GPU por frame en microsegundos durante secuencias de juego reales. |
| **`ppu_modo_cuadro`** | `cargo run --example ppu_modo_cuadro` | Vuelca el estado de la PPU (modo de fondo, scrolls, ventanas y unidad de mezcla) en un cuadro concreto de la ROM. |
| **`ppu_capas`** | `cargo run --example ppu_capas --features proto` | Comprueba que una copia del estado de un cuadro reproducida fuera del juego reproduce el framebuffer original. |
| **`ppu_estado`** | `cargo run --example ppu_estado --features proto` | Mide cuántos píxeles difieren del juego al aislar campos faltantes de estado en un cuadro. |
| **`ppu_linea`** | `cargo run --example ppu_linea --features proto` | Captura y valida el estado de cada línea visible justo antes del dibujo del haz de escaneo. |
| **`ppu_capas_linea`** | `cargo run --example ppu_capas_linea --features proto` | Evalúa la aportación individual de cada capa de fondo por línea en el cuadro especificado. |
| **`ppu_efectos_ventanas`** | `cargo run --example ppu_efectos_ventanas --features proto` | Redibuja cada línea aislando efectos especiales de color y máscaras de ventanas (WIN0/WIN1/WINOBJ). |
| **`ppu_pixeles_obj`** | `cargo run --example ppu_pixeles_obj --features proto` | Mide la aportación de píxeles de sprites (OBJ) y su interacción con las capas de fondo. |
| **`ppu_contenido_bg`** | `cargo run --example ppu_contenido_bg --features proto` | Mide el contenido y baldosas activas en BG0, BG1 y BG2 por línea de escaneo. |
| **`ppu_subcamino`** | `cargo run --example ppu_subcamino --features proto` | Valida subcaminos de composición rápida cuando intervienen sprites translúcidos. |
| **`ppu_camino_general`** | `cargo run --example ppu_camino_general --features proto` | Compara la composición del camino rápido contra el camino general forzado en el cuadro. |
| **`ppu_capa_ganadora`** | `cargo run --example ppu_capa_ganadora --features proto` | Determina qué capa resulta visible en cada píxel tras resolver prioridades y ventanas. |

---

## 2. Módulo de Soporte Compartido (`examples/comun/`)

Los arneses comparten una infraestructura común modular ubicada en [`examples/comun/`](comun/):
- **`gpu.rs`:** Inicialización simplificada de adaptadores, dispositivos lógicos y colas de `wgpu` para ejecución en modo ventana de escritorio o headless.
- **`entrada.rs`:** Ingestión de ROMs y carga de capturas de estado de VRAM/OAM para reproducir escenas deterministas.
- **`diagnostico.rs`:** Comparadores de memoria de pantalla, cálculo de diferencias PSNR/SSIM y exportación de imágenes PNG de diagnóstico.
- **`registro.rs` / `color.rs`:** Utilidades para decodificación de paletas BGR555 a RGBA y lectura de campos de registros MMIO (`DISPCNT`, `BLDCNT`).

---

## 3. Arneses Históricos Archivados (`docs/historico/ppu-shim/`)

Conforme a la decisión arquitectónica **[ADR 42](../docs/explicacion/DECISIONES.md#42-eliminación-definitiva-de-ppushim-software-en-favor-de-pipeline-gpu-directo-nativeppu)** y a la propuesta [`docs/propuestas/2026-10-23_eliminacion-del-ppu-shim.md`](../docs/propuestas/2026-10-23_eliminacion-del-ppu-shim.md), los **29 arneses de experimentación** específicos del rasterizador antiguo (`shim_*.rs`, `ppu_proto.rs`, `ppu_compare.rs`, `ppu_frente_a_frente.rs`) fueron retirados de este directorio y desvinculados de `Cargo.toml`.

Se conservan como archivo documental de consulta en [`docs/historico/ppu-shim/`](../docs/historico/ppu-shim/README.md). En este directorio permanecen activos únicamente los 18 experimentos vigentes basados en la PPU nativa.
