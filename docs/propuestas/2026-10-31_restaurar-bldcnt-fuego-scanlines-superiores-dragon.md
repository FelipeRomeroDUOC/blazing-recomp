# Propuesta: Restaurar BLDCNT de Llamarada (0x3F68) en Scanlines Superiores de la Cinemática del Dragón

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / Arquitectura PPU Nativa)
Subsistemas afectados: `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/pipeline_tests.rs`

---

## 1. Resumen Ejecutivo

En la cinemática del dragón de *Fire Emblem: The Blazing Blade*, las scanlines 0..99 (cielo y llamarada) deben ejecutarse con `BLDCNT = 0x3F68` (primer objetivo: capa BG3 de fuego; segundo objetivo: backdrop y fondos; modo: mezcla alfa), mientras que la caja inferior de subtítulos (scanlines 100..159) conmuta a `BLDCNT = 0x3F61` (primer objetivo: texto BG0).

La telemetría en dispositivo físico (`blazing-perf-23.txt`) demostró que durante la aparición de subtítulos, el valor `0x3F61` del texto se propaga espuriamente a las scanlines 0..99 (`bldcnt_l50 = 0x3F61`). El shader GPU (`src/ppu_nativa.wgsl`), al evaluar `0x3F61`, detecta que el bit 3 de BG3 está apagado, cancela el blending alfa y muestra los tiles crudos en rojo oscuro plano, apagando la llamarada.

Esta propuesta introduce la función determinista `sanear_bldcnt_scanlines_superiores` en `src/ppu_nativa.rs` para restaurar `0x3F68` en las scanlines 0..99 cuando la escena del dragón esté activa (`bg_prio3 == 3` y `bg_prio2 == 2`), preservando intacto el blending de texto (`0x3F61`) en las scanlines 100..159 y alineando el monitor de raster en `src/pipeline.rs`.

---

## 2. Contexto

- **Origen:** Hallazgo en telemetría de dispositivo Android (`blazing-perf-23.txt`), donde `bldcnt_l50 = 0x3F61` persiste en los 60 fotogramas de cada segundo durante la escritura de subtítulos.
- **Causa Raíz:** La subrutina de diálogo (`0x080CA402`), al preparar la caja de texto, escribe `0x3F61` en el buffer global `gLCDControlBuffer.bldcnt` (`0x030028ac`). La rutina de V-Blank (`0x080011bc`) vuelca dicho buffer al registro MMIO `0x04000050` al inicio del cuadro (scanline 0), sobrescribiendo el valor base `0x3F68` de la escena antes de la scanline 100.
- **Restricciones:** Cero impacto en el presupuesto de fotogramas (60 FPS estables); preservación estricta de la compatibilidad con GBATek en `src/ppu_nativa.wgsl`; no alterar el formato de telemetría de 28 columnas en `src/telemetria.rs`.

---

## 3. Suposiciones

1. En la cinemática del dragón de FE7, las scanlines 0..99 siempre presentan la jerarquía de capas `bg_prio3 == 3` (fuego) y `bg_prio2 == 2` (dragón).
2. El registro `BLDALPHA` en scanlines 0..99 mantiene coeficientes de mezcla alfa válidos (`EVA > 0`) provistos por la escena base.
3. El valor nominal correcto de `BLDCNT` para las scanlines 0..99 es `0x3F68` (primer objetivo BG3, segundo objetivo todos los fondos y backdrop, modo alfa 1).

---

## 4. Alcance

### Dentro del Alcance
- Implementar `sanear_bldcnt_scanlines_superiores` en `src/ppu_nativa.rs` para corregir `staging_comps[0..100]` cuando `bldcnt == 0x3F61` en la escena del dragón.
- Incorporar pruebas unitarias completas en `src/ppu_nativa_tests.rs`.
- Actualizar `MonitorRasterDragon::analizar_lineas` en `src/pipeline.rs` para validar que `0x3F68` es el estado saludable de la escena y que `lineas_blend` contabiliza las líneas con bit 3 activo.

### Fuera del Alcance
- Modificar el shader WGSL `src/ppu_nativa.wgsl` (el shader opera con exactitud conforme a GBATek).
- Modificar rutinas de decodificación C11 AOT en `src-c/`.
- Incrementar la versión de la aplicación en `Cargo.toml`.

---

## 5. Criterios de Aceptación

### Criterio 1: Restauración de BLDCNT en Scanlines Superiores
- **Tipo:** normal
- **Test:** `ppu_nativa_tests::test_sanear_bldcnt_restaura_fuego_en_lineas_superiores_dragon`
- **Dado** un array de 160 scanlines donde las scanlines 0..99 tienen `bldcnt == 0x3F61`, `bg_prio3 == 3` y `bg_prio2 == 2`.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** las scanlines 0..99 tienen `bldcnt == 0x3F68`.

### Criterio 2: Preservación de Scanlines de Subtítulos Inferiores
- **Tipo:** límite
- **Test:** `ppu_nativa_tests::test_sanear_bldcnt_preserva_texto_en_lineas_inferiores`
- **Dado** un array de 160 scanlines donde las scanlines 100..159 tienen `bldcnt == 0x3F61`.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** las scanlines 100..159 preservan inalterado `bldcnt == 0x3F61`.

### Criterio 3: Inocuidad en Otras Escenas del Juego
- **Tipo:** regresión
- **Test:** `ppu_nativa_tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** un array de scanlines con prioridades arbitrarias (`bg_prio3 != 3` o `bg_prio2 != 2`) y `bldcnt == 0x3F61`.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** ninguna scanline modifica su valor de `bldcnt`.

### Criterio 4: Suite Completa en Verde
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa` y `cargo test --lib pipeline`
- **Dado** el repositorio tras aplicar los cambios en `src/ppu_nativa.rs` y `src/pipeline.rs`.
- **Cuando** se ejecutan las suites de pruebas unitarias.
- **Entonces** ambas suites terminan con código de salida 0 y 0 fallos.

---

## 6. Plan de Implementación

### Archivo 1: `src/ppu_nativa.rs`
- Agregar la función pura `sanear_bldcnt_scanlines_superiores`:
```rust
    pub fn sanear_bldcnt_scanlines_superiores(comps: &mut [ConfigCompGpu; 160]) {
        for py in 0..100 {
            let c = &mut comps[py];
            if c.bldcnt == 0x3F61 && c.bg_prio3 == 3 && c.bg_prio2 == 2 {
                c.bldcnt = 0x3F68;
            }
        }
    }
```
- Invocar `Self::sanear_bldcnt_scanlines_superiores(staging_comps);` en `preparar_staging_filas` tras poblar `staging_comps`.

### Archivo 2: `src/ppu_nativa_tests.rs`
- Implementar los tests unitarios correspondientes a los Criterios 1, 2 y 3.

### Archivo 3: `src/pipeline.rs`
- En `MonitorRasterDragon::analizar_lineas`, actualizar la condición de detección de línea de blend para validar bit 3 (BG3) y modo alfa 1:
```rust
    let modo = (bld >> 6) & 3;
    if modo == 1 && (bld & 8) != 0 {
        lineas_blend += 1;
    }
```
- Asegurar que `bldcnt_l50 == 0x3F68` se registre como estado no anómalo.

---

## 7. Comparativa Antes / Ahora

### `src/ppu_nativa.rs`
**Antes:**
```rust
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
            }
```

**Ahora:**
```rust
            Some(lineas) => {
                let (capas, comps) = Self::decodificar_lineas(lineas);
                staging_capas.copy_from_slice(&capas);
                staging_comps.copy_from_slice(&comps);
                Self::sanear_bldcnt_scanlines_superiores(staging_comps);
            }
```

### `src/pipeline.rs`
**Antes:**
```rust
            let modo = (bld >> 6) & 3;
            if modo == 1 && (bld & 4) != 0 && (bld & 0x0800) != 0 {
                lineas_blend += 1;
            }
```

**Ahora:**
```rust
            let modo = (bld >> 6) & 3;
            if modo == 1 && (bld & 8) != 0 {
                lineas_blend += 1;
            }
```

---

## 8. Cómo lo Comprobaré

- **Automatizado:**
  - Ejecutar `cargo test --lib ppu_nativa` (debe pasar con 100% de éxito).
  - Ejecutar `cargo test --lib pipeline` (debe pasar con 100% de éxito).
  - *Cláusula "no detecta":* Un test que no verifique la preservación de las líneas 100..159 no detectaría la rotura del blending en la caja de subtítulos; los tests de frontera 99 y 100 lo garantizan.
- **Empaquetado APK:**
  - Ejecutar `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

---

## 9. Revisión Humana

1. Instalar el nuevo APK generado en el dispositivo Android.
2. Iniciar la reproducción del intro cinematográfico hasta la aparición del dragón y los subtítulos ("sacudió los cielos...").
3. Verificar visualmente que la llamarada amarilla brillante permanece encendida de forma continua y estable sobre el dragón mientras se escriben los subtítulos, sin parpadear ni apagarse a rojo oscuro.
