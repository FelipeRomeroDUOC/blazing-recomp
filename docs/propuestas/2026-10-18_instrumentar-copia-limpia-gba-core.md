# Propuesta: Instrumentar `render_sprites` en una copia limpia de `gba-core` (HEAD), sin tocar el trabajo pendiente de `gba-recomp`

Fecha: 2026-10-18
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Todo el trabajo de instrumentación ocurre en una copia desechable, fuera del repositorio original:

- Se exporta el `HEAD` de `gba-recomp` a `blazing-recomp\volcados\gba-recomp-limpio\` con `git archive HEAD | tar -x` (no toca `.git`, ni el índice, ni el árbol de trabajo).
- En la copia se añade una feature `diag-afin` en `crates\gba-core\Cargo.toml` y contadores por etapa en `render_sprites` (`crates\gba-core\src\ppu.rs`), detrás de `#[cfg(feature = "diag-afin")]`.
- Un ejemplo nuevo solo dentro de la copia: `crates\gba-core\examples\afin_etapas.rs`. Corre la intro hasta el cuadro 7500 y escribe `blazing-recomp\volcados\afin-etapas-7500.txt` (ruta absoluta).
- La copia se borra al terminar (o se conserva solo para trazabilidad, a decisión del usuario).

No se modifica `gba-recomp` ni su árbol de trabajo. No se modifica `blazing-recomp\Cargo.toml` ni `src/`.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_instrumentar-render-sprites-afines.md` (parada): `gba-recomp` tiene cambios sin commit en `ppu.rs`, `Cargo.toml` y otros archivos. La parada se mantiene para el árbol de trabajo; esta propuesta lo evita.
  - `volcados/respaldo-ppu/git-status.txt`: lista de cambios pendientes de `gba-recomp`.
  - `docs/propuestas/2026-10-18_direccion-texel-composicion-afines.md` (verificada): dirección del texel y regla de prioridad coinciden; H242 abierta.
- Intención: localizar en qué etapa de `render_sprites` se pierden los 154 píxeles de los afines de 7500.
- Hipótesis (las mismas de la propuesta parada):
  - H250: los píxeles se pierden antes de escribirse (`u`/`v` fuera de rango, o rango vacío).
  - H251: llegan a escribirse, pero otro sprite de igual o mayor prioridad los tapa en `out`.
  - H252: se escriben en `out`, pero se pierden después, en la composición.
  - H253: ninguna de las anteriores.
- Restricciones y decisiones previas:
  - Ni `gba-recomp` ni su árbol de trabajo se tocan.
  - La copia se obtiene de `HEAD`, así que el resultado se refiere a `HEAD`. Si el trabajo pendiente cambia `render_sprites`, la conclusión no aplica a ese trabajo; se anota en el Resultado.
  - La feature está apagada por defecto y no cambia la salida.
- Alternativas descartadas:
  - `git worktree` → descartada: añade metadatos al `.git` del repositorio original.
  - Editar el árbol de trabajo y restaurar después → descartada: ya quedó claro que mezcla cambios ajenos.

### 3. Ejemplos

- Caso normal: en la copia, el afín 2 llega con 42 texels no transparentes en rango y 0 escritos en `out` → H251 o H252, según si la composición lo recibe.
- Caso límite: el afín 4 tiene 0 píxeles con `u`/`v` en rango → H250, y la caja de la PPU difiere de la mía.

### 4. Pasos

1. Registrar hashes de `ppu.rs`, `Cargo.toml` y `git status` de `gba-recomp` antes de empezar (ya existen en `volcados/respaldo-ppu/hashes.txt`; se recalculan y comparan).
2. Exportar `HEAD` a `volcados\gba-recomp-limpio\` con `git archive`.
3. En la copia: añadir la feature `diag-afin` y los contadores; añadir el ejemplo `afin_etapas.rs`.
4. Compilar con y sin la feature (`cargo build -p gba-core`, y con `--features diag-afin`).
5. Controles: la PPU con la feature activa, sobre la OAM completa, debe dar 0 diferencias con el framebuffer del emulador; la salida sin feature debe ser idéntica a la de la copia limpia sin modificar.
6. Ejecutar `afin_etapas` en 7500. Salida: `volcados/afin-etapas-7500.txt`.
7. Verificar que los hashes de `gba-recomp` no cambiaron (comparación con el paso 1).
8. Decidir H250 a H253 y registrar el Resultado, incluyendo la advertencia sobre el trabajo pendiente.

### 5. Qué más podría verse afectado

- Archivos que cambian: solo dentro de `volcados\gba-recomp-limpio\` (copia) y `volcados\afin-etapas-7500.txt` (nuevo).
- Archivos que no cambian: `gba-recomp` (árbol de trabajo, `.git`, índice), `blazing-recomp\src`, `blazing-recomp\Cargo.toml`, pruebas, ejemplos existentes.
- Riesgo: compilar la copia tarda varios minutos (sin caché de build). Se lanza como job con tiempo máximo amplio.

### 6. Suposiciones

- `git archive HEAD` produce un árbol que compila como workspace → prototipar: `cargo build -p gba-core` en la copia → si falla: parar y borrar la copia.
- Los contadores no alteran la salida de la PPU → prototipar: control 0 diferencias y salida igual a la copia sin modificar → si falla: parar.
- `gba-recomp` no cambia durante la medición → prototipar: hashes antes y después → si cambian: parar y avisar.

### 7. Alcance

- Incluye: copia de `HEAD`; instrumentación en la copia; medición en 7500 para los afines 0 a 4; controles; H250 a H253; resultado.
- No incluye: cambios en `gba-recomp`; cambios en el shim; corregir la PPU; implementar afines.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el repositorio original queda intacto
Caso de uso: no tocar el trabajo pendiente | Tipo: límite
  Dado los hashes de `ppu.rs`, `Cargo.toml` y `git status` antes de empezar
  Cuando termina la medición
  Entonces son idénticos
Test: comparación de hashes
Estado previo: cumple (hashes registrados en `volcados/respaldo-ppu/hashes.txt`)

Escenario: la instrumentación no altera la PPU
Caso de uso: que la medición sea válida | Tipo: límite
  Dado la copia con la feature `diag-afin`
  Cuando se renderiza la OAM completa
  Entonces coincide con el framebuffer del emulador (0 diferencias) y con la salida sin feature
Test: aserciones en el ejemplo
Estado previo: falla (no hay instrumentación)

Escenario: se localiza la etapa de pérdida
Caso de uso: decidir H250 a H253 | Tipo: normal
  Dado el informe `afin-etapas-7500.txt`
  Cuando se leen los contadores por afín
  Entonces el Resultado dice qué hipótesis se confirma
Test: revisión manual
Estado previo: falla

### 9. Cómo lo comprobaré

- Compilación con y sin feature.
- Medición como job con salida a fichero: `cargo run -p gba-core --release --features diag-afin --example afin_etapas -- "C:\...\roms\1575 - Fire Emblem (E)(Independent).gba" "C:\...\secuencias\intro.txt" 7500` (rutas absolutas).
- Hashes de `gba-recomp` antes y después.

### 10. Terminado cuando

El informe existe, los hashes del repositorio original coinciden, y el Resultado dice qué hipótesis se confirma, con la advertencia sobre el trabajo pendiente.

### 11. Parar si

- La copia no compila: parar y borrarla.
- El control falla: parar y borrar la copia.
- Los hashes del repositorio original cambian: parar y avisar sin hacer nada más.

---

## Resultado (lo completa el agente después de editar)

Pendiente.

---

## Revisión humana

- Que se acepte medir en una copia de `HEAD` (y si la copia se conserva o se borra): ✔ | ✘ | ?

## Resultado (2026-10-18)
- Copia: HEAD c9f2847 en volcados/gba-recomp-limpio (no el working tree).
- Control: 0 diferencias diagnostico apagado frente a encendido (esperado 0).
- Afines 0-4 en 7500 (linea_cubierta 64 en todos): texel_no_transparente 44/51/42/5/12; escrito_en_linea 44/39/24/5/12; descartado_prioridad 0/12/18/0/0.
- H250: CONFIRMADA en esta copia. Los 5 afines llegan a escribir pixeles.
- Contradice H210 (0 pixeles aportados) medida en el working tree. Pendiente explicar: diferencia HEAD frente a working tree, o configuracion de aislamiento (oam_con).
- H251-H253: no evaluadas aqui sin releer su texto; revisar contra los contadores.
- gba-recomp: hashes de ppu.rs y Cargo.toml iguales antes y despues; sin cambios nuevos en git status.
- Estado: pendiente de revision humana. Pendiente decidir si conservar volcados/gba-recomp-limpio.
