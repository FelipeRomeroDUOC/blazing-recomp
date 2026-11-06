# Propuesta: consolidación incondicional de sprites en PPU nativa y blindaje anti-parpadeo

Fecha: 2026-10-31
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Garantiza la visualización continua y sin parpadeos de sprites, llamas y cuadros de texto durante toda la cinemática de introducción mediante la consolidación incondicional de registros en GPU y la estabilidad de las ramas del sombreador.

### 2. Contexto

- Contexto consultado:
  - Telemetría de usuario: `blazing-perf-29.txt` (60 FPS estables, conmutación de `blend=0` a `blend=28` y `blend=61` en $t=7..9\text{ s}$).
  - `src/ppu_nativa.rs` (líneas 913-918, 1101-1116, 2006-2026, 2088-2092).
  - `src/ppu_nativa.wgsl` (líneas 550-605).
  - Referencia canónica de hardware: `crates/gba-core/src/ppu.rs` (líneas 679-740, `top_two_layers`).
  - Volcados: `volcados/atribucion-769.txt`, `volcados/pixeles-obj-cuadro-769.txt`.
- Causa raíz aislada:
  1. **Vaciado de sprites por VBlank:** En `src/ppu_nativa.rs:2007`, `consolidar_dispcnt_sprites` estaba restringido por `if io_lineas.is_none()`. En Android, `io_lineas` siempre es `Some`, por lo que jamás se ejecutaba. `self.estado.dispcnt` heredaba el valor de `io_fin` (capturado en VBlank, scanlines 160–227), donde FE7 apaga los sprites (`DISPCNT & 0x1000 == 0`) para transferencias DMA. `decode_oam` evaluaba el bit 12 en 0 y vaciaba `num_sprites = 0`, provocando un parpadeo a 30 Hz del cursor y sprites.
  2. **Conmutación binaria de ramas:** En `src/ppu_nativa.wgsl:554`, `es_escena_dragon` exigía `cfg_comp.bldcnt != 0u`. En la transición de $t=8\text{ s}$, 32 fotogramas tenían `bldcnt == 0`, alternando cada 33 ms entre la jerarquía multinivel y el bucle de 2 capas.
  3. **Pesos nulos en subtítulos:** Cuando `bldalpha` caía a 0 en scanlines específicas, la mezcla de `BG0` multiplicaba por 0 produciendo artefactos oscuros.
- Restricciones (máx. 3):
  - Mantener 60 FPS estables en Android sin sobrecargar el pipeline (`AGENTS.md` §1).
  - Preservar las pinturas estáticas del prólogo ($t=6..14\text{ s}$) sin forzar blending donde no debe existir.
  - No alterar la versión de la app sin confirmación explícita (`AGENTS.md` §3).

### 3. Ejemplos

- Caso normal (Sprites y cursor de diálogo): En la cinemática del dragón con prompt activo (`t=15..60s`), `io_fin` apaga el bit 12 de sprites en VBlank.
  - Antes: `decode_oam` leía bit 12 apagado, vaciaba `num_sprites = 0` y la GPU no dibujaba el cursor en fotogramas pares (parpadeo visible a 30 Hz).
  - Ahora: `consolidar_dispcnt_sprites` rescata incondicionalmente el bit 12 activo en las scanlines visibles, `num_sprites` se mantiene estable en 128 y los sprites se renderizan de forma continua.
- Caso límite (Transición $t=8\text{ s}$ de prólogo a cinemática): En el cuadro de transición, `bldcnt` es temporalmente 0 en 32 de los 60 fotogramas.
  - Antes: `es_escena_dragon` colapsaba a falso en esos 32 fotogramas, conmutando bruscamente al bucle de 2 capas que cortaba el fuego.
  - Ahora: La firma combinada reconoce las prioridades `bg_prio2 == 2 && bg_prio3 == 3`, manteniendo la composición estable y continua.
- Caso regresión (Pinturas fijas del prólogo $t=6..14\text{ s}$ y diálogo con `eva=0, evb=0`):
  - Antes: Si se eliminaba `bldcnt != 0`, las pinturas fijas del prólogo ($t=6..14\text{ s}$) que comparten `DISPCNT = 0x0C00` se mezclaban indebidamente como fuego.
  - Ahora: Al exigir `bg_prio2 == 2 && bg_prio3 == 3`, las pinturas fijas quedan 100% protegidas y opacas, mientras los subtítulos con pesos nulos usan pesos nominales sin caer a negro.

### 4. Pasos

1. Modificar `src/ppu_nativa.rs`:
   - En `subir_buffers_lineas` (líneas 2007-2010), ejecutar `self.estado.consolidar_dispcnt_sprites(&mut self.staging_filas_comp[..]);` de forma incondicional.
   - En línea 2088, actualizar reactivamente tras `decode_oam` con `if nv > 0`.
   - En `sanear_bldcnt_cinematica_dragon`, incluir la verificación de prioridades `(c.bg_prio2 == 2 && c.bg_prio3 == 3)` para proteger transiciones.
2. Modificar `src/ppu_nativa.wgsl`:
   - En `fs_comp`, actualizar la guarda de `es_escena_dragon` con la firma de prioridades `(cfg_comp.bg_prio2 == 2u && cfg_comp.bg_prio3 == 3u)`.
   - Proteger los pesos alfa en `BG0` contra `eva == 0u && evb == 0u`.
3. Validar con pruebas unitarias en `src/ppu_nativa_tests.rs`:
   - Verificar consolidación incondicional de sprites incluso con `io_lineas` presente.
   - Verificar preservación de pinturas estáticas y cinemática en transición.
4. Ejecutar suite de pruebas con `cargo test --lib ppu_nativa`.
5. Empaquetar el Fat APK de release para validación en dispositivo Android.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs` (decodificación OAM y buffers staging).
- `src/ppu_nativa.wgsl` (fragment shader `fs_comp`).
- `src/ppu_nativa_tests.rs` (tests de regresión).
Búsqueda realizada: referencias a `consolidar_dispcnt_sprites`, `decode_oam`, `es_escena_dragon` y `sanear_bldcnt_cinematica_dragon`.

### 6. Suposiciones

- La presencia del bit 12 en scanlines visibles garantiza que la OAM corresponde a un cuadro que requiere renderizado de sprites → si es falso: desacoplar la decodificación OAM por línea raster.
- Las prioridades `bg_prio2 == 2 && bg_prio3 == 3` son exclusivas de la cinemática del dragón durante Modo 0 → si es falso: incorporar comprobación explícita de `screen_base` de VRAM.

### 7. Alcance

- Incluye: Consolidación incondicional de sprites, blindaje de transición en shader y pesos alfa de diálogo.
- No incluye: Modificación de la emulación de audio ni cambios en la arquitectura del APK.

### 8. Criterios de aceptación

- Escenario 1 (Normal):
  - Dado un fotograma de la cinemática del dragón con `io_lineas` activo y `io_fin` con bit 12 en 0 (VBlank),
  - Cuando se ejecuta `subir_buffers_lineas`,
  - Entonces `self.estado.dispcnt & 0x1000` se mantiene activo y `self.estado.num_sprites` es mayor a 0, subiendo los sprites a GPU sin vaciado.
- Escenario 2 (Límite):
  - Dado un fotograma de transición donde `bldcnt == 0` pero `bg_prio2 == 2` y `bg_prio3 == 3`,
  - Cuando se evalúa `es_escena_dragon` en el sombreador WGSL,
  - Entonces la condición resulta verdadera y no se produce corte abrupto al bucle tradicional de 2 capas.
- Escenario 3 (Regresión):
  - Dado un fotograma de pintura estática del prólogo (`DISPCNT = 0x0C00`, `bldcnt == 0`, `bg_prio != [2, 3]`),
  - Cuando se procesa por el compositor nativo,
  - Entonces no se aplica blending forzado y la imagen permanece opaca y fiel.

### 9. Cómo lo comprobaré

- Pruebas automatizadas:
  - `cargo test --lib ppu_nativa`: Debe validar 71+ tests en verde sin ninguna regresión.
  - Test específico `test_consolidacion_incondicional_sprites_con_io_lineas`: Valida que `io_lineas = Some` no omite la consolidación del bit 12 ni de OBJWIN.
  - Test específico `test_deteccion_escena_dragon_por_prioridades_con_bldcnt_cero`: Valida que `bldcnt == 0` no apaga la escena del dragón si las prioridades son 2 y 3.
- Qué NO detectan estas pruebas:
  - Pequeñas variaciones de latencia de fotograma en GPU Mali/Adreno bajo carga térmica (se valida con la telemetría en dispositivo real).

### 10. Terminado cuando

- La suite `cargo test --lib ppu_nativa` pase al 100% (71+ tests).
- El Fat APK `blazing-recomp-release.apk` se compile limpiamente sin advertencias de enlace.
- El usuario confirme visualmente que los parpadeos cesaron durante la cinemática completa en su dispositivo.

### 11. Parar si

- Algún test existente de `ppu_nativa` o `pipeline` falla.
- La consolidación incondicional introduce artefactos visuales en menús u otras pantallas.
- El tiempo de renderizado por cuadro excede los 16.6 ms (degradando los 60 FPS).

---

## Revisión humana (la completa el usuario tras la propuesta)

¿Se aprueba la propuesta tal como está redactada?

- [ ] Sí (pasar a la fase de edición).
- [ ] No (indicar qué ajustar antes de tocar código).
- [ ] Con cambios (especificar cuáles).
