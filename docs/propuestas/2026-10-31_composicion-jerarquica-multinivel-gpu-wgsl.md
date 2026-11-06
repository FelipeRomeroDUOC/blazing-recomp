# Propuesta: Composición Jerárquica Multinivel en GPU (Shader WGSL) y Blindaje de Staging

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / Shader WGSL, PPU Nativa y Pipeline)
Subsistemas afectados: `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/pipeline_tests.rs`

---

## 1. Resumen Ejecutivo

Esta propuesta supera de forma definitiva la limitación histórica de 2 capas de la Game Boy Advance mediante **composición jerárquica multinivel directamente en la GPU (Shader WGSL)**.
En la cinemática del dragón, el hardware clásico de GBA y el shader actual están limitados a solo 2 capas simultáneas por píxel (`primera` y `segunda`). Cuando aparece el diálogo (`BG0`), el shader mezcla el texto con el fuego (`BG3`), pero descarta por completo el fondo negro (`Backdrop`), provocando que la llamarada cambie bruscamente de brillo cada vez que una letra se imprime o el cursor titila.
La solución:
1. En `src/ppu_nativa.wgsl`, eleva la declaración de `backdrop`, `eva` y `evb` antes del bucle de prioridades e implementa la **pre-composición aditiva de `BG3` sobre `Backdrop`** en el shader de fragmento (`fs_comp`): si el fuego queda detrás del texto (`BG0`), se pre-mezcla con el fondo negro usando sus coeficientes alfa reales (`eva`, `evb`) antes de entregarse a la mezcla del diálogo, permitiendo que texto, fuego y fondo negro coexistan simultáneamente sin discontinuidades lumínicas.
2. En `src/ppu_nativa.rs`, blinda la preparación de líneas moviendo `Self::sanear_bldcnt_cinematica_dragon(staging_comps);` fuera del `match io_lineas` para que aplique siempre en ambos ramales (`Some` y `None`), y añade la condición `bldcnt != 0` para proteger las ilustraciones fijas del prólogo.
3. En `src/pipeline.rs`, acota el monitor con `bldcnt_l50 != 0` para erradicar los falsos positivos en las pinturas estáticas de "El Batir".

---

## 2. Contexto

- **Intención:** Eliminar de raíz los parpadeos intermitentes en la cinemática del dragón superando la restricción de 2 capas de la GBA con renderizado nativo moderno en GPU, manteniendo 60 FPS estables y preservando la fidelidad total en el resto del juego.
- **Restricciones:**
  - *Restricción 1 (Inocuidad en el 100% del resto del juego):* La regla de pre-composición multinivel solo debe activarse si `(cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u)`. Fuera de esta firma, el shader ejecuta el bucle de composición estándar de GBATek sin alterar ningún píxel en mapas, combates ni menús. Origen: Integridad visual del catálogo → Implica cero regresiones gráficas.
  - *Restricción 2 (Presupuesto de 60 FPS en Vulkan / OpenGL ES):* La pre-mezcla en WGSL debe ejecutarse en línea como una única instrucción aritmética de fragmento, sin pases adicionales de renderizado ni texturas intermedias. Origen: `AGENTS.md` §1 → Implica coste $\approx 0\text{ ms}$.
  - *Restricción 3 (Protección en ambos ramales de staging):* El saneamiento de registros no debe depender de si el frame llegó con `Some(lineas)` o con `None`. Origen: Concurrencia entre hilos de emulación y presentación → Implica aplicar saneamiento incondicional al final de `preparar_staging_filas`.
- **Alternativas Descartadas:**
  - *Descartada 1:* Añadir una cuarta pasada completa de renderizado en GPU solo para el fuego. Rechazada por sobrecoste innecesario de ancho de banda móvil; una función en línea en `fs_comp` logra el mismo resultado a coste cero.
  - *Descartada 2:* Modificar la rutina de texto en la ROM desensamblada. Rechazada por el principio de no alterar binarios ni lógica de guión del juego original.
- **Criterio ante imprevistos:** Si un dispositivo antiguo presentara problemas con la llamada anidada a `alpha_blend` en WGSL, se desenrollará la fórmula matemática directamente como suma de componentes vectoriales `r, g, b`.

---

## 3. Suposiciones

1. En la cinemática del dragón, el color del fuego en `BG3` se modula con los coeficientes alfa `eva` y `evb` calculados a partir de `cfg_comp.bldalpha` (con valor nominal `EVA=16, EVB=8` tras el saneamiento).
   - *Si es falso:* Se utilizará el ratio nominal 16u, 8u directamente.
   - *Prototipar:* no.
2. Las ilustraciones estáticas del prólogo ("El Batir" en $t=6..11\text{ s}$) tienen `bldcnt == 0x0000`, mientras que la cinemática activa del dragón tiene siempre `bldcnt != 0x0000` (`0x3F28`, `0x3F68` o `0x3F61`).
   - *Si es falso:* Se comprobará adicionalmente la prioridad de capas `bg_prio[3] == 3`.
   - *Prototipar:* no.
3. El compilador de shaders `wgpu` (naga) optimiza la rama `es_escena_dragon` en `fs_comp` como predicación simple sin divergencia de hilos.
   - *Si es falso:* El coste seguiría siendo despreciable frente al tiempo total de fragmento (~1 ms).
   - *Prototipar:* no.

---

## 4. Alcance

### Dentro del Alcance
- Modificar `src/ppu_nativa.wgsl` en `fs_comp` para:
  1. Mover la lectura de `backdrop` y el cálculo de `eva` y `evb` antes del bucle de prioridades `p`.
  2. Declarar `let es_escena_dragon = (cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u);`.
  3. Pre-mezclar `c3` con `backdrop` (`alpha_blend(c3, backdrop, eva, evb)`) al asignarse como `segunda` capa debajo de `BG0`.
  4. Prevenir corte prematuro cuando `primera_id == 3u` y garantizar mezcla con `Backdrop` en la unidad de mezcla final.
- Modificar `src/ppu_nativa.rs` en `preparar_staging_filas` para ejecutar `Self::sanear_bldcnt_cinematica_dragon(staging_comps);` fuera del `match io_lineas`, cubriendo ambos ramales (`Some` y `None`).
- Actualizar `sanear_bldcnt_cinematica_dragon` para exigir `c.bldcnt != 0`.
- Modificar `src/pipeline.rs` en `MonitorRasterDragon::analizar_lineas` para exigir `bldcnt_l50 != 0`.
- Añadir y actualizar pruebas unitarias en `src/ppu_nativa_tests.rs` (validación de Naga, cobertura de ramal `None` y preservación con `bldcnt == 0`) y `src/pipeline_tests.rs`.

### Fuera del Alcance
- Modificar la emulación de la CPU ni las rutinas AOT C11.
- Alterar el formato de persistencia `.sav` o la capa de audio.

---

## 5. Criterios de Aceptación

### Criterio 1: Validación Sintáctica y de Tipos de WGSL en Naga
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::el_shader_de_la_ppu_nativa_valida_con_naga`
- **Dado** el shader `src/ppu_nativa.wgsl` modificado con la lectura elevada de `backdrop` y pre-composición.
- **Cuando** se valida con el validador naga / wgpu.
- **Entonces** compila limpiamente sin errores de ámbito, sintaxis ni tipos.

### Criterio 2: Saneamiento Incondicional en Staging (Ramales Some y None)
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::test_preparar_staging_filas_sanea_en_ramal_none`
- **Dado** un fotograma de la cinemática del dragón donde `io_lineas` es `None`.
- **Cuando** se invoca `preparar_staging_filas(None, ...)`.
- **Entonces** las scanlines de `staging_comps` son saneadas con `c.bldcnt |= 0x2008` y `c.bldalpha = 0x0810`.

### Criterio 3: Inocuidad Absoluta en Mapas, Menús y Pinturas Estáticas
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** scanlines con `bldcnt == 0x0000` (pinturas estáticas del prólogo) o mapas con `BG1` (`0x1E00`).
- **Cuando** se procesa la scanline.
- **Entonces** ningún valor de `bldcnt` ni `bldalpha` se altera.

### Criterio 4: Cero Falsos Positivos en Telemetría en el Prólogo
- **Tipo:** límite
- **Test:** `cargo test --lib pipeline::tests::test_telemetria_dragon_ignora_escenas_ordinarias`
- **Dado** un volcado de fotograma de las pinturas fijas del prólogo con `dispcnt = 0x0C00` pero `bldcnt = 0x0000`.
- **Cuando** `MonitorRasterDragon::analizar_lineas` evalúa el fotograma.
- **Entonces** devuelve `hay_anomalia: false` y no se acumulan errores espurios.

---

## 6. Cómo lo comprobaré

1. **Validación del Shader WGSL y Tests de PPU:**
   `cargo test --lib ppu_nativa::tests`
   - *Pasa si:* 100% de los tests pasan, incluyendo `el_shader_de_la_ppu_nativa_valida_con_naga` y la prueba del ramal `None`.
2. **Pruebas de Pipeline:**
   `cargo test --lib pipeline::tests`
   - *Pasa si:* Los tests del monitor pasan sin falsos positivos en pantallas fijas con `bldcnt == 0`.
3. **Compilación y empaquetado Fat APK:**
   `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
   - *Pasa si:* Genera el APK listo para verificación en hardware Android.

---

## 7. Plan de Implementación

### Paso 1: Modificar `src/ppu_nativa.wgsl`
En la función `fs_comp`:
1. Mover la lectura de `backdrop` y el cómputo de `eva` y `evb` antes del bucle `for (var p: u32 = 0u; p < 4u; p = p + 1u)`:

```wgsl
    let backdrop = paleta[upy * 512u];
    let eva = min(16u, cfg_comp.bldalpha & 0x1Fu);
    let evb = min(16u, (cfg_comp.bldalpha >> 8u) & 0x1Fu);
    let evy = min(16u, cfg_comp.bldy & 0x1Fu);
    let es_escena_dragon = (cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u);
```

2. En la evaluación de `BG3` (dentro del bucle `p`):
```wgsl
        // BG3
        if (cfg_comp.bg_prio3 == p && (control & 8u) != 0u) {
            let c3 = color_capa(upy, 3u, upx, upy);
            if (c3 != 0xFFFFu) {
                if (!hay_primera) {
                    primera = c3;
                    primera_id = 3u;
                    hay_primera = true;
                    if (!es_escena_dragon && !necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                        corte = true;
                        break;
                    }
                } else {
                    if (es_escena_dragon) {
                        segunda = alpha_blend(c3, backdrop, eva, evb);
                    } else {
                        segunda = c3;
                    }
                    segunda_id = 3u;
                    hay_segunda = true;
                    corte = true;
                    break;
                }
            }
        }
```

3. En la unidad de mezcla final:
```wgsl
    var color = primera;
    if (es_escena_dragon && primera_id == 3u) {
        color = alpha_blend(primera, segunda, eva, evb);
    } else if (semi && (cfg_comp.bldcnt & (0x100u << segunda_id)) != 0u) {
        color = alpha_blend(primera, segunda, eva, evb);
...
```

### Paso 2: Modificar `src/ppu_nativa.rs`
1. En `preparar_staging_filas`, mover `Self::sanear_bldcnt_cinematica_dragon(staging_comps);` fuera del `match io_lineas`:

```rust
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
            }
        }
        Self::sanear_bldcnt_cinematica_dragon(staging_comps);
```

2. En `sanear_bldcnt_cinematica_dragon`, añadir `c.bldcnt != 0`:
```rust
if (c.dispcnt & 0x0E07) == 0x0C00 && c.bldcnt != 0 {
```

### Paso 3: Modificar `src/pipeline.rs`
En `MonitorRasterDragon::analizar_lineas`:
Añadir `&& bldcnt_l50 != 0` a la condición `es_escena_dragon`.

### Paso 4: Actualizar `src/ppu_nativa_tests.rs` y `src/pipeline_tests.rs`
- Añadir `test_preparar_staging_filas_sanea_en_ramal_none` probando `preparar_staging_filas(None, ...)`.
- Actualizar `test_sanear_bldcnt_no_altera_otras_escenas` con caso `bldcnt == 0`.
- Actualizar tests de `pipeline_tests` para verificar el descarte de falsos positivos con `bldcnt == 0`.

---

## 8. Revisión Humana

- [ ] Criterio 1: Desaparición total del parpadeo durante toda la cinemática del dragón en el dispositivo físico `✔ | ✘ | ?`.
- [ ] Criterio 2: La llamada al prólogo ($t=6..11\text{ s}$) ya no emite falsas anomalías en `blazing-perf.txt` `✔ | ✘ | ?`.
- [ ] Criterio 3: El resto del juego (combates, lluvia, menús) se visualiza idéntico y sin anomalías `✔ | ✘ | ?`.


---

## 9. Verificación Posterior

- **Fecha de Verificación:** 2026-10-31
- **Resultado:** Verificada con éxito en tests unitarios locales (69/69 tests en `ppu_nativa::tests`, 17/17 en `pipeline::tests`) y empaquetado Fat APK completado sin errores.
- **Comparación con lo predicho:**
  - *Predicho:* La pre-composición multinivel en WGSL eleva la lectura de `backdrop` y valida limpiamente con Naga. *Real:* Confirmado por `el_shader_de_la_ppu_nativa_valida_con_naga` (0 errores de validación WGSL).
  - *Predicho:* `preparar_staging_filas(None, ...)` sanea incondicionalmente las filas cuando la cinemática está activa. *Real:* Confirmado por `test_preparar_staging_filas_sanea_en_ramal_none`.
  - *Predicho:* Las pinturas estáticas del prólogo con `bldcnt == 0` no sufren alteración ni generan falsos positivos en el monitor. *Real:* Confirmado por `test_sanear_bldcnt_no_altera_otras_escenas` y `test_telemetria_dragon_ignora_escenas_ordinarias`.
- **Artefacto producido:** `target/blazing-recomp-release.apk` (42.72 MB).
