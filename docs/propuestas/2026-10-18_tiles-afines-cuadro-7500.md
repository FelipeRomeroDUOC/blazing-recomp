# Propuesta: Comprobar por qué los 5 afines de 7500 no aportan píxeles (tiles, modo de sprite y recorte)

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (medición ejecutada; conclusión parcial, ver Resultado)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se crea un ejemplo nuevo de solo lectura en `blazing-recomp/examples/shim_afin_tiles.rs`, con su entrada en `Cargo.toml`. No cambia `src/`, ni `gba-core`, ni los ejemplos existentes.

Para cada uno de los 5 afines de 7500 (índices 0 a 4) el ejemplo lee de la OAM y de la VRAM del cuadro:

- Modo del sprite (`attr0` bits 10-11): normal, semitransparente, ventana de OBJ o prohibido.
- Profundidad de color (`attr0` bit 13: 4 o 8 bpp), tile base (`attr2` bits 0-9), paleta y prioridad.
- Mapeo de tiles de DISPCNT (bit 6: 1D o 2D).
- Número de tiles que ocupa la caja (32×64 con doble tamaño, es decir, 64×128 píxeles) y cuántos de sus píxeles tienen índice distinto de 0 (no transparentes) en la VRAM.
- Cuántos de esos píxeles caen dentro de la textura según la fórmula de la PPU (`u` y `v` entre 0 y tamaño), que es lo que la PPU realmente dibuja.

Salida: `volcados/afin-tiles-7500.txt`, con una fila por afín y su diagnóstico.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_pixeles-afines-cuadro-7500.md` (verificada): los 5 afines aportan 0 píxeles en la PPU y en el shim; la línea base coincide con el emulador.
  - `gba-recomp\crates\gba-core\src\ppu.rs`, `render_sprites` (leído en la propuesta de referencia): el modo de sprite se lee de `attr0 >> 10 & 3`; los afines usan `u = ((pa*dx + pb*dy) >> 8) + w/2` y `v = ((pc*dx + pd*dy) >> 8) + h/2`, y se omiten los píxeles fuera de `0 ≤ u < w`, `0 ≤ v < h`.
- Intención: saber si los 5 afines no dibujan porque sus tiles están vacíos, porque son de modo ventana de OBJ, o porque la matriz los saca de la textura.
- Hipótesis:
  - H220: los tiles de los 5 afines están vacíos (todos los píxeles son 0). Si se confirma, no hay nada que dibujar y el 0 de diferencias es correcto.
  - H221: los afines son de modo ventana de OBJ (`attr0` bits 10-11 = 2). Si se confirma, no dibujan color: solo marcan ventana, y el shim no lo modela.
  - H222: los tiles tienen píxeles no transparentes, el modo es normal, pero la fórmula de la PPU deja fuera todos los píxeles. Si se confirma, la matriz o el recorte son la causa.
  - H223: ninguna de las anteriores. Si se confirma, hace falta otra propuesta.
- Restricciones y decisiones previas:
  - Solo el cuadro 7500, con `secuencias\intro.txt`.
  - Sin cambios en el shim ni en `gba-core`.
- Alternativas descartadas:
  - Implementar la transformación afín en el shim ahora → descartada: si los afines no dibujan nada, no hay que implementarla todavía.

### 3. Ejemplos

- Caso normal: afín 2 con tile base 0x120, 4 bpp, modo normal, 1.024 píxeles no transparentes en la VRAM y 0 dentro de la textura → H222.
- Caso límite: afín 4 con todos los tiles en 0 → H220 para ese sprite.

### 4. Pasos

1. Crear `examples/shim_afin_tiles.rs` (`required-features = ["proto"]`), reutilizando la captura y el parseo de OAM de `shim_afin_pixeles.rs`.
2. Para cada afín: leer `attr0`, `attr1`, `attr2`, el modo, la profundidad, el tile base y el mapeo de DISPCNT; contar píxeles no transparentes en la VRAM de sus tiles; contar cuántos de esos píxeles caen dentro de la textura según la fórmula de la PPU con la matriz real.
3. Controles: la PPU con la OAM completa debe seguir coincidiendo con el emulador (0 diferencias); si no, parar.
4. Escribir `volcados/afin-tiles-7500.txt`.
5. Decidir H220 a H223 y registrar el Resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_afin_tiles.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/afin-tiles-7500.txt` (nuevo).
- Archivos que no cambian: `src/*`, `gba-core`, ejemplos existentes, pruebas, propuestas anteriores.
- Riesgo: la lectura de tiles en 8 bpp o de 2D/1D puede equivocarse. Se usa la misma fórmula de direcciones que `render_sprites` de la PPU, y el control de la PPU frente al emulador lo respalda.

### 6. Suposiciones

- La dirección de los tiles de OBJ es `0x10000 + tile*32` (4 bpp) o `0x10000 + tile*64` (8 bpp), como en `render_sprites` → prototipar: el control con la OAM completa debe dar 0 diferencias → si es falsa: parar.
- El modo de sprite está en `attr0` bits 10-11 → prototipar: los 5 valores deben estar entre 0 y 3 y coincidir con la lectura de `render_sprites` → si es falsa: parar.

### 7. Alcance

- Incluye: los 5 afines de 7500; modo, profundidad, tiles, recorte; H220 a H223; resultado.
- No incluye: implementar afines o ventanas en el shim; otros cuadros; cambios en `gba-core`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se diagnostica cada afín
Caso de uso: decidir por qué no dibuja cada afín | Tipo: normal
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces existe `volcados/afin-tiles-7500.txt` con una fila por afín: modo, profundidad, tile base, píxeles no transparentes y píxeles dentro de la textura
Test: no se escribe prueba nueva; se revisa el informe
Estado previo: falla (el ejemplo no existe)

Escenario: el control de la PPU sigue dando 0 diferencias
Caso de uso: que la lectura de tiles sea válida | Tipo: límite
  Dado la OAM completa
  Cuando se renderiza la PPU
  Entonces coincide con el framebuffer del emulador (0 diferencias)
Test: aserción en el ejemplo
Estado previo: falla

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_afin_tiles -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500`, lanzado como job con salida a fichero. Pasa si existe el informe y el control da 0.
- Revisión manual de la tabla por afín.

### 10. Terminado cuando

Existe el informe y el Resultado dice si H220, H221, H222 o H223 se confirman para cada afín.

### 11. Parar si

- El control de la PPU con la OAM completa no da 0: parar, porque la lectura no es válida.
- Los modos o tiles salen fuera de rango para todos los afines: parar y revisar la lectura.
- La herramienta falla dos veces seguidas: parar y registrarlo en el Resultado.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Ejecutado: `examples/shim_afin_tiles.rs` (nuevo), entrada en `Cargo.toml`, informe `volcados/afin-tiles-7500.txt`. Sin cambios en `src/` ni en `gba-core`.

Control: PPU con OAM completa frente al emulador: 0 diferencias. DISPCNT 1f00 (BG0-3 y OBJ activos, mapeo 2D, sin ventanas).

Por afin (0 a 4):
- Modo normal en los cinco (no es ventana de OBJ).
- 4 bpp, prioridad 1, paletas 6 a 10, textura 16x32.
- Texeles no transparentes en la textura: 63, 104, 109, 58 y 83 de 512. No todos están vacíos.
- Píxeles de la caja dentro de la textura con la matriz real: 351, 243, 203, 93 y 32.
- De ellos, no transparentes: 44, 51, 42, 5 y 12.

Decisión:
- H220 (tiles vacíos): descartada.
- H221 (ventana de OBJ): descartada.
- H222 (tiles con píxeles pero matriz fuera de textura): descartada; hay píxeles no transparentes dentro de la textura.
- H223 (ninguna de las anteriores): **confirmada**. Los afines tienen píxeles visibles según la matriz, pero la PPU no los pinta.

Hipótesis de trabajo para H223 (no verificada): los píxeles del sprite quedan tapados por fondos opacos de mayor prioridad. Los sprites tienen prioridad 1 y BG0 tiene prioridad 0. En GBA, un fondo con prioridad menor que la del sprite gana, así que un fondo opaco debajo tapa el sprite. Hay que comprobarlo contando, por afin, los píxeles no transparentes que no quedan tapados por un fondo opaco de prioridad menor.

Próximo paso propuesto (sin aprobar): contar esos píxeles no tapados por fondo de cada afin, en la PPU y el shim. Requiere propuesta y aprobación.

---

## Revisión humana

- Que la decisión sobre afines (implementar o no la transformación) se tome con estos resultados: ✔ | ✘ | ?
