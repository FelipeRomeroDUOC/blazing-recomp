# Propuesta: Comprobar si el fundido de BLDALPHA aparece en otros cuadros y escenas, sin cambiar código

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se mide con `shim_mezcla_registros` en otros cuadros (4 cuadros: 500, 2500, 4750 de la intro y 2013 de la escena 2) si los píxeles X tienen EVA/EVB por línea con el mismo patrón de fundido que en el cuadro 1000, o si ese patrón es particular de ese cuadro.

Cambio en el ejemplo: solo que el control de X (5.290) pase a ser informativo cuando se le pasa un cuadro distinto de 1000. Se mantiene la comprobación del total de X contra la suma por línea. No se cambia ninguna lógica de clasificación.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_bldcnt-por-linea-cuadro-1000.md` (verificada): en el cuadro 1000, EVA/EVB cambian por línea (1/15 a 16/0), y BLDCNT por línea es `0F40` en los X.
  - `examples/shim_mezcla_registros.rs`: el control `ESPERADO_X = 5290` es fijo; para otros cuadros fallaría aunque la medición sea válida.
  - `volcados/cuadro-500-*`, `cuadro-2500-*`, `cuadro-4750-*`, `cuadro-10000-*`: ya existen cuadros de la intro con volcados; sirven como referencia de qué cuadros tienen contenido.
  - `volcados/escaneo-obj-escena2.txt`: cuadros 2013-2019 de la escena 2 con BG0 activa y sprites.
- Intención: saber si el fundido de `BLDALPHA` es un patrón de la escena o un caso aislado del cuadro 1000. Eso decide si vale la pena llevar los registros por línea al shim.
- Hipótesis:
  - H150: en al menos 2 de los 5 cuadros con X > 0, los EVA/EVB de X cambian por línea (no son constantes). Si se confirma, el fundido es habitual.
  - H151: en los cuadros con X > 0, la mayoría de píxeles X tienen EVA = EVB = 8 o un valor constante por línea. Si se confirma, el fundido es propio del cuadro 1000.
  - H152: el fundido aparece en la escena 2 (cuadro 2013) con el mismo patrón de rampa que en el cuadro 1000.
- Restricciones y decisiones previas:
  - Se usan solo los 5 cuadros de la lista. No se buscan más.
  - Se mantiene la definición de X de `shim_origen_x`.
  - Sin cambios en el shim.
- Alternativas descartadas:
  - Medir todos los cuadros de la intro → descartada: consume mucho tiempo y el resultado ya se puede decidir con 5 cuadros.

### 3. Ejemplos

- Caso normal: en el cuadro 2500, los X están en 20 líneas con EVA/EVB cambiando de 4/12 a 12/4. Cuenta en H150.
- Caso límite: en el cuadro 500 no hay X (0 píxeles). No cuenta para H150 ni para H151; se anota como "sin X".

### 4. Pasos

1. Hacer que el control de X sea parámetro: si se pasa un cuadro distinto de 1000, no se compara con 5.290; se imprime el total. (Cambio pequeño en `shim_mezcla_registros.rs`.)
2. Ejecutar `shim_mezcla_registros` en cada uno de los 5 cuadros (intro: 500, 2500, 4750; escena 2: 2013). Salidas en `volcados/mezcla-registros-cuadro-<N>.txt` (un archivo por cuadro, sin sobrescribir el de 1000). Para el cuadro 2013 se usa `secuencias\escena2.txt`.
3. Para cada cuadro: total de X, número de líneas con X, distribución de EVA/EVB por línea, y si EVA cambia de una línea a otra.
4. Escribir `volcados/fundido-resumen.txt` con una fila por cuadro.
5. Registrar el Resultado con H150, H151 y H152.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_mezcla_registros.rs` (un parámetro opcional), `volcados/mezcla-registros-cuadro-<N>.txt` (nuevos), `volcados/fundido-resumen.txt` (nuevo).
- Archivos que no cambian: `src/*`, `volcados/mezcla-registros-cuadro-1000.txt`, pruebas existentes.
- Riesgo: si un cuadro no tiene X, la comparación no se puede hacer. Se anota y no cuenta.

### 6. Suposiciones

- Los 5 cuadros son cuadros de juego válidos (no pantallas en blanco) → prototipar: sí, el escaneo previo confirmó contenido en 2013 y los volcados de la intro tienen imagen → si es falsa: se anota y se usa otro cuadro de la misma escena.
- La secuencia de escena 2 llega al cuadro 2013 de forma determinista → prototipar: sí, la firma se repitió en la búsqueda → si es falsa: parar.

### 7. Alcance

- Incluye: los 5 cuadros; EVA/EVB por línea; resumen; resultado de H150 a H152.
- No incluye: cambios en el shim; BLDCNT por línea en otros cuadros (ya se vio que es `0F40` en el 1000); ventanas; sprites afines; otras escenas.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el control de X del cuadro 1000 sigue pasando
Caso de uso: que el cambio no rompa la medición base | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control aportacion total X` vale 5290
Test: `tests/mezcla_registros_tests.rs::test_control` (se omite si no está definida `FE_ROM`)
Estado previo: pasa

Escenario: hay un resumen con los 5 cuadros
Caso de uso: decidir H150 a H152 | Tipo: normal
  Dado el resumen
  Cuando se leen sus filas
  Entonces hay una fila por cuadro, con total de X y si EVA cambia por línea
Test: no se escribe prueba nueva; el resumen queda en `volcados/fundido-resumen.txt`
Estado previo: falla (no existe)

### 9. Cómo lo comprobaré

- Ejecución de `shim_mezcla_registros` en los 5 cuadros → pasa si: existen los 5 informes → no detecta: otras escenas.
- Revisión manual de `volcados/fundido-resumen.txt`.
- Pruebas: `cargo test --features proto --test mezcla_registros_tests` con `FE_ROM` → pasa si: `test_control` pasa.

### 10. Terminado cuando

Hay 5 informes y el resumen, y el Resultado dice si H150, H151 y H152 se confirman o descartan.

### 11. Parar si

- El control del cuadro 1000 falla después del cambio: parar y revertir el parámetro.
- Un cuadro falla al ejecutarse: anotarlo y seguir con los demás; si fallan 3, parar.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Cambio en `examples/shim_mezcla_registros.rs`: el control de 5.290 solo se aplica al cuadro 1000. Tras el cambio, el cuadro 1000 sigue dando X = 5.290 y `test_control` y `test_suma_por_linea` pasan. Resumen en `volcados/fundido-resumen.txt`.

| Cuadro | Escena | X | Fundido por línea | BLDALPHA fin |
|---|---|---|---|---|
| 500 | intro | 0 | - | 030d |
| 1000 | intro | 5.290 | sí (1/15 a 16/0) | 0000 |
| 2500 | intro | 6.476 | sí (1/15 a 16/0) | 1000 |
| 4750 | intro | 0 | - | 1010 |
| 2013 | escena 2 | 0 | - | 0010 |

Lectura:
- **H150 confirmada**: 2 de 2 cuadros con X tienen el fundido, con la misma rampa (1/15 a 16/0). Evidencia limitada: solo 2 de los 5 cuadros tienen píxeles X.
- **H151 descartada**: el cuadro 2500 tiene el mismo patrón de fundido que el 1000, así que no es propio de un cuadro.
- **H152 no evaluable**: el cuadro 2013 de la escena 2 no tiene píxeles X, así que no se puede comparar.

Desviación: el control de 5.290 se restringió al cuadro 1000 (paso 1 del plan, previsto).

No medido: más cuadros con X; otras escenas con sprites semitransparentes sobre fondo.

Próximo paso propuesto (sin aprobar): buscar más cuadros con X para tener más evidencia de H150, antes de decidir llevar los registros de mezcla por línea al shim.

---

## Revisión humana

- Que la decisión sobre llevar los registros de mezcla por línea al shim se tome con estos resultados: ✔ | ✘ | ?
