# Propuesta: Los 47 experimentos dejan de llevar su propia copia del arnés

Fecha: 2026-10-22
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Quien lee un experimento de `examples/` deja de encontrarse 100 líneas de arnés copiadas antes de
la parte que mide: los 47 comparten un módulo (`examples/comun/`) que ya contiene el arnés, y cada
experimento se queda sólo con su pregunta y su `main`. Los nombres, los argumentos y los volcados
de los 47 no cambian.

### 2. Contexto

- Contexto consultado: los 47 ficheros de `examples/`, `Cargo.toml` (feature `proto` y los 47
  bloques `[[example]]`), `src/ppu_shim.rs` y `src/ppu_nativa.rs` (firmas de `new` y
  `render_frame`), `scripts/diagnostico/mutar_ppu_nativa.py`, `docs/explicacion/ppu-caminos-de-composicion.md`
  y las 39 propuestas de `docs/propuestas/` que invocan `--example`.
- Intención: hoy el arnés está escrito **47 veces** (20.574 líneas en total, de las que ~2.900 son
  copias del mismo código). El problema no es el tamaño: es que **una corrección al arnés hay que
  hacerla 45 veces**. Ya pasó con `leer_secuencia`, que tiene 45 copias en dos variantes, y con el
  arnés GPU, que tiene 29 copias en dos variantes (25 con el shim, 4 con la PPU nativa). El éxito
  es que arreglar el arnés sea tocar un fichero, y que los experimentos sigan midiendo exactamente
  lo mismo.
- Restricciones y decisiones previas (máx. 3):
  - Los 47 experimentos se quedan, con su nombre y su interfaz (`<rom> <secuencia> <cuadro>`):
    39 propuestas y dos páginas de `docs/explicacion/` los citan por nombre y con argumentos
    concretos (origen: `grep -rn -- "--example" docs/ scripts/`) → implica: no se renombra, no se
    borra y no se cambia el número ni el orden de los argumentos → protegida por «El inventario de
    experimentos no cambia» | testeable.
  - El módulo común vive en `examples/comun/mod.rs` y no en `src/` (origen: respuesta humana,
    2026-10-22) → implica: el crate que se empaqueta en el APK no gana módulos ni la referencia de
    módulos cambia; el precio es que el módulo se comprueba desde un test de integración con
    `#[path]`, no con `cargo test --lib` → protegida por «Los experimentos usan el módulo común y
    ninguno vuelve a definir sus ayudantes» | testeable.
  - La consolidación es sólo de los ayudantes duplicados; el `main()` de cada experimento se queda
    donde está (origen: respuesta humana, 2026-10-22) → implica: el bucle hasta el cuadro, los
    mensajes de uso y la escritura de `volcados/` no se reescriben; el cambio en cada fichero es
    «borrar las copias y añadir el `use`» → protegida por «La secuencia se lee igual que antes» |
    testeable.
- Alternativas descartadas (solo si las hubo):
  - Poner el arnés en `src/proto_arnes.rs` tras `feature = "proto"` → descartada porque mete en el
    crate que se publica un módulo que sólo existe para experimentos, y lo obliga a aparecer en
    `docs/referencia/api-modulos-rust.md`; el test con `#[path]` da la misma cobertura sin eso.
  - Un `main()` común parametrizado por banderas → descartada por la segunda restricción: cambia
    la forma de los 47 experimentos, y la mayoría deja de leerse como una pregunta con su código.
  - Borrar los `shim_afin_*` redundantes (12 variantes casi idénticas) → descartada porque rompe
    las citas de las propuestas antiguas, que son el registro de lo que se midió.
- Ante imprevistos, prioriza: que los 47 sigan midiendo lo mismo > que el fichero quede corto >
  que el módulo quede elegante.

### 3. Ejemplos

- Caso normal: `examples/shim_afin_bg.rs` (408 líneas) → antes: 29 líneas de `leer_secuencia`, 107
  del arnés GPU (`struct Gpu` + `impl Gpu`), `canal_8bit`, `cerca`, `BOTONES`, `FILA_GPU`,
  `TOLERANCIA` y `software_a_rgb` copiados dentro del fichero (≈148 líneas) → ahora: cinco líneas
  (`#[path = "comun/mod.rs"] mod comun;` y los `use`) y el fichero baja a ~260 líneas, con las
  mismas llamadas y el mismo volcado `volcados/afin-bg-<N>.txt`.
- Caso límite: `examples/ppu_compare.rs` usa una forma **distinta** de leer la secuencia: su
  `leer_secuencia` devuelve `Vec<Tramo>` con el `nombre` de cada tramo, no `Vec<(u32, u16)>` →
  ahora: llama a `comun::entrada::leer_tramos`, que devuelve `Tramo { nombre, cuadros,
  presionados }`; las dos formas salen del mismo parser privado, así que no hay dos gramáticas.
- Caso límite: `examples/ppu_capas_linea.rs` define un `dispcnt_de` con **11 variantes** leídas de
  una tabla `CAPAS`; no es una copia de las otras 11 (que tienen 4 variantes fijas) → ahora: ese
  fichero conserva su función y es la única definición de `fn dispcnt_de` que queda en `examples/`.

### 4. Pasos

1. Se crea `examples/comun/` con `mod.rs` (re-exporta), `entrada.rs`, `color.rs`, `registro.rs` y
   `gpu.rs`, con el código movido **tal cual** desde las copias mayoritarias.
2. Los 47 ficheros de `examples/` pierden sus copias y ganan `#[path = "comun/mod.rs"] mod comun;`
   más los `use` que necesiten; su `main()` no se toca salvo para llamar al módulo.
3. Se escribe `tests/arnes_experimentos_tests.rs`, que incluye el módulo con
   `#[path = "../examples/comun/mod.rs"] mod comun;`, y los cuatro escenarios del §8.
4. Se comprueba que los 47 compilan, que la suite sigue verde y que tres experimentos (software,
   shim y nativa) escriben los mismos volcados que antes con la misma ROM, secuencia y cuadro.
5. Se registra en la propuesta lo que aparezca fuera de alcance.

### 5. Qué más podría verse afectado

- `Cargo.toml` — los 47 bloques `[[example]]` no cambian. Hay que comprobar que
  `examples/comun/` **no** se autodescubre como ejemplo: la autodetección busca `examples/*.rs` y
  `examples/*/main.rs`, y `comun/mod.rs` no es ninguna de las dos cosas.
- `scripts/diagnostico/mutar_ppu_nativa.py` — línea 39: ejecuta `--example ppu_nativa_afin`. El
  script no se toca; el fichero sí cambia de forma, así que hay que volver a correrlo para
  comprobar que las mutaciones siguen muriendo.
- `docs/explicacion/ppu-caminos-de-composicion.md` — líneas 696 y 715: invocan
  `ppu_camino_general` y `ppu_tiempos` con argumentos concretos. Los dos pierden sus copias de
  `leer_secuencia`, y las dos invocaciones tienen que seguir funcionando igual.
- `docs/propuestas/` — 39 ficheros citan `--example` con su nombre. El registro no se retoca
  (regla del `docs/propuestas/README.md`), así que los nombres y los argumentos quedan congelados.
- `docs/referencia/api-modulos-rust.md` — **no cambia**: el módulo no está en `src/` y
  `scripts/extraer_referencias.py` no barre `examples/`.
- `volcados/` — las rutas de salida (`volcados/<nombre>-<cuadro>.txt`) no cambian; siguen siendo
  archivos locales sin versionar.
- Búsqueda hecha: `grep -rn -- "--example" scripts/ tests/ docs/ *.md` para enumerar quién invoca
  los experimentos, y un hash por bloque sobre los 47 ficheros para agrupar las copias idénticas.

### 6. Suposiciones

- `#[path = "comun/mod.rs"] mod comun;` funciona en un ejemplo y `#[path = "../examples/comun/mod.rs"] mod comun;`
  en un test de integración → prototipar: sí
  → si es falso: el módulo tiene que ir a `src/` tras `feature = "proto"`, y el plan cambia de
  sitio y gana un módulo en la referencia de la API.
  → resultado del prototipo: | no detecta: no dice si dos ejemplos que compilan el mismo módulo se
  pisan en tiempo de compilación (son crates distintos, así que no deberían).
- Un solo `struct Gpu` con dos constructores (`Gpu::nueva()` para el shim, `Gpu::nueva_nativa()`
  para la PPU nativa) sirve a las 29 copias → prototipar: sí
  → si es falso: harían falta dos arneses y la mitad del beneficio del §12.5 desaparece.
  → resultado del prototipo: | no detecta: no prueba que las dos rutas den la misma imagen en una
  GPU real.
- Los tres experimentos de muestra escriben volcados idénticos antes y después del cambio, con la
  misma ROM, secuencia y cuadro → prototipar: sí
  → si es falso: el módulo no reproduce la copia y hay que revisar el ayudante que difiera antes de
  seguir con los 47.
  → resultado del prototipo: | no detecta: no cubre los 44 experimentos que no se corran ni otra
  GPU distinta de la de esta máquina.

### 7. Alcance

- Incluye:
  - `examples/comun/` con el arnés compartido: secuencia, color/comparación, registros y GPU.
  - Los 47 ficheros de `examples/` perdiendo sus copias (≈2.900 líneas) y declarando el módulo.
  - `tests/arnes_experimentos_tests.rs` con los cuatro escenarios del §8.
- No incluye:
  - El `main()` de cada experimento: los argumentos, el bucle hasta el cuadro N y la escritura del
    volcado se quedan como están.
  - `dispcnt_de` de `ppu_capas_linea.rs`: no es una copia, resuelve 11 variantes desde una tabla.
  - Borrar, fusionar o renombrar experimentos; mover nada a `src/`; tocar `Cargo.toml`.
  - Versionar `secuencias/` o `volcados/` (hoy no están versionados; ver «Correcciones previas»).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Los experimentos usan el módulo común y ninguno vuelve a definir sus ayudantes
Caso de uso: alguien corrige el arnés y quiere que la corrección llegue a los 47 a la vez | Tipo: normal
  Dado que los 47 ficheros de `examples/` y el módulo `examples/comun/`
  Cuando el test recorre los 47 ficheros y el módulo
  Entonces hay 47 ficheros, cada uno contiene `mod comun;`, ninguno vuelve a declarar
  `fn leer_secuencia`, `const BOTONES`, `fn canal_8bit`, `fn cerca`, `struct Gpu`, `fn io16`,
  `fn io_con_variante` ni `fn software_a_rgb`, y `fn dispcnt_de` aparece exactamente una vez (en
  `ppu_capas_linea.rs`)
Test: `tests/arnes_experimentos_tests.rs::test_los_experimentos_usan_el_modulo_comun`
Estado previo: se completa al escribir el test: falla (hoy cada fichero declara sus copias)

Escenario: El inventario de experimentos no cambia
Caso de uso: se sigue una propuesta antigua que invoca un experimento por su nombre | Tipo: regresión
  Dado que `examples/` y `Cargo.toml`
  Cuando el test compara los nombres de los 47 ficheros con los 47 bloques `[[example]]`
  Entonces los dos conjuntos son iguales, tienen 47 elementos y cada bloque declara
  `required-features = ["proto"]`
Test: `tests/arnes_experimentos_tests.rs::test_el_inventario_coincide_con_cargo_toml`
Estado previo: se completa al escribir el test: pasa (regresión)

Escenario: La secuencia se lee igual que antes
Caso de uso: se reutiliza una secuencia de las que ya están escritas | Tipo: regresión
  Dado que el texto `"# comentario\n120 ninguno\n\n60 A+B\n30 R\n"`
  Cuando se llama a `comun::entrada::leer_secuencia`
  Entonces devuelve 3 tramos: `(120, 0x000)`, `(60, 0x003)` y `(30, 0x100)`, en ese orden
Test: `tests/arnes_experimentos_tests.rs::test_la_secuencia_se_lee_igual_que_antes`
Estado previo: se completa al escribir el test: pasa (regresión)

Escenario: Un botón desconocido se rechaza nombrando la línea
Caso de uso: alguien escribe `A+Foo` en una secuencia y el experimento tiene que decírselo | Tipo: error
  Dado que el texto `"10 A+Foo\n"`
  Cuando se llama a `comun::entrada::leer_secuencia`
  Entonces devuelve `Err` y el mensaje contiene `línea 1` y `Foo`
Test: `tests/arnes_experimentos_tests.rs::test_la_secuencia_rechaza_un_boton_desconocido`
Estado previo: se completa al escribir el test: falla (el módulo no existe todavía)

### 9. Cómo lo comprobaré

- Los experimentos usan el módulo común → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_los_experimentos_usan_el_modulo_comun`
  → pasa si: `1 passed`
  → no detecta: que el módulo común haga lo mismo que la copia que reemplaza; eso lo miran el
  tercer escenario y el contraste de volcados.
- El inventario no cambia → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_el_inventario_coincide_con_cargo_toml`
  → pasa si: `1 passed`
  → no detecta: que un experimento siga midiendo lo que decía; sólo que existe y conserva el nombre.
- La secuencia se lee igual → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_la_secuencia_se_lee_igual_que_antes`
  → pasa si: `1 passed`
  → no detecta: sintaxis que el test no escribe (tabuladores, `#` a mitad de línea, mayúsculas
  mezcladas); el parser las acepta o no igual que antes, y eso no se prueba.
- Un botón desconocido se rechaza → herramienta o comando:
  `cargo test --test arnes_experimentos_tests test_la_secuencia_rechaza_un_boton_desconocido`
  → pasa si: `1 passed`
  → no detecta: el texto exacto del mensaje más allá de `línea 1` y `Foo`.
- Los 47 compilan → herramienta o comando:
  `cargo build --examples --features proto`
  → pasa si: termina con `Finished` y sin líneas `error`
  → no detecta: warnings nuevos de código muerto; se revisan leyendo la salida (hoy no hay
  ninguno: `cargo build --examples --features proto` tarda 52,43 s y sale limpio).
- La salida numérica no cambia → herramienta o comando, con la ROM y la BIOS en `roms/` (no
  versionadas) y una GPU:
  ```bash
  # antes del cambio, con cada representante
  cargo run --release --features proto --example ppu_estado    -- "roms/<rom>.gba" secuencias/escena3.txt 60
  cargo run --release --features proto --example shim_afin_linea -- "roms/<rom>.gba" secuencias/escena3.txt 60
  cargo run --release --features proto --example ppu_nativa_afin -- "roms/<rom>.gba" secuencias/escena3.txt 60
  # copiar volcados/ a un directorio aparte, repetir después y diff
  ```
  → pasa si: los tres volcados son idénticos byte a byte antes y después, y el control de
  determinismo de los ejemplos con GPU no falla (si falla, el ejemplo sale con código 1 y no
  escribe volcado)
  → no detecta: los otros 44 experimentos, otra GPU u otro sistema, ni que el arnés siga siendo
  determinista en el tiempo.
- La suite sigue verde → herramienta o comando: `cargo test`
  → pasa si: código 0 y **285** tests (los 281 de hoy más los 4 nuevos)
  → no detecta: `src/platform.rs`, que es `cfg(android)` y no se compila en host.
- La documentación sigue sincronizada → herramienta o comando:
  `python scripts/extraer_referencias.py --check` y
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  → pasa si: el primero sale con código 0 y el segundo con `0 error(es), 0 aviso(s)`
  → no detecta: que las dos invocaciones citadas en `docs/explicacion/` sigan funcionando; eso lo
  mira el comando de compilación de los 47.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación, los 47 compilan y los tres volcados de muestra son
idénticos antes y después.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como "discrepada".

### 12. Desglose de la consolidación

#### 12.1 Lo que se mueve, medido

| Pieza | Copias | Variantes | Líneas por copia | Duplicadas |
| :--- | ---: | ---: | ---: | ---: |
| `fn leer_secuencia` | 45 | 2 (44 iguales + `ppu_compare`) | 29 | 1.247 |
| `struct Gpu` | 29 | 2 (25 shim + 4 nativa) | 8 | 192 |
| arnés GPU (`struct` + `impl`, hasta `main`) | 29 | el grupo mayor, 25 | 107 | 1.166 |
| `fn canal_8bit` | 30 | 1 | 3 | 87 |
| `fn dispcnt_de` | 13 | 3 (11 iguales + `ppu_capas_linea` + `ppu_contenido_bg`) | 8 | 80 |
| `fn cerca` | 28 | 2 (`[u8; 3]` y `&[u8] + índice`) | 3 | 78 |
| `fn io16` | 15 | 1 | 3 | 42 |
| `fn io_con_variante` | 5 | 1 | 7 | 28 |
| `fn software_a_rgb` | 3 | 1 | 9 | 18 |

Total: ≈2.900 líneas de 20.574 (14 %). Medido con un script que agrupa cada bloque por hash sobre
los 47 ficheros y cuenta `líneas × (copias − 1)` del grupo mayoritario.

#### 12.2 `examples/comun/mod.rs`

```rust
#![allow(dead_code)] // cada experimento usa sólo una parte del arnés
pub mod color;
pub mod entrada;
pub mod gpu;
pub mod registro;
```

El `allow(dead_code)` va aquí y no en cada ejemplo: sin él, los 47 compilan con avisos por los
ayudantes que ese experimento concreto no usa.

#### 12.3 `examples/comun/entrada.rs`

```rust
pub const BOTONES: [&str; 10] = ["A", "B", "Select", "Start", "Right", "Left", "Up", "Down", "R", "L"];
pub struct Tramo { pub nombre: String, pub cuadros: u32, pub presionados: u16 }
pub fn leer_secuencia(texto: &str) -> Result<Vec<(u32, u16)>, String>   // forma de 44
pub fn leer_tramos(texto: &str) -> Result<Vec<Tramo>, String>           // forma de ppu_compare
fn analizar(texto: &str) -> Result<Vec<Tramo>, String>                  // parser único, privado
```

Las dos formas públicas salen del mismo `analizar`: `leer_secuencia` proyecta a
`(tramo.cuadros, tramo.presionados)`. Así los 44 ficheros cambian **sólo** el nombre con el que
llaman (`leer_secuencia(...)` → `comun::entrada::leer_secuencia(...)`) y su `for (cuadros,
presionados) in tramos` se queda igual.

#### 12.4 `examples/comun/color.rs` y `examples/comun/registro.rs`

```rust
// color.rs
pub const TOLERANCIA: u8 = 1;
pub fn canal_8bit(v: u16) -> u8;
pub fn software_a_rgb(fb: &[u16]) -> Vec<u8>;
pub fn cerca(a: &[u8], b: &[u8], p: usize) -> bool;   // forma de shim_aportacion y los nativa
pub fn cerca_pixel(a: [u8; 3], b: [u8; 3]) -> bool;   // forma de las otras 27
```

```rust
// registro.rs
pub const BIT_OBJ: u16 = 0x1000;
pub const BITS_CAPAS: u16 = 0x1F00;
pub fn io16(io: &[u8], off: usize) -> u16;
pub fn dispcnt_de(variante: usize, base: u16) -> u16;        // las 4 variantes fijas
pub fn io_con_variante(io: &[u8], variante: usize) -> Vec<u8>;
```

`cerca_pixel` es una línea encima de `cerca` (`cerca(&a, &b, 0)` con los dos arrays), y las dos
devuelven lo mismo que las copias. `dispcnt_de` con 11 variantes se queda en `ppu_capas_linea.rs`
porque no es la misma función.

#### 12.5 `examples/comun/gpu.rs`

```rust
pub const ANCHO: usize = 240;
pub const ALTO: usize = 160;
pub const FILA_GPU: u32 = (ANCHO as u32 * 4).div_ceil(256) * 256;

enum Backend { Shim(PpuShimPipeline), Nativa(PpuNativaPipeline) }

pub struct Gpu { device: wgpu::Device, queue: wgpu::Queue, backend: Backend,
                 objetivo: wgpu::Texture, vista: wgpu::TextureView, lectura: wgpu::Buffer }

impl Gpu {
    pub fn nueva() -> Result<Self, String>;        // shim: los 25 ficheros no cambian su llamada
    pub fn nueva_nativa() -> Result<Self, String>; // nativa: `PpuNativaPipeline::new` devuelve Result
    pub fn renderiza(&mut self, vram: &[u8], oam: &[u8], palette: &[u8], io: &[u8]) -> Vec<u8>;
}
```

Las dos copias sólo difieren en el tipo del pipeline (`PpuShimPipeline` frente a
`PpuNativaPipeline`), y `render_frame` tiene la misma cabecera en los dos (`&mut self`, `device`,
`queue`, ...); el enum guarda cuál es. `Gpu::nueva()` mantiene la firma de las 25 copias del shim y
los 4 de la nativa cambian sólo `Gpu::nueva()` por `Gpu::nueva_nativa()`.

#### 12.6 El cambio en cada uno de los 47

1. `#[path = "comun/mod.rs"] mod comun;` arriba, después de los `use`.
2. Borrar las copias locales (el arnés GPU, `leer_secuencia`, `BOTONES`, `canal_8bit`, `cerca`,
   `io16`, `io_con_variante`, `software_a_rgb`, y las constantes `ANCHO`/`ALTO`/`FILA_GPU`/
   `TOLERANCIA`/`BIT_OBJ`/`BITS_CAPAS` cuando existan).
3. Sustituir cada llamada por la del módulo (`comun::entrada::leer_secuencia`,
   `comun::color::cerca`, `comun::gpu::Gpu`, ...). Si se prefiere, un `use` por fichero.
4. Borrar los `use` que quedan sin uso (por ejemplo `use blazing_recomp::ppu_shim::PpuShimPipeline;`,
   que pasa a vivir en el módulo).

Los dos ficheros que **no** son mecánicos: `ppu_compare.rs` (llama a `leer_tramos` y usa `nombre`)
y `ppu_capas_linea.rs` (conserva su `dispcnt_de`).

#### 12.7 `tests/arnes_experimentos_tests.rs`

```rust
#[path = "../examples/comun/mod.rs"]
mod comun;
```

Ese `#[path]` es lo que permite que el módulo, viviendo fuera del crate, tenga tests de verdad:
`comun::entrada::leer_secuencia` se llama desde aquí como desde cualquier ejemplo. Los otros tres
escenarios leen `examples/` y `Cargo.toml` con `std::fs`, sin compilar nada.

#### 12.8 Lo que queda por comprobar después de la aprobación

- Que `#[path]` funciona en las dos direcciones (ejemplo y test).
- Que un solo `Gpu` sirve para las 29 copias.
- Que los tres volcados de muestra no cambian.
- Que `cargo build --examples --features proto` sigue saliendo limpio y que
  `scripts/diagnostico/mutar_ppu_nativa.py` sigue matando sus mutaciones.

### Correcciones previas a presentar

- El primer recuento dijo «18 variantes del arnés GPU». Estaba mal: contaba desde `struct Gpu`
  hasta `fn main()`, y en medio hay ayudantes que no son del arnés. Contando el `struct` solo, son
  **2 variantes en 29 ficheros** (25 con el shim y 4 con la PPU nativa); las 18 salían de agrupar
  también lo de en medio.
- El primer diseño tenía un único `leer_secuencia` que devolvía `Vec<Tramo>`. Eso obliga a tocar el
  `for (cuadros, presionados) in tramos` de 44 ficheros, que es exactamente lo que la tercera
  restricción prohíbe. Se cambió a dos funciones públicas sobre un parser privado.
- El criterio 1 pedía «una sola definición de `dispcnt_de`». Al mirar las tres variantes se vio que
  `ppu_capas_linea.rs` no copia: resuelve 11 variantes desde una tabla `CAPAS`. El criterio pasó a
  «exactamente una, en `ppu_capas_linea.rs`», y las otras 11 copias sí se consolidan.
- `secuencias/` **no está versionado** (`git ls-files secuencias` no devuelve nada, aunque
  `.gitignore` no lo excluye). El criterio 3 no puede depender de `escena3.txt`; usa un texto que
  escribe el test. El comando manual del §9 sí usa `secuencias/escena3.txt`, y se advierte de que
  en un clon limpio no existe.
- Se comprobó que las dos rutas del arnés arrancan en esta máquina, así que el contraste de
  volcados es factible y no una suposición: `ppu_estado` en el cuadro 60 da `0 píxeles distintos`
  y `shim_afin_linea` en el cuadro 30 escribe `volcados/afin-linea-30.txt`.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno todavía: los prototipos van después de la aprobación.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Los experimentos usan el módulo común**: coincide
  Resultado real: 47 ficheros en `examples/`, los 47 con `mod comun;`, ninguno vuelve a declarar
  `fn leer_secuencia`, `const BOTONES`, `fn canal_8bit`, `fn cerca(`, `struct Gpu`, `fn io16`,
  `fn io_con_variante` ni `fn software_a_rgb`, y `fn dispcnt_de` aparece una sola vez, en
  `ppu_capas_linea.rs`. Estado previo anotado: **rojo por la razón correcta** (47 ficheros, pero
  a todos les faltaba `mod comun;`).
  Método: el anunciado (`cargo test --test arnes_experimentos_tests
  test_los_experimentos_usan_el_modulo_comun`).
- **El inventario no cambia**: coincide, con una excepción escrita en el test
  Resultado real: los 47 nombres de `Cargo.toml` y los 47 ficheros coinciden, sin altas ni bajas.
  Sólo 45 bloques declaran `required-features = ["proto"]`: `ppu_barrido_nativa` y
  `ppu_modo_cuadro` ya compilaban sin la feature antes del cambio, y el test lo fija con la lista
  `SIN_FEATURE` en vez de exigir 47. Estado previo: verde (regresión), como estaba previsto.
  Método: el anunciado.
- **La secuencia se lee igual que antes**: coincide, y se reforzó
  Resultado real: `"# comentario\n120 ninguno\n\n60 A+B\n30 R\n"` da exactamente
  `[(120, 0x000), (60, 0x003), (30, 0x100)]`. Estado previo: verde.
  Método: el anunciado, más un caso por botón que **no** estaba en el criterio (ver Discrepancias).
- **Un botón desconocido se rechaza nombrando la línea**: coincide
  Resultado real: `"10 A+Foo\n"` da `Err("línea 1: botón desconocido 'Foo'")`. Estado previo:
  rojo (el módulo no existía).
  Método: el anunciado.

### Contexto: ¿se respetó?

- Los 47 se quedan con su nombre y su interfaz → respetada: ningún `[[example]]` cambió de nombre
  y ningún `main` cambió su número ni su orden de argumentos (el test 2 lo fija).
- El módulo vive en `examples/comun/` → respetada: `src/` no ganó módulos, `Cargo.toml` no se tocó
  y `docs/referencia/api-modulos-rust.md` sigue sincronizada (`--check` en 0).
- Sólo se consolidan los ayudantes duplicados → respetada: los 47 `main` conservan su bucle, sus
  mensajes de uso y sus rutas de volcado; el diff de cada fichero es «fuera copias + `use`».
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: tres (el `#[path]` del test, los comentarios que falseaban la poda de
  `use`, y el CRLF de la primera pasada) → criterio de prioridad aplicado: sí, primero que los 47
  sigan midiendo lo mismo.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- Los 47 de `examples/` (4.976 líneas menos: de 20.574 a 15.598).
- `examples/comun/{mod,entrada,color,registro,gpu}.rs` (nuevos, 326 líneas).
- `tests/arnes_experimentos_tests.rs` (nuevo, 207 líneas, 4 tests).
- `docs/BACKLOG.md` (índice regenerado con el script del repositorio).

¿Coincide con lo anunciado? sí, salvo que el test incluye `entrada.rs` y no `mod.rs` (ver
Discrepancias).

### Discrepancias

- El §12.7 anunciaba que el test incluiría el módulo entero
  (`#[path = "../examples/comun/mod.rs"] mod comun;`). No compila como test de integración sin
  arrastrar `gpu.rs`, que necesita wgpu y los pipelines del motor bajo la feature `proto`. El test
  incluye **sólo** `entrada.rs`, que es autocontenido, y comprueba el resto leyendo ficheros →
  suposición falsa.
- El criterio 3 no distinguía `A` de `B`: la mutación que intercambia los dos primeros nombres de
  `BOTONES` **sobrevivía**, porque `A+B` da el mismo par de bits en cualquier orden. Se añadió un
  caso por botón (los diez, con su bit) y la mutación muere → omisión del criterio, corregida
  reforzando el test, no el código.
- Dos ficheros se quedaron fuera en la primera pasada (`ppu_banda_nativa`, `ppu_capas_linea`),
  porque los había clasificado como «no mecánicos». El criterio 1 (los 47 con `mod comun;`) los
  devolvió al alcance: `ppu_capas_linea` sólo necesitaba conservar su `dispcnt_de`, y
  `ppu_banda_nativa` tenía además un `impl Gpu` con un método que nunca se llamaba, así que al
  consolidar desaparece también esa advertencia → error de juicio.
- La poda de `use` miraba el fichero entero, comentarios incluidos. Las cabeceras `//!` de tres
  experimentos citan `PpuNativaPipeline` en prosa, así que el `use` se quedaba y salía una
  advertencia por importación sin usar. Se poda mirando el código sin comentarios → error de
  implementación.
- La primera conversión escribió los 47 ficheros con finales de línea CRLF (Python en Windows
  traduce `\n` al escribir); el resto del repositorio está en LF. Se rehízo con `newline="\n"` y se
  verificó con `grep -U $'\r'` → error de implementación.
- El aviso de `ppu_tiempos` (`variable c asignada y no usada`) sigue ahí: es del `main`, que no se
  tocó, y no lo introduce este cambio.

### Para la próxima vez

Antes de anunciar un `#[path]` en un test de integración, comprobar de qué depende el módulo que se
incluye: arrastrar un fichero con GPU obliga a que el test pida la feature `proto`.

### Cómo se comprobó

| Comprobación | Resultado |
|---|---|
| `cargo test` | **285 pasados, 0 fallidos** en 25 binarios (184 de `src/` + 101 de integración) |
| `cargo build --examples --features proto` | 47 experimentos construidos |
| Advertencias antes/después | 26 líneas `warning:` idénticas, salvo una menos (el método muerto de `ppu_banda_nativa`) |
| Contraste de volcados (5 experimentos, las dos rutas) | `diff -r` **sin diferencias**: salida y 8 volcados byte a byte iguales, incluidas las dos rutas GPU |
| Mutación del arnés | **5 de 5 muertas** (quitar `mod comun;`, reintroducir `leer_secuencia`, quitar el error de botón, ignorar comentarios, desordenar `BOTONES`) |
| `validar_docs.py .` | 0 error(es), 0 aviso(s) |
| `extraer_referencias.py --check` | código 0 |
| `auditar_labels.py` | todas las etiquetas cumplen |
| Rutas absolutas en lo tocado | 0 ficheros |
| `Cargo.toml`, `src/` | sin cambios |

### Fuera de alcance (hallazgos que no se arreglaron)

- `ppu_banda_nativa`, `shim_capa_x`, `shim_vram_bg` y otros conservan campos (`Pixel.idx`,
  `Replica.entrada`) que nunca se leen. Son advertencias anteriores al cambio y arreglarlas obliga a
  tocar sus `main`. Impacto: bajo.
- El recuento de tests del checklist de `AGENTS.md` dice 264 y ya hay 285. Impacto: bajo, pero
  induce a error al cerrar una tarea.

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

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

- ninguno: los cuatro escenarios se comprueban con tests, y la identidad numérica de los volcados
  con un comando (§9), no con ojos.
- Detectado solo por el humano:
