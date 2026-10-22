# Propuesta: Contar los píxeles de cada afín de 7500 que no quedan tapados por un fondo opaco de mayor prioridad

Fecha: 2026-10-18
Nivel: 2
Estado: discrepada (medición ejecutada; resultado contradice la hipótesis de partida, ver Resultado)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se crea un ejemplo nuevo de solo lectura en `blazing-recomp/examples/shim_afin_tapado.rs`, con su entrada en `Cargo.toml`. No cambia `src/`, ni `gba-core`, ni los ejemplos existentes.

Para cada uno de los 5 afines de 7500, el ejemplo recorre los píxeles de su caja que caen dentro de la textura con la matriz real y tienen texel no transparente (igual que en `shim_afin_tiles`). Para cada uno de esos píxeles en pantalla comprueba si lo tapa un fondo:

- Un fondo tapa al sprite si está habilitado en DISPCNT, es opaco en ese píxel y su prioridad (BGnCNT bits 0-1) es **estrictamente menor** que la del sprite (1). En GBA, a igual prioridad el sprite gana.
- Cuenta dos cosas por afín: píxeles tapados y píxeles no tapados.

Salida: `volcados/afin-tapado-7500.txt`.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_tiles-afines-cuadro-7500.md` (verificada, H223): los afines tienen texels no transparentes dentro de la textura y la PPU no los pinta. Sprites con prioridad 1; BG0 con prioridad 0.
  - `docs/propuestas/2026-10-18_pixeles-afines-cuadro-7500.md` (verificada): la PPU aporta 0 píxeles de los 5 afines, y el shim también.
  - `examples/shim_mezcla_registros.rs`: la función `fondo_texto` (lectura de capas de texto con los bits de BGnCNT) puede copiarse tal cual.
- Intención: separar dos causas posibles. Si todos los píxeles visibles del afín están tapados por fondos, el 0 es correcto (H230). Si hay píxeles no tapados, la PPU debería pintarlos y la causa está en otra parte (H231).
- Hipótesis:
  - H230: todos los píxeles no transparentes de cada afín quedan tapados por fondos opacos de prioridad menor. Si se confirma, el 0 de la PPU es correcto y el shim no tiene que dibujarlos todavía.
  - H231: hay píxeles no transparentes no tapados en al menos un afín. Si se confirma, la PPU debería pintarlos y el 0 de la medición anterior es un error de la PPU o de cómo se mide.
  - H232: mezcla, algunos afines con todos tapados y otros con píxeles no tapados. Si se confirma, hay que revisar por afín.
- Restricciones y decisiones previas:
  - Solo el cuadro 7500, con `secuencias\intro.txt`.
  - Sin cambios en el shim ni en `gba-core`.
  - Se aplica la regla de GBA de prioridad (estrictamente menor para tapar), no la contraria.
- Alternativas descartadas:
  - Medir dentro del shim con más capas de salida → descartada: primero hay que contar fuera del shim, con la regla simple.

### 3. Ejemplos

- Caso normal: un píxel del afín 0 tiene texel 5, BG0 opaco con prioridad 0 → tapado.
- Caso límite: un píxel del afín 2 tiene texel 3 y BG0 transparente, BG1 opaco con prioridad 1 → no tapado (igual prioridad, gana el sprite), y la PPU debería pintarlo.

### 4. Pasos

1. Crear `examples/shim_afin_tapado.rs` (`required-features = ["proto"]`), copiando `fondo_texto` de `shim_mezcla_registros.rs` y el recorrido de texels de `shim_afin_tiles.rs`.
2. Para cada afín, contar: píxeles no transparentes dentro de la textura y en pantalla; de ellos, tapados y no tapados.
3. Controles: la PPU con la OAM completa debe coincidir con el emulador (0 diferencias); las capas de fondo de la línea deben coincidir con las de la PPU en los píxeles no tapados donde el sprite no aparece (se compara el color de fondo que la PPU muestra en la línea base).
4. Escribir `volcados/afin-tapado-7500.txt`.
5. Decidir H230, H231 y H232 y registrar el Resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_afin_tapado.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/afin-tapado-7500.txt` (nuevo).
- Archivos que no cambian: `src/*`, `gba-core`, ejemplos existentes, pruebas, propuestas anteriores.
- Riesgo: la regla de prioridad se puede aplicar mal (por ejemplo, con igual prioridad). Se controla con la comparación de la PPU y se anota en el Resultado si H231 aparece por un error de regla.

### 6. Suposiciones

- Con igual prioridad, el sprite gana al fondo en GBA → prototipar: si el informe sale con más píxeles no tapados por igualdad que por desigualdad, revisar la regla antes de concluir → si es falsa: parar.
- Los fondos del modo 0 son texto (no afines) → prototipar: DISPCNT 1f00 indica modo 0 → si es falsa: parar.

### 7. Alcance

- Incluye: los 5 afines; tapado por fondo con prioridad estrictamente menor; H230 a H232; resultado.
- No incluye: implementar la regla de prioridad en el shim; sprites de otros cuadros; ventanas.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se cuentan píxeles tapados y no tapados de cada afín
Caso de uso: decidir si los fondos explican el 0 | Tipo: normal
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces existe `volcados/afin-tapado-7500.txt` con una fila por afín: píxeles no transparentes, tapados y no tapados
Test: no se escribe prueba nueva; se revisa el informe
Estado previo: falla (el ejemplo no existe)

Escenario: el control de la PPU sigue dando 0 diferencias
Caso de uso: que la medición sea válida | Tipo: límite
  Dado la OAM completa
  Cuando se renderiza la PPU
  Entonces coincide con el framebuffer del emulador
Test: aserción en el ejemplo
Estado previo: falla

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_afin_tapado -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500`, lanzado como job con salida a fichero. Pasa si existe el informe y el control da 0.
- Revisión manual de la tabla por afín y de tres píxeles de ejemplo.

### 10. Terminado cuando

Existe el informe y el Resultado dice si H230, H231 o H232 se confirman.

### 11. Parar si

- El control de la PPU no da 0: parar.
- Ningún píxel de fondo se detecta como opaco: parar y revisar `fondo_texto`.
- La herramienta falla dos veces seguidas: parar y registrarlo en el Resultado.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Ejecutado: `examples/shim_afin_tapado.rs` (nuevo), entrada en `Cargo.toml`, informe `volcados/afin-tapado-7500.txt`. Sin cambios en `src/` ni en `gba-core`.

Control: PPU con OAM completa frente al emulador: 0 diferencias. DISPCNT 1f00, prioridad del sprite 1.

Resultados (píxeles con color en pantalla / tapados por fondo / no tapados):
- Afin 0: 44 / 0 / 44
- Afin 1: 51 / 0 / 51
- Afin 2: 42 / 0 / 42
- Afin 3: 5 / 0 / 5
- Afin 4: 12 / 0 / 12

Decisión:
- H230 (todo tapado por fondos): **descartada**. Ningún píxel está tapado.
- H231 (hay píxeles no tapados): **confirmada**. Hay 154 píxeles no tapados en total que, según la regla, la PPU debería pintar.
- H232: no aplica.

**Contradicción:** la PPU aporta 0 píxeles de estos afines (medido en la propuesta anterior). Por tanto, la PPU no pinta 154 píxeles que debería pintar según esta regla. Causas posibles, sin verificar:
1. Mi `texel()` usa la dirección de tiles de sprites normales, y la PPU usa otra para los afines. Si fuera así, mis píxeles no son los de la PPU.
2. La PPU recorta o salta esos píxeles en el camino de composición (por ejemplo, en la vía recta), no en `render_sprites`.
3. Hay otra condición que no he modelado en la regla de tapado (por ejemplo, la prioridad de OBJ frente a fondo en la PPU).

Próximo paso propuesto (sin aprobar): leer en `gba-core/src/ppu.rs` cómo se calcula la dirección del texel en el camino afin (después de la línea 'Texture coordinates') y cómo llega el píxel de sprite a la composición. Comparar con `texel()` de `shim_afin_tiles.rs`. Requiere propuesta y aprobación.

---

## Revisión humana

- Que la decisión sobre los afines (dibujarlos o no en el shim) se tome con estos resultados: ✔ | ✘ | ?
