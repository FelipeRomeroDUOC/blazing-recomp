# Propuesta: Arnés de comparación entre la PPU por software y el shim GPU, con visor en vivo

Fecha: 2026-10-16
Nivel: 2
Estado: aprobada (el usuario aprobó el alcance el 2026-10-16)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Una herramienta para Windows que ejecuta Fire Emblem una sola vez con una secuencia fija de botones y muestra, cuadro a cuadro, la imagen de la PPU por software junto a la del shim GPU y la diferencia entre ambas, para ver en qué línea y en qué momento aparece cada glitch. Habrá un modo por lotes (informe) y un modo en vivo (ventana con controles).

### 2. Contexto

- **Contexto consultado:** `docs/propuestas/2026-10-16_fase-1-ppu-shim-gpu.md` (paso 4: `step_frame` desactiva `render_scanline`), `docs/propuestas/2026-10-16_fase-1-2-intercalado-prioridades-bg-obj-y-descarte-obj-window.md` (ventanas previstas para la Fase 1.4), `docs/propuestas/2026-10-13_arnes-de-fotogramas-dorados.md` y `tests/golden_frame_tests.rs`. Código leído: `src/pipeline.rs` (alrededor de las líneas 349 a 360, donde se copian VRAM, OAM, paleta e I/O una vez por cuadro), `src/ppu_shim.rs` (`decode_io_regs`, `render_frame`) y `src/ppu_shim.wgsl` (búsqueda de ventanas y blend, sin resultados).
- **Intención:** poder ver y controlar el mismo cuadro en ambas versiones, para medir cada corrección del shim contra la PPU por software sin tocar la ruta del juego. Éxito: en la intro y en el primer mapa se puede ver en qué línea y en qué cuadro aparece cada diferencia.
- **Restricciones y decisiones previas:**
  - La PPU por software se conserva y no se borra (respuesta humana, 2026-10-16) → implica: el código de software de `gba-core` debe seguir compilando y poder invocarse desde el harness, aunque el juego no lo use → protegida por: Escenario 3.
  - El shim GPU queda como único renderizador en el juego (respuesta humana, 2026-10-16) → implica: el harness lee la salida del shim y no cambia la ruta del juego → no testeable (es una decisión de diseño; se revisa en la revisión humana).
  - El trabajo es en el PC con Windows, no en Android (respuesta humana, 2026-10-16) → implica: el harness no se compila para Android → no testeable directamente; se verifica al correrlo en el PC.
  - La secuencia empieza con `A` para elegir idioma y después hay 2 a 3 minutos sin botones (respuesta humana, 2026-10-16) → implica: la secuencia incluye una espera de al menos 10.800 cuadros sin entradas → protegida por: Escenario 2.
- **Alternativas descartadas:**
  - Dos ejecuciones separadas del juego → descartada porque hay que sincronizarlas; una sola ejecución da las dos salidas del mismo cuadro.
  - Visor en Android → descartada porque retrasa cada iteración (compilar, instalar) y no tiene teclado.
- **Ante imprevistos, prioriza:** exactitud de la comparación sobre velocidad de ejecución.

### 3. Ejemplos

- **Caso normal:** secuencia `A` en el idioma → 3 minutos sin botones → `A` en los diálogos hasta el primer mapa. Antes: no hay forma de ver el software y el shim lado a lado. Ahora: una ejecución produce un informe con una fila por cuadro y la primera línea donde divergen.
- **Caso límite:** la ruta del ROM no existe. Antes: no aplica. Ahora: el programa termina con error claro y código distinto de cero, sin abrir ventana ni escribir informe.

### 4. Pasos

1. Prototipo: comprobar que `wgpu` crea un dispositivo sin superficie en este PC.
2. Prototipo: confirmar la API de estado de `gba-core` (`machine.rs`). Si existe, guardar el estado justo antes de la intro para no repetir el arranque.
3. Crear `examples/ppu_compare.rs`: carga el ROM, lee la secuencia, corre cuadro a cuadro, renderiza el software y el shim, compara y escribe el informe.
4. Crear `examples/ppu_live.rs`: la misma lógica dentro de una ventana con tres paneles (software, GPU, diferencia) y controles de teclado.
5. Agregar las dependencias de ventana solo para el visor (`[dev-dependencies]` o `[[example]]`).
6. Escribir las pruebas de los escenarios.

### 5. Qué más podría verse afectado

- `src/ppu_shim.rs` y `src/ppu_shim.wgsl`: solo lectura en esta propuesta.
- `src/pipeline.rs`: solo lectura.
- `gba-core/src/ppu.rs` y `gba-core/src/machine.rs`: lectura; puede haber un cambio mínimo para exponer el estado, solo si el prototipo 2 lo requiere.
- `Cargo.toml`: nuevas dependencias de ventana y de escritura de imágenes.
- **Interruptores existentes (verificados el 2026-10-16):**
  1. Runtime: `EmulationWorker` activa el shim al iniciar (`platform.rs` línea 1599, `set_gpu_ppu_enabled(true)`), sobre el `AtomicBool` de `pipeline.rs` (líneas 87 y 169 a 176) y `GameRunner.gpu_ppu_enabled` (`runner.rs` líneas 61 y 115 a 122). El harness puede usarlo para cambiar de ruta sin recompilar.
  2. Compilación: constante `FORZAR_PPU_SOFTWARE = false` (`platform.rs` línea 133), que anula el interruptor 1 en `use_gpu_shim` (línea 909) y en la cabecera del log (línea 1476).
- **Hallazgo sobre la referencia por software:** `render_scanline` se llama sin condición en cada línea visible (`gba-core\src\mem.rs` línea 709). Con el shim activo, la PPU por software sigue calculándose, así que el harness puede leer ambas salidas en una sola ejecución. También significa que el paso 4 de la Fase 1 (desactivar `render_scanline`) **no está implementado** en el código actual, y el ahorro de CPU que promete esa fase no está medido.

### 6. Suposiciones

- `wgpu` puede renderizar sin ventana en este PC (DX12 o Vulkan) → prototipar: sí → si es falsa: usar el adaptador de software de `wgpu`, o solo el visor → resultado del prototipo: verificada el 2026-10-16. Adaptador NVIDIA GeForce GTX 1650 Ti (Vulkan); el shim dibujó 38.244 de 38.400 píxeles con color.
- La secuencia completa (arranque, idioma, 3 minutos, diálogos hasta el mapa) corre en menos de 5 minutos en el PC → prototipar: sí → si es falsa: reducir la intro a puntos de control → resultado del prototipo: parcial (2026-10-16). Solo software: gba-core tarda 0,678 ms por cuadro, así que los 10.800 cuadros de espera tardan unos 7,3 s. No incluye el shim ni la lectura de la GPU, y la secuencia completa no se ha medido.
- `gba-core` es determinista: dos ejecuciones con la misma secuencia dan los mismos cuadros → prototipar: sí → si es falsa: el harness no sirve para comparar, y hay que buscar la fuente de no determinismo → resultado del prototipo: verificada el 2026-10-16. 600 de 600 cuadros con hash igual en dos corridas. Solo cubre los primeros 600 cuadros; la intro completa no se ha probado.
- Existe una API de estado en `gba-core` para guardar y restaurar → prototipar: no → resultado: falsa. `machine.rs` no expone guardado ni restauración de estado (la única coincidencia en `gba-core` era un comentario). Se usa solo la secuencia de botones; el arranque se repite en cada corrida.

### 7. Alcance

- **Incluye:** harness por lotes con informe, visor en vivo con tres paneles y control de teclado, archivo de secuencia de entradas, pruebas de los escenarios.
- **No incluye:** cambios en el shim para corregir glitches (eso es Fase 1.x), estado por línea, ventanas ni blend, cambios en Android, ROMs dentro del repositorio.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: dos ejecuciones con la misma secuencia producen los mismos cuadros
Caso de uso: repetir una prueba después de un cambio para saber si el resultado cambió | Tipo: regresión
  Dado que la secuencia de la intro está en un archivo de texto y el ROM está en una ruta local
  Cuando el harness corre dos veces seguidas sin cambios en el código
  Entonces los hashes de los 10.800 cuadros de la espera son iguales en ambas corridas
Test: `tests/ppu_compare_tests.rs::test_mismo_rom_misma_secuencia_mismos_cuadros` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (el harness no existe)

Escenario: la intro se recorre durante 3 minutos sin botones y el informe tiene una fila por cuadro
Caso de uso: revisar qué pasa en la intro, que es donde están los peores glitches | Tipo: normal
  Dado una secuencia con `A` al inicio y 10.800 cuadros sin entradas después
  Cuando el harness procesa la secuencia
  Entonces el informe tiene 10.800 filas, cada una con el porcentaje de píxeles distintos y la primera línea divergente (o `-` si no hay diferencias)
Test: `tests/ppu_compare_tests.rs::test_intro_genera_una_fila_por_cuadro`
Estado previo: falla (el harness no existe)

Escenario: la PPU por software sigue disponible como referencia
Caso de uso: comparar el shim contra la referencia sin cambiar el juego | Tipo: regresión
  Dado que el juego corre con el shim GPU
  Cuando el harness pide la imagen por software del mismo cuadro
  Entonces la imagen tiene 240 × 160 píxeles y coincide con el framebuffer que produce `gba-core` en ese cuadro
Test: `tests/ppu_compare_tests.rs::test_referencia_software_coincide_con_framebuffer`
Estado previo: pasa (regresión; el código de software ya existe)

Escenario: error claro si el ROM no existe
Caso de uso: ejecutar el harness con una ruta equivocada | Tipo: error
  Dado que la ruta del ROM no existe
  Cuando se ejecuta `cargo run --example ppu_compare -- ruta_falsa.gba`
  Entonces el programa termina con código distinto de 0, muestra `ROM no encontrado: ruta_falsa.gba` y no escribe informe
Test: `tests/ppu_compare_tests.rs::test_rom_inexistente_falla_con_mensaje`
Estado previo: falla (el harness no existe)

Escenario (visible): el visor en vivo responde a pausa y avance de cuadro
Caso de uso: examinar un cuadro con un glitch | Tipo: normal
  Dado el visor abierto en la intro
  Cuando se pausa y se avanza un cuadro
  Entonces los tres paneles se actualizan y la diferencia muestra el mismo cuadro
Test: no testeable con código; revisión humana.

### 9. Cómo lo comprobaré

- Determinismo → `cargo test --test ppu_compare_tests test_mismo_rom_misma_secuencia_mismos_cuadros` con `FE_ROM` apuntando al ROM → pasa si: los hashes de las dos corridas coinciden → no detecta: diferencias que aparezcan solo con otro hardware.
- Informe de la intro → `cargo run --release --example ppu_compare -- %FE_ROM% secuencias\intro.txt` → pasa si: el informe tiene 10.800 filas → no detecta: si la imagen se ve bien (eso lo mira una persona).
- ROM inexistente → `cargo run --example ppu_compare -- ruta_falsa.gba` → pasa si: código distinto de cero y mensaje exacto → no detecta: errores en otros parámetros.
- Referencia por software → `cargo test --test ppu_compare_tests test_referencia_software_coincide_con_framebuffer` → pasa si: coinciden los 240 × 160 píxeles → no detecta: errores en el propio framebuffer de software.
- Visor en vivo (visible) → revisión humana: abrir `cargo run --example ppu_live -- %FE_ROM%`, pausar, avanzar un cuadro y mirar los paneles → no detecta: diferencias de color que solo se notan en la pantalla del televisor.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, los dos prototipos tienen resultado, y el informe de la intro se generó en el PC.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

Ninguna.

## Fuera de alcance (hallazgos)

- **Captura única por cuadro (causa probable de las bandas):** el plan de Fase 1 (paso 4) y `src/pipeline.rs` toman VRAM, OAM, paleta e I/O una sola vez por cuadro, después de `step_frame()`. El shim aplica ese estado a todas las líneas, aunque el juego cambie registros a mitad del cuadro (como ocurre con los efectos de las capturas 5 y 6). Esto contradice la premisa de la Fase 1 y necesita una propuesta propia de estado por línea.
- **Ventanas y blend:** `src/ppu_shim.wgsl` no tiene ninguna mención a ventanas ni blend en la revisión de hoy. La Fase 1.4 lo tiene previsto.
- **Solo modo 0:** `decode_io_regs` y `render_frame` solo tratan el modo 0 (comentario en `ppu_shim.rs`). Las escenas en otros modos (por ejemplo, la pantalla de título) no quedan cubiertas.

## Revisión humana

- Visor en vivo: que los tres paneles sean legibles y que los controles de teclado respondan como se espera.
- Secuencia: que el arranque, el idioma y los diálogos lleguen al primer mapa de Fire Emblem.

---

## Resultado (lo completa el agente después de editar)

Prototipos 1 a 3 con resultado el 2026-10-16 (sección 6). Pendiente: pasos 3 a 6 y criterios de aceptación.
