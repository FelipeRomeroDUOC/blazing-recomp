# Propuesta: PPU nativa sin *storage buffer* en la etapa de vértices

Fecha: 2026-10-21
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La PPU nativa deja de pedir al dispositivo un *storage buffer* visible en la etapa de vértices, de
modo que la app ya no aborta con «CRASH CRÍTICO» al arrancar en un dispositivo cuyo backend solo
da *storage buffers* a la etapa de fragmento.

### 2. Contexto

- Contexto consultado: la captura del crash en el dispositivo (`wgpu error: Validation Error`,
  `Device::create_bind_group_layout, label = 'ppu-nativa-bgl-obj'`, `Binding 3 entry is invalid`,
  `Downlevel flags DownlevelFlags(VERTEX_STORAGE) are required but not supported on the device`),
  `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/platform.rs` (el orden de backends y el respaldo
  a OpenGL ES) y `wgpu 26.0.1` (`wgpu_core.rs:1087`).
- Intención: que la ruta de la PPU nativa arranque en cualquier dispositivo con WebGPU, incluidos
  los que no tienen Vulkan y acaban en el backend de OpenGL ES. El éxito es que el dispositivo
  cree el layout de enlaces de la pasada de sprites sin quejarse de banderas *downlevel*.
- Restricciones y decisiones previas (máx. 3):
  - El motor base (`gba-recomp`) y el rev fijado `de4edf59` no se tocan (origen: `AGENTS.md` §4.1) →
    implica: el arreglo vive entero en este repositorio → protegida por «El motor base y el shim no
    cambian» | testeable.
  - La imagen que produce la PPU nativa no cambia: sigue cuadrando píxel a píxel con la referencia
    (origen: `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`, estado «verificada por agente») →
    implica: el arreglo se limita a **cómo viaja la geometría** al vértice, no a lo que se dibuja →
    protegida por «La imagen de la PPU nativa no cambia» | testeable.
  - La exigencia de `DownlevelFlags::VERTEX_STORAGE` se quita en vez de exigirse (origen: respuesta
    humana, 2026-10-21: «estoy de acuerdo con quitar la exigencia») → implica: ninguna línea de
    producción vuelve a pedir esa capacidad al dispositivo → protegida por «Ningún storage buffer es
    visible en la etapa de vértices» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Exigir `VERTEX_STORAGE` como requisito del dispositivo al pedir el *device* → descartada porque
    deja la app sin GPU justo en el dispositivo que hay que arreglar.
  - Bajar la PPU nativa a la PPU por software → descartada porque esa ruta ya existe
    (`src/ppu_shim.rs`) y es la que arrastra los 89-105 ms de `present` que justifican la PPU nativa.
  - Poner la lista de sprites en un búfer uniforme → descartada porque el tamaño de la lista varía
    con el cuadro y no cabe en el límite de 64 KB de un bloque uniforme.
- Ante imprevistos, prioriza: que la app arranque sin crash sobre la fidelidad de la imagen, y anota
  cualquier desviación de la imagen como discrepancia de la propuesta.

### 3. Ejemplos

- Caso normal: dispositivo con Vulkan → antes: arranca y dibuja → ahora: arranca y dibuja igual.
- Caso límite: dispositivo sin Vulkan, con OpenGL ES que no da `VERTEX_STORAGE` → antes: `CRASH
  CRÍTICO (PANIC): wgpu error: Validation Error ... Binding 3 entry is invalid` al crear
  `ppu-nativa-bgl-obj` → ahora: el layout se crea y la pasada de sprites se dibuja.

### 4. Pasos

1. `vs_obj` deja de leer la lista de sprites y recibe la geometría y la clave de prioridad como
   atributos de instancia (`@location(1)` y `@location(2)`), que son WebGPU básico.
2. En Rust, `entradas_bgl_obj` declara `BINDING_SPRITES` solo para `FRAGMENT`, y aparecen dos
   búferes de instancias (`VERTEX | COPY_DST`) con su `VertexBufferLayout` en paso de instancia.
3. El fragmento sigue leyendo la lista completa de sprites, que es donde hacen falta la paleta, la
   mezcla y los parámetros de la transformación afín.
4. Los tests de regresión comprueban, por análisis estructural del WGSL con `naga`, que la exigencia
   ha desaparecido y que la geometría llega por el camino nuevo.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs`: `entradas_bgl_obj`, `entradas_bgl_obj_ventana`, `PpuNativaPipeline` (dos
  búferes y dos *staging* nuevos), `layout_instancia()` y las dos pasadas de sprites del
  `render_frame`.
- `src/ppu_nativa.wgsl`: `vs_obj` y la visibilidad de `sprites`.
- `src/ppu_nativa_tests.rs` y `tests/ppu_nativa_tests.rs`: los tres tests de regresión.
- `scripts/diagnostico/mutar_ppu_nativa.py`: los mutantes que reintroducen el error antiguo.
- `src/platform.rs`: no se toca; el respaldo de backend (Vulkan → OpenGL ES) ya existía.

### 6. Suposiciones

- El dispositivo que falló tiene OpenGL ES y no Vulkan, y el respaldo de `src/platform.rs` lo está
  usando (el dispositivo se creó: el error es de validación del layout, no de adaptador) → prototipar:
  no → si es falsa: el arreglo sigue siendo válido, pero habría que revisar el orden de backends →
  resultado del prototipo: no detectado: el registro del crash no dice qué backend se usó.
- El fragmento puede seguir leyendo la lista de sprites desde un *storage buffer* con visibilidad
  `FRAGMENT` en ese mismo dispositivo → prototipar: no → si es falsa: habría que llevar también la
  paleta y la mezcla al vértice → resultado del prototipo: no detectado: requiere el dispositivo.
- La imagen no depende de que el vértice lea los sprites: el vértice solo coloca el quad, y el
  fragmento decide el color y descarta lo transparente → prototipar: sí → si es falsa: el sprite
  saldría como un quad opaco → resultado del prototipo: el arnés de comparación sigue dando 0
  diferencias de 38400 contra la referencia (los dos cuadros de aceptación).

### 7. Alcance

- Incluye: la visibilidad de `BINDING_SPRITES`, el `vs_obj` del shader, los búferes de instancias y
  sus layouts, los tres tests de regresión y los mutantes correspondientes.
- No incluye: la aritmética de la unidad de mezcla, la decodificación de OAM, la composición por
  píxel, el orden de backends de `src/platform.rs` y el borrado del shim (paso 8 de la fase 1).
- No incluye: medir el coste en el dispositivo ni encender el interruptor de producción.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Ningún storage buffer es visible en la etapa de vértices
Caso de uso: el dispositivo del usuario arranca la PPU nativa | Tipo: regresión
  Dado que el módulo declara sus enlaces de forma explícita y el shader usa `sprites` en el fragmento
  Cuando se analizan las entradas del layout de enlaces de la pasada de sprites y la visibilidad que
  el shader le da a cada variable global
  Entonces ninguna entrada de tipo *storage buffer* declara `VERTEX` en su visibilidad, y el
  `BINDING_SPRITES` solo declara `FRAGMENT`
Test: `cargo test --release --lib ppu_nativa::tests::ningun_storage_buffer_es_visible_en_la_etapa_de_vertices`
Estado previo: falla (el layout declaraba `VERTEX | FRAGMENT` para `BINDING_SPRITES`)

Escenario: Ningún punto de entrada de vértices lee la lista de sprites
Caso de uso: el dispositivo del usuario ve los sprites en su sitio | Tipo: regresión
  Dado que el shader expone `vs_quad` y `vs_obj` como puntos de entrada de vértices
  Cuando se analiza el WGSL con `naga` y se comprueba qué variables globales lee cada punto de entrada
  Entonces ningún punto de entrada de vértices lee la variable `sprites`, y `vs_obj` recibe la
  geometría por atributos de instancia
Test: `cargo test --release --test ppu_nativa_tests ningun_punto_de_entrada_de_vertices_lee_la_lista_de_sprites`
Estado previo: falla (el `vs_obj` leía `sprites`)

Escenario: El búfer de instancias lleva los campos que el vértice necesita
Caso de uso: cada sprite se dibuja en su posición y con su prioridad | Tipo: regresión
  Dado que el vértice necesita la esquina de la caja, su tamaño y la clave de prioridad
  Cuando se compara la estructura de la instancia de Rust con los atributos que declara el shader y
  con los campos que el fragmento sigue leyendo de la lista de sprites
  Entonces la instancia lleva la geometría (4 `f32`) y la clave (2 `u32`), el *layout* declara esos
  campos con el formato y el desplazamiento correctos, y ningún campo de la lista de sprites queda
  fuera del búfer de instancias
Test: `cargo test --release --lib ppu_nativa::tests::el_buffer_de_instancias_lleva_lo_que_el_vertices_necesita`
Estado previo: falla (el búfer de instancias no existía)

Escenario: La imagen de la PPU nativa no cambia
Caso de uso: el juego se ve igual que antes del arreglo | Tipo: regresión
  Dado que la fase 1 reproduce la imagen de la PPU de referencia píxel a píxel
  Cuando el arnés de comparación vuelve a renderizar los dos cuadros de aceptación
  Entonces las diferencias contra la referencia de fin de cuadro siguen siendo 0 de 38400 en los
  cuadros 5300 y 7500
Test: `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Ningún storage buffer es visible en la etapa de vértices → herramienta o comando:
  `cargo test --release --lib ppu_nativa::tests::ningun_storage_buffer_es_visible_en_la_etapa_de_vertices`
  → pasa si: termina con «test result: ok» → no detecta: no prueba que el dispositivo acepte el
  layout; eso solo se ve en el dispositivo.
- Ningún punto de entrada de vértices lee la lista de sprites → herramienta o comando:
  `cargo test --release --test ppu_nativa_tests ningun_punto_de_entrada_de_vertices_lee_la_lista_de_sprites`
  → pasa si: termina con «test result: ok» → no detecta: no comprueba el resultado en pantalla.
- El búfer de instancias lleva los campos que el vértice necesita → herramienta o comando:
  `cargo test --release --lib ppu_nativa::tests::el_buffer_de_instancias_lleva_lo_que_el_vertices_necesita`
  → pasa si: termina con «test result: ok» → no detecta: no mide el coste de subir el búfer.
- La imagen de la PPU nativa no cambia → herramienta o comando:
  `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide`
  → pasa si: 0 diferencias de 38400 → no detecta: los dos cuadros no usan fondos en modo bitmap ni
  fondos afines, que §7 de la fase 1 saca del alcance.
- La suite entera sigue verde → herramienta o comando: `cargo test --release` → pasa si: 261 pruebas
  en verde y 0 rojas → no detecta: los tests de la PPU nativa no necesitan GPU.
- Los tests detectan una vuelta atrás → herramienta o comando:
  `python scripts/diagnostico/mutar_ppu_nativa.py` → pasa si: los 8 mutantes mueren y los ficheros
  quedan restaurados → no detecta: solo prueba los mutantes escritos, no cualquier error posible.
- La app arranca en el dispositivo → revisión humana: que la app abra y no muestre la pantalla de
  «CRASH CRÍTICO» → no detecta: no se puede automatizar aquí.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- La primera versión del test de la etapa de vértices solo miraba el texto del WGSL; se sustituyó por
  un análisis con `naga` (`naga::Binding::Location` da la visibilidad real que el shader le da a
  cada variable global), que no se puede engañar con un comentario o un reformateo.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Ningún storage buffer es visible en la etapa de vértices: coincide
  Resultado real: el test pasa; `entradas_bgl_obj` declara `BINDING_SPRITES` con
  `wgpu::ShaderStages::FRAGMENT` y ninguna entrada de storage declara `VERTEX`.
  Método: el anunciado.
- Ningún punto de entrada de vértices lee la lista de sprites: coincide
  Resultado real: el test pasa; `vs_obj` lee `inst_geometria` (`@location(1)`) e `inst_clave`
  (`@location(2)`) y no toca `sprites`.
  Método: el anunciado.
- El búfer de instancias lleva los campos que el vértice necesita: coincide
  Resultado real: el test pasa; `SpriteInstanciaGpu` lleva `[f32; 4]` de geometría y `[u32; 2]` de
  clave, y `layout_instancia()` los declara con `Float32x4` en el desplazamiento 0 y `Uint32x2` en
  el 16, en modo instancia.
  Método: el anunciado.
- La imagen de la PPU nativa no cambia: coincide
  Resultado real: el arnés vuelve a dar 0 diferencias de 38400 en los dos cuadros de aceptación
  (5300 y 7500) contra la referencia de fin de cuadro.
  Método: el anunciado.
- La suite entera sigue verde: coincide
  Resultado real: `cargo test --release` → 261 pruebas en verde, 0 rojas. Los dos avisos que
  aparecen (`FALLBACK` sin usar en `gba-core` y `data_offset` sin leer en `packaging_tests`) son
  previos a este cambio.
  Método: el anunciado.
- Los tests detectan una vuelta atrás: coincide
  Resultado real: `python scripts/diagnostico/mutar_ppu_nativa.py` mata los 8 mutantes: tres de la
  aritmética de la mezcla (redondeo a media, carril de verde de 6 bits, saturación de aclarar) y
  cinco que reintroducen el error antiguo (storage visible en vértices, `vs_obj` leyendo `sprites`,
  atributos de instancia movidos de ubicación, clave fuera del búfer de instancias, zancada del
  búfer de instancias cambiada). El script restaura `src/ppu_nativa.rs` y `src/ppu_nativa.wgsl` en
  memoria y sobre el disco en un bloque `finally` y verifica después que los tests vuelven a pasar.
  Método: el anunciado.
- La app arranca en el dispositivo: no pudo comprobarse
  Resultado real: no hay dispositivo en este entorno. La causa que produce el crash sí está
  identificada y comprobada en el texto: `Binding 3 entry is invalid` es el `BINDING_SPRITES` con
  visibilidad `VERTEX`, y `DownlevelFlags(VERTEX_STORAGE) are required but not supported on the
  device` es exactamente lo que dice el registro.
  Método: distinto: lo anunciado no se puede correr sin el dispositivo; el origen del crash se
  verificó contra el mensaje de `wgpu` y contra el esquema de los layouts de Rust.

### Contexto: ¿se respetó?

- El motor base y el rev fijado no se tocan → respetada. `src/ppu_nativa.rs` no importa nada de
  `gba-recomp` ni de `gba-core`; el test «El motor base y el shim no cambian» sigue en verde.
- La imagen de la PPU nativa no cambia → respetada: 0 diferencias de 38400 en los dos cuadros.
- Se quita la exigencia de `VERTEX_STORAGE` en vez de exigirse → respetada: no queda ninguna línea
  que pida esa capacidad, y tres tests lo comprueban estructuralmente.
- ¿Se tocó algo que alguna restricción prohibía? no: el cambio se limita al shader, al pipeline de
  la PPU nativa, sus tests y el script de mutación.
- Imprevistos que surgieron: `VertexBufferLayout` es `Clone` pero no `Copy` en `wgpu 26.0.1`, así que
  el layout de instancias se construye dos veces (una por pipeline) en vez de clonarse → criterio de
  prioridad aplicado: sí.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/ppu_nativa.wgsl`: `vs_obj` recibe la geometría y la clave por atributos de instancia; la
  lista de sprites queda documentada como de solo fragmento.
- `src/ppu_nativa.rs`: `SpriteInstanciaGpu` (nuevo), `entradas_bgl_obj` y `entradas_bgl_obj_ventana`
  con `BINDING_SPRITES` en `FRAGMENT`, `layout_instancia()` (nuevo), dos búferes de instancias y
  sus *staging*, y las dos pasadas de sprites que los suben y los enlazan.
- `src/ppu_nativa_tests.rs`: dos tests de regresión (etapa de vértices y búfer de instancias).
- `tests/ppu_nativa_tests.rs`: un test de regresión (ningún punto de entrada de vértices lee la lista
  de sprites), con análisis estructural del WGSL por `naga`.
- `scripts/diagnostico/mutar_ppu_nativa.py`: los cinco mutantes que reintroducen el error antiguo.

¿Coincide con lo anunciado? sí.

### Discrepancias

- **La causa que anunció el usuario y la que dice el registro no son la misma.** El usuario atribuyó
  el fallo a la falta de Vulkan; el registro de `wgpu` dice que el dispositivo se creó (si no, el
  fallo habría sido al pedir el adaptador) y que lo que faltaba era la capacidad `VERTEX_STORAGE`,
  que el estándar de OpenGL ES permite no tener. El arreglo es el mismo en los dos casos —dejar de
  exigir esa capacidad—, pero conviene que quede escrito el motivo real: la ausencia de Vulkan solo
  forzó el backend de OpenGL ES, y el OpenGL ES es el que no da *storage buffers* al vértice.
  → clasificación: información faltante.

### Para la próxima vez

Cuando el registro de `wgpu` nombre una bandera *downlevel*, traducirla a su requisito del
dispositivo y quitar la exigencia en el layout, no pedir más capacidad al dispositivo.

### Fuera de alcance (hallazgos que no se arreglaron)

- La clave de profundidad que desempata los sprites es `prioridad * 128 + índice de OAM`, y cabe en
  el rango `[0, 1]` del adjunto de profundidad solo porque `MAX_SPRITES` es 128 y la prioridad
  máxima es 3 (`3 * 128 + 127 = 511 < 512`): si el máximo de sprites creciera, la clave se saldría
  del rango y el desempate dejaría de ser correcto. | impacto: bajo
- Los búferes de sprites reservan 128 entradas (8 KB el *storage buffer* y 3 KB el de instancias)
  aunque en un cuadro normal haya menos de veinte sprites: el tamaño reservado no se ajusta al
  cuadro y la memoria de vídeo no se devuelve. | impacto: bajo
- La geometría de cada sprite está duplicada: la misma información viaja en el *storage buffer* (que
  lee el fragmento) y en el búfer de instancias (que lee el vértice), en dos formatos distintos. |
  impacto: bajo

---

## Revisión humana

- Que la app abra en el dispositivo sin la pantalla de «CRASH CRÍTICO» y que los sprites aparezcan en
  su sitio (no testeable en este entorno): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
