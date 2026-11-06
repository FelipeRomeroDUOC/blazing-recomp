# Propuesta: erradicación definitiva de parpadeos en cinemática del dragón por purga de OBJWIN y estabilización en PPU nativa

- Fecha: 2026-11-02
- Nivel: 2 (arquitectura de renderizado en GPU nativa; ciclo de vida OAM, purga de ventana de sprites y shaders WGSL)
Estado: verificada por agente

---

## 1. Contexto y Diagnóstico Causal

El usuario reporta que en la cinemática del dragón de *Fire Emblem: The Blazing Blade*, **el parpadeo de pantalla persiste en Android, pero no aparece en PC** al ejecutar el visor interactivo `ppu_doble.exe`.

El análisis técnico de telemetría (`blazing-perf-32.txt`) y de los flujos de ejecución en GPU (`src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/platform.rs`) demostró la causa raíz exacta:

### Causa Raíz 1: Desfase temporal en la purga de `OBJWIN` (`0x8000`) en `subir_buffers_lineas`
1. En la cinemática del dragón (cuadro 5602 y adyacentes), la rutina de inicialización de cajas de diálogo en FE7 escribe temporalmente `DISPCNT = 0x9F00` o `0x1F00` (activando el bit 15 `0x8000`: Ventana de Sprites `OBJWIN`), mientras que los registros de ventana `WINOUT` valen `0x0000`.
2. En `src/ppu_nativa.rs:2054-2165` (`subir_buffers_lineas`), el orden de ejecución era:
   - Línea 2055: `preparar_staging_filas` (llena `staging_filas_comp` con `0x8000` activo desde `io_lineas`).
   - Línea 2078: `consolidar_dispcnt_sprites(&mut staging_filas_comp)`. En este punto, **`self.estado.decode_oam` aún no se ha ejecutado para el fotograma actual**, por lo que `self.estado.num_sprites_ventana` conserva el valor del fotograma anterior ($K$).
   - Línea 2093: Si `oam_cambio` es verdadero, se ejecuta `self.estado.decode_oam(oam)` y actualiza `self.estado.num_sprites_ventana` a `0` ($nv = 0$).
   - Línea 2157: `if nv > 0 { self.estado.consolidar_dispcnt_sprites(...); }`. **¡Si $nv == 0$, `consolidar_dispcnt_sprites` nunca se vuelve a ejecutar!**
3. **Efecto de pantalla en negro (Flicker):** Al cambiar de un fotograma con sprites de ventana a uno sin ellos, `staging_filas_comp` se subía a la GPU con `c.dispcnt & 0x8000` encendido en todas las líneas. En el shader WGSL (`src/ppu_nativa.wgsl:516-545`), al no haber ningún sprite dibujado en `tex_obj_ventana`, cada píxel cae al control `WINOUT & 0x3F = 0x0000`, inhabilitando todas las capas (`BG0`, `BG1`, `BG2`, `BG3`, `OBJ`) y devolviendo negro absoluto `[0, 0, 0]` durante ese fotograma.

### Causa Raíz 2: Falta de purga bidireccional en `consolidar_dispcnt_sprites`
En `src/ppu_nativa.rs:1344-1354`, si `hay_objwin_linea` era verdadero pero `self.num_sprites_ventana == 0`, la función hacía `self.dispcnt |= 0x8000;`. Si no hay sprites de ventana en la OAM, `OBJWIN` no puede existir bajo ninguna circunstancia y `self.dispcnt` debe mantener `0x8000` apagado de forma inequívoca.

### Causa Raíz 3: Doble seguro ausente en shader WGSL
En `src/ppu_nativa.wgsl:519-545`, `objwin_on` no comprobaba si los sprites generales están habilitados (`disp_obj_on`), y si ninguna ventana regular (`WIN0`/`WIN1`) estaba activa y `WINOUT & 0x3F == 0`, un falso positivo de `OBJWIN` apagaba la totalidad de la pantalla.

### Causa Raíz 4: Omisión de saneado con `io_lineas` en Android
En `src/ppu_nativa.rs:1200-1205`, cuando `io_lineas` es `Some` (el camino de producción en Android), `sanear_bldcnt_cinematica_dragon` no se ejecutaba, permitiendo que fluctuaciones espurias en `BLDALPHA` atenuaran o apagaran la mezcla del fuego contra el fondo.

---

## 2. Predicción Concreta del Cambio

### Archivos Afectados
1. `src/ppu_nativa.rs`:
   - En `subir_buffers_lineas`: decodificar la OAM antes de preparar `staging_filas` y consolidar sprites, garantizando que el conteo de sprites de ventana pertenezca estrictamente al fotograma actual.
   - En `consolidar_dispcnt_sprites`: apagar `0x8000` tanto en `self.dispcnt` como en todas las scanlines de `comps` si `self.num_sprites_ventana == 0`.
   - En `preparar_staging_filas`: aplicar `sanear_bldcnt_cinematica_dragon` también cuando `io_lineas` es `Some`.
2. `src/ppu_nativa.wgsl`:
   - En `fs_comp`: supeditar `objwin_on` a `disp_obj_on`, y si no hay ventana regular activa y `WINOUT` es cero, defaultear a `0x3Fu` en lugar de apagar todas las capas espuriamente.
3. `src/ppu_nativa_tests.rs`:
   - Pruebas unitarias para la transición OAM con sprites de ventana a 0 sprites de ventana en el siguiente fotograma, verificando que `staging_filas_comp` purgue `0x8000` de inmediato sin fotogramas negros intermedios.

---

### Snippets Antes / Después

#### 1. En `src/ppu_nativa.rs` — `consolidar_dispcnt_sprites` (~línea 1344)

**Antes:**
```rust
        if hay_objwin_linea || self.num_sprites_ventana > 0 {
            self.dispcnt |= 0x8000;
        } else {
            self.dispcnt &= !0x8000;
        }

        if self.num_sprites_ventana == 0 {
            for c in comps.iter_mut() {
                c.dispcnt &= !0x8000;
            }
        }
```

**Ahora:**
```rust
        if self.num_sprites_ventana > 0 && hay_objwin_linea {
            self.dispcnt |= 0x8000;
        } else {
            self.dispcnt &= !0x8000;
            for c in comps.iter_mut() {
                c.dispcnt &= !0x8000;
            }
        }
```

---

#### 2. En `src/ppu_nativa.rs` — `preparar_staging_filas` (~línea 1200)

**Antes:**
```rust
        match io_lineas {
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
            }
            None => { ...
```

**Ahora:**
```rust
        match io_lineas {
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
                Self::sanear_bldcnt_cinematica_dragon(staging_comps);
            }
            None => { ...
```

---

#### 3. En `src/ppu_nativa.rs` — Reordenamiento en `subir_buffers_lineas` (~línea 2054)

**Antes:**
```rust
        // 4. Configuración de capas y composición por línea (160 filas).
        self.estado.preparar_staging_filas(
            io_lineas,
            &mut self.staging_filas_capa,
            &mut self.staging_filas_comp,
        );
        // ... (filas_capa upload) ...
        // 5. Sprites:
        self.estado.consolidar_dispcnt_sprites(&mut self.staging_filas_comp[..]);
        // ... decode_oam y solo consolida si nv > 0 ...
```

**Ahora:**
```rust
        // 4. Sprites (OAM): decodificar antes de las capas para que num_sprites
        //    y num_sprites_ventana reflejen fielmente el fotograma actual.
        let dispcnt_oam = self.estado.dispcnt & 0x1040;
        let oam_valida = oam.len() >= 1024;
        let oam_cambio = primera
            || !oam_valida
            || dispcnt_oam != self.dispcnt_oam_previo
            || (oam_valida && self.oam_previa[..] != oam[..1024]);

        if oam_cambio {
            if oam_valida {
                self.oam_previa.copy_from_slice(&oam[..1024]);
            }
            self.dispcnt_oam_previo = dispcnt_oam;
            self.estado.decode_oam(oam);
            // ... (upload staging_sprites si cambió) ...
        }

        // 5. Configuración de capas y composición por línea (160 filas).
        self.estado.preparar_staging_filas(
            io_lineas,
            &mut self.staging_filas_capa,
            &mut self.staging_filas_comp,
        );
        self.estado.consolidar_dispcnt_sprites(&mut self.staging_filas_comp[..]);
```

---

#### 4. En `src/ppu_nativa.wgsl` — Fallback de seguridad en `fs_comp` (~línea 519 y 538)

**Antes:**
```wgsl
    let win0_on = (cfg_comp.dispcnt & 0x2000u) != 0u;
    let win1_on = (cfg_comp.dispcnt & 0x4000u) != 0u;
    let objwin_on = (cfg_comp.dispcnt & 0x8000u) != 0u;
...
        } else if (objwin_on) {
            let obj_window = textureLoad(tex_obj_ventana, coord, 0).r != 0u;
            if (obj_window) {
                control = (cfg_comp.winout >> 8u) & 0x3Fu;
            } else {
                control = cfg_comp.winout & 0x3Fu;
            }
        }
```

**Ahora:**
```wgsl
    let win0_on = (cfg_comp.dispcnt & 0x2000u) != 0u;
    let win1_on = (cfg_comp.dispcnt & 0x4000u) != 0u;
    let objwin_on = (cfg_comp.dispcnt & 0x8000u) != 0u && disp_obj_on;
...
        } else if (objwin_on) {
            let obj_window = textureLoad(tex_obj_ventana, coord, 0).r != 0u;
            if (obj_window) {
                control = (cfg_comp.winout >> 8u) & 0x3Fu;
            } else if (!win0_on && !win1_on && (cfg_comp.winout & 0x3Fu) == 0u) {
                control = 0x3Fu;
            } else {
                control = cfg_comp.winout & 0x3Fu;
            }
        }
```

---

## 3. Plan de Verificación

1. **Compilación y suite de pruebas Rust:**
   - Ejecución de `cargo test --lib ppu_nativa` (debe pasar el 100% de los 83+ tests existentes y la nueva prueba de transición OAM sin parpadeo).
2. **Prueba de regresión de la intro:**
   - Verificar que los cuadros de la intro (360, 441, 442) mantengan 0 diferencias bit a bit frente a software.
3. **Empaquetado Fat APK Android:**
   - Delegar a `APK Builder` la compilación y generación de `target/blazing-recomp-release.apk` para prueba directa en hardware.
