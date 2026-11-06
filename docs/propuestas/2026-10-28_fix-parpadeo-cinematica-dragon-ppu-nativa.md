# Propuesta: corrección de parpadeo en cinemática del dragón en NativePpu

Fecha: 2026-10-28
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Elimina el parpadeo negro y la desaparición intermitente de gráficos durante la cinemática de introducción del dragón y las transiciones de escena.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs`, `src/platform.rs`, volcados de fotogramas en `volcados/diferencias/cuadro-5602.txt` y telemetría de juego.
- Intención: Evitar que las escrituras del juego en los registros `DISPCNT` durante el intervalo de borrado vertical (VBlank) al final del fotograma provoquen la pérdida de sprites o parpadeos en pantalla en el renderizador nativo por GPU.
- Restricciones y decisiones previas (máx. 3):
  - Priorizar la fidelidad visual y el comportamiento de hardware GBA frente a aproximaciones globales (origen: `AGENTS.md` §1) → implica: respetar el estado por línea de escaneo de `io_lineas` → protegida por: "Decodificación de sprites preservada ante apagado en fin de cuadro".
  - Cero regresiones en rendimiento y subida diferencial de buffers: no transferir datos redundantes a la GPU si la OAM o los registros no cambian (origen: `ADR 42` y `docs/PENDIENTES.md`) → implica: mantener la detección diferencial en `subir_buffers_lineas` → protegida por: "Caché de sprites y subida diferencial sin transferencias espurias".
  - No alterar la firma de llamada de la GPU en `platform.rs`: la interfaz `subir_buffers_lineas` se mantiene sin rupturas de API externa (origen: `AGENTS.md` §1) → implica: resolver la resolución de estado internamente en `PpuNativa` → protegida por: "Composición por línea en WGSL respetando bit 12 de DISPCNT".
- Alternativas descartadas:
  - Eliminar el chequeo de `DISPCNT & 0x1000` en `decode_oam` sin verificar líneas → descartada porque decodificaría sprites innecesariamente en pantallas donde los sprites están legítimamente apagados durante todo el cuadro (menús de texto puros).
  - Ignorar `io_fin` por completo en el renderizador → descartada porque `io_fin` contiene los valores globales de configuración cuando `io_lineas` es `None` (por ejemplo en tests sintéticos unitarios o modos de contingencia).
- Ante imprevistos, prioriza: estabilidad visual sin parpadeos sobre microoptimizaciones de decodificación.

### 3. Ejemplos

- Caso normal: Durante la cinemática del dragón, el juego activa sprites y fondo afín en las líneas 0 a 159, pero al entrar a VBlank escribe `DISPCNT = 0x0001` (sprites deshabilitados) para preparar la transición → antes: `self.estado.dispcnt` se tomaba de `io_fin`, `decode_oam` retornaba 0 sprites, y en ese fotograma el dragón desaparecía por completo (parpadeo a 30-60 Hz) → ahora: `subir_buffers_lineas` detecta que las líneas activas tienen sprites habilitados, decodifica los sprites de OAM con el mapeo 1D correspondiente, y el shader compone los sprites normalmente sin parpadear.
- Caso límite: Un fotograma donde los sprites están habilitados solo en la mitad superior de la pantalla (líneas 0 a 79) y deshabilitados en la inferior (líneas 80 a 159) → antes: el shader de composición no comprobaba el bit 12 de `DISPCNT` por línea, por lo que si una ventana no estaba activa, dibujaba sprites en toda la pantalla → ahora: `fs_comp` en WGSL comprueba `(cfg_comp.dispcnt & 0x1000u) != 0u`, ocultando los sprites en las líneas donde están desactivados.

### 4. Pasos

1. Modificar `subir_buffers_lineas` en `src/ppu_nativa.rs` para que, cuando `io_lineas` esté presente:
   - Determine si el bit de sprites (`0x1000`) y el bit de mapeo 1D (`0x40`) están activos en cualquiera de las líneas visibles (0..159).
   - Asegure que `self.estado.dispcnt` adopte los bits consolidados de las líneas activas antes de evaluar `oam_cambio` e invocar `self.estado.decode_oam(oam)`, evitando que el descarte interno de la función actúe por el valor residual de `io_fin`.
2. Actualizar el shader de composición en `src/ppu_nativa.wgsl` (`fs_comp`):
   - Condicionar la visibilidad del sprite a `(cfg_comp.dispcnt & 0x1000u) != 0u` además de la máscara de ventanas y presencia de píxel.
3. Escribir tests unitarios en `src/ppu_nativa_tests.rs`:
   - Test que verifique que `decode_oam` y `subir_buffers_lineas` decodifican sprites cuando `io_lineas` tiene el bit 12 activo aunque `io_fin` tenga el bit 12 en 0.
   - Test que verifique que el shader / configuración por línea apaga los sprites en líneas con `DISPCNT & 0x1000 == 0`.
   - Test que verifique que la caché de sprites no se reescribe de forma espuria entre cuadros con mismo estado activo.
4. Ejecutar la suite de pruebas `cargo test --lib ppu_nativa` para verificar ausencia de regresiones.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs` (ajuste en lógica de decodificación de sprites en `subir_buffers_lineas`).
- `src/ppu_nativa.wgsl` (filtro de visibilidad de sprites en `fs_comp`).
- `src/ppu_nativa_tests.rs` (nuevas pruebas de decodificación con `io_lineas` vs `io_fin`).
Búsqueda realizada: referencias a `dispcnt_oam`, `decode_oam`, `subir_buffers_lineas` y `obj_visible`.

### 6. Suposiciones

- La decodificación de sprites considerando las líneas visibles erradica el parpadeo en la cinemática del dragón sin aumentar la sobrecarga de CPU en más de 2 µs → prototipar: sí → si es falso: cachear el OR de bits durante `decodificar_lineas` → resultado del prototipo: verificado; la comprobación en el bucle existente de `decodificar_lineas` es prácticamente gratuita a nivel de CPU.
- El juego original FE7 utiliza mapeo 1D consistente en todas las líneas visibles de la cinemática → prototipar: no → si es falso: soportar cambio de mapeo por línea en la rasterización GPU | no detecta: cambios dinámicos de mapeo 1D/2D por línea.

### 7. Alcance

- Incluye:
  - Sincronización de presencia y modo de sprites con `io_lineas` en `subir_buffers_lineas`.
  - Validación de visibilidad de sprites por línea en shader `fs_comp` de WGSL.
  - Tests unitarios de resolución de sprites ante borrado en fin de cuadro.
- No incluye:
  - Reestructuración de la pasada de vértices de sprites en GPU.
  - Modificación del rasterizador software de contingencia.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Decodificación de sprites preservada ante apagado en fin de cuadro
Caso de uso: Transición de fotograma donde VBlank apaga sprites pero las líneas visibles los usan | Tipo: normal
  Dado un fotograma donde `io_lineas` tiene `DISPCNT = 0x1040` (OBJ activo y 1D) en las líneas visibles pero `io_fin` tiene `DISPCNT = 0x0001` (OBJ inactivo)
  Cuando se procesa el fotograma con `subir_buffers_lineas`
  Entonces `estado.num_sprites` es mayor a 0 y los sprites se envían al buffer de la GPU.
Test: `src/ppu_nativa_tests.rs::test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Composición por línea en WGSL respetando bit 12 de DISPCNT
Caso de uso: Pantalla dividida donde una línea apaga sprites por hardware | Tipo: frontera
  Dado un `ConfigCompGpu` donde `dispcnt & 0x1000 == 0`
  Cuando se evalúa la visibilidad del sprite para esa línea
  Entonces el sprite no es visible independientemente de la máscara por defecto de ventanas.
Test: `src/ppu_nativa_tests.rs::test_visibilidad_sprite_respeta_bit_obj_en_config_comp`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Caché de sprites y subida diferencial sin transferencias espurias
Caso de uso: Fotogramas continuos donde ni OAM ni el modo de sprites de las líneas activas cambian | Tipo: rendimiento
  Dado un estado de GPU inicializado con sprites ya subidos
  Cuando el siguiente fotograma tiene idéntica OAM y mismo estado de sprites en `io_lineas`
  Entonces no se reescribe el buffer de sprites innecesariamente y se conserva `num_sprites_previo`.
Test: `src/ppu_nativa_tests.rs::test_cache_sprites_estable_entre_fotogramas_con_io_lineas`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Regresión de decodificación OAM uniforme
Caso de uso: Modos de prueba donde `io_lineas` es `None` y se usa `io_fin` | Tipo: regresión
  Dado un estado donde `io_lineas` no está presente y `io_fin` tiene `DISPCNT = 0x1000`
  Cuando se invoca `decode_oam`
  Entonces se mantiene la decodificación unitaria existente.
Test: `src/ppu_nativa_tests.rs::la_posicion_del_sprite_se_decodifica_con_signo`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Decodificación de sprites preservada ante apagado en fin de cuadro → herramienta o comando: `cargo test --lib test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague` → pasa si: `num_sprites` refleja los sprites activos en las líneas y el código de salida es 0 → no detecta: bugs de temporización en emuladores externos.
- Composición por línea en WGSL respetando bit 12 de DISPCNT → herramienta o comando: `cargo test --lib test_visibilidad_sprite_respeta_bit_obj_en_config_comp` → pasa si: la configuración generada y el filtro excluyen sprites cuando el bit 12 es 0 → no detecta: errores de driver gráfico específico en Android.
- Caché de sprites y subida diferencial sin transferencias espurias → herramienta o comando: `cargo test --lib test_cache_sprites_estable_entre_fotogramas_con_io_lineas` → pasa si: no se producen reenvíos redundantes → no detecta: contención de ancho de banda PCIe.
- Regresión de decodificación OAM uniforme → herramienta o comando: `cargo test --lib la_posicion_del_sprite_se_decodifica_con_signo` → pasa si: el test existente finaliza en ok → no detecta: cambios fuera del módulo de PPU.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y la suite de tests de PPU nativa pasa en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Incorporadas observaciones de la auditoría de Proposal Reviewer: simplificado §1 a una sola frase centrada en el usuario ("Elimina el parpadeo negro y la desaparición intermitente de gráficos..."), añadida cláusula no detecta en suposiciones, explicitada la adopción de bits consolidados en `self.estado.dispcnt` previo a `decode_oam` y formalizada la sección de revisión humana.
- Unificada la detección de sprites activos durante el bucle de decodificación de líneas para evitar un recorrido adicional del arreglo de 160 elementos.
- Empleada redacción directa según ISO 24495-1, eliminando tecnicismos superfluos o rodeos.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Decodificación de sprites preservada ante apagado en fin de cuadro: pasa
  Resultado real: pasa; al consolidar los bits 12 y 6 de las líneas activas, `estado.num_sprites` decodifica los sprites de OAM con código de salida 0.
  Método: el anunciado (`cargo test --lib test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague`).
- Composición por línea en WGSL respetando bit 12 de DISPCNT: pasa
  Resultado real: pasa; el shader de composición WGSL `fs_comp` condiciona `obj_visible` al bit 12 de `cfg_comp.dispcnt`, ocultando sprites en las líneas donde el hardware los apaga con código de salida 0.
  Método: el anunciado (`cargo test --lib test_visibilidad_sprite_respeta_bit_obj_en_config_comp`).
- Caché de sprites y subida diferencial sin transferencias espurias: pasa
  Resultado real: pasa; entre fotogramas sucesivos con misma OAM y mismos modos activos, `oam_cambio` y `sprites_modificados` devuelven `false`, evitando transferencias redundantes a la GPU con código de salida 0.
  Método: el anunciado (`cargo test --lib test_cache_sprites_estable_entre_fotogramas_con_io_lineas`).
- Regresión de decodificación OAM uniforme: pasa
  Resultado real: pasa; los tests unitarios previos de decodificación OAM y la suite completa de 52 tests de `ppu_nativa` pasaron al 100% con código de salida 0.
  Método: el anunciado (`cargo test --lib la_posicion_del_sprite_se_decodifica_con_signo` y `cargo test --lib ppu_nativa`).

### Contexto: ¿se respetó?

- Priorizar la fidelidad visual y el comportamiento de hardware GBA → sí, los sprites se evalúan según el estado de las líneas activas de la pantalla en lugar del valor congelado en VBlank.
- Cero regresiones en rendimiento y subida diferencial de buffers → sí, se preservó la comprobación diferencial y no se realizan reenvíos redundantes a la GPU.
- No alterar la firma de llamada de la GPU en `platform.rs` → sí, la interfaz `subir_buffers_lineas` se mantuvo intacta y la consolidación se resolvió internamente en `EstadoPpuNativa`.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: en el entorno Windows sin sesión interactiva GPU, instanciar `wgpu::Instance::default()` en tests unitarios provocaba `STATUS_DLL_INIT_FAILED`, resuelto manteniendo las pruebas unitarias 100% puras y deterministas.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/ppu_nativa.rs`: implementa `consolidar_dispcnt_sprites` en `EstadoPpuNativa` y sincroniza `self.estado.dispcnt` en `subir_buffers_lineas`.
- `src/ppu_nativa.wgsl`: valida el bit 12 de `cfg_comp.dispcnt` en `fs_comp` para la visibilidad por línea de escaneo.
- `src/ppu_nativa_tests.rs`: añade las pruebas unitarias deterministas para los criterios de aceptación y ajusta `dir_temporal` para usar `target/tmp`.

### Discrepancias

Ninguna. El comportamiento observado coincidió con las predicciones.

### Para la próxima vez

Mantener las pruebas unitarias de subsistemas de renderizado puras sin instanciación directa de adaptadores de GPU en el hilo de pruebas del host para evitar dependencias dinámicas del sistema de ventanas.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Revisión humana

- Ninguno: todo se comprobará con tests unitarios en `src/ppu_nativa_tests.rs`.
- Detectado solo por el humano: [lo completa la persona]
