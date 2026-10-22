# Propuesta: Buscar otra escena con BG0 activa y sprites sobre fondo vacío, para medir H110 y H111

Fecha: 2026-10-18
Nivel: 2
Estado: cerrada (sin candidatos; ver Resultado). Aprobada por instrucción del usuario: "Probemos con la 1, si no aporta nada lo damos por suficiente".

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se busca un cuadro de otra escena donde, con OBJ apagado, BG0 tenga píxeles que el shim pinte con sprites sobre píxeles vacíos de la referencia (H110 > 0). Con ese cuadro se mide H111 (la corrección de `decode_oam` los elimina).

Solo se añade una secuencia nueva (`secuencias/escena3.txt`) y mediciones. El código del shim no cambia en el paso de búsqueda. Para la medición "antes" se usa el mismo método que en la medición anterior (quitar temporalmente la comprobación de OBJ y restaurarla); esto se anota en el Resultado.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_decode-oam-otro-cuadro-regresion.md` (discrepada): el cuadro 2013 tiene H110 = 0; en la escena 2 no hay sprites sobre BG0 vacía.
  - `volcados/escaneo-obj-escena2.txt`: 7 candidatos con BG0 y sin pantalla en blanco, todos en 2013–2019, misma firma.
  - `examples/shim_escanear_obj.rs`: escaneo de solo lectura por cuadro.
- Intención: que el juego pase por pantallas distintas (menú, mapa, combate) para hallar un cuadro donde los sprites caigan sobre fondo vacío.
- Hipótesis:
  - H120: la secuencia `escena3.txt` alcanza al menos una escena nueva con OBJ apagado, BG0 activa y sprites visibles.
  - H121: en esa escena, con OBJ apagado, H110 > 0 (sprites que el shim pinta sobre BG0 vacía).
  - H122: después de la corrección, H111 = 0 en ese cuadro.
- Restricciones y decisiones previas:
  - Mismo criterio de filtro que el escaneo anterior: BG0 activa, sin pantalla en blanco, sprites visibles, sin afines ni ventana OBJ, firma distinta de 1000.
  - Si no se encuentra escena con H121 > 0 tras la secuencia, se da por suficiente la medición del cuadro 1000, como se acordó.
- Alternativas descartadas:
  - Medir sin filtro → descartada: ya se vio que da cuadros sin capas.
  - Cambiar el criterio de H110 → descartada: no se cambia la hipótesis para que el resultado pase.

### 3. Ejemplos

- Caso normal: un cuadro de menú con DISPCNT = 0x1100 pasa a 0x0100 en la medición, OBJ apagado, sprites sobre texto vacío: H110 > 0.
- Caso límite: un cuadro de escena donde BG0 cubre toda la pantalla: H110 = 0 aunque haya sprites. Se descarta.

### 4. Pasos

1. Crear `secuencias/escena3.txt` con botones variados (Start, A, B, Right, Down) en bloques de 120 cuadros, unos 4.000 cuadros.
2. Ejecutar `shim_escanear_obj` con `escena3.txt` y cuadro de referencia 1000. Salida: `volcados/escaneo-obj-escena3.txt`.
3. Elegir el primer cuadro candidato con H121 posible. Para comprobarlo, ejecutar `shim_obj_apagado` con esa secuencia y cuadro (etiqueta `prueba3`). Si H110 = 0, descartar y probar el siguiente candidato (máximo 3).
4. Si hay un cuadro con H110 > 0: medición "antes" quitando temporalmente la comprobación de OBJ en `decode_oam` (etiqueta `antes3`), restaurar, y medición "despues" (etiqueta `despues3`).
5. Registrar cifras, cuadro elegido y resultado de H120 a H122.
6. Si no hay ningún cuadro con H121 > 0 tras los 3 intentos: parar y registrar que no se encontró, sin cambios en código.

### 5. Qué más podría verse afectado

- Archivos que cambian: `secuencias/escena3.txt` (nuevo); `volcados/*` (generados); `src/ppu_shim.rs` solo de forma temporal en el paso 4 (se restaura y se verifica el tamaño, 35.362 bytes).
- Archivos que no cambian: `examples/*` existentes, `gba-core`, el juego, la APK.
- Riesgo: quitar y restaurar la comprobación en `src/ppu_shim.rs` puede dejar el archivo distinto si falla algo a mitad. Se verifica el tamaño y el contenido al final.

### 6. Suposiciones

- La ROM tiene al menos una escena adicional alcanzable con Start y A → prototipar: sí, el escaneo lo confirma → si es falsa: parar en el paso 6.
- `shim_obj_apagado` y el escaneo usan el mismo cuadro para la misma secuencia (determinismo) → prototipar: sí, la firma de 1000 se repite entre ejecuciones → si es falsa: parar.

### 7. Alcance

- Incluye: búsqueda en una secuencia nueva, verificación de H110 en hasta 3 cuadros, medición antes y después en el cuadro elegido, resultado.
- No incluye: otras escenas más allá de la búsqueda; sprites afines; ventana OBJ; BG1 y BG3; cambios permanentes en el shim; Android; APK.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se encuentra un cuadro con sprites sobre BG0 vacía
Caso de uso: H121 | Tipo: normal
  Dado el escaneo de `escena3.txt`
  Cuando se mide `bg0 candidatos sin OBJ` en el cuadro elegido
  Entonces el valor es mayor que 0
Test: `tests/obj_escena3_tests.rs::test_hay_candidatos` (se omite si no está definida `FE_ROM`)
Estado previo: falla (no existe la medición)

Escenario: la corrección elimina esos píxeles
Caso de uso: H122 | Tipo: normal
  Dado el cuadro elegido
  Cuando se compara antes y después
  Entonces `bg0 candidatos sin OBJ` pasa de mayor que 0 a 0
Test: `tests/obj_escena3_tests.rs::test_eliminados` (se omite si no está definida `FE_ROM`)
Estado previo: falla (no existe la medición)

### 9. Cómo lo comprobaré

- Escaneo: `cargo run --release --features proto --example shim_escanear_obj -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\escena3.txt escena3 1000` → pasa si: existe la salida con candidatos → no detecta: escenas que solo aparecen después de 4.000 cuadros.
- Medición: `shim_obj_apagado` con etiquetas `antes3` y `despues3` → pasa si: H121 y H122 se cumplen.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test obj_escena3_tests`.

### 10. Terminado cuando

Se cumplen los criterios, o se agotan los 3 intentos del paso 3 sin H121 (en ese caso se registra y se da por suficiente la medición del cuadro 1000).

### 11. Parar si

- El escaneo no encuentra ningún candidato nuevo: parar y registrar.
- Después del paso 4, `src/ppu_shim.rs` no tiene 35.362 bytes: parar, restaurar y avisar.
- Tres intentos sin H121: parar y registrar.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-18. Implementado: `secuencias/escena3.txt` (botones Start, A, B, Right y Down; tiempos no medidos). No se modificó código del shim.

Escaneo (`volcados/escaneo-obj-escena3.txt`): 15.125 cuadros, referencia 1000, firma de escena `ba0f0586a073c6af`. **0 cuadros candidatos** con los filtros: BG0 activa, sin pantalla en blanco, OBJ apagado, sprites visibles, sin afines ni ventana OBJ, y firma distinta de la referencia.

Por el criterio del paso 6, no se hicieron mediciones "antes" ni "después". H120 no se confirma con esta secuencia.

Nota: la firma de la referencia 1000 en esta secuencia (`ba0f...`) no es la de la secuencia de la escena 2 (`6a59...`). Son secuencias distintas, así que el cuadro 1000 no es la misma escena en ambas. No altera el resultado.

Sin cambios en `src/ppu_shim.rs` en este paso. Tampoco se escribió `tests/obj_escena3_tests.rs`.

Conclusión: no se encontró otra escena que cumpla el criterio en esta secuencia. Según lo acordado, se da por suficiente la medición del cuadro 1000 (H100 710 → 0, H101 6.240 → 0, H102 igual).

---

## Revisión humana

- Que la búsqueda de escena (H120 a H122) se dé por concluida con el resultado obtenido: ✔ | ✘ | ?
