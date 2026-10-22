# Propuesta: Medir si el shim dibuja mal los sprites afines en el cuadro 7500, sin cambiar código

Fecha: 2026-10-18
Nivel: 2
Estado: discrepada (medición ejecutada; el conteo de afines por línea es erróneo, ver Resultado)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se crea un ejemplo nuevo de solo lectura (`examples/shim_afin_mide.rs`) que compara, en el cuadro 7500 de la intro, el shim completo con la PPU por software, pixel a pixel, sin usar la función `sprites()` (que no reconstruye afines). Para cada línea cuenta cuántos sprites afines hay en OAM y cuántos píxeles difieren. Si las diferencias se concentran en las líneas con afines, el shim los dibuja mal.

No cambia el shim ni los ejemplos existentes.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_causa-errores-6000-7500-8000.md` (verificada): el cuadro 7500 falla por un sprite afín (sprite 0, línea 0).
  - `src/ppu_shim.rs`, `decode_oam` (leído antes): los sprites con `attr0 & 0x100` (afines) entran en la lista con su posición, tamaño y tile, pero sin matriz de transformación; `flags` no incluye rotación ni escala.
  - `src/ppu_shim.rs`, `decode_oam`: `is_affine` se usa para desactivar hflip y vflip, pero no se aplica ninguna transformación.
- Intención: saber si los sprites afines se dibujan mal en el shim (y cuánto), antes de decidir si implementar la transformación.
- Hipótesis:
  - H180: en el cuadro 7500, las líneas con sprites afines tienen más diferencias entre shim y PPU que las líneas sin afines.
  - H181: en las líneas con afines, el número de píxeles distintos supera 0 en todas; las líneas sin afines tienen 0 diferencias. Si se confirma, el error es solo de afines.
  - H182: hay diferencias fuera de las líneas con afines. Si se confirma, hay otra causa además de los afines.
- Restricciones y decisiones previas:
  - Solo el cuadro 7500.
  - La comparación es el shim completo (sin reconstrucción de capas) frente a la PPU por software, con la misma definición de diferencia que `shim_origen_x` (±1 por canal).
  - Sin cambios en código existente.
- Alternativas descartadas:
  - Implementar la transformación afín ahora → descartada: primero hay que medir cuánto error aporta.

### 3. Ejemplos

- Caso normal: la línea 40 tiene un sprite afín y 25 píxeles distintos. Cuenta en H180.
- Caso límite: la línea 100 no tiene afines y 0 diferencias. Cuenta en H181.

### 4. Pasos

1. Crear `examples/shim_afin_mide.rs` (`required-features = ["proto"]`). Reutiliza el recorrido de la intro, el `Gpu` y la lectura de OAM de `shim_mezcla_registros`, copiados.
2. Ejecutar sobre el cuadro 7500 con `secuencias\intro.txt`. Salida: `volcados/afin-7500.txt` con: total de píxeles distintos; por línea, número de sprites afines (`attr0 & 0x100`) y número de píxeles distintos.
3. Calcular: diferencias en líneas con afines, diferencias en líneas sin afines (H180, H181, H182).
4. Registrar el Resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_afin_mide.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/afin-7500.txt` (nuevo).
- Archivos que no cambian: `src/*`, ejemplos existentes, pruebas, propuestas anteriores.
- Riesgo: la comparación del shim completo incluye capas y ventanas que el shim no modela; las diferencias de esas capas pueden contaminar el resultado. Se anota como limitación, y H182 existe para detectarlo.

### 6. Suposiciones

- La PPU por software es la referencia correcta en el cuadro 7500 → prototipar: sí, es la referencia usada en todas las medidas anteriores → si es falsa: parar.
- El cuadro 7500 se reproduce igual entre ejecuciones → prototipar: sí, el mensaje de error fue idéntico en dos ejecuciones → si es falsa: parar.

### 7. Alcance

- Incluye: cuadro 7500; diferencias por línea; afines por línea; H180 a H182; resultado.
- No incluye: implementar afines; ventanas; otros cuadros (6000 y 8000 quedan para después); cambios en el shim.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el ejemplo corre sin fallar en el cuadro 7500
Caso de uso: que la medición exista | Tipo: límite
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces existe `volcados/afin-7500.txt` con el total de píxeles distintos
Test: no se escribe prueba nueva; el resultado queda en el archivo
Estado previo: falla (el ejemplo no existe)

Escenario: se decide H180, H181 y H182
Caso de uso: decidir si el error es de afines | Tipo: normal
  Dado el informe
  Cuando se comparan las líneas con y sin afines
  Entonces se puede decidir H180, H181 y H182
Test: no se escribe prueba nueva
Estado previo: falla

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_afin_mide -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500` → pasa si: existe `volcados/afin-7500.txt` con las líneas del paso 2.
- Revisión manual de la tabla de líneas.

### 10. Terminado cuando

Existe el informe y el Resultado dice si H180, H181 y H182 se confirman o descartan.

### 11. Parar si

- La PPU por software no coincide con la referencia del cuadro (control de reconstrucción no nulo): parar, porque la comparación no es válida.
- Dos intentos fallidos seguidos: detenerse y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Ejecutado: `examples/shim_afin_mide.rs` (nuevo), entrada en `Cargo.toml`, informe `volcados/afin-7500.txt`. Sin cambios en `src/`.

- Control de reconstrucción frente al framebuffer del emulador: 0 diferencias. La referencia es válida.
- Diferencias shim frente a PPU V0: 0 en las 160 líneas (total 0).
- H180 (líneas con afines con más diferencias): no confirmada; no hay diferencias en ninguna línea.
- H182 (diferencias fuera de líneas con afines): descartada; no hay diferencias.
- H181 (líneas sin afines con 0 diferencias): no evaluable. Todas las líneas salen con 5 afines.

**Error de mi ejemplo, no del shim:** el conteo de afines por línea cuenta los 128 sprites de OAM, no solo los que cubren la línea. Por eso sale 5 en todas las líneas y H181 no se puede evaluar. Hay que corregirlo (contar solo afines cuyo rango vertical incluye la línea) antes de concluir sobre H181.

**Lectura provisional:** que shim y PPU coincidan al píxel significa que, en este cuadro, los 5 sprites afines se dibujan igual en ambos. Eso puede pasar porque la matriz es identidad o porque los sprites no caen en pantalla. No se puede afirmar que el shim dibuje bien los afines sin comprobarlo.

Próximo paso propuesto (sin aprobar): corregir el conteo por línea, mostrar la matriz (parámetros de rotación/escala) y la posición de cada afín en 7500, y volver a medir. Requiere nueva propuesta y aprobación.

---

## Revisión humana

- Que la decisión sobre implementar la transformación afín se tome con estos resultados: ✔ | ✘ | ?
