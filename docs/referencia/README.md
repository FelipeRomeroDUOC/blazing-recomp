---
tipo: referencia
verificado: si
---

# Documentación de Referencia Técnica

Este directorio agrupa las especificaciones técnicas puras, formatos de archivo y esquemas de configuración de **`blazing-recomp`**, siguiendo el cuadrante de **Referencia** del estándar **Diátaxis**.

---

## Índice de Especificaciones y Referencias

| Documento | Descripción |
| :--- | :--- |
| **[`api-modulos-rust.md`](api-modulos-rust.md)** | Referencia viva de structs, enums, funciones y constantes de Rust, extraída directamente desde el código en `src/` por `scripts/extraer_referencias.py`. |
| **[`CONFIGURACION_PACK.md`](CONFIGURACION_PACK.md)** | Especificación técnica del manifiesto [`pack.toml`](../../pack.toml): empaquetado, hashes SHA-256 de ROM/BIOS, flags de runtime y opciones de compilación. |
| **[`FORMATO_LABELS.md`](FORMATO_LABELS.md)** | Especificación del mapa de símbolos [`blazing.labels.toml`](../../blazing.labels.toml) (`gba-labels v2`): atributos `address`, `mode`, `name` y alineación. |
| **[`../explicacion/arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md)** | Explicación de la arquitectura global del port, ciclo de vida Android, renderizado GPU y flujo de eventos. |

