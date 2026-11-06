# Propuesta: erradicación del latch parasitario de OBJWIN, soporte de fades y telemetría de capas

Fecha: 2026-10-31
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Elimina de raíz los parpadeos en negro corrigiendo el latch de OBJWIN y los fades del dragón, e incorpora telemetría por cambio de estado en `blazing-perf.txt` que audita qué dibuja cada capa y verifica que la corrección esté activa.

### 2. Contexto

- Contexto consultado:
  - Petición del usuario: Añadir la corrección, agregar log a la telemetría `.txt` que mida si la corrección se aplica, y registrar las capas que se renderizan y qué dibuja cada capa únicamente cuando al menos una capa cambie.
  - Telemetría continua de usuario: `blazing-perf-30.txt` (86 segundos continuos a 60 FPS estables con parpadeos intermitentes reportados).
  - Volcado causal: `volcados/diferencias/cuadro-5602.txt` (`pixel testigo x=0 y=0: software [3A, 52, 4A] nativa [00, 00, 00] ventana sin ventanas capas [BG0,BG1,BG2,BG3,OBJ]`).
  - Código fuente: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/platform.rs`, `src/perf.rs`.
- Causa raíz aislada:
  1. **Latch parasitario de OBJWIN en `consolidar_dispcnt_sprites`:** El campo persistente `self.dispcnt` acumulaba `self.dispcnt |= 0x8000` sin limpiarse nunca al cambiar de cuadro. Cada cuadro forzaba `c.dispcnt |= 0x8000` en las 160 scanlines (`comps.iter_mut()`), incluso en líneas donde el hardware GBA tenía el bit 15 en 0.
  2. **Colapso a negro en WGSL por `WINOUT == 0`:** Al recibir `c.dispcnt` con `0x8000` forzado en cuadros con `num_sprites_ventana == 0`, `ppu_nativa.wgsl` evaluaba `objwin_on = true`. Al no haber píxeles de ventana OBJ, caía en `control = cfg_comp.winout & 0x3Fu`. En escenas normales, `WINOUT` es `0x0000`, por lo que `control` colapsaba a `0x00`, apagando todas las capas (`BG0..BG3`, `OBJ`) y devolviendo `backdrop` (negro `0x0000`).
  3. **Omisión de Modos 2 y 3 en la cinemática del dragón:** En `ppu_nativa.wgsl:558-615`, `es_escena_dragon` ignoraba los modos `modo == 2u` (Brighten con `evy`) y `modo == 3u` (Darken con `evy`).
  4. **Visibilidad de capas en telemetría:** No existía una línea en `blazing-perf.txt` que registrara en qué momento una capa se enciende o se apaga y el rol de cada capa (texto, dragón, fuego, sprites).
- Restricciones:
  - Mantener 60.0 FPS estables sin sobrecargar el bucle de eventos (`AGENTS.md` §1).
  - Escribir la telemetría de capas exclusivamente cuando ocurra una mutación en su configuración o visibilidad, evitando saturar el archivo de texto.
  - No incrementar la versión del proyecto sin confirmación previa del usuario (`AGENTS.md` §3).

### 3. Criterios de aceptación (comprobables)

- Escenario [Regresión]: consolidación de DISPCNT en un cuadro sin sprites de ventana
  - Dado un estado de PPU donde `num_sprites_ventana == 0` y `comps` contiene líneas con o sin `0x8000`
  - Cuando se ejecuta `consolidar_dispcnt_sprites(&mut comps)`
  - Entonces `self.dispcnt & 0x8000` queda en 0, ninguna línea de `comps` tiene el bit `0x8000` forzado, y si es falso (ej. `num_sprites_ventana > 0` y `hay_objwin_linea`), `self.dispcnt & 0x8000` se activa únicamente en correspondencia con los sprites reales.
  - Test: `test_consolidar_dispcnt_purga_objwin_sin_sprites` en `src/ppu_nativa_tests.rs` (`cargo test -p blazing-recomp -- test_consolidar_dispcnt_purga_objwin_sin_sprites`).

- Escenario [Límite]: evaluación de OBJWIN en el sombreador WGSL
  - Dado un píxel procesado en `fs_comp` con `cfg_comp.dispcnt`
  - Cuando se evalúa `objwin_on`
  - Entonces solo es verdadero si tanto el bit 15 (`0x8000`) como el bit 12 (`0x1000`, sprites activos) están presentes, y si es falso (ej. el bit 12 está apagado o no hay píxeles de ventana OBJ), `objwin_on` evalúa en `false` y no activa recortes que apaguen capas hacia `winout == 0`.
  - Test: `test_shader_compilacion` en `src/pipeline_tests.rs` (`cargo test -p blazing-recomp -- test_shader_compilacion`).

- Escenario [Normal]: modulación de brillo y oscurecimiento en la escena del dragón
  - Dado un fotograma de la cinemática del dragón con `efectos` activo y `modo == 2` (aclarado) o `modo == 3` (oscurecido)
  - Cuando se ejecuta la composición jerárquica en GPU o CPU (`compone_pixel_jerarquica_dragon`)
  - Entonces el color acumulado resultante aplica `brighten(color_acc, evy)` o `darken(color_acc, evy)` respectivamente, y si es falso (ej. `efectos` está inactivo o `modo` no es 2 ni 3), el color acumulado resultante se mantiene inalterado.
  - Test: `test_dragon_fades_brighten_darken` en `src/ppu_nativa_tests.rs` (`cargo test -p blazing-recomp -- test_dragon_fades_brighten_darken`).

- Escenario [Normal]: telemetría diferencial de capas en `blazing-perf.txt`
  - Dado un cambio en el estado de visibilidad o rol de cualquiera de las capas (BG0..BG3, OBJ) o modo de composición
  - Cuando el bucle de presentación procesa el cuadro en `src/platform.rs`
  - Entonces emite una línea `# capas ...` a `blazing-perf.txt` describiendo el estado de cada capa, su función y confirmando la aplicación de la corrección, y si es falso (el estado de capas no cambió respecto al cuadro previo), no escribe ninguna línea redundante.
  - Test: `test_telemetria_capas_diferencial` en `src/perf_tests.rs` (`cargo test -p blazing-recomp -- test_telemetria_capas_diferencial`).

### 4. Qué código va a cambiar (ejemplos concretos)

- Archivos:
  - `src/ppu_nativa.rs`
  - `src/ppu_nativa.wgsl`
  - `src/platform.rs`
  - `src/perf.rs`
  - `src/ppu_nativa_tests.rs`
  - `src/perf_tests.rs`

Ejemplo antes/ahora en `src/ppu_nativa.rs` (`consolidar_dispcnt_sprites`):
```rust
// ANTES (latch parasitario que atrapaba 0x8000 de por vida y lo forzaba a todas las líneas):
        for c in comps.iter() {
            if c.dispcnt & 0x1000 != 0 {
                hay_obj = true;
                if c.dispcnt & 0x40 != 0 {
                    obj_1d = true;
                }
            }
            if c.dispcnt & 0x8000 != 0 {
                self.dispcnt |= 0x8000;
            }
        }
        if self.num_sprites_ventana > 0 {
            self.dispcnt |= 0x8000;
        }
        if self.dispcnt & 0x8000 != 0 {
            for c in comps.iter_mut() {
                c.dispcnt |= 0x8000;
            }
        }

// AHORA (gestión estricta por cuadro: limpia 0x8000 si no hay sprites de ventana en OAM):
        let mut hay_objwin_linea = false;
        for c in comps.iter() {
            if c.dispcnt & 0x1000 != 0 {
                hay_obj = true;
                if c.dispcnt & 0x40 != 0 {
                    obj_1d = true;
                }
            }
            if c.dispcnt & 0x8000 != 0 {
                hay_objwin_linea = true;
            }
        }
        if self.num_sprites_ventana > 0 && hay_objwin_linea {
            self.dispcnt |= 0x8000;
        } else {
            self.dispcnt &= !0x8000;
            for c in comps.iter_mut() {
                c.dispcnt &= !0x8000;
            }
        }
```

Ejemplo antes/ahora en `src/ppu_nativa.wgsl` (filtro de `objwin_on` y fades en `es_escena_dragon`):
```wgsl
// ANTES:
    let objwin_on = (cfg_comp.dispcnt & 0x8000u) != 0u;
    ...
    if (es_escena_dragon) {
        ...
        // 4. Sprites...
        let r8 = ...
        return vec4<f32>(...);
    }

// AHORA:
    let objwin_on = (cfg_comp.dispcnt & 0x8000u) != 0u && (cfg_comp.dispcnt & 0x1000u) != 0u;
    ...
    if (es_escena_dragon) {
        ...
        // 4. Sprites...
        if (efectos) {
            if (modo == 2u) {
                color_acc = brighten(color_acc, evy);
            } else if (modo == 3u) {
                color_acc = darken(color_acc, evy);
            }
        }
        let r8 = ...
        return vec4<f32>(...);
    }
```

Ejemplo telemetría en `src/platform.rs` (muestreo diferencial de capas hacia `blazing-perf.txt`):
```rust
// Nueva emisión diferencial en PlatformState:
if let Some(msg) = self.evaluar_cambio_capas(ppu_state, phases.es_nativa) {
    log::info!("PERF: {}", msg);
    self.append_perf_line(&msg);
}
// Formato de línea resultante:
// # capas frame=1240 bg0=ON(prio=0,texto) bg1=OFF bg2=ON(prio=2,dragon) bg3=ON(prio=3,fuego) obj=ON(cnt=12) comp=multicapa correccion=APLICADA
```

### 5. Plan paso a paso

1. Modificar `consolidar_dispcnt_sprites` en `src/ppu_nativa.rs` para limpiar incondicionalmente el bit `0x8000` si `self.num_sprites_ventana == 0`.
2. Actualizar `compone_pixel_jerarquica_dragon` en `src/ppu_nativa.rs` para aceptar `evy: u32` y procesar `modo == 2` (brighten) y `modo == 3` (darken) si `efectos` está activo.
3. Actualizar `src/ppu_nativa.wgsl` para condicionar `objwin_on` a `(cfg_comp.dispcnt & 0x1000u) != 0u` y aplicar `brighten` y `darken` al final de `es_escena_dragon`.
4. Implementar el detector de cambio de capas en `src/platform.rs` / `src/perf.rs`, emitiendo `# capas ...` a `blazing-perf.txt` únicamente en transiciones.
5. Añadir pruebas unitarias de regresión en `src/ppu_nativa_tests.rs` y `src/perf_tests.rs`.
6. Ejecutar la suite de pruebas unitarias (`cargo test -p blazing-recomp`).
7. Generar el paquete Fat APK para verificación en hardware Android.

---

## Predicción vs Realidad (se completa tras ejecutar)

### 1. Qué se predijo que cambiaría
- Corrección de `consolidar_dispcnt_sprites` en `src/ppu_nativa.rs` eliminando el latch permanente de `0x8000`.
- Soporte de modos 2 y 3 en la función jerárquica del dragón en Rust y WGSL.
- Blindaje de `objwin_on` en `src/ppu_nativa.wgsl`.
- Telemetría `# capas ...` en `blazing-perf.txt` ante cada cambio de capa.

### 2. Qué cambió realmente
- `src/ppu_nativa.rs`: Latch de `0x8000` eliminado; purga `0x8000` en `self.dispcnt` y en las 160 scanlines cuando `num_sprites_ventana == 0`.
- `src/ppu_nativa.wgsl`: `objwin_on` blindado contra `disp_obj_on`, y modos 2 (brighten) y 3 (darken) integrados en `es_escena_dragon`.
- `src/platform.rs` & `src/perf.rs`: Añadido `registrar_cambio_capas` y `formatear_telemetria_capas`, persistiendo `# capas ...` en `blazing-perf.txt` únicamente en transiciones de capas.
- `src/ppu_nativa_tests.rs` & `src/perf_tests.rs`: Añadidos tests unitarios `test_consolidar_dispcnt_purga_objwin_sin_sprites`, `test_dragon_fades_brighten_darken` y `test_telemetria_capas_diferencial`.
- Totalmente probado: 75 tests en `ppu_nativa`, 17 en `pipeline` y 10 en `perf` pasados con 0 errores.
- Fat APK empaquetado: `target/blazing-recomp-release.apk` (42.73 MB).

### 3. Explicación de las discrepancias
- Ninguna discrepancia funcional frente a lo predicho. Se ajustó un préstamo en `src/platform.rs` para permitir la mutación de estado sin colisionar con la lectura inmutable del último paquete de fotograma.
