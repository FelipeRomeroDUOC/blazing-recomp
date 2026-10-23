---
tipo: explicacion
verificado: si
---

# Documentación de Explicación y Decisiones Arquitectónicas

Este directorio agrupa los documentos conceptuales, análisis de rendimiento y justificaciones de diseño técnico de **`blazing-recomp`**, siguiendo el cuadrante de **Explicación** (*Explanation*) del estándar **Diátaxis**.

Su propósito es responder al *«por qué»* de las decisiones de ingeniería, contrastar alternativas evaluadas y documentar la arquitectura profunda del port nativo de *Fire Emblem: The Blazing Blade* en Android.

> [!NOTE]
> **Distinción de Ámbito: `docs/explicacion/` frente a `docs/fuentes/`:**
> - **`docs/explicacion/` (este directorio):** Explica **cómo funciona este proyecto por dentro**: diseño del software, estructura de módulos en Rust y C11, compromisos de ingeniería y el registro de decisiones arquitectónicas del port.
> - **[`docs/fuentes/`](../fuentes/README.md):** Contiene las especificaciones técnicas del **mundo exterior objetivo**: silicio y temporización de la Game Boy Advance, arquitectura ISA ARMv4T/AArch32, pipelines de GPUs móviles TBDR y subsistemas nativos de Android (AAudio, SAF). Consultar `fuentes/` cuando se requiera conocer las restricciones físicas o estándares oficiales del hardware.

---

## Índice de Documentos de Arquitectura y Diseño

| Documento | Enfoque y Contenido Principal |
| :--- | :--- |
| **[`DECISIONES.md`](DECISIONES.md)** | **Registro Histórico de Decisiones Arquitectónicas (ADRs 1 al 46):** Justificación detallada de cada compromiso de diseño, desde la sala limpia SHA-256 hasta la adopción de `NativePpu` por GPU, texturas R8 y mitigación de cuellos de botella UMA. |
| **[`arquitectura-port-android.md`](arquitectura-port-android.md)** | **Arquitectura Global del Port Android:** Modelo en tres capas (`NativeActivity`, Runtime en Rust y lógica C11), ciclo de vida de superficies `ANativeWindow`, controlador táctil procedural y pipeline gráfico `NativePpu`. |
| **[`arquitectura-c-recompilado.md`](arquitectura-c-recompilado.md)** | **Modelo de Recompilación Estática AOT:** Guía de lectura del código C11 generado (`src-c/`), unidades de traducción de hasta 16 MB, macros de banderas de estado y tabla de despacho `rcg_blocks[]`. |
| **[`hardware-shim-y-eliminacion-de-emulacion.md`](hardware-shim-y-eliminacion-de-emulacion.md)** | **Fundamentos del Hardware Shim:** Por qué se eliminó la emulación tradicional de la PPU por software en CPU para delegar el renderizado directamente en la GPU a través de `wgpu`. |
| **[`ppu-caminos-de-composicion.md`](ppu-caminos-de-composicion.md)** | **Caminos de Composición de la PPU:** Análisis exhaustivo de los modos de fondo (texto y afines), prioridades de capa, ventanas de recorte y efectos de mezcla alfa y brillo. |
| **[`analisis_rendimiento.md`](analisis_rendimiento.md)** | **Evolución del Rendimiento y Telemetría:** Diagnóstico histórico de cuellos de botella en hardware real (desde los 10 FPS en modo INT hasta los 59.7 FPS con GPU nativa en TV Stick). |
| **[`roadmap-y-versionado.md`](roadmap-y-versionado.md)** | **Hoja de Ruta y Versiones:** Planificación por fases del proyecto, hitos de entrega y convenciones de control de versiones. |
