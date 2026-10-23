# Propuesta: Eliminación de register spilling, bucle anidado y lecturas redundantes en el shader de composición de la PPU nativa

Fecha: 2026-10-23
Nivel: 2
Estado: revisada por humano

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl), se optimiza la función de composición por píxel `fs_comp` y las funciones de muestreo de fondos para eliminar el *register spilling* a memoria privada (*stack*) en GPUs móviles *tile-based* (como la Mali-G31 bajo OpenGL ES 3.0):

1. **Eliminación de la copia completa de `filas_capa[upy]` (`272 bytes`) y de la indexación dinámica `cfg_capa.capas[b]` y `prio[b]`:**
   - En lugar de hacer `let cfg_capa = filas_capa[upy];` (que carga los 4 fondos = `68` registros de 32 bits en una variable local) e indexar `cfg_capa.capas[b]` con la variable dinámica `b` del bucle (lo que obliga al compilador GLSL de Mali a volcar los `272 bytes` a RAM privada por cada uno de los `38.400` píxeles), se pasa `(upy, idx_capa)` con índices constantes (`0u`, `1u`, `2u`, `3u`) o se accede únicamente a `filas_capa[upy].capas[0u..3u]` cuando la capa está habilitada en `control`.
2. **Sustitución del bucle anidado `4 × 4` (`16` iteraciones con ramificación dinámica) por evaluación desenrollada con índices constantes y resolución directa de las 2 capas superiores:**
   - Se añade una función auxiliar `acumular_capa(color_candidato: u32, id_candidato: u32, ...)` o evaluación por prioridad sin indexar arreglos locales con variables dinámicas, y solo se muestrea un fondo `b` si:
     1. está activo en la máscara de ventana (`(control & (1u << b)) != 0u`),
     2. está habilitado en el modo actual (`bg_prio != 0xFFu`), y
     3. su prioridad `bg_prio` es menor o igual que la peor prioridad que aún necesitamos resolver (por ejemplo, si ya tenemos `primera` y no hay mezcla de 2.º objetivo ni sprite semitransparente, cualquier capa de prioridad mayor o posterior es descartada sin tocar VRAM ni paleta).
3. **Poda de lectura de `tex_obj_ventana` y de evaluación del 2.º objetivo (`segunda`) cuando no hay mezcla:**
   - `textureLoad(tex_obj_ventana, coord, 0)` solo se invoca cuando `objwin_on` (`(cfg_comp.dispcnt & 0x8000u) != 0u`) es verdadero (en la telemetría del TV Stick, `win=0` la mayor parte del tiempo, ahorrando `38.400` lecturas de textura por fotograma).
   - Si una vez hallada la `primera` capa ganadora se sabe que **no** se necesita la `segunda` capa (porque `primera` no es un sprite semitransparente y además la unidad de mezcla no está en modo alfa `modo == 1u` con `primera` como 1.er objetivo), el bucle se detiene inmediatamente tras encontrar `primera` (`corte = true`), reduciendo a la mitad las lecturas de VRAM de fondos en píxeles opacos.

### 2. Contexto

- Contexto consultado: telemetría real del TV Stick (`blazing-perf-tv-28.txt` vs sesión anterior), [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl), [`src/ppu_nativa.rs`](../../src/ppu_nativa.rs) y [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs).
- Intención: al comparar las dos telemetrías del TV Stick (CPU 4× Cortex-A53 @ 1344 MHz, GPU Mali-G31, `backend=OpenGL`), quedó demostrado por qué el rendimiento seguía en `31–35 FPS` (`dibujo ≈ 30,0 ms`):
  - En la propuesta anterior, `subida_ms` incluía el grabado de pases (`grabar_pases`) y el primer `queue.submit()`. Al mover `grabar_pases` a `codificacion_ms` y aplicar subida diferencial:
    - `subida` bajó de `6,8–9,5 ms` a `3,1–5,2 ms` (ahorro real de `~3,0 ms` de CPU/bus).
    - `codificacion` pasó de `1,5 ms` a `4,2 ms` (porque ahora mide `grabar_pases` + `submit`).
    - Sin embargo, **`present` permaneció en `20,0–23,5 ms`** y la suma `subida + codificacion + present` siguió clavada en **`~30,5 ms` (`32 FPS`)**.
  - En OpenGL ES sobre Android, `eglSwapBuffers` (`present`) bloquea hasta que la GPU Mali-G31 termina de rasterizar los fragmentos pendientes y llega el borde del VSync (`2 × 16,66 ms = 33,3 ms` cuando el cuadro tarda más de `16,66 ms`).
  - La causa en el shader [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl) (líneas 472 y 529–565) es un antipatrón clásico en GPUs móviles:
    1. `let cfg_capa = filas_capa[upy];` copia una estructura de `272 bytes` (`68` palabras de 32 bits) a una variable local del hilo de fragmento.
    2. Dentro del bucle `for (var p = 0u; p < 4u; p++) { for (var b = 0u; b < 4u; b++) { ... color_capa(cfg_capa.capas[b], ...) } }`, se indexa `cfg_capa.capas[b]` y `prio[b]` usando una variable dinámica `b`. En la arquitectura Mali (Bifrost G31, 1 solo núcleo de sombreado a ~650 MHz), los registros del procesador no admiten direccionamiento indirecto por variable: el compilador de Naga/GLSL vuelca los `272 bytes` de `cfg_capa` y los `16 bytes` de `prio` a la memoria privada en RAM (*stack spilling* con tráfico L2/DRAM) en **cada uno de los 38.400 píxeles**.
    3. Además, siempre busca tanto `primera` como `segunda` capa incluso cuando `primera` es opaca y no participa en mezcla alfa (`modo != 1`), muestreando fondos tapados innecesariamente.
- Restricciones y decisiones previas (máx. 3):
  - Mantener el 100 % de exactitud visual píxel a píxel frente a la PPU de referencia de `gba-core` (origen: `2026-10-19_ppu-nativa-fase-1.md` y pruebas de aceptación en `tests/ppu_nativa_tests.rs`) → implica: la poda del 2.º objetivo (`segunda`) solo puede activarse cuando matemáticamente `segunda` no interviene en la ecuación final de color (es decir, cuando no hay sprite semitransparente en `primera` y tampoco se cumple `efectos && modo == 1u && (bldcnt & (1u << primera_id)) != 0u`) → protegida por «El shader sin indexación dinámica ni búsqueda redundante del 2.º objetivo conserva 0 diferencias visuales» | testeable.
  - Conservar intactas las estructuras y *bindings* entre Rust y WGSL (`CapaFondo`, `ConfigCapa`, `ConfigComp`, `SpriteNativa` y `BINDING_*`) (origen: `tests/ppu_nativa_tests.rs::los_desplazamientos_de_las_estructuras_coinciden`) → implica: optimizar el cuerpo de `fs_comp` y `color_capa` en `src/ppu_nativa.wgsl` sin alterar el layout de memoria de los *Storage Buffers* → protegida por «Los desplazamientos, tamaños y bindings del shader siguen coincidiendo con Rust» | testeable.
  - No modificar `src/ppu_shim.*` ni el motor base fijado en `tests/datos/ppu-nativa-hashes-base.txt` (origen: `AGENTS.md` §4.1) → implica: cambios confinados a `src/ppu_nativa.wgsl` y `tests/ppu_nativa_tests.rs` → protegida por «El motor base y el shim no cambian» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Reducir la resolución interna de la GBA (`240 × 160`) → descartada porque violaría la fidelidad nativa píxel a píxel.
- Ante imprevistos, prioriza: exactitud bit a bit del color de salida > eliminación de arrays locales indexados dinámicamente > corte temprano tras hallar `primera` cuando no hay mezcla alfa.

### 3. Ejemplos

- Caso normal (eliminación de *register spilling* por indexación dinámica en `fs_comp`): al evaluar las capas de fondo en el píxel `(upx, upy)` → antes: `let cfg_capa = filas_capa[upy];` copiaba `272 bytes` a una variable local y el bucle interior hacía `color_capa(cfg_capa.capas[b], upx, upy)` con `b` variable (`0..4`), forzando *stack spilling* en GLSL ES 3.0 → ahora: `fs_comp` no copia `ConfigCapa` a una variable local indexada dinámicamente ni usa un array `prio[b]`; cada fondo (`0u`, `1u`, `2u`, `3u`) se consulta con índice constante (`filas_capa[upy].capas[0u]`, etc.) y solo si su prioridad coincide con `p` y su bit en `control` está activo.
- Caso normal (corte temprano cuando el 1.er objetivo no usa mezcla alfa): en el mapa o en escenas con `modo == 0` (sin mezcla) o `modo == 2/3` (aclarar/oscurecer, que solo usan `primera` y nunca `segunda`), o en `modo == 1` (mezcla alfa) cuando la capa ganadora `primera` **no** tiene activo su bit de 1.er objetivo en `BLDCNT` y tampoco es un sprite semitransparente → antes: después de encontrar `primera` (por ejemplo, un sprite opaco de unidad o `BG0`), el bucle seguía recorriendo las prioridades restantes y leyendo VRAM/paleta de `BG1`, `BG2` y `BG3` para encontrar `segunda`, aunque `segunda` jamás se usaría → ahora: una función `necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, bldcnt) -> bool` comprueba si `segunda` puede influir en el color final; si devuelve `false`, activa `corte = true` inmediatamente después de asignar `primera`.
- Caso normal (lectura perezosa de `tex_obj_ventana`): en una línea donde `DISPCNT` bit 15 (`objwin_on`) es `0` → antes: `fs_comp` ejecutaba siempre `let obj_window = textureLoad(tex_obj_ventana, coord, 0).r != 0u;` en la línea 491 antes de comprobar `objwin_on` → ahora: solo invoca `textureLoad(tex_obj_ventana, coord, 0)` dentro de la rama `else if (objwin_on)` cuando el píxel no cayó ni en `WIN0` ni en `WIN1`.

### 4. Pasos

1. En [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl):
   - En `indice_paleta_obj`: leer únicamente `filas_capa[py_clamped].modo` (ya lo hace, verificar que no copie la estructura).
   - En `fs_comp`:
     - Eliminar `let cfg_capa = filas_capa[upy];` y `var prio = array<u32, 4>(...);`.
     - Mover `textureLoad(tex_obj_ventana, coord, 0)` para que solo se ejecute si `objwin_on` es verdadero y el píxel no entró en `en_win0` ni en `en_win1`.
     - Definir una función auxiliar pura `necesita_segunda(primera_id: u32, obj_visible: bool, obj_semi: bool, efectos: bool, modo: u32, bldcnt: u32) -> bool` que devuelva `true` únicamente si `(obj_visible && primera_id == 4u && obj_semi) || (efectos && modo == 1u && (bldcnt & (1u << primera_id)) != 0u)`.
     - Desenrollar la evaluación de los 4 fondos (`0u`, `1u`, `2u`, `3u`) dentro de cada nivel de prioridad `p` usando índices constantes `filas_capa[upy].capas[0u]`, `filas_capa[upy].capas[1u]`, `filas_capa[upy].capas[2u]` y `filas_capa[upy].capas[3u]` y comparando contra `cfg_comp.bg_prio0`, `bg_prio1`, `bg_prio2` y `bg_prio3` directamente, sin indexación dinámica `capas[b]` ni `prio[b]`.
     - Al encontrar `primera` (sea sprite o fondo), si `!necesita_segunda(...)`, cortar inmediatamente la búsqueda (`corte = true`) sin evaluar capas inferiores.
2. En [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs):
   - Añadir un test `test_fs_comp_no_indexa_arreglos_locales_dinamicamente_y_valida_glsl` que compile `PPU_NATIVA_WGSL` con `naga`, valide el módulo y verifique que no existe indexación dinámica `capas[b]` ni `prio[b]` en `fs_comp` y que el shader sigue pasando todas las pruebas de estructura y de fidelidad visual.
3. Ejecutar los 5 criterios del Pre-Commit Gatekeeper (`cargo test`, `validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py` y auditoría de rutas).

### 5. Qué más podría verse afectado

- [`src/ppu_nativa.wgsl`](../../src/ppu_nativa.wgsl) — optimización de `fs_comp` (eliminación de copia local de `ConfigCapa`, desenrollado de acceso a `capas[0u..3u]`, lectura perezosa de `tex_obj_ventana` y corte temprano cuando no se requiere `segunda`).
- [`tests/ppu_nativa_tests.rs`](../../tests/ppu_nativa_tests.rs) — nuevo test de validación estática del shader y ejecución de la suite de regresión.

### 6. Suposiciones

- En la GPU Mali-G31 (`backend=OpenGL`), la eliminación del *register spilling* de `ConfigCapa` (`272 B`/píxel) y el corte temprano de la segunda capa reducen el tiempo de fragmento de `fs_comp` lo suficiente para que el cuadro entre dentro del presupuesto de un VSync (`16,66 ms`) → prototipar: se valida primero con `naga` y la suite de GPU en host, y luego en el TV Stick con `blazing-perf.txt`.

### 7. Alcance

- Incluye:
  - Eliminación de `cfg_capa.capas[b]` y `prio[b]` (indexación dinámica sobre estructuras locales) en `src/ppu_nativa.wgsl`.
  - Corte temprano tras hallar `primera` cuando la unidad de mezcla no requiere `segunda`.
  - Lectura perezosa de `tex_obj_ventana` solo cuando `objwin_on` está activo.
  - Test de integración en `tests/ppu_nativa_tests.rs`.
- No incluye:
  - Cambios en `src/ppu_nativa.rs`, `src/platform.rs` ni `gba-core`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El shader `fs_comp` prescinde de indexación dinámica de estructuras locales y corta la búsqueda cuando no se requiere segunda capa
Caso de uso: evitar que el compilador GLSL de Mali-G31 vuelque `272 bytes` por píxel al stack y evitar lecturas de VRAM de capas tapadas cuando no hay mezcla alfa | Tipo: normal
  Dado que el código fuente de `src/ppu_nativa.wgsl`
  Cuando se analiza y valida con `naga` y se inspecciona `fs_comp`
  Entonces el módulo valida sin errores, no contiene indexación dinámica `capas[b]` ni `prio[b]`, evalúa `necesita_segunda` para cortar tras hallar `primera` cuando no hay mezcla alfa, y solo lee `tex_obj_ventana` bajo `objwin_on`
Test: `tests/ppu_nativa_tests.rs::test_fs_comp_no_indexa_arreglos_locales_dinamicamente`
Estado previo: se completa al escribir el test: falla (actualmente `fs_comp` usa `cfg_capa.capas[b]` y `prio[b]` y lee `tex_obj_ventana` incondicionalmente)

Escenario: El shader optimizado conserva 0 diferencias visuales y pasa toda la suite de la PPU nativa
Caso de uso: garantizar que el desenrollado de capas y el corte temprano del 2.º objetivo sean matemáticamente equivalentes a la especificación de `gba-core` | Tipo: regresión
  Dado que la suite de pruebas de `blazing-recomp` (`src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs`)
  Cuando se ejecuta `cargo test`
  Entonces el 100 % de los tests unitarios y de integración pasan en verde
Test: `tests/ppu_nativa_tests.rs::los_desplazamientos_de_las_estructuras_coinciden`
Estado previo: se completa al escribir el test: pasa (regresión)

### 9. Cómo lo comprobaré

- El shader `fs_comp` prescinde de indexación dinámica de estructuras locales y corta la búsqueda cuando no se requiere segunda capa → herramienta o comando:
  `cargo test --test ppu_nativa_tests test_fs_comp_no_indexa_arreglos_locales_dinamicamente`
  → pasa si: `1 passed`
  → no detecta: tiempos de ejecución de fragmento en la GPU Mali-G31 real.
- El shader optimizado conserva 0 diferencias visuales y pasa toda la suite de la PPU nativa → herramienta o comando:
  `cargo test`
  → pasa si: el 100 % de los tests pasan en verde, seguido de `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check`.
  → no detecta: tasa de FPS en el TV Stick.

### Correcciones previas (si las hubo)

- Viene de la discrepancia registrada en [`2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md`](2026-10-23_optimizacion-subida-diferencial-y-encoder-unico-ppu-nativa.md), donde la telemetría `blazing-perf-tv-28.txt` demostró que bajar `subida` de `~7,5 ms` a `~4,3 ms` no redujo `dibujo` (`~30 ms`) porque `present` (`~21 ms`) está esperando a que la GPU Mali-G31 termine de ejecutar `fs_comp`.

---

## Resultado (lo completa el agente después de editar y verificar)

Estado final: verificada por agente

### Predicho vs. real

- **Eliminación de register spilling por arreglos locales indexados dinámicamente:**
  - *Predicho:* Eliminar `let cfg_capa = filas_capa[upy]` (`272 bytes`) y el arreglo `prio[b]` de variables locales, accediendo a los fondos con índices constantes `filas_capa[upy].capas[0u..3u]` y comparando contra `cfg_comp.bg_prio0..3`.
  - *Real:* Implementado en `src/ppu_nativa.wgsl` (`fs_comp`). Desenrollado por fondo (BG0, BG1, BG2, BG3) dentro de cada nivel de prioridad `0..4`, eliminando completamente la indexación dinámica y el volcado a RAM privada. Validado con `test_fs_comp_no_indexa_arreglos_locales_dinamicamente`.
- **Corte temprano de búsqueda del 2.º objetivo:**
  - *Predicho:* Función `necesita_segunda` que detecta si la unidad de mezcla realmente requiere la segunda capa o si es un sprite semitransparente; si devuelve `false`, fijar `corte = true` y salir del bucle inmediatamente tras hallar `primera`.
  - *Real:* Implementada la función `necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, bldcnt)`. En píxeles opacos sin mezcla alfa, la búsqueda se detiene en cuanto se encuentra `primera`, ahorrando lecturas de VRAM y paleta de capas inferiores.
- **Lectura perezosa de la ventana de sprites (`OBJWIN`):**
  - *Predicho:* Invocar `textureLoad(tex_obj_ventana, coord, 0)` solo cuando `objwin_on` es verdadero y el píxel no cayó en `WIN0` ni `WIN1`.
  - *Real:* `textureLoad(tex_obj_ventana, coord, 0)` se movió al bloque condicional `else if (objwin_on)`.
- **Exactitud visual e integridad de la suite:**
  - *Predicho:* 100 % de tests en verde sin alterar la exactitud de los colores de salida.
  - *Real:* Los 15 tests de `tests/ppu_nativa_tests.rs` (incluyendo validación Naga, fidelidad de mezcla, extremos y casos límite) y los 190 tests unitarios de `src/` pasaron al 100 % en verde.

### Qué se respetó del contexto

- La interfaz de bindings, descriptores y buffers entre Rust y WGSL permanece 100 % idéntica.
- `gba-core`, `src/ppu_shim.*` y el pipeline de emulación no se modificaron.
- Cero rutas absolutas locales en el repositorio.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno en el shader `src/ppu_nativa.wgsl`.

## Revisión humana (solo si el cambio tiene parte visible o de uso)

- [x] Compilar el APK con `scripts/package_apk.ps1`, instalarlo en el TV Stick (Mali-G31) y comprobar en `blazing-perf.txt` si `present` baja y los FPS suben a 60 FPS estables.

Veredicto del usuario: aprobado (`blazing-perf-tv-29.txt`: media de 59,6 FPS a lo largo de 272 segundos; el tiempo de dibujo bajó de ~30,0 ms a 16,5 ms de media con tramos sostenidos de 8–10 ms).
