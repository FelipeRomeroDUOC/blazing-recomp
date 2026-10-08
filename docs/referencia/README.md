# Documentación de Referencia Técnica

Este directorio agrupa las especificaciones técnicas puras, formatos de archivo y esquemas de configuración de **`blazing-recomp`**, siguiendo el cuadrante de **Referencia** del estándar **Diátaxis**.

---

## Índice de Especificaciones

| Documento | Descripción |
| :--- | :--- |
| **[`CONFIGURACION_PACK.md`](CONFIGURACION_PACK.md)** | Especificación técnica exhaustiva del archivo de manifiesto [`pack.toml`](../../pack.toml): claves de empaquetado, hashes SHA-256 de ROM/BIOS, flags de runtime y opciones de compilación. |
| **[`FORMATO_LABELS.md`](FORMATO_LABELS.md)** | Especificación del formato de mapa de funciones y símbolos [`blazing.labels.toml`](../../blazing.labels.toml) (`gba-labels v2`): atributos `address`, `mode`, `name`, reglas de alineación y enriquecimiento de depuración. |
| **[`../ARQUITECTURA_C.md`](../ARQUITECTURA_C.md)** | Arquitectura y modelo de ejecución de las 17,207 funciones en C11 emitidas por el recompilador: registros `c[0..16]`, interfaz [`RtApi`](../ARQUITECTURA_C.md#5-la-interfaz-con-el-hardware-rtapi) y macros de banderas. |
