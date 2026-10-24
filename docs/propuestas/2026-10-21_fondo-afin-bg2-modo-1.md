# Propuesta: El fondo afín BG2 del modo 1 aparece en la PPU nativa

Fecha: 2026-10-21
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En las pantallas del juego con el fondo girando y deformándose por debajo del texto (los menús y
las transiciones), la imagen deja de estar completa: ese fondo no se dibuja y se ve en su lugar el
color liso del fondo de la pantalla. Con este cambio, el fondo deformable se dibuja igual que lo
dibuja el hardware, y la imagen de esas pantallas deja de diferir de la del emulador.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs` (`CapaFondoGpu`, `CapaFondoGpu::de_io`,
  `EstadoPpuGpu::decode_io_regs`, `config_capa`, `config_comp`, `render_frame`),
  `src/ppu_nativa.wgsl` (`struct CapaFondo`, `fn color_capa`, `fn fs_capa`, `fn fs_comp`,
  `fn vram_byte`), `src/ppu_nativa_tests.rs` (suite satélite), `tests/ppu_nativa_tests.rs`
  (paridad de offsets con el shader, criterios de la fase 1, informe de 5300 y de 7500),
  `examples/ppu_nativa_compara.rs` (arnés de comparación), `examples/_tmp_afin_bg2.rs` y
  `examples/_tmp_afin_regs.rs` (diagnósticos temporales de este mismo cuadro),
  `../gba-recomp/crates/gba-core/src/ppu.rs` (`render_scanline`, `render_affine_bg`, `compose`,
  `sign_extend28`), `../gba-recomp/crates/gba-core/src/mem.rs` (`MemMap::aff_ref`, `aff_dirty`),
  `volcados/gba-recomp-limpio/crates/gba-core/src/ppu.rs` (rev fijado `de4edf5`),
  `volcados/ppu-nativa-900.txt` (informe del cuadro 900), `secuencias/escena3.txt`,
  `tests/datos/ppu-nativa-hashes-base.txt`, `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`,
  `docs/BACKLOG.md`, `AGENTS.md`.
- Intención: la PPU nativa (la ruta de dibujo por defecto del port) se valida píxel a píxel contra
  la PPU por software de `gba-core`. En el cuadro 900 de `escena3` esa comparación da **5222 píxeles
  distintos de 38400**, y el diagnóstico muestra que los 5222 son exactamente el fondo afín BG2 que
  la nativa no dibuja. El éxito es que ese cuadro dé **0 diferencias** y que el camino afín
  (incluido el wrap y los mapas que no salen en ese cuadro) quede covered por pruebas.
- Restricciones y decisiones previas (máx. 3):
  - La aritmética del fondo afín es la de `gba-core`, no una interpretación propia (origen:
    `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`, estado «verificada por agente») → implica:
    punto fijo 20.8, ancla por línea `ref + línea·PB/PD`, paso por píxel `PA`/`PC`, wrap con
    `rem_euclid` del tamaño del mapa, transparente fuera del mapa, entrada de mapa de 1 byte,
    índice 0 transparente y paleta de fondo → protegida por «El cuadro 900 de escena3 con fondo afín
    coincide con la referencia de fin de cuadro» y «El fondo afín fuera del mapa y con wrap se
    comporta como la referencia» | testeable.
  - La PPU nativa compone con el estado de fin de cuadro, no con el estado por línea (origen:
    `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md` y `docs/BACKLOG.md`, registro de hallazgos de
    la fase 1) → implica: el ancla de cada línea se deriva de los registros de fin de cuadro
    (`BG2X`, `BG2Y`, `BG2PB`, `BG2PD`); no se reproduce un reenganche del ancla si el juego escribe
    `BG2X`/`BG2Y` a mitad de cuadro → protegida por «El cuadro 900 de escena3 con fondo afín coincide
    con la referencia de fin de cuadro» | testeable.
  - El motor base y el shim están congelados por hash durante la fase 1 (origen:
    `tests/datos/ppu-nativa-hashes-base.txt` y `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`) →
    implica: no se toca `src/ppu_shim.rs`, `src/ppu_shim.wgsl` ni `crates/gba-core` → protegida por
    «Los cuadros de aceptación de la fase 1 siguen coincidiendo y el motor y el shim no cambian» |
    testeable.
- Alternativas descartadas (solo si las hubo):
  - Calcular el fondo afín en Rust y subir la imagen ya compuesta a la GPU → descartada porque la
    PPU nativa compone en una sola pasada de GPU; hacerlo en CPU cambiaría la arquitectura de la
    ruta por defecto y el coste por cuadro, y no hace falta: el muestreo afín es de una operación
    por píxel.
  - Añadir el fondo afín al *shim* en lugar de a la PPU nativa → descartada porque el shim está
    congelado por hash en esta fase y porque la ruta por defecto del port es la PPU nativa.
  - Corregir la diferencia ajustando el arnés o el criterio (por ejemplo, aceptando 5222
    diferencias) → descartada porque el criterio de la fase 1 es 0 diferencias contra la referencia
    de fin de cuadro; el afín es una carencia de la nativa, no un artefacto del arnés.
- Ante imprevistos, prioriza: que el cuadro 900 dé 0 diferencias contra la referencia de fin de
  cuadro > que los casos límite que no salen en ese cuadro (wrap, mapas de 64 y 128 tiles) queden
  cubiertos por el arnés sintético.

### 3. Ejemplos

- Caso normal: el cuadro 900 de `secuencias/escena3.txt` (`DISPCNT = 0x1F01`: modo 1 con los
  fondos de texto BG0 y BG1 y el fondo afín BG2 activados por el bit 10; `BG2CNT = 0x4F0A`:
  prioridad 2, base de caracteres `0x8000`, base de mapa `0x7800`, sin wrap, mapa de 32 tiles
  (256 px); `BG2PA = 116`, `BG2PB = 246`, `BG2PC = -123`, `BG2PD = 232`, `BG2X = 0xFFFF7BE0`,
  `BG2Y = 0xFFFFF4A8`) → antes: **5222 de 38400** píxeles distintos de la referencia de fin de
  cuadro (la diferencia empieza en la línea 110; la nativa coincide **exactamente**, 0 de 38400,
  con la misma referencia a la que se le apaga el bit 10 de `DISPCNT`) → ahora: **0 de 38400** y el
  informe marca «criterio de la fase 1 … CUMPLIDO».
- Caso límite: un mapa afín de 32 tiles (256 px) con el wrap apagado y el punto de referencia fuera
  del mapa → antes: los píxeles afectados salen del color liso del fondo de pantalla (BG2 no se
  dibuja) → ahora: la capa BG2 es **transparente** en esos píxeles y se ve la capa de debajo. Con
  el wrap activado y el punto de referencia **negativo** → las coordenadas dan la vuelta al tamaño
  del mapa (módulo 256 px) y el píxel toma el color del tile del mapa al que corresponde.

### 4. Pasos

1. `src/ppu_nativa.rs`: `CapaFondoGpu` gana ocho campos de 32 bits (`es_afin`, `afin_wrap`,
   `afin_pa`, `afin_pb`, `afin_pc`, `afin_pd`, `afin_ref_x`, `afin_ref_y`) y una constructora nueva
   `de_io_afin(io, bg, enabled)` que los decodifica: `PA`/`PB`/`PC`/`PD` como enteros de 16 bits
   con signo (0x20-0x26 para BG2, 0x30-0x36 para BG3) y `BG2X`/`BG2Y` con los 28 bits con signo
   (`sign_extend28`, igual que la referencia); el wrap es el bit 13 de `BGxCNT` y el tamaño del
   mapa sigue siendo el campo `screen_size` (bits 14-15), que en el afín significa 16, 32, 64 o
   128 tiles.
2. `src/ppu_nativa.rs`: `decode_io_regs` admite BG2 en modo 1 (hoy solo admite los fondos de texto:
   `modo 1 => bg < 2`) cuando `DISPCNT` bit 10 está activo, y le da su prioridad (`BG2CNT` bits
   0-1) para que la composición lo tenga en cuenta; `bg_prio[2]` pasa de `0xFF` a ese valor.
3. `src/ppu_nativa.wgsl`: `struct CapaFondo` replica los ocho campos nuevos con los mismos nombres
   y los mismos offsets que en Rust (el test de paridad de offsets lo exige).
4. `src/ppu_nativa.wgsl`: `color_capa` deriva a la nueva `color_capa_afin` cuando `es_afin != 0`.
5. `src/ppu_nativa.wgsl`: `color_capa_afin` implementa el muestreo de la referencia: ancla de la
   línea (`ref_x + línea·PB`, `ref_y + línea·PD`), paso por píxel (`PA` en horizontal, `PC` en
   vertical), desplazamiento de 8 bits con signo, wrap con `rem_euclid` del tamaño del mapa,
   transparente fuera del mapa cuando no hay wrap, entrada de mapa de 1 byte en
   `screen_base + (ty/8)·size_tiles + tx/8`, byte de píxel en `char_base + tile·64 + (ty%8)·8 +
   tx%8` (fuera de los 64 KB de caracteres, transparente), índice 0 transparente y el resto, el
   color de la paleta de fondo.
6. `tests/ppu_nativa_tests.rs`: el test de paridad de offsets incluye los ocho campos nuevos, y se
   añade el test que lee `volcados/ppu-nativa-900.txt` y exige 0 diferencias.
7. `src/ppu_nativa_tests.rs`: test unitario de la decodificación con signos (sin ROM y sin GPU).
8. `examples/ppu_nativa_afin.rs` (nuevo) y `Cargo.toml`: arnés sintético que monta un estado de
   I/O y de VRAM con el fondo afín y lo compara con `gba_core::ppu::render_scanline` sobre el mismo
   estado (casos: mapa visible, wrap apagado con la referencia fuera del mapa, wrap activado con la
   referencia negativa, mapas de 16/32/64/128 tiles); escribe `volcados/ppu-nativa-afin.txt`.
9. `scripts/diagnostico/mutar_ppu_nativa.py`: mutaciones del camino afín (el desplazamiento de 8
   bits, el intercambio de `PB` y `PD`, y la extensión de signo del ancla) que el test del cuadro
   900 tiene que detectar.
10. Documentación: `docs/referencia/api-modulos-rust.md` (los campos nuevos de `CapaFondoGpu`) y
    `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md` (la PPU nativa ya cubre el fondo
    afín del modo 1).

### 5. Qué más podría verse afectado

- `tests/ppu_nativa_tests.rs::los_desplazamientos_de_las_estructuras_coinciden`: compara los offsets
  de `CapaFondoGpu` con los miembros de `struct CapaFondo` del shader; con los campos nuevos hay que
  añadir las ocho entradas.
- `tests/ppu_nativa_tests.rs::los_buffers_de_rust_caben_lo_que_el_shader_lee`: `ConfigCapaGpu` crece
  de 144 a 272 bytes; el búfer uniforme se dimensiona con `size_of`, así que no hay que tocarlo, pero
  el test lo comprueba.
- `src/ppu_nativa.wgsl::fs_comp`: la composición ya es genérica sobre las cuatro capas y sus
  prioridades, pero hasta ahora `bg_prio2` valía `0xFF`; con BG2 activo pasa a valer la prioridad de
  `BG2CNT` y el bit 2 de la máscara de control (habilitado por defecto con `0x3F`) decide si se ve.
- `src/ppu_nativa.rs::EstadoPpuGpu::decode_io_regs`: la condición de modo pasa a distinguir la capa
  afín de las de texto en el modo 1.
- `docs/referencia/api-modulos-rust.md` y `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`.
- Búsqueda hecha: `grep -rn "CapaFondoGpu\|struct CapaFondo\|screen_size\|bg_prio\|de_io(" src/ tests/ examples/`.

### 6. Suposiciones

- Las 5222 diferencias del cuadro 900 son exactamente el fondo afín BG2 y nada más → prototipar: sí
  → si es falso: habría que buscar otra causa de las diferencias y el alcance de la propuesta
  cambiaría.
  → resultado del prototipo: **confirmado**. `examples/_tmp_afin_bg2.rs` da 0 de 38400 entre la
  nativa y la referencia a la que se le apaga el bit 10 de `DISPCNT`, y 5222 entre la referencia con
  BG2 y la misma referencia sin BG2; la primera diferencia está en la línea 110.
- El ancla por línea del afín se puede derivar del estado de fin de cuadro (`ref + línea·PB/PD`) →
  prototipar: sí → si es falso: habría que llevar el estado por línea a la GPU, que es justo lo que
  la fase 1 dejó fuera de alcance.
  → resultado del prototipo: **confirmado** leyendo `render_scanline` de `gba-core`: el acumulador
  se engancha en la línea 0 con `sign_extend28(BG2X/BG2Y)` y se avanza con `PB`/`PD` al final de
  cada línea; la referencia de fin de cuadro hace exactamente eso porque durante la reconstrucción
  no hay escrituras de I/O.
- El mapa afín cabe en los 96 KB (0x18000) de VRAM que ya tiene el búfer de la PPU nativa →
  prototipar: no → el peor caso (base 0xF800 con 128 tiles) llega a 0x137FF, dentro del rango, y la
  referencia lee ese mismo rango; no cambia el plan.

### 7. Alcance

- Incluye:
  - El fondo afín BG2 del modo 1: `BG2CNT` (base de caracteres, base de mapa, wrap, tamaño),
    `BG2PA`/`BG2PB`/`BG2PC`/`BG2PD` y `BG2X`/`BG2Y`, con wrap y con los cuatro tamaños de mapa.
  - La decodificación de esos registros en la PPU nativa y su paso al shader.
  - El arnés sintético de los casos límite, sus tests y la documentación afectada.
- No incluye:
  - El modo 2 (BG2 y BG3 afines) ni los modos 3, 4 y 5 (bitmap): la PPU nativa sigue sin dibujarlos.
  - El reenganche del ancla cuando el juego escribe `BG2X`/`BG2Y` a mitad de cuadro: la PPU nativa
    compone con el estado de fin de cuadro, como ya está documentado para el resto.
  - Los sprites afines (ya están implementados) y el *shim* (congelado por hash en esta fase).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El cuadro 900 de escena3 con fondo afín coincide con la referencia de fin de cuadro
Caso de uso: el jugador entra en un menú y ve el fondo deformado por debajo del texto | Tipo: normal
  Dado que la ROM oficial, la secuencia `secuencias/escena3.txt` y el cuadro 900 alcanzado, en el
  que el juego está en modo 1 con `DISPCNT = 0x1F01` y el fondo afín BG2 activados
  Cuando la PPU nativa compone ese cuadro y `examples/ppu_nativa_compara.rs` lo compara píxel a
  píxel con la referencia de `gba-core` alimentada con el mismo estado de fin de cuadro
  Entonces el informe `volcados/ppu-nativa-900.txt` dice «diferencias frente a la referencia de
  estado de fin de cuadro: 0 de 38400» y «criterio de la fase 1 (0 diferencias en las lineas sin
  afines): CUMPLIDO» (hoy dice 5222 de 38400 y «NO CUMPLIDO»)
Test: `tests/ppu_nativa_tests.rs::test_cuadro_900_escena3_fondo_afin_coincide`
Estado previo: falla

Escenario: El fondo afín fuera del mapa y con wrap se comporta como la referencia
Caso de uso: una escena con un fondo afín cuyo punto de referencia está fuera del mapa, o con el
  wrap activado y un punto de referencia negativo | Tipo: límite
  Dado que un estado sintético con `DISPCNT` en modo 1 y `BG2CNT` con wrap apagado y el punto de
  referencia fuera del mapa, y el mismo estado con el wrap activado y el punto de referencia
  negativo
  Cuando la PPU nativa compone ese estado y el arnés `examples/ppu_nativa_afin.rs` lo compara con
  `gba_core::ppu::render_scanline` sobre el mismo estado, en los cuatro tamaños de mapa
  Entonces el informe `volcados/ppu-nativa-afin.txt` da 0 diferencias de 38400 en los cuatro
  casos, y el caso del wrap muestra el mismo píxel que la referencia (la vuelta al mapa, no el
  recorte)
Test: `tests/ppu_nativa_tests.rs::test_fondo_afin_casos_limite_coinciden`
Estado previo: falla (el arnés no existe y la PPU nativa no dibuja BG2: hoy el caso del mapa
  visible difiere en todos sus píxeles)

Escenario: Los cuadros de aceptación de la fase 1 siguen coincidiendo y el motor y el shim no
  cambian
Caso de uso: el jugador sigue viendo los menús y las pantallas de la introducción igual que antes
  | Tipo: regresión
  Dado que los cuadros 5300 y 7500 de la secuencia de introducción, que no tienen fondo afín
  Cuando se compone el cambio y se vuelven a generar los dos informes
  Entonces los dos informes siguen dando 0 diferencias frente a la referencia de fin de cuadro, y
  los hashes de `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `src/ppu_shim_tests.rs` y de los dos
  ficheros del motor base siguen siendo los de `tests/datos/ppu-nativa-hashes-base.txt`
Test: `tests/ppu_nativa_tests.rs::test_lineas_sin_afines_5300_coinciden`,
  `tests/ppu_nativa_tests.rs::test_cuadro_7500_sin_efectos_coincide` y
  `tests/ppu_nativa_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: pasa

Escenario: Los registros del fondo afín se decodifican con su signo
Caso de uso: una escena con un fondo afín desplazado o escalado en negativo | Tipo: error
  Dado que `BG2PA = 0xFF9C` (−100), `BG2PB = 0x0001`, `BG2PC = 0xFF85` (−123),
  `BG2PD = 0x00E8`, `BG2X = 0x0FFFFFFF` (28 bits: −1), `BG2Y = 0x10000000` (el bit 28 está fuera de
  los 28 bits del registro) y un `BG2CNT` con el wrap activo y los bits 14-15 = 3 (128 tiles)
  Cuando se decodifica la capa BG2 de un `DISPCNT` de modo 1 con el bit 10 activo
  Entonces `afin_pa` vale −100, `afin_pc` vale −123, `afin_ref_x` vale −1, `afin_ref_y` vale 0 (el
  bit 28 se descarta), `afin_wrap` vale 1 y `screen_size` vale 3 (128 tiles)
Test: `src/ppu_nativa_tests.rs::los_registros_del_fondo_afin_llevan_el_signo`
  (se ejecuta como `ppu_nativa::tests::los_registros_del_fondo_afin_llevan_el_signo`)
Estado previo: falla (los campos `afin_*` todavía no existen: el test no compila)

### 9. Cómo lo comprobaré

- El cuadro 900 de escena3 con fondo afín coincide con la referencia de fin de cuadro →
  herramienta o comando: `cargo run --release --features proto --example ppu_nativa_compara -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/escena3.txt 900`
  y después `cargo test --release --test ppu_nativa_tests -- test_cuadro_900_escena3_fondo_afin_coincide --exact`
  → pasa si: el informe dice 0 de 38400 y «CUMPLIDO», y el test pasa → no detecta: si el cuadro 900
  dejara de tener fondo afín visible, el criterio se cumpliría sin haber dibujado nada; por eso el
  escenario del wrap y del mapa cubre ese hueco.
- El fondo afín fuera del mapa y con wrap se comporta como la referencia → herramienta o comando:
  `cargo run --release --features proto --example ppu_nativa_afin -- "roms/1575 - Fire Emblem (E)(Independent).gba"`
  y después `cargo test --release --test ppu_nativa_tests -- test_fondo_afin_casos_limite_coinciden --exact`
  → pasa si: el informe del arnés da 0 diferencias en los cuatro casos → no detecta: necesita GPU;
  si la máquina no tiene adaptador gráfico, el arnés no corre y el test se omite con un mensaje
  (igual que los que hoy necesitan `FE_ROM`).
- Los cuadros de aceptación de la fase 1 siguen coincidiendo y el motor y el shim no cambian →
  herramienta o comando: `cargo test --release --test ppu_nativa_tests`
  → pasa si: los tres tests citados pasan → no detecta: no cubre cuadros con un fondo afín distinto
  del del cuadro 900.
- Los registros del fondo afín se decodifican con su signo → herramienta o comando:
  `cargo test --release --lib -- ppu_nativa::tests::los_registros_del_fondo_afin_llevan_el_signo --exact`
  → pasa si: los seis valores decodificados son los esperados → no detecta: no comprueba el
  muestreo en pantalla (eso lo comprueba el arnés sintético).
- Robustez de los cuatro criterios → herramienta o comando:
  `python scripts/diagnostico/mutar_ppu_nativa.py` (con las tres mutaciones del camino afín añadidas)
  → pasa si: cada mutación hace fallar el test del cuadro 900 y los ficheros quedan restaurados →
  no detecta: no muta la aritmética de la composición, que ya está cubierta por otros tests.
- La suite entera y la documentación → herramienta o comando: `cargo test`,
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y
  `python scripts/extraer_referencias.py --check`
  → pasa si: los tres salen con código 0 → no detecta: el validador no revisa `docs/propuestas/**`.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación: el cuadro 900 da 0 diferencias, los casos límite del
arnés sintético dan 0 diferencias, los tres tests de la fase 1 siguen en verde y la suite completa
pasa.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- El primer borrador daba el tamaño del mapa afín como 32/64/128/256 tiles: es 16/32/64/128 tiles
  (16 << bits 14-15), y el tamaño en píxeles es el doble en cada eje (`size_tiles * 8`).
- El primer borrador comparaba el cuadro 900 con la referencia por línea: la comparación correcta es
  con la referencia de **fin de cuadro**, que es la que la PPU nativa puede reproducir; la referencia
  por línea sigue fuera de alcance porque el juego escribe `BG2X`/`BG2Y` a mitad de cuadro.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El cuadro 900 de escena3 con fondo afín coincide con la referencia de fin de cuadro** → coincide
  Resultado real: `volcados/ppu-nativa-900.txt` dice «diferencias frente a la referencia de estado
  de fin de cuadro: 0 de 38400» y «criterio de la fase 1 (0 diferencias en las lineas sin afines):
  CUMPLIDO» (antes: 5222 de 38400 y «NO CUMPLIDO»). El test
  `tests/ppu_nativa_tests.rs::test_cuadro_900_escena3_fondo_afin_coincide` pasa.
  Método: el anunciado.
- **El fondo afín fuera del mapa y con wrap se comporta como la referencia** → coincide
  Resultado real: `volcados/ppu-nativa-afin.txt` da 0 diferencias de 38400 en los siete casos (mapa
  visible; wrap apagado con la referencia fuera del mapa; wrap activado con la referencia negativa;
  y los mapas de 16, 32, 64 y 128 tiles): «total: 0 diferencias de 268800». El test
  `tests/ppu_nativa_tests.rs::test_fondo_afin_casos_limite_coinciden` pasa.
  Método: el anunciado.
- **Los cuadros de aceptación de la fase 1 siguen coincidiendo y el motor y el shim no cambian** →
  coincide
  Resultado real: `test_lineas_sin_afines_5300_coinciden`, `test_cuadro_7500_sin_efectos_coincide` y
  `test_el_motor_base_y_el_shim_no_cambian` pasan; los hashes de `src/ppu_shim.rs`,
  `src/ppu_shim.wgsl`, `src/ppu_shim_tests.rs` y de los dos ficheros del motor base siguen siendo
  los de `tests/datos/ppu-nativa-hashes-base.txt`.
  Método: el anunciado.
- **Los registros del fondo afín se decodifican con su signo** → coincide
  Resultado real: `ppu_nativa::tests::los_registros_del_fondo_afin_llevan_el_signo` pasa con
  `afin_pa = -100`, `afin_pc = -123`, `afin_ref_x = -1`, `afin_ref_y = 0`, `afin_wrap = 1` y
  `screen_size = 3`.
  Método: el anunciado.
- **Robustez: las tres mutaciones del camino afín las detectan los tests** → coincide
  Resultado real: `python scripts/diagnostico/mutar_ppu_nativa.py` da «OK» en las 11 mutaciones (las
  tres del afín: el desplazamiento de 8 bits, el intercambio de `PB`/`PD` y el ancla sin extensión
  de signo) y los ficheros quedan restaurados byte a byte. Las dos primeras mutaciones del afín son
  del shader, así que el script vuelve a correr el arnés sintético antes de comprobar el test: sin
  eso, el test leería el informe viejo y la mutación pasaría desapercibida.
  Método: el anunciado, con una corrección: el script de mutación escribía los ficheros mutados con
  finales CRLF en Windows, así que al restaurar el fichero quedaba con finales distintos de los
  originales (el contenido era idéntico, pero el hash del fichero sí cambiaba).

### Contexto: ¿se respetó?

- La aritmética del fondo afín es la de `gba-core` (punto fijo 20.8, ancla por línea, paso por
  píxel, wrap, entrada de mapa de 1 byte, índice 0 transparente) → respetada: el shader replica
  `render_affine_bg` y el arnés lo contrasta contra `gba_core::ppu::render_scanline` sobre el mismo
  estado.
- La PPU nativa compone con el estado de fin de cuadro → respetada: el ancla de cada línea se
  deriva de `BG2X`/`BG2Y` de fin de cuadro; no se reprodujo el reenganche del ancla.
- El motor base y el shim están congelados por hash → respetada: no se tocó `src/ppu_shim.rs`,
  `src/ppu_shim.wgsl` ni `crates/gba-core`, y el test de hashes sigue en verde.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: dos. (1) El script de mutación traducía los finales de línea al
  escribir en Windows: se corrigió para escribir sin traducir y se devolvieron `src/ppu_nativa.rs` y
  `src/ppu_nativa.wgsl` a LF. (2) La referencia técnica generada no cubre `src/ppu_nativa.rs` (solo
  una lista curada de módulos en `scripts/extraer_referencias.py`), así que los campos nuevos de
  `CapaFondoGpu` no tienen sitio ahí → criterio de prioridad aplicado: sí, la documentación de los
  registros del afín quedó en la página de explicación, que es la que el lector del port consulta.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/ppu_nativa.rs`: los ocho campos del afín en `CapaFondoGpu`, `de_io_afin`, la admisión de BG2
  en modo 1 y su prioridad en `decode_io_regs`.
- `src/ppu_nativa.wgsl`: los mismos ocho campos en `struct CapaFondo` y `color_capa_afin`.
- `src/ppu_nativa_tests.rs`: el test unitario de los signos de los registros.
- `tests/ppu_nativa_tests.rs`: la paridad de offsets con los campos nuevos, el test del cuadro 900 y
  el test de los casos límite del afín.
- `examples/ppu_nativa_afin.rs` y `Cargo.toml`: el arnés sintético y su entrada de ejemplo.
- `scripts/diagnostico/mutar_ppu_nativa.py`: las tres mutaciones del camino afín, el arnés que hay
  que correr antes de comprobarlas y la escritura sin traducir los finales de línea.
- `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`: la PPU nativa ya cubre el fondo
  afín del modo 1.
- `docs/propuestas/2026-10-21_fondo-afin-bg2-modo-1.md` (esta ficha) y `docs/BACKLOG.md` (índice y
  registro de hallazgos).
- `volcados/ppu-nativa-900.txt` y `volcados/ppu-nativa-afin.txt`: los informes regenerados.

¿Coincide con lo anunciado? no: el paso 10 blindaba `docs/referencia/api-modulos-rust.md`, pero ese
documento se genera desde los módulos **públicos** de una lista curada y `src/ppu_nativa.rs` no está
en ella; `CapaFondoGpu` es privada. La documentación de los campos nuevos quedó en la página de
explicación, y el `--check` de la referencia sigue en verde sin cambios.

### Discrepancias

- La referencia técnica generada no incluye `src/ppu_nativa.rs` → información faltante: el plan
  asumía que ese documento cubría la PPU nativa. El sitio de los campos nuevos es la página de
  explicación (`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`).
- El script de mutación dejaba los ficheros mutados con finales CRLF al restaurar → error de
  implementación: el contenido volvía a ser el original, pero los finales de línea cambiaban. Se
  corrigió la escritura del script y se devolvieron los dos ficheros a LF.

### Para la próxima vez

- Antes de prometer un cambio en `docs/referencia/api-modulos-rust.md`, comprobar si el módulo entra
  en la lista curada de `scripts/extraer_referencias.py`: si no entra, el documento no se genera y
  la nota va al cuadrante de explicación.

### Fuera de alcance (hallazgos que no se arreglaron)

- La PPU nativa compone con el estado de fin de cuadro: en el cuadro 900 el juego escribe
  `BG2X`/`BG2Y` durante el cuadro, así que frente a la referencia **por línea** quedan 13247
  diferencias de 38400 (las dos referencias difieren en eso). | impacto: medio
- El shim solo tiene capas de fondo del modo 0 (scroll, sin transformación): no dibuja el fondo
  afín BG2, así que con la PPU nativa apagada (archivo de marca `ppu-nativa-off.flag`) los menús
  con el fondo girando salen sin esa capa. | impacto: medio
- La PPU nativa sigue sin dibujar el modo 2 (BG2 y BG3 afines) ni los modos 3, 4 y 5 (bitmap): el
  juego no los usa, pero un fondo de bits saldría en negro y el modo 2 no se deformaría. | impacto:
  medio

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- El fondo afín de los menús se ve deformado y sin saltos en el dispositivo, y la imagen no
  parpadea al cambiar de escena (visible): ✔ | ✘ | ?
- Detectado solo por el humano:
