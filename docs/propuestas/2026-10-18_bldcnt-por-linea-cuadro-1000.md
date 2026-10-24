# Propuesta: Medir BLDCNT por línea en el cuadro 1000 para confirmar H142, sin cambiar código

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `examples/shim_mezcla_registros.rs` (ya existente, verificada), se añade la distribución por línea de `BLDCNT` para los 5.290 píxeles X: cuántos tienen el bit de backdrop (0x2000) encendido en el registro de su línea, y cuántos el bit de OBJ de primer destino (0x0010).

No cambia el shim ni la lógica de clasificación. Solo se añade una salida y una comprobación nueva en el ejemplo.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_bldcnt-bldalpha-cuadro-1000.md` (verificada por agente): `BLDCNT` de fin = `3f28`; EVA/EVB por línea cambian como un fundido; H140 confirmada; H142 no concluyente, porque solo se midió `BLDCNT` de fin.
  - `examples/shim_mezcla_registros.rs`: ya lee `capturas[l].io` para `BLDALPHA` por línea (función `dist_linea`); el mismo patrón sirve para `BLDCNT` (offset 0x50).
- Intención: saber si el bit de backdrop cambia por línea. Si está apagado en las líneas de X, la PPU no mezcla con el fondo en esas líneas, y el shim sí lo hace.
- Hipótesis:
  - H142 (revisada): en las líneas donde están los píxeles X, el bit de backdrop (0x2000) de `BLDCNT` está encendido en todas. Si se confirma, el fundido viene solo de `BLDALPHA`, y H142 queda descartada.
  - H143: el bit de backdrop se apaga en algunas líneas de X. Si se confirma, el shim aplica mal la mezcla con el fondo en esas líneas.
  - H144: el bit de OBJ de primer destino (0x0010) no importa para estos sprites (los semitransparentes mezclan sin ese bit). Si se confirma, la regla de `componer` es correcta en ese punto.
- Restricciones y decisiones previas:
  - Mismo cuadro 1000, misma definición de X (V0 ≠ V1 y V2 = V3) y mismo control (X = 5.290).
  - No se cambian grupos ni lógica; solo se añade la medición por línea.
- Alternativas descartadas:
  - Medir `BLDCNT` en todos los píxeles de la pantalla → descartada: solo importan los píxeles X.

### 3. Ejemplos

- Caso normal: un píxel X en la línea 40 con `BLDCNT` por línea = 0x3F28 (0x2000 encendido). Cuenta como backdrop encendido.
- Caso límite: un píxel X en la línea 120 con `BLDCNT` = 0x1F28 (0x2000 apagado). Cuenta en H143.

### 4. Pasos

1. Confirmar que `shim_mezcla_registros.rs` no cambió desde la última ejecución (tamaño y control X = 5.290).
2. Añadir a la salida: por cada píxel X, el `BLDCNT` de su línea; contar los valores distintos y cuántos tienen 0x2000 encendido y cuántos 0x0010 encendido.
3. Ejecutar con cuadro 1000. Salida: `volcados/mezcla-registros-cuadro-1000.txt` (se sobrescribe; se copia antes a `volcados-antes-bldcnt/`).
4. Ejecutar `cargo test --features proto --test mezcla_registros_tests` con `FE_ROM`.
5. Registrar el Resultado: H142, H143 y H144.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_mezcla_registros.rs` (solo añadidos), `volcados/mezcla-registros-cuadro-1000.txt` (se regenera; se conserva copia).
- Archivos que no cambian: `src/*`, el resto de ejemplos, pruebas y propuestas anteriores.
- Riesgo: el informe se sobrescribe. La copia de respaldo del paso 3 lo evita.

### 6. Suposiciones

- Las capturas por línea tienen `linea` igual al índice (ya usado en el ejemplo) → prototipar: sí, el ejemplo ya lo asume y el control pasa → si es falsa: parar.
- El cuadro 1000 es el mismo entre ejecuciones (determinista) → prototipar: sí, el control X = 5.290 se repite → si es falsa: parar.

### 7. Alcance

- Incluye: `BLDCNT` por línea para los píxeles X del cuadro 1000; H142 a H144; prueba; resultado.
- No incluye: otros cuadros; `BLDALPHA` (ya medido); cambios en el shim; grupos A y "ninguno" de la propuesta anterior (discrepancia abierta).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el control de X se mantiene
Caso de uso: que la medición sea válida | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control aportacion total X` vale 5290
Test: `tests/mezcla_registros_tests.rs::test_control` (se omite si no está definida `FE_ROM`)
Estado previo: pasa (medición anterior)

Escenario: se cuenta el bit de backdrop por línea
Caso de uso: decidir H142 y H143 | Tipo: normal
  Dado el informe
  Cuando se leen `X con backdrop 0x2000 encendido por linea` y `X con backdrop apagado por linea`
  Entonces su suma es 5290
Test: `tests/mezcla_registros_tests.rs::test_backdrop_por_linea` (se omite si no está definida `FE_ROM`)
Estado previo: falla (la línea no existe todavía)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_mezcla_registros -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: el informe incluye las líneas nuevas y el control vale 5.290 → no detecta: otros cuadros.
- Pruebas: `cargo test --features proto --test mezcla_registros_tests` con `FE_ROM` → pasa si: las dos pruebas pasan.

### 10. Terminado cuando

Se cumplen los criterios y el Resultado dice cuántos píxeles X tienen backdrop encendido por línea, cuántos lo tienen apagado, y si H142, H143 y H144 se confirman o descartan.

### 11. Parar si

- El control de X no vale 5.290: parar.
- Dos intentos fallidos seguidos: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Cambio solo en `examples/shim_mezcla_registros.rs` (salida nueva: `BLDCNT` por línea). Copia del informe anterior en `volcados-antes-bldcnt/`. Pruebas: `test_control` y `test_suma_por_linea` pasan.

Resultados (cuadro 1000, 5.290 píxeles X):
- Control X = 5.290: correcto.
- **BLDCNT por línea de los 5.290 X: `0F40` en todos.** Bit de backdrop (0x2000) apagado en el 100 %. Bit de OBJ primer destino (0x0010) apagado en el 100 %.
- EVA = EVB = 8 en su línea: 240 (igual que antes).

Lectura:
- **H142 descartada**: el bit de backdrop no está encendido en las línas de X. Además, la medición anterior con `3f28` era del estado de fin, que no representa lo que ve la PPU en esos píxeles. Mi lectura anterior ("backdrop encendido") era incorrecta.
- **H143 no aplica como se planteó**: el bit está apagado en todas las líneas, no en algunas. Eso es consistente con que los X tengan fondo debajo (grupo "semi con fondo" de la propuesta anterior): la mezcla con el backdrop no interviene.
- **H144 confirmada**: el bit de OBJ primer destino está apagado en todas las líneas de X, y la PPU mezcla igual. La regla de `componer` no depende de ese bit para sprites semitransparentes.

Qué queda claro: en estos píxeles, la PPU mezcla el sprite semitransparente con un fondo (target 2 = BG0 a BG3, `0F00`, y primer destino BG2 `0x40`), y el coeficiente cambia por línea (`BLDALPHA`). El shim usa un 50/50 fijo. Eso confirma H140 de la propuesta anterior.

Desviaciones: el cambio al ejemplo (BLDCNT por línea) no estaba en el plan original, pero era parte de H142 (“confirmar por línea”) y no cambia la lógica de clasificación. Anotado.

No medido: otros cuadros; si la mezcla por línea aparece en otras escenas.

---

## Revisión humana

- Que H142 (ahora por línea) y la decisión sobre el paso siguiente se acepten: ✔ | ✘ | ?
