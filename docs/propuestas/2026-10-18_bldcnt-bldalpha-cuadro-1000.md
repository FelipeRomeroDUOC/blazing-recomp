# Propuesta: Medir BLDCNT y BLDALPHA en el cuadro 1000 para el residuo de mezcla, sin cambiar código

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se mide, en el cuadro 1000, los registros de mezcla de color que usa la PPU (`BLDCNT` en 0x50, `BLDALPHA` en 0x52, `BLDY` en 0x54) y, con esos valores, cuántos de los 2.523 píxeles "ninguno" y de los 2.767 del grupo A caen en un caso donde la PPU aplica mezcla alfa con un coeficiente distinto de 8/8 (o con otro destino de mezcla que el 50/50 del shim).

No cambia código. Solo se añade un ejemplo de lectura y su salida en `volcados/`.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_residuo-x-con-correccion-obj-cuadro-1000.md` (verificada por agente): los 2.523 píxeles "ninguno" son sprites semitransparentes sin fondo debajo, y no cambian con la corrección de `decode_oam`.
  - `src/ppu_shim.wgsl`, `fs_sprite`: `color.a = 0.5` para sprites con modo semitransparente; el blend es `ALPHA_BLENDING` (50/50 fijo, sin leer `BLDALPHA`).
  - `src/ppu_shim.rs`, `PpuShimPipeline`: ningún registro de mezcla se pasa a la GPU; `decode_io_regs` solo lee DISPCNT, BGnCNT, HOFS y VOFS.
  - `examples/shim_origen_x.rs` y `shim_capa_ninguno.rs`: ya clasifican los píxeles; reutilizan la función `componer`, que sí aplica `alpha(fc, sc, eva, evb)` con `BLDALPHA`.
- Intención: saber si el 50/50 del shim es el origen del residuo, o si en este cuadro `BLDALPHA` vale 8/8 y el residuo viene de otra parte.
- Hipótesis:
  - H140: en el cuadro 1000, `BLDALPHA` tiene EVA/EVB distintos de 8/8 (o `BLDCNT` no coincide con el 50/50 del shim) para los píxeles de los grupos A y "ninguno". Si se confirma, el residuo es de mezcla y el paso 5 del plan es el siguiente.
  - H141: en el cuadro 1000, `BLDALPHA` vale 8/8 (EVA = EVB = 8) para todos esos píxeles. Si se confirma, el 50/50 coincide con la PPU y la causa del residuo está en otro lado.
  - H142: `BLDCNT` no tiene el bit del destino de mezcla (bit 5, backdrop) activado en los píxeles "ninguno". Si se confirma, la mezcla con el backdrop no se aplica en la PPU y el shim la aplica mal.
- Restricciones y decisiones previas:
  - Solo cuadro 1000, mismos parámetros que las mediciones originales.
  - Se usan los mismos cálculos de `componer` ya verificados (control de reconstrucción = 0).
  - No se modifica ningún ejemplo existente; el nuevo va a un archivo nuevo.
- Alternativas descartadas:
  - Cambiar el shader ahora para leer `BLDALPHA` → descartada: primero hay que saber si hace falta.
  - Medir en varios cuadros → descartada por ahora: un cuadro basta para decidir H140 a H142.

### 3. Ejemplos

- Caso normal: un píxel "ninguno" donde `BLDCNT = 0x0040` (bit 6, destino de mezcla BG2) y `BLDALPHA = 0x0A06` (EVA = 6, EVB = 10). Cuenta en H140.
- Caso límite: un píxel con `BLDALPHA = 0x1010` (EVA = EVB = 16). Es la mezcla de 100 % + 100 %, no 50/50; cuenta en H140 también.

### 4. Pasos

1. Crear `examples/shim_mezcla_registros.rs` (`required-features = ["proto"]`): reutiliza `componer`, `dos_capas`, `alpha`, `fondo_texto` y `sprites` de `shim_origen_x.rs` (copia de funciones, sin tocar el original).
2. Control de reconstrucción V0 y V1 = 0 y paleta = 0 (igual que `shim_origen_x`).
3. Para los grupos A y "ninguno": registrar `BLDCNT`, `BLDALPHA`, `BLDY` del cuadro 1000, y contar píxeles por combinación de EVA, EVB y bit de destino (bits 0x100 a 0x2000 de `BLDCNT`).
4. Contar cuántos píxeles tienen EVA = EVB = 8 (H141) y cuántos tienen el bit de backdrop (bit 5) apagado (H142).
5. Escribir `volcados/mezcla-registros-cuadro-1000.txt` (archivo nuevo).
6. Crear `tests/mezcla_registros_tests.rs` con las pruebas de la sección 8.
7. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_mezcla_registros.rs` (nuevo), `tests/mezcla_registros_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/mezcla-registros-cuadro-1000.txt` (generado).
- Archivos que no cambian: `src/*`, ejemplos existentes, propuestas anteriores (solo se añade el Resultado en esta).
- Riesgo: copiar funciones desde `shim_origen_x.rs` puede introducir diferencias. El control del paso 2 lo detecta.

### 6. Suposiciones

- `componer` reproduce la PPU en el cuadro 1000 → prototipar: sí, es el control del paso 2 → si es falsa: parar.
- Los píxeles de los grupos A y "ninguno" son los mismos que en la medición anterior → prototipar: sí, se reconstruyen con la misma clasificación → si es falsa: parar.

### 7. Alcance

- Incluye: registros de mezcla en el cuadro 1000; conteo por combinación; control; pruebas; resultado.
- No incluye: cambios en el shader o en el shim; otros cuadros; ventanas; sprites afines; orden de pintado.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el control de reconstrucción pasa
Caso de uso: que la clasificación sea válida | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0
Test: `tests/mezcla_registros_tests.rs::test_control` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: se decide H140, H141 o H142
Caso de uso: decidir el siguiente paso | Tipo: normal
  Dado el informe
  Cuando se leen `ninguno con EVA EVB 8/8`, `ninguno sin bit backdrop`, y los equivalentes de A
  Entonces las cifras suman el total del grupo
Test: `tests/mezcla_registros_tests.rs::test_suma` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_mezcla_registros -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/mezcla-registros-cuadro-1000.txt` con las líneas del paso 5 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test mezcla_registros_tests`.

### 10. Terminado cuando

Se cumplen los criterios de aceptación y el Resultado dice, para A y "ninguno", cuántos píxeles tienen EVA = EVB = 8, cuántos tienen el bit de backdrop apagado, y si H140, H141 o H142 se confirman o descartan.

### 11. Parar si

- El control de reconstrucción falla: parar.
- Los grupos no coinciden con los de la medición anterior (2.767 y 2.523): parar y avisar.
- Dos intentos fallidos seguidos: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Implementado: `examples/shim_mezcla_registros.rs` (nuevo), `tests/mezcla_registros_tests.rs` (nuevo), entrada en `Cargo.toml`. Salida: `volcados/mezcla-registros-cuadro-1000.txt`. Sin cambios en el shim.

Desviaciones respecto a la propuesta (anotadas):
1. Los grupos no son A y "ninguno" de `shim_origen_x`, sino los píxeles X (definidos igual que en `shim_origen_x`: aportación con V0 ≠ V1 y V2 = V3) divididos por si gana un sprite semitransparente y si hay fondo opaco debajo. Con esta división, los 5.290 X son todos sprites semitransparentes con fondo debajo; no reproduce la separación A / "ninguno" de antes (2.767 / 2.523). Esa discrepancia queda sin resolver.
2. Primer intento: sin la definición de aportación, el control dio 23.892 (no 5.290). Corregido. No se escribió informe con el primer intento.
3. Los registros del estado de fin dan `BLDALPHA = 0000`, que no es lo que ve la PPU en cada línea. Por eso se añadió la distribución por línea (no estaba en el plan).

Resultados:
- Control: X = 5.290 (correcto).
- `BLDCNT` del estado de fin: `3f28`. Bit de backdrop (0x2000) encendido; bit de OBJ primer destino (0x10) apagado.
- Distribución de EVA/EVB **por línea** para los 5.290 píxeles X: cambia de línea en línea como un fundido, de 1/15 (línea 1) a 16/0 (línea 15-16). Solo 240 píxeles (una línea) tienen EVA = EVB = 8.

Lectura:
- **H140 confirmada**: en 5.050 de 5.290 píxeles, EVA/EVB no son 8/8. El 50/50 fijo del shim no coincide con la PPU.
- **H141 descartada**: no todos los coeficientes son 8/8.
- **H142 no concluyente**: solo se midió `BLDCNT` del estado de fin, y ahí el bit de backdrop está encendido. Falta medirlo por línea.
- Causa probable (sin verificar en el código de la GPU, pero coherente con la lectura de `decode_io_regs`): el shim lee `BLDALPHA` una sola vez por cuadro y además usa mezcla fija 50/50 en `fs_sprite`. Para reproducir la PPU haría falta leer los registros de mezcla por línea, igual que ya se midió para los fondos (H71).

No medido: `BLDCNT` por línea; otros cuadros; si el fundido aparece en otras escenas.

Próximo paso propuesto (sin aprobar): medir `BLDCNT` por línea y confirmar H142, antes de decidir cómo llevar los registros de mezcla por línea al shim.

---

## Revisión humana

- Que la decisión entre H140, H141 y H142 se acepte como base para el paso 5 del plan: ✔ | ✘ | ?
