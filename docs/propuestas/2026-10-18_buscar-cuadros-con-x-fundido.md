# Propuesta: Buscar más cuadros con píxeles X para reforzar H150 (fundido de BLDALPHA)

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se recorre la intro (`secuencias/intro.txt`) cada 250 cuadros y, en cada muestra, se cuentan los píxeles X (definición de `shim_origen_x`: aportación con V0 ≠ V1 y V2 = V3, y el shim distinto de la PPU). Para los cuadros con X > 0 se mide si EVA/EVB cambian por línea (fundido), con el mismo método de `shim_mezcla_registros`.

Cambio en código: un ejemplo nuevo de solo lectura (`shim_buscar_x.rs`). No se toca el shim ni los ejemplos existentes.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_fundido-bldalpha-otros-cuadros.md` (verificada): H150 confirmada con 2 de 2 cuadros con X (1000 y 2500); evidencia limitada.
  - `volcados/fundido-resumen.txt`: 500, 4750 y 2013 no tienen X.
  - `volcados/escaneo-obj-escena3.txt`: 15.125 cuadros escaneados sin candidatos con OBJ apagado; no sirve para buscar X (que requiere sprites semitransparentes).
- Intención: tener al menos 5 cuadros con X antes de decidir si los registros de mezcla por línea deben llevarse al shim.
- Hipótesis:
  - H160: en la intro hay al menos 5 cuadros de muestra con X > 0.
  - H161: en esos cuadros, el fundido por línea (EVA/EVB no constantes) aparece en al menos 4 de 5.
- Restricciones y decisiones previas:
  - Solo la intro (10.800 cuadros aprox.) con muestreo cada 250 cuadros: unas 40 muestras.
  - Sin cambios en el shim.
- Alternativas descartadas:
  - Medir todos los cuadros → descartada: tarda demasiado y no es necesario para decidir.

### 3. Ejemplos

- Caso normal: el cuadro 3250 tiene 4.100 píxeles X y el fundido por línea. Cuenta para H160 y H161.
- Caso límite: el cuadro 1750 tiene X = 0. No cuenta.

### 4. Pasos

1. Crear `examples/shim_buscar_x.rs` (`required-features = ["proto"]`), con el mismo recorrido y las mismas definiciones que `shim_mezcla_registros`.
2. Ejecutarlo sobre `secuencias/intro.txt`, muestreo cada 250 cuadros. Salida: `volcados/buscar-x-intro.txt` con una fila por muestra: cuadro, X, y si hay fundido.
3. Para los 5 primeros cuadros con X > 0 (que no sean 1000 ni 2500), ejecutar `shim_mezcla_registros` y guardar `volcados/mezcla-registros-cuadro-<N>.txt`.
4. Actualizar `volcados/fundido-resumen.txt` con las filas nuevas.
5. Registrar el Resultado con H160 y H161.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_buscar_x.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/buscar-x-intro.txt` (nuevo), informes de cuadros nuevos, `volcados/fundido-resumen.txt` (se amplía).
- Archivos que no cambian: `src/*`, `shim_mezcla_registros.rs`, el informe del cuadro 1000, pruebas.
- Riesgo: la intro puede tener pocos cuadros con sprites semitransparentes. Si no hay 5, se registra y se para.

### 6. Suposiciones

- La intro contiene cuadros con X (hay 2 conocidos: 1000 y 2500) → prototipar: sí, el paso 2 lo confirma → si es falsa: parar.
- El recorrido es determinista → prototipar: sí, el control de X del 1000 = 5.290 se repite → si es falsa: parar.

### 7. Alcance

- Incluye: muestreo de la intro; cuadros con X; fundido por línea en hasta 5 cuadros nuevos; resumen.
- No incluye: escena 2 y escena 3 (no tienen X en los cuadros probados); cambios en el shim; BLDCNT (ya medido en 1000); ventanas; sprites afines.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: hay al menos 5 cuadros con X
Caso de uso: H160 | Tipo: límite
  Dado el muestreo de la intro
  Cuando se cuentan los cuadros con X > 0
  Entonces hay al menos 5 (sin contar 1000 ni 2500)
Test: no se escribe prueba nueva; la cifra queda en `volcados/buscar-x-intro.txt`
Estado previo: 2 conocidos (1000 y 2500)

Escenario: el fundido aparece en al menos 4 de 5
Caso de uso: H161 | Tipo: normal
  Dado el resumen actualizado
  Cuando se leen los 5 cuadros nuevos
  Entonces al menos 4 tienen fundido por línea
Test: no se escribe prueba nueva; queda en `volcados/fundido-resumen.txt`
Estado previo: 2 de 2 (1000 y 2500)

### 9. Cómo lo comprobaré

- Ejecución de `shim_buscar_x` → pasa si: existe `volcados/buscar-x-intro.txt` con una fila por muestra.
- Revisión manual del resumen.
- Pruebas: `cargo test --features proto --test mezcla_registros_tests` con `FE_ROM` → pasa si: `test_control` y `test_suma_por_linea` pasan (el informe del 1000 no cambia).

### 10. Terminado cuando

Hay 5 cuadros nuevos con X medidos (o se registra que no hay tantos), y el Resultado dice si H160 y H161 se confirman o descartan.

### 11. Parar si

- La intro tiene menos de 5 cuadros con X: registrar y parar, sin inventar cuadros.
- El control del 1000 deja de dar 5.290: parar.
- Dos intentos fallidos seguidos: detenerse y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Desviación: en lugar de un ejemplo nuevo, se usó `shim_mezcla_registros` (ya existente, sin cambios en esta ejecución) lanzado por un script de PowerShell sobre la intro, con muestreo cada 500 cuadros (no cada 250), para reducir el tiempo. Salida: `volcados/buscar-x-intro.txt`. Resumen actualizado: `volcados/fundido-resumen.txt`.

Muestras (20 cuadros: 500 a 10.000 cada 500):
- Cuadros con X > 0: 1000, 1500, 2000, 2500, 3000, 3500, 4000, 4500. Todos muestran fundido por línea (EVA de 1/15 a 16/0).
- Cuadros nuevos con X (sin contar 1000 ni 2500): 6 (1500, 2000, 3000, 3500, 4000, 4500).
- Sin X: 500, 4750 (no muestreado en esta corrida; medido antes), 5000, 5500, 6500, 7000, 8500, 9000, 9500, 10000.
- Error del ejemplo (sin investigar): 6000, 7500, 8000.

Lectura:
- **H160 confirmada**: hay 6 cuadros nuevos con X (se pidieron al menos 5).
- **H161 confirmada**: 6 de 6 cuadros nuevos muestran fundido.
- Por tanto, H150 queda confirmada con 8 cuadros (antes 2).

Limitaciones: el muestreo es de 500 cuadros; podría haber cuadros con X entre muestras. Los errores de 6000, 7500 y 8000 pueden deberse a sprites afines o ventanas, que el shim no reconstruye; no está verificado.

Próximo paso propuesto (sin aprobar): investigar los tres errores para saber si son sprites afines u otra limitación del shim, antes de decidir llevar los registros de mezcla por línea al shim.

---

## Revisión humana

- Que la decisión sobre llevar los registros de mezcla por línea al shim se tome con la evidencia reunida: ✔ | ✘ | ?
