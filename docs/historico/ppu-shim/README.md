# Registro histórico de arneses y pruebas del PpuShim

Este directorio conserva como registro documental los 29 arneses de experimentación y las 10 suites de prueba basadas en informes que dependían del rasterizador antiguo `PpuShim`.

## Contexto y motivo de archivo

Conforme a la decisión de diseño adoptada en el **ADR 42** ([`docs/explicacion/DECISIONES.md`](../../explicacion/DECISIONES.md)), el proyecto consolidó la PPU nativa como el único pipeline de aceleración por GPU y eliminó el código del shim.

Por indicación del usuario (2026-10-23):
* Estos archivos no se compilan ni se ejecutan en la suite de pruebas activa.
* Se mantienen aquí fuera de `examples/` y `tests/` para consultar las mediciones, exploraciones matemáticas y algoritmos de comparación que guiaron el desarrollo de la PPU nativa sin alterar el historial técnico del proyecto.

## Contenido archivado (39 archivos)

### Arneses de experimentación (29 archivos de `examples/`)
* **Comparativas directas:** `ppu_proto.rs`, `ppu_compare.rs`, `ppu_frente_a_frente.rs`.
* **Modos afines y fondos del shim:** `shim_afin_bg.rs`, `shim_afin_bg0.rs`, `shim_afin_bldcnt.rs`, `shim_afin_capa.rs`, `shim_afin_desplaz.rs`, `shim_afin_fondos.rs`, `shim_afin_linea.rs`, `shim_afin_mezcla.rs`, `shim_afin_mide.rs`, `shim_afin_pixeles.rs`, `shim_afin_scroll.rs`, `shim_afin_sinmezcla.rs`, `shim_afin_sinmezcla2.rs`, `shim_afin_tapado.rs`, `shim_afin_tiles.rs`.
* **Capas, sprites y VRAM:** `shim_aportacion.rs`, `shim_bg0_710.rs`, `shim_capa_ninguno.rs`, `shim_capa_x.rs`, `shim_contenido_bg.rs`, `shim_escanear_obj.rs`, `shim_mezcla_registros.rs`, `shim_obj_apagado.rs`, `shim_origen_x.rs`, `shim_sprite_debajo.rs`, `shim_vram_bg.rs`.

### Pruebas de informes (10 archivos de `tests/`)
* `capa_ninguno_tests.rs`, `contenido_bg_shim_tests.rs`, `frente_a_frente_tests.rs`, `identificar_x_tests.rs`, `mezcla_registros_tests.rs`, `obj_apagado_tests.rs`, `origen_x_tests.rs`, `shim_aportacion_tests.rs`, `shim_fondo_tests.rs`, `sprite_debajo_tests.rs`.
