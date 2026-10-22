# Propuesta: Volver a medir el residuo de color "X" (2.523 píxeles) con la corrección de decode_oam, cuadro 1000

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se vuelven a ejecutar, sin cambiar código, los ejemplos ya existentes que clasificaron los píxeles "X" del cuadro 1000 (`shim_origen_x` y `shim_capa_ninguno`), con la corrección de `decode_oam` ya aplicada. Así se sabe cuántos de los 2.523 píxeles del grupo "ninguno" siguen sin explicación.

No se escribe código nuevo. Solo se ejecutan ejemplos existentes y se comparan sus cifras con las anteriores.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md` (verificada por agente): A 2.767; B 0; C 0; ninguno 2.523.
  - `docs/propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md` (verificada por agente): el grupo "ninguno" no tiene capa de fondo ni mezcla debajo; H50, H51 y H52 = 0.
  - `docs/propuestas/2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md` (verificada por agente): la corrección elimina sprites con OBJ apagado. Los 2.523 píxeles se midieron antes de la corrección.
- Intención: separar el residuo que la corrección ya explica del que queda.
- Hipótesis:
  - H130: parte de los 2.523 píxeles del grupo "ninguno" eran sprites dibujados con OBJ apagado. Si se confirma, la corrección los elimina.
  - H131: después de la corrección, el grupo "ninguno" baja de 2.523 a un número menor de 2.523 (no cero, porque la comparación se hace con el shim corregido).
  - H132: el grupo A (2.767, mezcla sprite + BG2) no cambia, porque ese grupo se midió con OBJ encendido.
- Restricciones y decisiones previas:
  - Solo cuadro 1000, mismos parámetros que las mediciones originales.
  - No se modifican ejemplos ni pruebas existentes; los resultados nuevos van a archivos nuevos o con etiqueta nueva.
- Alternativas descartadas:
  - Escribir un ejemplo nuevo para medir de cero → descartada: los existentes ya miden esto y reducen el riesgo de cambiar el método.

### 3. Ejemplos

- Caso normal: un píxel X del grupo "ninguno" con un sprite con OBJ apagado. Antes: cuenta en "ninguno". Después: desaparece del shim, así que no cuenta.
- Caso límite: un píxel X de grupo A (mezcla sprite + BG2) con OBJ encendido. No debería cambiar (H132).

### 4. Pasos

1. Confirmar que la corrección está aplicada: `src/ppu_shim.rs` tiene la condición de `dispcnt & (1 << 12)` en `decode_oam` y 35.362 bytes.
2. Ejecutar `shim_origen_x` con cuadro 1000, etiqueta nueva `despues-origen`. Salida: `volcados/origen-x-despues-cuadro-1000.txt` (si el ejemplo no admite etiqueta, se copia la salida a ese nombre y se anota).
3. Ejecutar `shim_capa_ninguno` con cuadro 1000, etiqueta nueva `despues-ninguno`.
4. Comparar cada cifra con la propuesta original (A, B, C, ninguno; H50, H51, H52).
5. Escribir `volcados/residuo-x-despues-cuadro-1000.txt` con la tabla antes/después.
6. Registrar el Resultado con H130 a H132 confirmadas o descartadas.

### 5. Qué más podría verse afectado

- Archivos que cambian: solo `volcados/*` nuevos.
- Archivos que no cambian: todo el código fuente, ejemplos, pruebas, propuestas anteriores (solo se añade el Resultado en esta propuesta).
- Riesgo: si un ejemplo no tiene opción de etiqueta, su salida anterior puede sobrescribirse. Por eso el paso 2 pide copiar la salida antes de ejecutar, o redirigir a otro nombre.

### 6. Suposiciones

- Los ejemplos existentes siguen funcionando con el shim actual → prototipar: sí, se ejecutan en el paso 2 → si es falsa: parar y avisar.
- La salida original de los ejemplos está disponible para comparar → prototipar: sí, en los volcados locales `origen-x-output.txt` y `capa-ninguno-output.txt` (fuera del repo, no versionados) → si falta: comparar con las cifras de las propuestas (sección 2).

### 7. Alcance

- Incluye: volver a ejecutar dos ejemplos existentes en el cuadro 1000; comparar cifras; resultado.
- No incluye: cambios en el shim; otros cuadros; ventanas; sprites afines; orden de pintado; mezcla de color (son los pasos 2 a 5 del plan).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el grupo "ninguno" baja con la corrección
Caso de uso: H130 y H131 | Tipo: normal
  Dado el cuadro 1000 con la corrección
  Cuando se ejecuta `shim_capa_ninguno`
  Entonces el número de píxeles del grupo "ninguno" es menor que 2.523
Test: no se escribe prueba nueva; la medición queda en `volcados/residuo-x-despues-cuadro-1000.txt`
Estado previo: 2.523 (medido antes de la corrección)

Escenario: el grupo A no cambia
Caso de uso: H132 | Tipo: límite
  Dado el cuadro 1000 con OBJ encendido
  Cuando se ejecuta `shim_origen_x`
  Entonces el grupo A sigue en 2.767 (± 0)
Test: no se escribe prueba nueva
Estado previo: 2.767

### 9. Cómo lo comprobaré

- Ejecución de `shim_origen_x` y `shim_capa_ninguno` en el cuadro 1000 → pasa si: las cifras se pueden comparar con las de la sección 2 → no detecta: otros cuadros.
- Comparación manual de la tabla de `volcados/residuo-x-despues-cuadro-1000.txt`.

### 10. Terminado cuando

Se tiene la tabla antes/después para A, B, C y "ninguno", y el Resultado dice si H130 a H132 se confirman o descartan.

### 11. Parar si

- Un ejemplo falla al compilar o ejecutar: parar y avisar.
- La salida anterior no está disponible y no se puede reconstruir: parar y avisar sin inventar cifras.
- Las cifras de A cambian sin que se espere (H132 descartada): parar y avisar antes de sacar conclusiones.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Sin cambios de código. Se copiaron antes los volúmenes en `volcados-antes-residuo/` (copia de respaldo).

| Medida (cuadro 1000) | Antes | Después |
|---|---|---|
| aportación total X | 5.290 | 5.290 |
| origen A (mezcla sprite + BG2) | 2.767 | 2.767 |
| origen B (backdrop) | 0 | 0 |
| origen C (otra paleta) | 0 | 0 |
| origen ninguno | 2.523 | 2.523 |
| ninguno con sprite semitransparente | 2.523 | 2.523 |
| ninguno con sprite no semitransparente | 0 | 0 |
| ninguno sin sprite | 0 | 0 |

Lectura:
- **H130 descartada**: ninguno de los 2.523 píxeles del grupo "ninguno" cambia con la corrección. No eran sprites con OBJ apagado: la medición se hizo con OBJ encendido y son sprites visibles.
- **H131 descartada**: el grupo "ninguno" no baja.
- **H132 confirmada**: el grupo A no cambia (2.767).

Qué dice esto: los 2.523 píxeles son todos píxeles donde gana un sprite semitransparente y no hay fondo debajo. El color del shim no coincide con la PPU. La causa más probable (sin verificar) es la mezcla: el shim usa un 50/50 fijo (`color.a = 0.5` con `ALPHA_BLENDING` en `fs_sprite`), mientras la PPU usa `BLDALPHA` (EVA/EVB) y `BLDCNT`, y el sprite se mezcla con el backdrop. Por tanto, el residuo X pasa al paso 5 del plan (mezcla de color), no al paso 1.

No medido: si el valor de `BLDALPHA` en este cuadro es 8/8 (en cuyo caso el 50/50 sería correcto y la causa estaría en otro sitio). Esto debe comprobarse antes de corregir la mezcla.

Próximo paso propuesto (sin aprobar): medir los valores de `BLDCNT` y `BLDALPHA` en el cuadro 1000 y en los dos grupos (A y ninguno), y comparar con el 50/50 del shim.

---

## Revisión humana

- Que la medición del residuo X se acepte como base para decidir el paso 2 (ventanas): ✔ | ✘ | ?
