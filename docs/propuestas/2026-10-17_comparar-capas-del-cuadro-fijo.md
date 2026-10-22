# Propuesta: Comparar por capas la imagen fija del cuadro 1000 entre la PPU por software y el shim

Fecha: 2026-10-17
Nivel: 2
Estado: aprobada (el usuario aprobó el alcance el 2026-10-17)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Para la escena fija del cuadro 1000 de la intro, el informe indica qué capa (fondo 0 a 3, o sprites) tiene píxeles distintos entre la PPU por software y el shim GPU, para saber qué parte del dibujo falla.

### 2. Contexto

- Contexto consultado: `docs/propuestas/2026-10-16_contar-lineas-con-cambio-de-registros.md` (resultado: en el cuadro 1000 no cambia ningún registro, paleta, VRAM ni OAM durante el dibujo, y 138 filas difieren); `ppu-compare-informe.csv` y los volcados `volcados/cuadro-1000-*.rgb`; `blazing-recomp/src/ppu_shim.rs` (`decode_io_regs` recibe `io` como parámetro); `blazing-recomp/src/ppu_shim.wgsl` (`fs_bg` descarta los píxeles transparentes con `discard`, verificado el 2026-10-17). `gba-core/src/ppu.rs` (`render_scanline`) no se ha leído para esta propuesta; eso se hace en el paso 1.
- Intención: saber si la diferencia del cuadro 1000 viene de un fondo concreto o de los sprites. Éxito: un informe con una fila por capa y su número de píxeles distintos, para decidir qué corregir en la Fase 1.x.
- Restricciones y decisiones previas (máx. 3):
  - La PPU por software se conserva y no se borra (origen: respuesta humana, 2026-10-16) → implica: la PPU por software sigue siendo la referencia y no se cambia para este experimento → protegida por: "La copia del estado reproduce el juego".
  - El experimento no cambia la ruta del juego (origen: propuesta del arnés, sección 2) → implica: el juego sigue corriendo igual; el experimento usa una copia del estado del cuadro 1000 → protegida por: "La copia del estado reproduce el juego".
  - El trabajo es en el PC con Windows, no en Android (origen: respuesta humana, 2026-10-16) → implica: el experimento no se compila para Android → no testeable directamente; se verifica al correrlo en el PC.
- Alternativas descartadas:
  - Apagar capas en el juego en vivo para ver el efecto → descartada porque cambia el juego y su estado para los cuadros siguientes.
  - Cambiar la PPU por software para que dibuje por capa → descartada porque cambiaría la referencia que se quiere comparar.
- Ante imprevistos, prioriza: que la referencia no cambie, sobre tener el desglose completo por capa.

### 3. Ejemplos

- Caso normal: estado del cuadro 1000 → antes: 138 filas difieren y no sabemos de qué capa vienen → ahora: el informe lista BG0, BG1, BG2, BG3 y OBJ, cada una con su conteo de píxeles distintos.
- Caso límite: capa desactivada en DISPCNT → antes: no aplica → ahora: su conteo es 0 en ambos renderizadores.

### 4. Pasos

1. Leer `gba-recomp/crates/gba-core/src/ppu.rs` para ver cómo `render_scanline` recorre las capas, y revisar si `MemMap` puede construirse o copiarse fuera del juego (solo lectura).
2. Prototipo: crear una copia del estado del cuadro 1000 (io, paleta, VRAM y OAM) y renderizarla con la PPU por software. Comparar con el framebuffer del juego en ese cuadro: debe dar 0 diferencias.
3. Para cada capa (BG0, BG1, BG2, BG3, OBJ): hacer una copia del estado con DISPCNT que deje solo esa capa, renderizarla con la PPU por software y con el shim, y contar los píxeles distintos.
4. Añadir al harness una opción para escribir `volcados/capas-cuadro-N.txt` con el informe por capa.
5. Escribir las pruebas de los escenarios de la sección 8.

### 5. Qué más podría verse afectado

- Búsqueda de referencias: se hizo solo sobre `ppu_shim.rs` y `ppu_shim.wgsl`. La búsqueda de funciones por capa en `ppu.rs` se hace en el paso 1.
- Archivos que probablemente cambian: `examples/ppu_compare.rs` (nueva opción) y `tests/ppu_capas_tests.rs` (nuevo).
- Archivos que no cambian: `gba-core` (la PPU por software no se modifica), `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `src/pipeline.rs`, el juego y Android.

### 6. Suposiciones

- La copia del estado reproduce el framebuffer del juego en el cuadro 1000 → prototipar: sí → si es falsa: el estado que se copia no es completo (por ejemplo, falta un buffer interno) y hay que copiar más cosas antes de comparar por capa → resultado del prototipo: pendiente | no detecta: estado interno de la PPU que no está en io, paleta, VRAM ni OAM.
- El shim respeta un DISPCNT modificado sin otros efectos → prototipar: sí → si es falsa: el desglose por capa no es comparable y hay que revisar cómo el shim lee el registro → resultado del prototipo: pendiente | no detecta: efectos que dependen de otros registros que no se cambian.
- Las capas se pueden aislar en la PPU por software solo cambiando DISPCNT → prototipar: no → si es falsa: habrá que añadir un parámetro al renderizador por software para aislar capas, lo que requiere otra propuesta → resultado: no probado | no detecta: efectos de prioridad entre capas que cambian al aislar.

### 7. Alcance

- Incluye: copia del estado del cuadro 1000, desglose por capa con la PPU por software y el shim, informe en archivo, pruebas.
- No incluye: cambios en la PPU por software, en el shim o en el juego; corrección de las diferencias encontradas (eso es Fase 1.x); Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la copia del estado reproduce el juego en el cuadro 1000
Caso de uso: confiar en que el desglose parte de la misma escena | Tipo: regresión
  Dado el estado del cuadro 1000 de la intro
  Cuando se renderiza la copia con todas las capas activas con la PPU por software
  Entonces los 38.400 píxeles coinciden con el framebuffer del juego en ese cuadro (0 diferencias)
Test: `tests/ppu_capas_tests.rs::test_copia_reproduce_cuadro_1000` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (la copia no existe)

Escenario: el informe lista cada capa con su conteo de píxeles distintos
Caso de uso: ver de qué capa vienen las diferencias del cuadro 1000 | Tipo: normal
  Dado el estado del cuadro 1000
  Cuando se corre el desglose por capa
  Entonces el archivo `volcados/capas-cuadro-1000.txt` tiene 5 filas (BG0, BG1, BG2, BG3, OBJ) y cada conteo está entre 0 y 38.400
Test: `tests/ppu_capas_tests.rs::test_informe_por_capa_tiene_cinco_filas`
Estado previo: falla (el desglose no existe)

Escenario: una capa desactivada no produce diferencias
Caso de uso: comprobar que el desglose respeta DISPCNT | Tipo: límite
  Dado el estado del cuadro 1000 con el bit de BG2 apagado en DISPCNT
  Cuando se corre el desglose para BG2
  Entonces el conteo de BG2 es 0 en ambos renderizadores
Test: `tests/ppu_capas_tests.rs::test_capa_apagada_no_difiere`
Estado previo: falla (el desglose no existe)

### 9. Cómo lo comprobaré

- Copia reproduce el juego → `cargo test --test ppu_capas_tests test_copia_reproduce_cuadro_1000` con `FE_ROM` apuntando al ROM → pasa si: 0 diferencias → no detecta: estado interno que no está en io, paleta, VRAM ni OAM.
- Informe por capa → `cargo run --release --features proto --example ppu_compare -- <rom> secuencias\intro.txt ppu-compare-informe.csv 1000` y revisar `volcados\capas-cuadro-1000.txt` → pasa si: 5 filas con conteos entre 0 y 38.400 → no detecta: si el conteo de cada capa coincide con lo que se ve en pantalla.
- Capa apagada → `cargo test --test ppu_capas_tests test_capa_apagada_no_difiere` → pasa si: conteo 0 en ambos → no detecta: efectos de prioridad entre capas.
- Visual (visible) → revisión humana: mirar la imagen de la capa con más diferencias en `volcados\` → no detecta: diferencias de color que solo se notan en la TV.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación, el prototipo 2 tiene resultado y el informe por capa del cuadro 1000 está escrito en el PC.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno hasta ahora.

---

## Resultado (lo completa el agente después de editar)

Prototipo 2 ejecutado el 2026-10-17 (`examples/ppu_capas.rs`, cuadro 1000): la copia del estado (E/S, paleta, VRAM y OAM) da 22.185 píxeles distintos de 38.400 (esperado 0). Criterio no cumplido. Ver `2026-10-17_buscar-estado-faltante-cuadro-1000.md`: copiar todos los campos públicos da el mismo resultado; `2026-10-17_captura-por-linea-cuadro-1000.md`: redibujar cada línea desde su propio estado da 0 píxeles distintos. El paso 3 de esta propuesta queda reemplazado por `2026-10-17_capas-por-aportacion-cuadro-1000.md`.

Estado de la propuesta: sin cerrar; su plan continúa en la propuesta nueva.

---

## Revisión humana

- Que el informe por capa identifique una causa que se pueda corregir en la Fase 1.x: ✔