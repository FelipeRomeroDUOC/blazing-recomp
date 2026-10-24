# Propuesta: Identificar qué capa gana en cada píxel que cambia OBJ en el cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: revisada por humano (la ampliación de la reconstrucción a los modos de mezcla fue elegida por el usuario el 2026-10-17, opción (a))

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Hay 5.290 píxeles del cuadro 1000 que cambian al quitar OBJ, aunque OBJ sola no los dibuja. Ya se descartaron el camino de ruta (`camino-rapido-contra-general-cuadro-1000`) y la mezcla de color (`BLDCNT = 0` deja 6.240). Esta propuesta mira píxel por píxel qué capa gana en cada variante y qué sprite lo cubre, para ver el mecanismo.

No se cambia `ppu.rs`, `gba-core`, el shim ni el juego. El experimento reconstruye la regla de composición dentro del ejemplo, usando solo las capturas por línea.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_camino-rapido-contra-general-cuadro-1000.md` (Resultado): H13 descartada, H14 confirmada. La diferencia de OBJ es de composición, no de ruta.
  - `docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`: `BLDCNT = 0` deja 6.240 píxeles de diferencia en OBJ. La mezcla no explica la diferencia.
  - `gba-recomp/crates/gba-core/src/ppu.rs` (lectura del 2026-10-17):
    - `render_sprites` recorre OAM de 0 a 127. Un sprite gana un píxel si su prioridad es menor que la del píxel actual. Con prioridad igual, gana el primero del OAM.
    - `top_two_layers` y `straight_obj` deciden la capa de arriba comparando la prioridad de OBJ con la de cada fondo. OBJ gana frente a un fondo con prioridad igual o mayor.
    - Los sprites semitransparentes (`obj.semi`) mezclan con la segunda capa solo si `BLDCNT` la marca como objetivo.
  - Versión del ejemplo `ppu_pixeles_obj` que ya corrió: no es la de la propuesta anterior y no está verificada. Su salida se usa solo para encontrar píxeles de partida, y el experimento vuelve a medirlos.
- Intención: para cada píxel que cambia al quitar OBJ, saber qué capa gana en V0 (completa) y en V1 (sin OBJ), y qué sprite y qué color tiene OBJ ahí. Éxito: una tabla por categoría que dé el mecanismo, con un control que confirme que la reconstrucción de la regla coincide con la PPU.
- Hipótesis:
  - H15: en los píxeles del cambio, OBJ gana la capa de arriba en V0 y su color es igual al color de fondo de V2 (solo OBJ). Así la cobertura de OBJ sola lo mide como nada. Se confirma si la mayoría de esos píxeles tienen color OBJ igual al de fondo en V2.
  - H16: en esos píxeles, la capa de arriba en V0 es un fondo y OBJ solo cambia la segunda capa. Se confirma si la capa de arriba en V0 es un fondo y OBJ no gana en ningún píxel de la categoría.
  - H17: OBJ gana con un sprite que la PPU deja de dibujar en V2 por algún motivo de ventana o prioridad. Se confirma si, para esos píxeles, el sprite ganador en V0 no aparece en V2.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. Esta propuesta la lee; no la cambia.
  - El trabajo corre en el PC con Windows.
  - Cualquier cambio en `ppu.rs` requiere otra propuesta y aprobación.
- Alternativas descartadas:
  - Pedir a `ppu.rs` que exponga qué capa gana → descartada: cambiar la PPU requiere aprobación aparte, y el experimento puede reconstruir la regla leyendo las mismas capturas.
  - Usar solo la salida anterior de 5.290 píxeles → descartada: no está verificada. Se recalcula dentro de este experimento.

### 3. Ejemplos

- Caso normal: un píxel donde V0 muestra `3062` y V1 muestra `3462`. El informe dice que en V0 gana un fondo (capa 2), que OBJ no gana en ese píxel, que OBJ no tiene color ahí, y que el sprite de OAM número 17 existe, pero con prioridad mayor que la del fondo. Así se ve por qué OBJ cambia el píxel sin dibujarse.
- Caso límite: si la reconstrucción no coincide con la PPU en un píxel, el experimento para: no se puede confiar en la regla reconstruida.

### 4. Pasos

1. Crear `examples/ppu_capa_ganadora.rs` (con `required-features = ["proto"]`). Reutiliza la captura por línea del cuadro N.
2. Reconstruir, dentro del ejemplo, la regla de composición del camino general para cada línea:
   - sprite ganador por píxel (OAM en orden, prioridad estrictamente menor gana);
   - capa de arriba y segunda capa según prioridades de BG y de OBJ;
   - color final, con la regla de `BLDCNT` solo si hay mezcla (no debería haberla en el cuadro 1000, según el diagnóstico previo; si la hay, se reporta).
3. Control: la reconstrucción de V0 y de V1 debe coincidir con la PPU en el 100 % de los píxeles. Si no, parar.
4. Para cada píxel de la aportación fuera de cobertura (V0 distinto de V1 y V2 igual a V3):
   - capa ganadora en V0 y en V1;
   - sprite ganador en V0 (índice OAM, prioridad, color, si es semitransparente);
   - si el sprite ganador aparece en V2 (solo OBJ) en ese píxel.
5. Clasificar cada píxel en una categoría según H15, H16 y H17, y contar por categoría. Las categorías deben sumar el total de píxeles.
6. Escribir `volcados/capa-ganadora-cuadro-1000.txt`: un resumen con los conteos por categoría, el control, y una línea `pixel` por cada píxel clasificado.
7. Crear `tests/capa_ganadora_tests.rs` con las pruebas de la sección 8.
8. Añadir la entrada `[[example]]` en `Cargo.toml`, ejecutar y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_capa_ganadora.rs` (nuevo), `tests/capa_ganadora_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/capa-ganadora-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego y la APK.
- Riesgo: reconstruir la regla puede diferir de la PPU real en detalles (por ejemplo, flips de sprites o mapeo de tiles). El control de la paso 3 lo detecta.

### 6. Suposiciones

- La regla de OAM (orden e prioridad) reconstruida coincide con `render_sprites` → prototipar: sí, con el control de V0 y V1 → si es falsa: parar.
- En el cuadro 1000 no hay mezcla de color que afecte a los píxeles de la aportación → prototipar: sí, midiendo `BLDCNT` y la semitransparencia en esos píxeles → si es falsa: se reporta por categoría, no se descarta.

### 7. Alcance

- Incluye: reconstrucción de la regla de composición en el ejemplo, clasificación por píxel, informe y pruebas.
- No incluye: cambiar `ppu.rs` o corregir nada; el shim; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que la clasificación de píxeles sea confiable | Tipo: límite
  Dado el cuadro 1000 con sus capturas por línea
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0` y `control reconstruccion V1` son 0
Test: `tests/capa_ganadora_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: las categorías suman el total de píxeles de la aportación
Caso de uso: que ningún píxel quede sin clasificar | Tipo: normal
  Dado el informe del ejemplo
  Cuando se suman sus categorías
  Entonces la suma es igual a `capa aportacion fuera total`
Test: `tests/capa_ganadora_tests.rs::test_categorias_suman_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da el mecanismo por separado
Caso de uso: decidir entre H15, H16 y H17 | Tipo: normal
  Dado el informe del ejemplo
  Cuando se leen sus líneas de categoría
  Entonces existen las líneas `capa categoria H15`, `capa categoria H16` y `capa categoria H17`
Test: `tests/capa_ganadora_tests.rs::test_informe_da_categorias` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_capa_ganadora -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\capa-ganadora-output.txt 2>&1` → pasa si: el informe tiene todas las líneas de la sección 8 → no detecta: mecanismos que solo aparezcan en otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test capa_ganadora_tests` → pasa si: las tres pruebas pasan.
- Control: la reconstrucción de V0 y V1 debe dar 0 diferencias con la PPU. Si no, se para.
- Regresión: `git diff --stat` → pasa si: no aparecen `gba-core`, `ppu.rs`, `ppu_shim.rs` ni `ppu_shim.wgsl`.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice qué categoría explica la mayoría de los 5.290 píxeles, y si H15, H16 y H17 se confirman o se descartan.

### 11. Parar si

- La reconstrucción no coincide con la PPU en V0 o V1: parar, porque la clasificación no sería confiable.
- La reconstrucción necesita leer campos privados de `ppu.rs`: parar y pedir aprobación para exponerlos.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Intento 1 (2026-10-17): el ejemplo compila (solo avisos) y se detiene en la línea 131, variante 0, con el error `modo de efecto distinto de 0: no reconstruido`. La reconstrucción de esta propuesta solo cubre el modo de efecto 0 (sin mezcla de fondos ni brillo/oscurecer), así que el control no llega a correr sobre todo el cuadro. No se escribió `volcados/capa-ganadora-cuadro-1000.txt` ni hubo clasificación. Las pruebas no se corrieron con datos.

Intento 2 (2026-10-17, con la regla ampliada a los modos 1, 2 y 3): compila. Control 0 en V0, V1, V2 y V3. Aportación fuera de cobertura: 5.290 píxeles, todos clasificados. H15: 5.290; H16: 0; H17: 0; otro: 0; sin_sprite: 0; suma: 5.290. Las tres pruebas de aceptación pasan.

Lectura: en los 5.290 píxeles, el sprite gana la capa de arriba en V0, el mismo sprite se dibuja en solo-OBJ, y su color es igual al del fondo sin capas (V3). Esto confirma H15 y descarta H16 y H17 en este cuadro. Del conteo de V1 no se analizó qué capa aparece debajo (fl1).

Decisión del usuario (2026-10-17): opción (a).

---

## Revisión humana

- Que la reconstrucción de la regla coincida con la PPU, y que H15, H16 y H17 queden confirmadas o descartadas con sus conteos: ✔ (aceptada por el usuario el 2026-10-17: H15 confirmada con 5.290 píxeles; H16 y H17 descartadas en este cuadro)
