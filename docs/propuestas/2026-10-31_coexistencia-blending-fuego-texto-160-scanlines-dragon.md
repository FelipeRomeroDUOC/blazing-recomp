# Propuesta: Coexistencia de Blending de Fuego y Texto en las 160 Scanlines de la Cinemática del Dragón

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / Arquitectura PPU Nativa)
Subsistemas afectados: `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`

---

## 1. Resumen Ejecutivo

Esta propuesta erradica definitivamente el parpadeo en la cinemática del dragón extendiendo la estabilización de mezcla a la pantalla completa (scanlines 0..159). La solución combina los objetivos de blending con `bldcnt |= 0x2008` (primer objetivo BG3 y segundo objetivo Backdrop) y asegura el modo alfa con `bldalpha = 0x0810`, permitiendo la coexistencia simultánea y sin fisuras del fuego brillante y los subtítulos en todas las scanlines donde el dragón y la llamarada estén activos (`(c.dispcnt & 0x0C00) == 0x0C00`).

---

## 2. Contexto

- **Origen:** Confirmación del usuario de que el parpadeo persistía de forma localizada tras la fase anterior.
- **Causa Raíz:**
  1. **Acotación incompleta de scanlines:** El saneamiento previo solo abarcaba scanlines 0..99 (`for py in 0..100`). La cinemática del dragón ocupa las 160 scanlines completas; en las scanlines 100..159 (franja inferior), `BLDCNT = 0x3F61` continuaba extinguiendo el fuego circundante al texto.
  2. **Fragilidad de la guarda de prioridades:** `c.bg_prio3 == 3 && c.bg_prio2 == 2` decae a `0xFF` si alguna scanline no tenía la capa habilitada en el muestreo de línea.
- **Restricciones:** 60 FPS estables sin coste de rendimiento; preservación de compatibilidad con GBATek en el shader WGSL; inocuidad absoluta fuera de la cinemática del dragón.

---

## 3. Suposiciones

1. En la cinemática del dragón de FE7, las capas BG2 (dragón) y BG3 (fuego) se encuentran habilitadas en `DISPCNT` (`(c.dispcnt & 0x0C00) == 0x0C00`) en modo 0 (`(c.dispcnt & 7) == 0`).
2. En la GBA, `BLDCNT` permite bits múltiples en el primer objetivo: si BG0 (bit 0) y BG3 (bit 3) están activos simultáneamente (`0x3F69`), el texto se mezcla sobre el fondo y el fuego se mezcla sobre el backdrop en la misma scanline sin conflicto.
3. El resplandor amarillo requiere coeficientes `BLDALPHA = 0x0810` (EVA=16, EVB=8) y modo alfa (bits 6-7 = `01b`).

---

## 4. Alcance

### Dentro del Alcance
- Renombrar y refactorizar `sanear_bldcnt_scanlines_superiores` a `sanear_bldcnt_cinematica_dragon` en `src/ppu_nativa.rs` abarcando las 160 scanlines (`for py in 0..160`).
- Validar la escena mediante `(c.dispcnt & 0x0C07) == 0x0C00` (Modo 0 con BG2 y BG3 activos).
- Aplicar `c.bldcnt |= 0x2008` (BG3 1st target + BD 2nd target), fijar modo alfa (`(c.bldcnt & !0xC0) | 0x40`) si el modo es normal (0), y fijar `c.bldalpha = 0x0810`.
- Actualizar y ampliar la suite de pruebas unitarias en `src/ppu_nativa_tests.rs`.

### Fuera del Alcance
- Modificar el shader WGSL o la capa de audio/plataforma.

---

## 5. Criterios de Aceptación

### Criterio 1: Coexistencia de Blending en Franja Inferior (Scanlines 100..159)
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_coexistencia_fuego_y_texto_160_scanlines`
- **Dado** un fotograma de la cinemática del dragón con `dispcnt == 0x0D00` donde las scanlines 100..159 tienen `bldcnt == 0x3F61` (modo texto).
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** las scanlines 100..159 tienen `bldcnt == 0x3F69` (coexistencia de bit 0 BG0 y bit 3 BG3) y `bldalpha == 0x0810`.

### Criterio 2: Saneamiento Completo de Scanlines Superiores (0..99)
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_coexistencia_fuego_y_texto_160_scanlines`
- **Dado** scanlines 0..99 con `bldcnt == 0x2861` o `0x3F28` y `dispcnt == 0x0D00`.
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** las scanlines 0..99 tienen el bit 3 activo, modo alfa 1 y `bldalpha == 0x0810`.

### Criterio 3: Inocuidad Fuera de la Cinemática del Dragón
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** scanlines de escenas con `(dispcnt & 0x0C00) != 0x0C00` (p.ej. mapas con solo BG0/BG1 o modo afín).
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** ningún valor de `bldcnt` ni `bldalpha` sufre alteración.

### Criterio 4: Suite Completa en Verde
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa`
- **Dado** el repositorio tras las modificaciones.
- **Cuando** se ejecuta la suite unitaria.
- **Entonces** finaliza con código de salida 0 y 0 errores.

---

## 6. Plan de Implementación

### Archivo 1: `src/ppu_nativa.rs`
- Renombrar y actualizar la función:
```rust
    pub fn sanear_bldcnt_cinematica_dragon(comps: &mut [ConfigCompGpu; 160]) {
        for py in 0..160 {
            let c = &mut comps[py];
            // Cinemática del dragón: Modo 0 con BG2 (dragón) y BG3 (fuego) habilitados.
            if (c.dispcnt & 0x0C07) == 0x0C00 {
                // Asegura BG3 como primer objetivo y Backdrop como segundo objetivo.
                c.bldcnt |= 0x2008;
                // Si el modo de mezcla está apagado (modo 0), fuerza modo alfa (modo 1).
                if ((c.bldcnt >> 6) & 3) == 0 {
                    c.bldcnt = (c.bldcnt & !0xC0) | 0x40;
                }
                if c.bldalpha != 0x0810 {
                    c.bldalpha = 0x0810;
                }
            }
        }
    }
```
- Actualizar la llamada en `preparar_staging_filas` a `Self::sanear_bldcnt_cinematica_dragon(staging_comps);`.

### Archivo 2: `src/ppu_nativa_tests.rs`
- Actualizar los tests unitarios para verificar las 160 scanlines y la coexistencia de `0x3F69`.

---

## 7. Comparativa Antes / Ahora

### `src/ppu_nativa.rs`
**Antes:**
```rust
    pub fn sanear_bldcnt_scanlines_superiores(comps: &mut [ConfigCompGpu; 160]) {
        for py in 0..100 {
            let c = &mut comps[py];
            if c.bg_prio3 == 3 && c.bg_prio2 == 2 {
                let modo = (c.bldcnt >> 6) & 3;
                let bg3_en_tgt1 = (c.bldcnt & 0x08) != 0;
                if !bg3_en_tgt1 || modo != 1 {
                    c.bldcnt = 0x3F68;
                }
                if c.bldalpha != 0x0810 {
                    c.bldalpha = 0x0810;
                }
            }
        }
    }
```

**Ahora:**
```rust
    pub fn sanear_bldcnt_cinematica_dragon(comps: &mut [ConfigCompGpu; 160]) {
        for py in 0..160 {
            let c = &mut comps[py];
            if (c.dispcnt & 0x0C07) == 0x0C00 {
                c.bldcnt |= 0x2008;
                if ((c.bldcnt >> 6) & 3) == 0 {
                    c.bldcnt = (c.bldcnt & !0xC0) | 0x40;
                }
                if c.bldalpha != 0x0810 {
                    c.bldalpha = 0x0810;
                }
            }
        }
    }
```

---

## 8. Cómo lo Comprobaré

- **Automatizado:**
  - `cargo test --lib ppu_nativa` (100% de éxito).
  - *Cláusula "no detecta":* Un test que solo examine scanlines 0..99 no detectaría la extinción del fuego en la franja inferior 100..159; el Criterio 1 prueba explícitamente las scanlines 100..159.
- **Empaquetado APK:**
  - `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

---

## 9. Revisión Humana

1. Instalar el nuevo APK en el dispositivo Android.
2. Comprobar que en toda la pantalla (de arriba a abajo, incluyendo detrás y alrededor de la caja de subtítulos) la llamarada arde continua, amarilla y brillante, sin líneas de corte horizontal ni parpadeos.
