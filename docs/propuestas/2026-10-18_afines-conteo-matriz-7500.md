# Propuesta: Corregir el conteo de afines por línea y mostrar su matriz y posición en el cuadro 7500

Fecha: 2026-10-18
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se modifica solo el ejemplo nuevo `examples/shim_afin_mide.rs`, que es el que escribí en la propuesta anterior:

- El conteo de afines por línea solo incluirá los sprites afines cuyo rango vertical cubre esa línea (hoy cuenta los 128 de OAM).
- Se añade un informe de cada sprite afín del cuadro 7500: índice, posición (x, y), tamaño, parámetro de matriz (pa, pb, pc, pd) y si cae en la pantalla en alguna línea.
- Se vuelve a ejecutar en 7500 y se guarda en `volcados/afin-7500.txt` (sobrescribe el anterior, cuyo conteo era erróneo).

No cambia `src/`, ni otros ejemplos, ni pruebas.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_sprites-afines-cuadro-7500.md` (discrepada): 0 diferencias shim/PPU en 7500; el conteo de afines por línea era global, no por línea.
  - Formato de OAM (GBA): el parámetro de rotación/escala de un sprite afín está en `attr1` bits 9–13 (índice de grupo 0–31). Los valores pa, pb, pc, pd están en el campo de `attr3` de cada entrada de OAM, en las posiciones 0x06 + 0x08·n, 0x0E + 0x08·n, 0x16 + 0x08·n, 0x1E + 0x08·n para el grupo n (bytes de OAM, little-endian, valores con signo de 8.8 bits).
- Intención: saber si los afines de 7500 están en pantalla y si su matriz es identidad. Con eso se decide si la diferencia 0 viene de que no se ven o de que el shim los dibuja bien.
- Hipótesis:
  - H190: los afines de 7500 no caen en ninguna línea visible (0 líneas). Si se confirma, el 0 de diferencias es vacío.
  - H191: al menos un afín cae en pantalla y su matriz no es identidad (pa ≠ 0x0100 o pd ≠ 0x0100, o pb/pc ≠ 0). Si se confirma, el shim dibuja mal ese afín y la comparación debía dar diferencias; el 0 es sospechoso.
  - H192: al menos un afín cae en pantalla con matriz identidad. Si se confirma, en esas líneas el 0 de diferencias es válido.
- Restricciones y decisiones previas:
  - Solo el cuadro 7500.
  - Mismo control de reconstrucción que antes (debe dar 0 contra el framebuffer del emulador), o no se escribe el informe.
  - Sin cambios en el shim.
- Alternativas descartadas:
  - Implementar la transformación afín ahora → descartada: primero hay que saber si los afines se ven.

### 3. Ejemplos

- Caso normal: sprite afín 2 en x=100, y=40, 16×16, pa=0x0100, pd=0x0100 → cuenta para las líneas 40 a 55 en H192.
- Caso límite: sprite afín 4 con y=170 (fuera de pantalla) → no cuenta en ninguna línea; aparece en el informe como H190.

### 4. Pasos

1. Editar `examples/shim_afin_mide.rs`: conteo por línea con rango vertical; bloque de informe por sprite afín (índice, x, y, tamaño, grupo, pa, pb, pc, pd, líneas visibles).
2. Ejecutar sobre 7500 con `secuencias\intro.txt`. Salida: `volcados/afin-7500.txt`.
3. Calcular H190, H191 y H192 a partir del informe.
4. Registrar el Resultado. Si H191 se confirma, anotar que el 0 de diferencias de la medición anterior era incorrecto y que la comparación no detecta el error.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_afin_mide.rs`, `volcados/afin-7500.txt`.
- Archivos que no cambian: `src/*`, otros ejemplos, pruebas, `Cargo.toml`.
- Riesgo: el parámetro de matriz debe leerse con el formato correcto (signo y 8.8). Si el formato está mal, H191 y H192 no son fiables. Se anota en el Resultado si se detecta.

### 6. Suposiciones

- El formato de la matriz en OAM (bytes 0x06, 0x0E, 0x16, 0x1E de cada grupo) es el de la GBA → prototipar: el informe debe mostrar pa = 0x0100 para al menos un afín o una matriz identidad coherente → si es falsa: parar y revisar el formato.
- `attr1` bits 9–13 indican el grupo de matriz → prototipar: los grupos mostrados deben ser índices 0–31 → si es falsa: parar.

### 7. Alcance

- Incluye: conteo corregido por línea; informe de afines; H190 a H192; resultado.
- No incluye: implementar afines; otros cuadros; cambios en el shim.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el conteo por línea solo incluye afines que cubren la línea
Caso de uso: que el conteo sea correcto | Tipo: normal
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces el número de afines por línea es menor o igual que 5 y varía entre líneas si los afines no cubren toda la pantalla
Test: no se escribe prueba nueva; se revisa `volcados/afin-7500.txt`
Estado previo: falla (el conteo es 5 en todas las líneas)

Escenario: el informe lista cada afín con su matriz
Caso de uso: decidir H190 a H192 | Tipo: normal
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces el informe tiene una fila por afín con x, y, tamaño, grupo, pa, pb, pc, pd
Test: no se escribe prueba nueva
Estado previo: falla (no existe el bloque)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_afin_mide -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500`, lanzado como job con salida a fichero; pasa si `volcados/afin-7500.txt` tiene el bloque de afines y el control da 0.
- Revisión manual del informe de afines.

### 10. Terminado cuando

Existe el informe con el conteo corregido y el bloque de afines, y el Resultado dice si H190, H191 y H192 se confirman o descartan.

### 11. Parar si

- El control de reconstrucción no da 0: parar, porque la referencia no es válida.
- El formato de matriz resulta incoherente (valores fuera de rango para todos los afines): parar y revisar la lectura antes de concluir.
- Dos intentos fallidos seguidos: marcar como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado con la aprobación: corrección de `examples/shim_afin_mide.rs` y nueva medición de 7500. Sin cambios en `src/`. Detalle completo en `2026-10-18_afines-conteo-matriz-7500.resultado.md`.

- Corrección: pa de cada grupo en 0x06 + 0x20·grupo (la propuesta decía 0x08·grupo; error mío).
- Control frente al framebuffer del emulador: 0 diferencias.
- Diferencias shim frente a PPU V0: 0.
- 5 afines, todos visibles en filas 24 a 87, matrices no identidad.
- H190 descartada; H191 confirmada; H192 descartada; H180 no confirmada; H181 cumplida pero trivial; H182 descartada.

Lectura: el shim coincide con la PPU en un caso con matrices no identidad. Eso no prueba que ambos las apliquen; hay que comprobar gba-core.

---

## Revisión humana

- Que la decisión sobre implementar la transformación afín se tome con estos resultados: ✔ | ✘ | ?
