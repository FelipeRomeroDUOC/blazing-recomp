# Propuesta: Comprobar si la PPU de referencia (gba-core) aplica la matriz de los sprites afines

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Solo lectura. No se modifica código ni se ejecuta nada que escriba en el repositorio. Se leen:

- `gba-core/src/ppu.rs`: cómo `render_scanline` trata los sprites afines (`attr0` bit 8) y si usa los parámetros pa, pb, pc, pd de la OAM.
- `examples/ppu_afin_y_linea.rs`: qué mide y si cubre sprites afines.
- `docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md`: conclusiones previas sobre afines.

Si hace falta, un ejemplo de solo lectura que compare la PPU con un sprite afín de matriz identidad frente a una matriz no identidad en el mismo cuadro. Si se necesita, se propone aparte.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_afines-conteo-matriz-7500.md` (verificada): en 7500 hay 5 afines con matriz no identidad, visibles; shim y PPU coinciden al píxel. Eso solo tiene sentido si la PPU tampoco aplica la matriz, o si ambos la aplican igual.
  - `src/ppu_shim.rs` (`decode_oam`): trata los afines como sprites normales, sin matriz.
- Intención: saber si la PPU de referencia aplica la matriz. Si no lo hace, no sirve como referencia para afines y hay que buscar otra (por ejemplo, un cálculo a mano de la transformación).
- Hipótesis:
  - H200: `gba-core::ppu` aplica pa, pb, pc, pd a los sprites afines. Si se confirma, la PPU es buena referencia y el 0 de diferencias significa que el shim también los dibuja igual, lo que contradice que ignore la matriz; habrá que explicarlo.
  - H201: `gba-core::ppu` no aplica la matriz (trata el afín como normal). Si se confirma, la referencia no sirve para afines.
  - H202: el diagnóstico previo de 10-17 ya concluye H200 o H201. Si se confirma, la conclusión se reutiliza y no hace falta medir.
- Restricciones y decisiones previas:
  - Solo lectura.
  - Sin cambios en `src/`, ni en `gba-core`.
- Alternativas descartadas:
  - Implementar la transformación afín en el shim ahora → descartada: primero hay que saber qué es la referencia.

### 3. Ejemplos

- Caso normal: `ppu.rs` contiene un bloque que, para `attr0 & 0x100`, lee `pa` en `0x06 + 0x20·grupo` y calcula coordenadas de textura → H200.
- Caso límite: `ppu.rs` solo tiene `if attr0 & 0x100 != 0 { continue; }` o trata el afín como normal → H201.

### 4. Pasos

1. Leer `gba-core/src/ppu.rs` buscando el tratamiento de sprites afines (`0x100`, `pa`, `pb`, `pc`, `pd`, `affine`).
2. Leer `examples/ppu_afin_y_linea.rs` y `docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md`.
3. Decidir H200, H201 y H202 con las citas de línea.
4. Registrar el Resultado. Si H201 se confirma, proponer la alternativa de referencia en una propuesta aparte.

### 5. Qué más podría verse afectado

- Archivos que cambian: solo esta propuesta (sección Resultado).
- Archivos que no cambian: todo el código y los informes.
- Riesgo: la lectura puede ser larga. Se lee por partes, buscando los términos, sin volcar el archivo entero al chat.

### 6. Suposiciones

- `gba-core` está en `blazing-recomp/gba-core/` (o la ruta que use el `Cargo.toml`) → prototipar: se busca el archivo; si no existe, parar y preguntar.
- El diagnóstico de 10-17 existe en la ruta indicada → prototipar: se intenta leer; si no se puede, parar y registrarlo.

### 7. Alcance

- Incluye: lectura de `ppu.rs` sobre afines; lectura del ejemplo y del diagnóstico previo; decisión de H200 a H202.
- No incluye: cambios de código; nuevas mediciones (salvo que el usuario las apruebe después); implementar afines.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se determina si la PPU aplica la matriz
Caso de uso: decidir si la referencia sirve para afines | Tipo: normal
  Dado `gba-core/src/ppu.rs`
  Cuando se busca el tratamiento de sprites afines
  Entonces el Resultado cita las líneas y dice H200 o H201
Test: no se escribe prueba nueva; la decisión queda en el Resultado
Estado previo: falla (no está decidido)

Escenario: no se modifica ningún archivo de código
Caso de uso: solo lectura | Tipo: límite
  Dado el repositorio
  Cuando termina la propuesta
  Entonces `git status` (o la lista de cambios) solo muestra esta propuesta
Test: revisión manual de los archivos modificados
Estado previo: cumple

### 9. Cómo lo comprobaré

- Revisión manual de las líneas citadas.
- Comprobación de que no hay cambios fuera de la propuesta (lista de archivos modificados).

### 10. Terminado cuando

El Resultado dice si H200, H201 o H202 se confirman o descartan, con citas de línea.

### 11. Parar si

- No se encuentra `gba-core/src/ppu.rs`: parar y pedir la ruta.
- La lectura de archivos falla de nuevo: parar y registrarlo en el Resultado.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Solo lectura; sin cambios en el repositorio.

Nota de ruta: `gba-core` no está dentro de `blazing-recomp`. Está en `../gba-recomp/crates/gba-core/src/ppu.rs` (lo indica la dependencia de `Cargo.toml`).

Lectura de `ppu.rs` (líneas 420 a 462, aproximadas; el bloque está en `render_sprites`):
- `let affine = a0 & 0x100 != 0;` (línea ~420).
- Doble tamaño solo para afines: `double = affine && a0 & 0x200 != 0`; caja de `w*2 x h*2`.
- Parámetros: `g = ((a1 >> 9) & 0x1F) * 32`; pa = OAM[g+6], pb = [g+14], pc = [g+22], pd = [g+30], como i16. Sin afín: (0x100, 0, 0, 0x100).
- Coordenadas de textura: `u = ((pa*dx + pb*dy) >> 8) + w/2`, `v = ((pc*dx + pd*dy) >> 8) + h/2`, con `dx`, `dy` desde el centro de la caja. Si sale de la textura, el píxel se omite.

Decisión:
- H200 (la PPU aplica la matriz): **confirmada**.
- H201 (la PPU no aplica la matriz): descartada.
- H202 (el diagnóstico de 17 de octubre ya lo concluye): no evaluada; no leí `docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md` ni `examples/ppu_afin_y_linea.rs`.

Consecuencia: la PPU sirve como referencia para afines.

**Contradicción pendiente:** en 7500, la PPU aplica matrices no identidad a 5 afines visibles, y shim y PPU coinciden en los 240 × 160 píxeles. Eso solo se explica si esos sprites no dejan píxeles opacos en la pantalla (por ejemplo, tiles vacíos) o si el shim también los dibuja con matriz. No se sabe cuál. Hay que contar cuántos píxeles opacos aporta cada afín en la PPU y en el shim antes de concluir nada sobre la corrección.

Próximo paso propuesto (sin aprobar): contar, para cada afín de 7500, los píxeles opacos que aporta la PPU y el shim, y comparar con la PPU sin afines. Requiere propuesta y aprobación.

---

## Revisión humana

- Que la referencia para afines se decida con este resultado: ✔ | ✘ | ?
