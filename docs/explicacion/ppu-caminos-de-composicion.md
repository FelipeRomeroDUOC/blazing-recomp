---
tipo: explicacion
verificado: si
actualizado: 2026-10-24
---

# Cómo decide la PPU por software qué camino dibuja cada línea

> [!NOTE] Contexto Histórico, Entorno y Alcance Arquitectónico
> Este documento es una **página de estudio histórico y análisis de ingeniería inversa** centrada en el funcionamiento interno del rasterizador por software en CPU de `gba-core` (`crates/gba-core/src/ppu.rs`) y los experimentos previos de `PpuShim`.
> 
> En la arquitectura de producción de **`blazing-recomp`**, el rasterizado por software en CPU fue retirado definitivamente en favor del pipeline de renderizado directo por GPU (**`NativePpu`** en `src/ppu_nativa.rs` vía `wgpu`), tal como documenta el **[ADR 42](DECISIONES.md#42-eliminación-definitiva-de-ppushim-software-en-favor-de-pipeline-gpu-directo-nativeppu)**. Este análisis se conserva íntegro por su alto valor didáctico para comprender la lógica de prioridad de capas, ventanas y efectos de mezcla del hardware original de la Game Boy Advance.
> 
> **Requisito del repositorio hermano para ejercicios prácticos:**  
> Las rutas de archivo (`crates/gba-core/...`) y los comandos de prueba interactivos de esta página asumen la presencia del repositorio oficial clonado adyacente en `../gba-recomp` (`git clone https://github.com/JRickey/gba-recomp.git ../gba-recomp`). Si no dispones del repositorio hermano local, este documento debe leerse como análisis teórico conceptual del hardware de video de GBA.

Página de estudio del motor hermano `gba-recomp` (crate `gba-core`, módulo `ppu.rs`): por qué hay
dos caminos para componer una línea, qué los separa, cómo se calcula el color de un píxel, y por
qué la **ventana de sprites** (la que activa el bit 15 de `DISPCNT`) tiene que mirar el camino por
píxel y no el atajo. Al final hay un recorrido con comandos, las respuestas a los ejercicios y un
glosario.

La página asume que sabes lo básico de Rust y de la GBA (registros, VRAM, OAM). Si no, el bloque 1
empieza desde cero.

> ⏱ Tiempo estimado: 75 min · Nivel: intermedio · Motor: `../gba-recomp`, `crates/gba-core/src/ppu.rs`

---

## Cómo usar esta guía

1. Lee los bloques en orden; cada uno se sostiene sobre los anteriores.
2. Cuando llegues a un **Tu turno**, para y explica la respuesta con tus palabras. Después compárala
   con la sección *Respuestas de los «Tu turno»*.
3. Puedes parar en cualquier **Pausa** y retomar después.
4. Los bloques 7 y 8 son los que justifican el resto: el 7 enseña cómo funciona el atajo por dentro
   y el 8 cuenta un defecto real que se encontró al escribir esta misma página.

## Progreso

- [ ] 1. Contexto: qué dibuja la PPU y en qué presupuesto
- [ ] 2. Contexto: la anatomía de una línea
- [ ] 3. Contexto: el control de un píxel
- [ ] 4. Contexto: los dos caminos, span y tramo
- [ ] 5. Idea central: la cadena `razon_camino`
- [ ] 6. Idea central: el efecto de color, inerte o real
- [ ] 7. Idea central: la aritmética del alpha
- [ ] 8. Alternativas: qué se descartó y por qué
- [ ] 9. Alternativas: el precio medido
- [ ] 10. El atajo por dentro: spans y clave de tramo
- [ ] 11. Un defecto real: la clave de tramo aliase la prioridad del sprite
- [ ] 12. Cómo se prueba sin cronómetro
- [ ] 13. Los contadores y lo que cuentan
- [ ] 14. Recorrido con comandos
- [ ] 15. Resultado y glosario

---

## Contexto

### 1. Qué dibuja la PPU y en qué presupuesto

La PPU por software dibuja **una línea de 240 píxeles cada vez**, al final de cada HBlank, y deja el
resultado en un framebuffer RGB555. El coste importa: el port corre a 60 fotogramas por segundo en
un móvil, y cada fotograma son 160 líneas.

Eso son 240 × 160 × 60 = **2.304.000 píxeles por segundo**, y cada píxel puede tener que decidir
qué capa gana, cuál es la segunda capa y qué mezcla se le aplica. La pregunta que organiza todo el
módulo es: **¿se puede decidir eso una vez por línea en vez de una vez por píxel?** La respuesta es
«a veces sí», y el trabajo consiste en saber cuándo.

### 2. La anatomía de una línea

Todo empieza en `render_scanline`, que hace tres cosas en este orden:

```rust
pub fn render_scanline(mem: &mut MemMap, line: usize) {
    // 1. Cada fondo activo se rasteriza a su propia línea de 240 píxeles.
    //    bg_lines: [Line; 4]  (Line = [u16; 240], 0xFFFF = transparente)
    // 2. Los sprites se rasterizan a obj_line (color + prioridad) y a la
    //    máscara de la ventana de sprites, obj_window.
    // 3. compose() elige el camino y escribe en mem.framebuffer.
}
```

Tres estructuras de salida, y un consumidor:

| estructura | qué contiene | quién la lee |
| :--- | :--- | :--- |
| `bg_lines: [Line; 4]` | el color de cada fondo en cada píxel de la línea | `compose` |
| `obj_line: [ObjPixel; 240]` | el color y la prioridad del sprite que tapa cada píxel | `compose` |
| `obj_window: [bool; 240]` | si ese píxel está dentro de la **ventana de sprites** | `compose` |
| `mem.framebuffer` | el resultado final, RGB555 | el presentador |

El paso 1 y el 2 no toman decisiones de mezcla: solo rasterizan. **Todas las decisiones pasan por
`compose`**, y por eso `compose` es el único sitio donde hay que mirar para entender qué dibuja el
juego.

> Nota sobre el orden: los sprites se rasterizan **después** de los fondos y **antes** de componer.
> Por eso el sprite sabe si la ventana de sprites está activa, y por eso `compose` puede preguntar
> por `obj.semi` (hay un píxel de sprite translúcido en la línea) sin volver a recorrer la OAM.

### 3. El control de un píxel

Antes de los caminos hay un concepto que hay que tener claro, porque es lo que separa el camino
rápido del general.

Para cada píxel, el hardware decide **qué capas están visibles** con una máscara de 6 bits:

| bit | capa |
| :--- | :--- |
| 0-3 | fondos BG0, BG1, BG2, BG3 |
| 4 | sprites (OBJ) |
| 5 | efectos de color |

Esa máscara se llama *control* en el código. El control de un píxel depende de **en qué ventana está
ese píxel**:

| dónde está el píxel | de dónde sale el control |
| :--- | :--- |
| dentro de WIN0 | los bits 0-5 de `WININ` |
| dentro de WIN1 | los bits 8-13 de `WININ` |
| dentro de la **ventana de sprites** | los bits 8-13 de `WINOUT` |
| en cualquier otro sitio | los bits 0-5 de `WINOUT` |

El código lo dice en una sola función, `control_at`:

```rust
let control_at = |x: usize| -> u16 {
    if !line_has_window {
        default_control
    } else if in_win0_y && in_window_range(win0h_x1, win0h_x2, x as u8) {
        winin & 0x3F
    } else if in_win1_y && in_window_range(win1h_x1, win1h_x2, x as u8) {
        (winin >> 8) & 0x3F
    } else if objwin_on && obj_window[x] {
        (winout >> 8) & 0x3F
    } else {
        winout & 0x3F
    }
};
```

Tres cosas que se leen aquí y que conviene recordar:

1. **El control se pregunta píxel a píxel.** Esa función es la razón de existir del camino general.
2. **La ventana de sprites (OBJ window) no es un rectángulo.** No hay `WININ`/`WINOUT` que la
   delimite: su máscara la dibujan los sprites en modo ventana (atributo 0, bits 10-11 = 2). En
   `render_sprites` eso es una línea: `if mode == 2 { window[sx_usize] = true; }`. Su forma depende
   de dónde estén los sprites en la OAM, no de ningún registro de ventana.
3. **Sin ventanas, el control es `0x3F`** (todas las capas) y no hace falta preguntar nada:
   `default_control` lo resuelve de antemano.

> **Tu turno.** Explica con tus palabras por qué el atajo puede partir la línea en los bordes de
> WIN0 y WIN1, y por qué eso no le sirve para la ventana de sprites.

☕ *Pausa: aquí puedes parar.*

### 4. Los dos caminos, span y tramo

**El camino general** (el del upstream fijado `de4edf5`, y el que la PPU nativa del port reproduce
píxel a píxel) recorre los 240 píxeles de la línea y, para cada uno, calcula el control y resuelve
la capa de arriba y la segunda capa. Es la referencia de fidelidad: aplica la ventana de sprites
porque pregunta el control **píxel a píxel**.

**El camino rápido** es un atajo: si el control de la línea es constante, la línea se parte en
tramos (spans) y cada tramo se resuelve de una vez. Hay dos variantes:

- `compose_straight`: para los efectos de aclarar y oscurecer (modos 2 y 3 de `BLDCNT`), que solo
  dependen del color de la capa de arriba, y también para las líneas sin efecto.
- `compose_alpha_tramos`: para el modo alpha (modo 1), que sí necesita las dos capas, pero donde
  la pareja (capa de arriba, segunda capa) se resuelve **una vez por racha de píxeles** en vez de
  una por píxel.

Ojo con dos palabras que el código usa para cosas distintas:

- **span**: trozo de línea con control constante. Los corta el atajo en los bordes de WIN0 y WIN1.
  Un span es un intervalo de `x` con un `u16` de control.
- **tramo**: racha de píxeles con la misma *clave de tramo* (las mismas capas opacas habilitadas y
  la misma prioridad de sprite). Es la unidad que usa `compose_alpha_tramos`. Un tramo es un
  intervalo de `x` **dentro** de un span.

La jerarquía es: **span → tramo → píxel**. El span dice qué control se aplica; el tramo dice qué
pareja de capas gana; el píxel es el color.

### El atajo, en el código

```rust
let mut cuts = [0usize, VISIBLE_WIDTH, 0, 0, 0, 0];
let mut ncuts = 2;
if in_win0_y {
    cuts[ncuts] = (win0h >> 8) as usize;
    cuts[ncuts + 1] = (win0h as u8) as usize;
    ncuts += 2;
}
if in_win1_y {
    cuts[ncuts] = (win1h >> 8) as usize;
    cuts[ncuts + 1] = (win1h as u8) as usize;
    ncuts += 2;
}
```

Ese es **todo** lo que el atajo sabe hacer: partir la línea en los bordes horizontales de WIN0 y
WIN1. Cada span toma el control de su primer píxel, y si no está en ninguna de esas dos ventanas,
ese control es el de fuera (`winout & 0x3F`).

La consecuencia es directa: **el atajo no puede componer la máscara de la ventana de sprites**,
porque esa máscara no tiene bordes conocidos de antemano. Solo el camino general, que pregunta el
control por píxel, puede aplicarla.

---

## Idea central

### 5. La cadena `razon_camino`

La PPU decide **una sola vez por línea** qué camino sigue, y esa decisión se toma en una cadena
llamada `razon_camino`. El orden de las razones no es un detalle de estilo: es el contrato que
dice *qué impide el atajo*.

```rust
let razon_camino = if objwin_on {
    2 // ventana de sprites
} else if !efecto_inerte && !con_efecto {
    1u8 // efecto de color real que exige la segunda capa
} else if obj_semi {
    3 // sprite translúcido
} else {
    0 // vía rápida
};
```

Se lee así:

1. **¿Hay ventana de sprites?** → camino general por píxel. Es la primera razón porque es la que
   más restringe: el atajo no puede saber el control de cada píxel.
2. **¿Hay un efecto de color real que necesite la segunda capa?** (el alpha del modo 1) → se intenta el
   camino por tramos, que resuelve la pareja de capas una vez por racha.
3. **¿Hay algún sprite translúcido?** → camino general por píxel: solo él mira `obj.semi`.
4. Si nada de eso → vía rápida.

Y la tabla con lo que decide cada entrada:

| entrada | de dónde sale | qué significa |
| :--- | :--- | :--- |
| `objwin_on` | `DISPCNT` bit 15 | la ventana de sprites está habilitada |
| `efecto_inerte` | tabla del bloque 6 | el efecto de color no puede cambiar ningún píxel |
| `con_efecto` | `BLDY != 0` y hay capas objetivo | hay un aclarado u oscurecido real |
| `obj_semi` | `render_sprites` | hay algún píxel de sprite translúcido en la línea |

La misma decisión alimenta los contadores de telemetría, así que el instrumento no puede divergir
de lo que hace el código: si `razon_camino` vale 2, la línea va a contar en `descartes_ventana`,
y en ningún otro.

Una sutileza: la razón 1 (alpha real) no siempre significa «camino general». En el código, el
camino por tramos se usa cuando `razon_camino == 1` **y** no hay sprite translúcido:

```rust
let alpha_por_tramos = razon_camino == 1 && (!obj_semi || alpha_tramos_forzado());
```

`alpha_tramos_forzado` solo existe con la feature `perf-video` y sirve para comparar los dos caminos
sobre la misma línea en el diagnóstico. En un binario normal, una línea con alpha real y un sprite
translúcido va píxel a píxel.

### Qué estaba mal antes

La cadena miraba **el efecto de color antes que la ventana de sprites**:

```rust
// antes del arreglo
let razon_camino = if !efecto_inerte && !con_efecto {
    1u8 // efecto de color real que exige la segunda capa
} else if objwin_on {
    2 // ventana de sprites
} else if obj_semi {
    3 // sprite translúcido
} else {
    0 // vía rápida
};
```

Con ese orden, una línea que tenía **a la vez** ventana de sprites y alpha real se colaba por el
primer brazo: `razon_camino` valía 1, se tomaba el camino por tramos, y el atajo aplicaba el
control de fuera de la ventana a **toda** la línea, incluidos los píxeles que estaban dentro de la
ventana de sprites. Ahí el control de dentro apaga un fondo (en el juego, `WINOUT` bits 8-13), así
que la capa de arriba cambia y el alpha se aplica a otra pareja de capas: el píxel sale distinto.

En el cuadro 700 de la secuencia de la escena 3 eso eran **3428 píxeles distintos en 22 líneas**,
y solo en las líneas que tenían las dos cosas a la vez.

> **Tu turno.** Una línea tiene la ventana de sprites activada y un alpha real con segundos
> objetivos. ¿Qué camino toma con el orden antiguo y qué toma con el nuevo? ¿Por qué el orden
> decide el resultado y no solo la velocidad?

### 6. El efecto de color, inerte o real

`efecto_inerte` responde a una pregunta precisa: **¿puede este efecto de color cambiar algún
píxel?** Si no puede, la línea se compone como si no hubiera efecto, y el atajo es válido.

```rust
let efecto_inerte = match blend_mode {
    0 => true,
    1 => bldcnt & 0x3F00 == 0,
    _ => evy == 0 || bldcnt & 0x3F == 0,
};
```

| modo de `BLDCNT` (bits 6-7) | efecto | cuándo es inerte |
| :--- | :--- | :--- |
| 0 | ninguno | siempre |
| 1 | alpha | no hay ninguna capa en la máscara de segundos objetivos (`0x3F00`) |
| 2 | aclarar | `BLDY = 0` o no hay ninguna capa objetivo (`0x3F`) |
| 3 | oscurecer | `BLDY = 0` o no hay ninguna capa objetivo (`0x3F`) |

El modo 1 merece una frase aparte: es el único que **necesita las dos capas** para saber el
resultado (mezcla la de arriba con la de debajo), así que es el único que el atajo no puede aplicar
sin mirar la segunda capa. Un alpha sin segundos objetivos no mezcla nada, así que sí puede ir por
el atajo.

Los modos 2 y 3 sí puede: su resultado depende solo del color de la capa de arriba. Eso es lo que
representa `EfectoTramo`:

```rust
struct EfectoTramo {
    oscurecer: bool, // true = modo 3, false = modo 2
    evy: u32,        // BLDY: 1..16
    objetivos: u16,  // bits 0-5 de BLDCNT: qué capas son objetivo
}
```

y `con_efecto = efecto_tramo.evy != 0`: si el efecto es inerte, `con_efecto` es falso y la línea se
compone recta, sin tocar nada.

### 7. La aritmética del alpha

El resultado de una mezcla no es el promedio simple: el hardware mezcla en un carril de 6 bits por
canal y redondea. Las tres funciones que lo hacen están al final de `ppu.rs`:

```rust
fn split6(c: u16) -> (u32, u32, u32) {
    ((c & 31) as u32, (((c >> 5) & 31) as u32) << 1, ((c >> 10) & 31) as u32)
}

fn alpha_blend(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = split6(a);
    let (br, bg, bb) = split6(b);
    let r = ((ar * eva + br * evb + 8) >> 4).min(31);
    let g = ((ag * eva + bg * evb + 8) >> 4).min(63);
    let b = ((ab * eva + bb * evb + 8) >> 4).min(31);
    join6(r, g, b)
}
```

Lo que hay que mirar:

- **Rojo y azul son de 5 bits** (0..31). **El verde es de 6 bits** (0..63): `split6` multiplica el
  verde por 2 al separar y `join6` lo divide por 2 al juntar. Ese bit extra es el bit 15 de la
  entrada de paleta (PRAM), que se enmascara al leer la paleta.
- **`+ 8` antes de `>> 4`**: es el redondeo «half-up» del hardware. Sin ese `+ 8`, el resultado
  perdería medio nivel por canal.
- **`EVA` y `EVB` van de 0 a 16** y el total siempre es 16, así que la mezcla es una media
  ponderada.

#### Un ejemplo completo, el píxel x = 103 de la escena del bloque 10

En esa escena, el color crudo de **BG1** en x = 103 es `0x03E8` (rojo 8, verde 31) y el de **BG0** es
`0x011F` (rojo 31, verde 8 en el campo de 5 bits, es decir 16 en el carril de 6 bits).

Con `EVA = EVB = 8`, la mezcla de BG0 sobre BG1 da:

| canal | cálculo | resultado |
| :--- | :--- | :--- |
| rojo | (31·8 + 8·8 + 8) >> 4 = 320 >> 4 | 20 |
| verde | (16·8 + 62·8 + 8) >> 4 = 632 >> 4 | 39 → se guardan 39 >> 1 = 19 |
| azul | (0·8 + 0·8 + 8) >> 4 | 0 |

Y `join6(20, 39, 0)` = 20 | (19 << 5) = **0x0274**. Ese es exactamente el color que pintaba el atajo
defectuoso en x = 103, donde la referencia pinta `0x03E8` (el color crudo de BG1, sin alpha).

> **Tu turno.** En x = 103, la referencia da `0x03E8` y el atajo antiguo daba `0x0274`. Explica de
> dónde sale cada número: qué color tiene cada capa en ese píxel y qué operación se aplicó en cada
> caso. ¿Por qué la referencia no mezcla?

---

## Alternativas y compromisos

Cuando se encontró el defecto había tres caminos, y la ficha de la propuesta
([`2026-10-21_ventana-obj-camino-rapido.md`](../propuestas/2026-10-21_ventana-obj-camino-rapido.md))
los comparó así:

**1. Arreglar la PPU nativa del port.** Descartada por medición: con el camino general del
upstream forzado, la PPU nativa da **0 píxeles distintos de 38400** en el cuadro 700. El error no
estaba en el destino, estaba en la referencia.

**2. Hacer que el atajo componga la máscara de la ventana de sprites.** Es la solución «bonita»:
tramos de longitud variable, calculados a partir de la máscara de sprites, en lugar de perder el
atajo. Se pospuso porque mete cambios de estructura en el camino caliente (el array fijo de 5 spans
se vuelve de tamaño variable) y su beneficio no estaba medido. Quedó anotada como mejora
posterior en el [`BACKLOG`](../BACKLOG.md) con impacto medio.

**3. Reordenar la cadena de razones.** Es la que se aplicó: mínima, local, y deja el criterio
escrito en el propio código.

El compromiso que gobierna la decisión está en la propia ficha: *primero la exactitud de la
referencia, aunque el camino rápido pierda rendimiento en las escenas afectadas*. Por eso se
eligió la opción 3 y no la 2: la 3 arregla el defecto hoy con un cambio pequeño, y deja la 2 como
mejora medible.

Hay una cuarta diferencia entre las dos PPU que conviene tener presente: **la PPU nativa del port
(`src/ppu_nativa.rs`) compone siempre por píxel**, en la GPU, con la misma aritmética y las mismas
reglas de `compose()`. No tiene atajo, así que no tiene esta clase de defecto; y por eso el defecto
del bloque 11 no cambia lo que se ve en el móvil por defecto (la PPU nativa es la ruta de dibujo por
defecto desde la propuesta
[`2026-10-21_ppu-nativa-activa-por-defecto.md`](../propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md)),
solo lo que se ve cuando la PPU nativa está apagada por marca y se dibuja por software.

### 9. El precio, medido

Con la corrección, las líneas con ventana de sprites dejan de usar el atajo. En la ventana de
cuadros 600..800 de la escena 3 (32.000 líneas):

| | antes del arreglo | después |
| :--- | :--- | :--- |
| líneas por el atajo | 27.865 | 1.920 (el 6 %) |
| líneas por píxel | 615 | 26.560 |
| tiempo de PPU (mediana de 3 tomas) | 121,57 ms | 144,40 ms (+18,8 %) |
| tiempo de PPU del camino general (el mismo código) | 154,23 ms | 154,77 ms (+0,3 %) |

La última fila es la que da la escala del ruido: el camino general **no cambió de código** entre
las dos columnas y se movió un 0,3 %, así que la máquina tiene un ruido de ±5 %. El +18,8 % está
muy por encima de ese ruido. El motivo es concreto: 26.080 de las 32.000 líneas (el 81,5 %) tenían
a la vez ventana de sprites y alpha real, y ahora van píxel a píxel.

Sobre el **cuadro completo** el efecto es pequeño: el `reloj` de la pareja de medición fue de
656,25 ms antes y 658,11 ms después (+0,3 %), porque la PPU son unos 145 ms de esos ~657 ms.

> **Tu turno.** Dos rutas de código idénticas miden 154,23 ms y 154,77 ms. ¿Qué te dice esa
> diferencia sobre el tamaño de un porcentaje de coste medido en una sola toma?

---

## Bloque 10 — El atajo por dentro: spans y clave de tramo

Este bloque es el que falta para entender por qué el atajo es rápido y por qué puede equivocarse.

### Del span al tramo

`compose` calcula los spans (los cortes por los bordes de WIN0 y WIN1) y luego llama a una de las
dos funciones de composición. La del modo 1 es `compose_alpha_tramos`:

```rust
for &(inicio, fin, control) in spans {
    let pesos = pesos_de_capa(control);
    let mut x = inicio;
    while x < fin {
        let (_, arriba, _, segunda) =
            top_two_layers(bg_lines, bg_prio, control, x, &obj_line[x], backdrop);
        let clave = clave_de_tramo(bg_lines, obj_line, pesos, x);
        let mut y = x + 1;
        while y < fin && clave_de_tramo(bg_lines, obj_line, pesos, y) == clave {
            y += 1;
        }
        componer_tramo(mem, /* … */, x, y, control, arriba, segunda, /* … */);
        x = y;
    }
}
```

El bucle hace tres cosas, en este orden:

1. **Resuelve la pareja de capas en el primer píxel del tramo** (`top_two_layers`): qué capa está
   arriba y cuál es la segunda.
2. **Calcula la clave de tramo** de ese píxel.
3. **Extiende el tramo** mientras la clave sea la misma, y pinta todo el tramo con la pareja
   resuelta en el paso 1.

Ahí está el atajo: `top_two_layers` se llama **una vez por tramo**, no una vez por píxel. En una
línea de 240 píxeles con una sola pareja de capas, son 1 llamada en vez de 240.

### Qué es la clave de tramo

```rust
fn clave_de_tramo(bg_lines, obj_line, pesos, x) -> u16 {
    let mut mascara = 0u16;
    for i in 0..4 {
        mascara |= if bg_lines[i][x] != TRANSPARENT { pesos[i] } else { 0 };
    }
    let prioridad = obj_line[x].priority;
    mascara | if prioridad != 0xFF { 0x100 | prioridad as u16 } else { 0 }
}
```

- **`pesos_de_capa(control)`** da el bit que le toca a cada fondo: `1 << i` si el control de la
  ventana lo habilita, 0 si no. Así la máscara de la clave respeta las ventanas.
- **La máscara** (bits 0-3) dice qué fondos están **habilitados y opacos** en ese píxel.
- **`0x100 | prioridad`** dice que hay un sprite visible y con qué prioridad. `0x100` es la marca de
  «hay sprite» y la prioridad (0..3) va en los bits bajos.

La idea es: **si dos píxeles tienen la misma clave, resuelven la misma pareja (capa de arriba,
segunda capa)**, así que un tramo entero se puede pintar de una vez. Los sprites y los fondos compiten
por prioridad, y la prioridad del sprite es lo único del sprite que puede cambiar quién gana.

### Un recorrido completo

Escena: modo 0, BG0 (prioridad 0) y BG1 (prioridad 1) opacas en toda la línea, sin ventanas, sin
sprites, `BLDCNT = 0x0241` (modo alpha, BG0 primer objetivo, BG1 segundo objetivo) y
`BLDALPHA = 0x0808` (EVA = EVB = 8).

| paso | qué ocurre | control | clave | pareja (arriba, segunda) |
| :--- | :--- | :--- | :--- | :--- |
| 1 | control por defecto (no hay ventanas) | `0x3F` | — | — |
| 2 | pesos: BG0 → `0x01`, BG1 → `0x02` | `0x3F` | — | — |
| 3 | clave en x = 0: BG0 y BG1 opacas, sin sprite | `0x3F` | `0x003` | (BG0, BG1) |
| 4 | la clave no cambia en los 240 píxeles | `0x3F` | `0x003` | (BG0, BG1) |
| 5 | `componer_tramo` mezcla los 240 píxeles de una vez | `0x3F` | — | — |

Resultado: **una** llamada a `top_two_layers` para toda la línea, en vez de 240. Y el color de cada
píxel es `alpha_blend(BG0[x], BG1[x], 8, 8)`.

Si la escena tuviera además un sprite opaco en x = 20..27, la clave de esos 8 píxeles sería
`0x103` (máscara `0x003` + marca `0x100` + prioridad `0`) y la línea se partiría en **tres** tramos:
`0..19`, `20..27` y `28..239`. Tres llamadas a `top_two_layers` en vez de 240.

### El atajo recto, para comparar

`compose_straight` es más simple: no necesita la segunda capa, así que no hay clave ni tramos. Para
cada span, mira el control y se queda con el primer fondo opaco que el control habilite (con el
sprite intercalado por prioridad). Si el efecto es de aclarar u oscurecer, lo aplica al color
ganador; si además el fondo de pantalla es objetivo del efecto, lo transforma una sola vez por
span.

---

## Bloque 11 — Un defecto real: la clave de tramo aliase la prioridad del sprite

Esta sección cuenta un defecto que **se encontró al escribir esta página**, no al hacer el arreglo
de la ventana de sprites. Se reproduce y se mide; el arreglo no está hecho y vive en su propia
propuesta:
[`2026-10-22_clave-de-tramo-aliase-la-prioridad.md`](../propuestas/2026-10-22_clave-de-tramo-aliase-la-prioridad.md).

### Qué pasa

La clave de tramo (bloque 10) mete dos cosas en el mismo número de 16 bits:

- la **máscara de fondos** en los bits 0-3;
- la **prioridad del sprite** en `0x100 | prioridad`, con `prioridad` entre 0 y 3 → **los bits 0-1**.

Los bits 0-1 se usan dos veces. Dos píxeles distintos pueden producir la misma clave:

| píxel | máscara de fondos | sprite | clave |
| :--- | :--- | :--- | :--- |
| x = 7 | BG0 opaca → `0x01` (BG1 transparente) | prioridad 2 | `0x01 \| 0x102` = `0x103` |
| x = 8 | BG0 y BG1 opacas → `0x03` | prioridad 0 | `0x03 \| 0x100` = `0x103` |

La misma clave, y sin embargo los dos píxeles **no** resuelven la misma pareja:

| píxel | qué gana | por qué |
| :--- | :--- | :--- |
| x = 7 | (BG0, OBJ) | el sprite tiene prioridad 2 y BG0 tiene prioridad 0: gana el fondo |
| x = 8 | (OBJ, BG0) | el sprite tiene prioridad 0: a igual prioridad, el sprite gana al fondo |

Como el tramo se pinta con la pareja resuelta en su **primer** píxel, el segundo se pinta con la
pareja equivocada.

### Cómo se reproduce

Escena: modo 0, BG0 (prioridad 0) opaca en toda la línea, BG1 (prioridad 1) opaca **solo** en
x = 8..15, dos sprites opacos que no se solapan (sprite 0 en x = 0..7 con prioridad 2, sprite 1 en
x = 8..15 con prioridad 0), `BLDCNT = 0x0241` y `BLDALPHA = 0x0808`.

| x | atajo (por tramos) | camino general (referencia) |
| ---: | :--- | :--- |
| 0-7 | `0x003F`, `0x005F`, … (BG0) | igual |
| 8 | `0x003F` (BG0) | `0x03E0` (el sprite) |
| 9 | `0x005F` (BG0) | `0x03E0` |
| 10 | `0x007F` (BG0) | `0x03E0` |
| 11 | `0x009F` (BG0) | `0x03E0` |
| 12 | `0x00BF` (BG0) | `0x03E0` |
| 13 | `0x00DF` (BG0) | `0x03E0` |
| 14 | `0x00FF` (BG0) | `0x03E0` |
| 15 | `0x011F` (BG0) | `0x03E0` (el sprite) |

**8 píxeles distintos de 240**, y la diferencia es que el atajo pinta el fondo donde el hardware
pinta el sprite.

### Dónde está y cuánto pesa

- Está en `clave_de_tramo`, en el **parche local** del motor (`../gba-recomp`), no en el upstream
  fijado `de4edf5`: el upstream **no tiene camino rápido**, compone siempre píxel a píxel. Por eso
  el defecto no existe en el motor tal y como se descarga.
- Solo afecta al modo 1 (alpha), que es el único que usa la clave de tramo.
- Solo se ve cuando un sprite **cambia de prioridad** justo donde un fondo **cambia de opacidad**,
  y en las líneas donde ese tramo llega a pintarse. Es raro, pero es un píxel equivocado, y el
  criterio del port es que el camino rápido tiene que dar **exactamente** los mismos píxeles que el
  camino general.
- La PPU nativa del port (GPU, siempre por píxel) no lo sufre.

> **Tu turno.** La clave de tramo guarda la máscara de fondos en los bits 0-3 y la prioridad del
> sprite en `0x100 | prioridad`. ¿Qué dos cenas distintas podrían acabar con la misma clave? ¿Y qué
> habría que cambiar para que eso no sea posible?

---

## Bloque 12 — Cómo se prueba sin cronómetro

Medir tiempos en un test es frágil. El motor tiene una forma mejor de demostrar que el atajo no
busca la segunda capa en cada píxel: **contar las búsquedas**.

```rust
#[cfg(test)]
std::thread_local! {
    static BUSQUEDAS_DOS_CAPAS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
```

Cada llamada a `top_two_layers` (la función que resuelve la pareja de capas de un píxel) suma uno.
La cuenta es **por hilo** a propósito: los tests corren en paralelo y una cuenta global mezclaría
las líneas de un test con las de otro.

El test de la ventana de sprites comprueba las dos cosas:

```rust
assert_eq!(
    BUSQUEDAS_DOS_CAPAS.with(|busquedas| busquedas.get()),
    VISIBLE_WIDTH as u64,
    "con la ventana de sprites la línea no adopta el atajo por tramos"
);
```

240 búsquedas: una por píxel, que es la firma del camino general. Y después, la comparación de
píxeles con el camino general forzado (`SIN_VIA_RAPIDA` en los tests, `forzar_camino_general` en el
diagnóstico).

**El detalle que hace que el test valga**: antes del arreglo, este test fallaba con «3 búsquedas
de la segunda capa en vez de 240». No con una diferencia de color, sino con un número. Eso
significa que el test detecta el defecto *aunque la imagen saliera bien por casualidad*, que es
justo lo que pasaba en muchas escenas.

> **Tu turno.** ¿Por qué no basta con comparar la imagen del atajo con la del camino general, y por
> qué el contador de búsquedas es una prueba más fuerte?

---

## Bloque 13 — Los contadores y lo que cuentan

El motor lleva un reparto de líneas por camino, que es lo que alimenta la telemetría del port
(ver la [guía de medición](../guias/medir-rendimiento.md)):

| contador | qué cuenta |
| :--- | :--- |
| `lineas_rectas` | líneas que fueron por la vía rápida (o por la recta con efecto) |
| `lineas_alpha_tramos` | líneas compuestas por tramos (alpha real) |
| `lineas_camino_general` | líneas que fueron píxel a píxel |
| `descartes_ventana` | líneas descartadas por la ventana de sprites |
| `descartes_mezcla` | líneas descartadas por un efecto de color que exige la segunda capa |
| `descartes_semi` | líneas descartadas por un sprite translúcido |
| `lineas_efecto_alpha` | líneas con alpha real que el atajo deja solo por el alpha |

Un detalle que cambió con el arreglo y que conviene tener presente al leer la telemetría: una
línea con ventana de sprites **y** alpha real ahora se cuenta en `descartes_ventana` y **no** en
`lineas_efecto_alpha`. Antes se contaba al revés. La suma de las cuatro cuentas sigue cuadrando
con las 160 líneas del fotograma, porque las razones son excluyentes: se anota la primera que
aplica.

Hay dos contadores más que no son de camino sino de coste: `ppu_ns`, `bg_ns`, `sprite_ns` y
`compose_ns` (nanosegundos acumulados por fase). Son los que alimentan el desglose por fases de la
telemetría (`ejec`, `ppu`, `apu`, `dibujo`, `present`).

Una nota sobre un nombre: el comentario de `mem.rs` describe `lineas_efecto_alpha` como «de las
anteriores» (o sea, de las líneas que ya usaban el atajo), pero el código lo cuenta en la rama del
descarte por alpha, que es justo donde caen las líneas que **no** usan el atajo. Es una discrepancia
pequeña entre el comentario del motor y su código, y con el arreglo de la ventana de sprites se nota
más (esas líneas tampoco están en `lineas_rectas`). Para la telemetría manda el código; el
comentario es lo que queda desfasado.

> **Tu turno.** Con la corrección, una línea tiene a la vez ventana de sprites y alpha real. ¿En
> qué contador se cuenta y por qué las cuatro cuentas siguen siendo excluyentes entre sí?

---

## Bloque 14 — El recorrido con comandos

Todo lo de abajo se ejecutó con el código actual y da los números que se citan arriba.

### 1. La suite del motor, con la escena de la ventana de sprites

```bash
cd ../gba-recomp
cargo test -p gba-core --features perf-video
```

Sale en verde: 88 pruebas unitarias y 10 de integración. Para ver solo la del defecto:

```bash
cargo test -p gba-core --features perf-video alpha_por_tramos_con_ventana_obj_dibuja_lo_mismo_que_el_camino_general
```

### 2. El cuadro real del juego (cuadro 700 de la escena 3)

Este comando necesita la ROM del juego, que el repositorio no incluye por licencia.

```bash
cargo run --release --features proto --example ppu_camino_general -- roms/<tu-rom>.gba secuencias/escena3.txt 700
```

Salida con el arreglo:

```text
resumen cuadro 700: camino rápido vs juego = 0 píxeles distintos de 38400 (esperado 0)
camino control: 0
camino D0: 0
camino D1: 0
camino lineas con diferencia: 0
```

`D0` son los píxeles distintos entre el camino rápido y el camino general; `D1`, lo mismo con los
sprites apagados. Antes del arreglo, `D0` valía 3428 en 22 líneas.

### 3. El reparto de líneas y el coste

```bash
cargo run --release --features proto --example ppu_tiempos -- roms/<tu-rom>.gba secuencias/escena3.txt 600 20
```

```text
ventana medida: cuadros 600..620
  camino rápido:  PPU 15.59 ms, composición 5.48 ms, reloj 62.96 ms
    líneas: rectas 0, alpha por tramos 0, camino general 3200, descartes (ventana 3200, semi 0, mezcla 0)
```

3200 líneas = 20 fotogramas × 160 líneas, y las 3200 van por píxel: en esa ventana del juego la
ventana de sprites está activa en todas las líneas, así que ninguna puede usar el atajo.

### 4. El mapa del código

Para leer el módulo con orden, esta es la ruta que sigue esta página:

| función | qué hace |
| :--- | :--- |
| `render_scanline` | el punto de entrada: fondos, sprites y `compose` |
| `render_text_bg`, `render_affine_bg`, `render_bitmap_bg` | rasterizan cada fondo a su línea |
| `render_sprites` | rasteriza los sprites y llena la máscara `obj_window` |
| `compose` | decide el camino y escribe el framebuffer |
| `razon_camino`, `efecto_inerte`, `EfectoTramo` | la decisión y el efecto que la aplica |
| `control_at` | el control de un píxel (el camino general) |
| `top_two_layers` | la pareja (capa de arriba, segunda capa) de un píxel |
| `compose_straight`, `straight_span` | el atajo sin segunda capa (modos 2 y 3, o sin efecto) |
| `compose_alpha_tramos`, `clave_de_tramo`, `componer_tramo` | el atajo con segunda capa (modo 1) |
| `alpha_blend`, `brighten`, `darken`, `split6`, `join6` | la aritmética de la mezcla |

---

## Resultado

Después de esta página deberías poder:

- **Explicar** por qué el atajo puede partir una línea en los bordes de WIN0 y WIN1, y por qué la
  ventana de sprites no cabe en ese modelo.
- **Decir** qué significa cada número de la cadena `razon_camino` y por qué el orden importa.
- **Distinguir** span (control constante) de tramo (misma pareja de capas) y decir qué bits
  resumen.
- **Leer** la tabla del bloque 7 y explicar de dónde sale cada color, incluido el carril de 6 bits
  del verde.
- **Reproducir** el defecto de la ventana de sprites con el contador de búsquedas y el coste con los
  ejemplos del bloque 14.
- **Reconocer** un defecto de aliasing en una clave de bits y reproducirlo con una escena mínima.

Lo que queda fuera del arreglo, anotado en la ficha y en el
[`BACKLOG`](../BACKLOG.md): el atajo sigue sin componer la máscara de la ventana de sprites (se
podría recuperar parte del rendimiento con tramos de longitud variable), el shim de GPU del port
(`src/ppu_shim.rs`, retirado en el ADR 42) carecía de ventana de sprites por hardware porque era otro camino de dibujo, y la
clave de tramo tiene el defecto de aliasing del bloque 11.

---

## Respuestas de los «Tu turno»

**1. (Contexto) Por qué el atajo puede partir la línea en los bordes de WIN0 y WIN1, y por qué eso
no le sirve para la ventana de sprites.**
Porque dentro de WIN0 (o de WIN1) el control es una constante (`WININ` o `WININ >> 8`) y solo
cambia al cruzar el borde horizontal de la ventana. La ventana de sprites, en cambio, no tiene
bordes conocidos: su máscara la dibujan los sprites, píxel a píxel. Un atajo que solo corta en
bordes de ventana no puede representarla, y aplicaría el control de fuera a toda la línea.

**2. (Idea central) Qué camino toma la línea con ventana de sprites y alpha real, y por qué el
orden decide el resultado.**
Con el orden antiguo tomaba el camino por tramos (`razon_camino` = 1) y aplicaba el control de
fuera a toda la línea; con el nuevo toma el camino general por píxel (`razon_camino` = 2), que
pregunta el control en cada píxel y respeta la máscara. El orden no cambia solo la velocidad:
cambia qué control se aplica a cada píxel, y por tanto el color.

**3. (Bloque 7) De dónde salen `0x03E8` y `0x0274` en x = 103.**
En x = 103 el tile 1 de BG1 tiene el índice de color 8, y BG1 usa la paleta 2 (`0x03E0 | índice`):
el color crudo es rojo 8, verde 31, que es `0x03E8`. En la referencia, dentro de la ventana de
sprites, el control apaga BG0, así que la capa de arriba es BG1 y no hay alpha (BG1 no es primer
objetivo de `BLDCNT`): se pinta el color crudo. El atajo antiguo usaba el control de fuera, con
BG0 como capa de arriba, y aplicaba el alpha 8/8 entre BG0 (rojo 31, verde 16 en el carril de 6
bits) y BG1 (rojo 8, verde 62): sale rojo (31·8 + 8·8 + 8) >> 4 = 20 y verde (16·8 + 62·8 + 8) >> 4 =
39, que al guardarse en 5 bits es 19; con `join6` eso es `0x0274`.

**4. (Alternativas) Qué dice la diferencia entre 154,23 ms y 154,77 ms sobre el tamaño de un
porcentaje de coste medido en una sola toma.**
Que dos rutas de código idénticas se miden con un ruido del ±5 %. Cualquier porcentaje de coste
medido en una sola toma por debajo de ese margen no significa nada; hay que medir por triplicado y
alternando los estados.

**5. (Bloque 11) Qué dos cenas distintas podrían acabar con la misma clave, y qué habría que
cambiar.**
Dos píxeles en los que un fondo cambia de opacidad a la vez que el sprite cambia de prioridad: la
máscara de fondos (bits 0-3) y la prioridad (bits 0-1) se solapan, y el «OR» de las dos cosas
puede dar el mismo número para pares distintos. Habría que mover la prioridad a bits que la máscara
no use (por ejemplo `0x100 | (prioridad << 4)`, con la marca de «hay sprite» en `0x200`), o
comparar la clave con dos campos separados en vez de con un solo número.

**6. (Bloque 12) Por qué el contador de búsquedas es una prueba más fuerte que comparar imágenes.**
Porque compara el *comportamiento* y no el resultado. Dos caminos pueden coincidir píxel a píxel
en una escena concreta y seguir siendo distintos en general; el contador dice que el atajo dejó
de usarse en esa línea, con independencia de los colores que salgan. Por eso el test se comprobó
en rojo con «3 búsquedas en vez de 240» y no con una diferencia de color.

**7. (Bloque 13) En qué contador se cuenta la línea con ventana de sprites y alpha real.**
En `descartes_ventana`, porque la ventana de sprites es la primera razón que impide el atajo. Las
cuatro cuentas son excluyentes porque `razon_camino` es una decisión con ramas mutuamente
exclusivas: se anota la primera razón que aplica y solo esa. A cambio, esas líneas ya no entran en
`lineas_efecto_alpha`, que es el reparto por modo del efecto de color.

---

## Glosario

| término | qué es |
| :--- | :--- |
| **control** | máscara de 6 bits que dice qué capas están visibles en un píxel |
| **span** | tramo de línea con control constante; lo corta el atajo en los bordes de WIN0/WIN1 |
| **tramo** | racha de píxeles con la misma clave de tramo; la unidad de `compose_alpha_tramos` |
| **clave de tramo** | número de 16 bits que resume qué fondos opacos hay y con qué prioridad tapa el sprite |
| **capa de arriba / segunda capa** | la primera y la segunda capa visibles de un píxel, en ese orden |
| **efecto inerte** | efecto de color que no puede cambiar ningún píxel de la línea |
| **modo de BLDCNT** | bits 6-7: 0 ninguno, 1 alpha, 2 aclarar, 3 oscurecer |
| **EVA / EVB / EVY** | pesos de la mezcla: 0-16, la suma de EVA y EVB es 16 |
| **ventana de sprites** | ventana OBJ: la máscara que dibujan los sprites en modo ventana, no un rectángulo |
| **carril de 6 bits** | el verde se mezcla con 6 bits (0-63), el rojo y el azul con 5 (0-31) |
| **half-up** | el redondeo del hardware: `+ 8` antes de dividir por 16 |
