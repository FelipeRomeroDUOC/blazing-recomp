# Propuesta: erradicación de falso positivo de composición del dragón y restauración del pipeline estándar

Fecha: 2026-11-01
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Erradica el falso positivo de la heurística de la cinemática del dragón en Rust y shaders WGSL, aislando `sanear_bldcnt_cinematica_dragon()` del flujo autoritativo de `io_lineas` y restaurando la tubería estándar de prioridades GBA para el 100 % de los fotogramas normales del juego.

### 2. Contexto

- Contexto consultado:
  - Volcados recientes en `volcados/diferencias/`: `cuadros 289, 535, 616, 617, 685, 984, 985, 2048, 2265, 3108`.
  - Reportes de ablación: En los 10 cuadros, el veredicto es idéntico: *"el residuo cae a 0 con sin BG2, sin BG3, sin BLDCNT"*. En el cuadro 617, fallan 37.433 de 38.400 píxeles (97.48 % de la pantalla).
  - Volcados auténticos de la cinemática del dragón: `volcados/atribucion-769.txt`, `volcados/afin-y-linea.txt` (muestran que con `io_lineas` activo, la tubería por software tiene exactamente 0 diferencias frente al hardware GBA).
  - Código fuente: `src/ppu_nativa.rs` (líneas 1170-1188, 1200-1220), `src/ppu_nativa.wgsl` (líneas 554-624), `src/perf.rs`.
- Causa raíz aislada:
  1. **Falso positivo por comprobación laxa de prioridades (`bg_prio2 == 3 && bg_prio3 == 3`):** En GBA Modo 0, prácticamente todas las pantallas normales (logos iniciales, título, prólogo, menús y mapas) usan BG2 y BG3 en prioridad 3 como fondos lejanos. La comprobación añadida en la Propuesta 13 disparó la lógica del dragón en fotogramas ordinarios con mezcla desactivada.
  2. **Mutación de registros auténticos en Rust:** `sanear_bldcnt_cinematica_dragon()` se ejecutaba incluso con `io_lineas` presente (`Some`), forzando `bldcnt |= 0x2008` (mezcla de BG3 con Backdrop), activando modo alfa y fijando `bldalpha = 0x0810`, destruyendo los registros reales de la ROM.
  3. **Secuestro de prioridades en WGSL:** La rama `if (es_escena_dragon)` anuló la tubería estándar de hardware en cualquier fotograma con fondos en prioridad 3, forzando la mezcla de BG3 y orden rígido de capas.
- Restricciones:
  - Mantener 60.0 FPS estables sin sobrecargar la GPU (`AGENTS.md` §1).
  - Cero regresiones en la cinemática del dragón real (`cuadro 5602` / `769`) preservando la mezcla auténtica provista por `io_lineas`.
  - No incrementar la versión del proyecto sin confirmación previa del usuario (`AGENTS.md` §3).

### 3. Criterios de aceptación (comprobables)

- Escenario [Regresión]: fotogramas ordinarios de título, menús y diálogos no sufren alteración de BLDCNT ni forzado de escena dragón
  - Dado un estado de PPU en Modo 0 correspondiente a pantallas normales (cuadros 289..3108) donde `bg_prio2 == 3` y `bg_prio3 == 3` pero `(bldcnt & 0x2008) != 0x2008` o el modo de mezcla no es alfa (`((bldcnt >> 6) & 3) != 1`)
  - Cuando se procesa `preparar_staging_filas` con `io_lineas` y se ejecuta el shader `fs_comp`
  - Entonces `bldcnt` y `bldalpha` se conservan intactos sin mutación, `es_escena_dragon` evalúa en `false`, y la pantalla se procesa íntegramente por la tubería estándar GBA de 2 objetivos con 0 píxeles de residuo por BLDCNT.
  - Test: `cargo test -p blazing-recomp -- test_sanear_bldcnt_respeta_fondos_ordinarios_prio3 --exact`

- Escenario [Normal]: preservación autoritativa de registros cuando `io_lineas` está presente
  - Dado un lote de 160 scanlines con `io_lineas` capturadas (`Some(lineas)`)
  - Cuando se ejecuta `preparar_staging_filas`
  - Entonces `staging_comps` preserva exactamente los valores decodificados de `io_lineas` sin invocar `sanear_bldcnt_cinematica_dragon()`.
  - Test: `cargo test -p blazing-recomp -- test_staging_lineas_inmune_a_saneamiento_dragon --exact`

- Escenario [Límite]: detección precisa de la cinemática del dragón con BG1 activo y mezcla alfa nominal
  - Dado un cuadro de la cinemática del dragón donde `dispcnt` tiene Modo 0 con BG2 y BG3 activos (incluyendo `0x1F00` con BG1 activo), `(bldcnt & 0x2008) == 0x2008` y modo de mezcla alfa activo (`((bldcnt >> 6) & 3) == 1`)
  - Cuando se evalúa la condición de la escena del dragón en WGSL o Rust
  - Entonces evalúa en `true`, manteniendo la composición multicapa jerárquica sin interferir en el resto del juego.
  - Test: `cargo test -p blazing-recomp -- test_deteccion_escena_dragon_estricta --exact`

### 4. Qué código va a cambiar (ejemplos concretos)

- Archivos:
  - `src/ppu_nativa.rs`
  - `src/ppu_nativa.wgsl`
  - `src/perf.rs`
  - `src/ppu_nativa_tests.rs`

Ejemplo antes/ahora en `src/ppu_nativa.rs`:
```rust
// ANTES (mutaba staging incondicionalmente, incluso con io_lineas activo):
        match io_lineas {
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
            }
            None => { ... }
        }
        Self::sanear_bldcnt_cinematica_dragon(staging_comps);

// AHORA (io_lineas es autoritativo y no se muta; el saneamiento solo opera como contingencia en el ramal None si coincide la firma de mezcla del fuego):
        match io_lineas {
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
            }
            None => {
                let cfg_capa = self.config_capa();
                let cfg_comp = self.config_comp();
                staging_capas.fill(cfg_capa);
                staging_comps.fill(cfg_comp);
                self.consolidar_dispcnt_fondos(&staging_comps[..]);
                self.propagar_fondos_a_staging(staging_capas, staging_comps);
                Self::sanear_bldcnt_cinematica_dragon(staging_comps);
            }
        }
```

Ejemplo antes/ahora en `src/ppu_nativa.wgsl`:
```wgsl
// ANTES (heurística ultra-laxa que secuestraba toda la pantalla en escenas normales):
let es_escena_dragon = (cfg_comp.dispcnt & 7u) == 0u && (cfg_comp.dispcnt & 0x0C00u) == 0x0C00u && ((cfg_comp.bg_prio2 == 3u && cfg_comp.bg_prio3 == 3u) || (cfg_comp.bldcnt & 8u) != 0u);

// AHORA (firma estricta de la cinemática: Modo 0, BG2 y BG3 activos, con mezcla alfa activa de BG3 sobre Backdrop):
let es_escena_dragon = (cfg_comp.dispcnt & 7u) == 0u 
    && (cfg_comp.dispcnt & 0x0C00u) == 0x0C00u 
    && (cfg_comp.bldcnt & 0x2008u) == 0x2008u 
    && ((cfg_comp.bldcnt >> 6u) & 3u) == 1u;
```

---

## Resultado (lo escribe el agente después de editar y medir)

1. **Implementación de código:**
   - `src/ppu_nativa.rs`: `sanear_bldcnt_cinematica_dragon` restringido a la firma estricta `(c.dispcnt & 7) == 0 && (c.dispcnt & 0x0C00) == 0x0C00 && (c.bldcnt & 0x2008) == 0x2008 && ((c.bldcnt >> 6) & 3) == 1`. En `preparar_staging_filas`, su invocación quedó aislada exclusivamente en el ramal `None` (contingencia sin raster por línea); cuando `io_lineas` es `Some(lineas)`, los registros de la ROM se preservan de forma autoritativa.
   - `src/ppu_nativa.wgsl`: Línea 554 actualizada con la condición estricta de escena del dragón exigiendo bits `0x2008` en `bldcnt` y modo alfa (`((bldcnt >> 6) & 3) == 1`).
   - `src/perf.rs`: Condición `es_dragon` en telemetría sincronizada con la misma firma estricta.

2. **Verificación y Pruebas Unitarias:**
   - Se añadieron y pasaron satisfactoriamente los 3 tests unitarios previstos:
     - `test_sanear_bldcnt_respeta_fondos_ordinarios_prio3`: OK.
     - `test_staging_lineas_inmune_a_saneamiento_dragon`: OK.
     - `test_deteccion_escena_dragon_estricta`: OK.
   - Se ajustaron los tests existentes (`test_sanear_bldcnt_coexistencia_fuego_y_texto_160_scanlines`, `test_sanear_bldcnt_restaura_variante_dialogo_2869`, `test_sanear_bldcnt_permite_sprites_obj_y_cursor`, `test_sanear_bldcnt_permite_sprites_aislados`, `test_deteccion_escena_dragon_con_bg1_activo`, `test_deteccion_escena_dragon_por_prioridades_con_bldcnt_cero` y `test_preparar_staging_filas_sanea_en_ramal_none`) para reflejar la firma estricta.
   - Suite completa de unit tests (`cargo test -p blazing-recomp --lib`): 234 pasados, 0 fallados.
   - Compilación release del visor (`cargo build --release --features proto --bin ppu_doble`): exitosa en 33.61s.
