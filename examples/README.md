# Directorio de Ejemplos y Bancos de Prueba de la PPU

Este directorio contiene los arneses ejecutables de micro-benchmarking, experimentación gráfica y validación de regresión visual de **`blazing-recomp`**.

---

## 1. Arneses Activos de Producción (`NativePpu`)

Los siguientes ejemplos constituyen los bancos de prueba operativos para validar y diagnosticar el pipeline de renderizado nativo por GPU (`wgpu`):

| Ejemplo | Comando de Ejecución | Propósito y Validación |
| :--- | :--- | :--- |
| **`ppu_barrido_nativa`** | `cargo run --example ppu_barrido_nativa` | Ejecuta el barrido de scanlines y valida la sincronización de búferes en la ventana de VBlank sin requerir features adicionales. |
| **`ppu_banda_nativa`** | `cargo run --example ppu_banda_nativa --features proto` | Prueba la división en bandas horizontales y el procesamiento de franjas de scanlines forzando el camino general. |
| **`ppu_nativa_afin`** | `cargo run --example ppu_nativa_afin --features proto` | Banco de pruebas para verificar transformaciones afines de fondos (Modos 1 y 2) y rotación/escalado afín en sprites. |
| **`ppu_nativa_compara`** | `cargo run --example ppu_nativa_compara --features proto` | Compara cuadro a cuadro la salida visual generada por `NativePpu` frente a los *golden frames* de referencia. |
| **`ppu_nativa_atribuye`** | `cargo run --example ppu_nativa_atribuye --features proto` | Atribuye y desglosa el tiempo de GPU consumido por cada pase (subida de texturas, shaders de capas, compositor final). |
| **`ppu_afin_y_linea`** | `cargo run --example ppu_afin_y_linea --features proto` | Evalúa la precisión del cálculo de coordenadas afines por línea de escaneo. |
| **`ppu_tiempos`** | `cargo run --example ppu_tiempos --features proto` | Mide la latencia de CPU y GPU por frame en microsegundos durante secuencias de juego reales. |

---

## 2. Módulo de Soporte Compartido (`examples/comun/`)

Los arneses comparten una infraestructura común modular ubicada en [`examples/comun/`](comun/):
- **`gpu.rs`:** Inicialización simplificada de adaptadores, dispositivos lógicos y colas de `wgpu` para ejecución en modo ventana de escritorio o headless.
- **`entrada.rs`:** Ingestión de ROMs y carga de capturas de estado de VRAM/OAM para reproducir escenas deterministas.
- **`diagnostico.rs`:** Comparadores de memoria de pantalla, cálculo de diferencias PSNR/SSIM y exportación de imágenes PNG de diagnóstico.
- **`registro.rs` / `color.rs`:** Utilidades para decodificación de paletas BGR555 a RGBA y lectura de campos de registros MMIO (`DISPCNT`, `BLDCNT`).

---

## 3. Arneses Históricos de Investigación (`shim_*` y primeros prototipos de PPU)

El directorio conserva **40 arneses experimentales** creados durante la fase de desarrollo e investigación de la PPU (19 al 23 de octubre de 2026):

- **Familia `shim_*` (26 archivos):** Experimentos de bajo nivel diseñados para aislar el comportamiento del renderizador de scanlines en CPU (`PpuShim`), incluyendo subrutinas para depurar capas individuales (`shim_capa_x.rs`, `shim_bg0_710.rs`), mezclas alfa (`shim_mezcla_registros.rs`), prioridad de sprites (`shim_sprite_debajo.rs`) y fondos afines.
- **Familia `ppu_*` tempranos (14 archivos):** Primeros prototipos de emulación híbrida (`ppu_proto.rs`, `ppu_compare.rs`, `ppu_linea.rs`, `ppu_camino_general.rs`, etc.).

### Por qué se conservan en `Cargo.toml`:
Con la adopción de la decisión arquitectónica **[ADR 42](docs/explicacion/DECISIONES.md#42-eliminación-definitiva-de-ppushim-software-en-favor-de-pipeline-gpu-directo-nativeppu)**, el proyecto eliminó definitivamente la ruta de renderizado de scanlines por CPU en favor de la GPU nativa directa (`NativePpu`). 

No obstante, estos 40 arneses históricos se conservan registrados en `Cargo.toml` y protegidos por la suite de pruebas ([`tests/arnes_experimentos_tests.rs`](../tests/arnes_experimentos_tests.rs)) para garantizar que cualquier propuesta o experimento histórico pueda ser reproducido fielmente por su nombre exacto.
