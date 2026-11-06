# Propuesta: Ampliar Saneamiento de BLDCNT y Restaurar BLDALPHA en Scanlines Superiores del Dragón

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / Arquitectura PPU Nativa)
Subsistemas afectados: `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`

---

## 1. Resumen Ejecutivo

Esta propuesta elimina las fluctuaciones de brillo y parpadeos residuales en la cinemática del dragón generalizando el saneamiento de las scanlines 0..99. Para ello, restaura automáticamente `BLDCNT = 0x3F68` ante cualquier variante de diálogo donde el fuego no sea primer objetivo de mezcla, y fija desacopladamente los coeficientes de saturación luminosa en `BLDALPHA = 0x0810` (EVA=16, EVB=8).

---

## 2. Contexto

- **Origen:** Detección de parpadeo sutil en dispositivo Android; telemetría confirma variantes de diálogo con `bldcnt_l50 = 0x2861` y mutaciones dinámicas de `BLDALPHA` durante la escritura de subtítulos.
- **Causa Raíz:**
  1. `0x2861` (segundo objetivo selectivo BG3+BD) eludía la comparación estricta previa (`== 0x3F61`).
  2. La rutina de diálogo en ROM modifica `gLCDControlBuffer.bldalpha` alterando los pesos de mezcla alfa de la llamarada.
- **Restricciones:** 60 FPS estables sin degradación; inocuidad fuera de la cinemática del dragón (`bg_prio3 == 3 && bg_prio2 == 2`); preservación íntegra de scanlines 100..159.

---

## 3. Suposiciones

1. La cinemática del dragón en scanlines 0..99 siempre presenta `bg_prio3 == 3` y `bg_prio2 == 2`.
2. El fuego requiere la combinación canónica `BLDCNT = 0x3F68` y `BLDALPHA = 0x0810` para su saturación luminosa máxima.
3. Las scanlines 100..159 corresponden a la caja de diálogo y no deben ser modificadas.

---

## 4. Alcance

### Dentro del Alcance
- Generalizar `sanear_bldcnt_scanlines_superiores` en `src/ppu_nativa.rs` para restaurar `bldcnt = 0x3F68` si el bit 3 está apagado o el modo no es alfa, y desacopladamente fijar `bldalpha = 0x0810` si difiere de dicho valor nominal.
- Añadir tests unitarios exhaustivos en `src/ppu_nativa_tests.rs`.

### Fuera del Alcance
- Modificar el shader WGSL o el arnés de telemetría.

---

## 5. Criterios de Aceptación

### Criterio 1: Restauración de Variantes BLDCNT (0x2861)
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_restaura_variante_dialogo_2861`
- **Dado** scanlines 0..99 con `bldcnt == 0x2861` bajo la escena del dragón.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** las scanlines 0..99 tienen `bldcnt == 0x3F68`.

### Criterio 2: Restauración Desacoplada de BLDALPHA
- **Tipo:** límite
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_restaura_bldalpha_desacoplado`
- **Dado** scanlines 0..99 con `bldcnt == 0x3F68` pero `bldalpha == 0x0408` (atenuado).
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** `bldcnt` permanece en `0x3F68` y `bldalpha` se restaura a `0x0810`.

### Criterio 3: Preservación de Caja Inferior de Texto
- **Tipo:** límite
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_preserva_texto_en_lineas_inferiores`
- **Dado** scanlines 100..159 con `bldcnt == 0x2861` y `bldalpha == 0x0408`.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** las scanlines 100..159 mantienen intactos sus valores originales.

### Criterio 4: Inocuidad en Escenas Ajenas
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa_tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** scanlines con `bg_prio3 != 3` o `bg_prio2 != 2`.
- **Cuando** se ejecuta `sanear_bldcnt_scanlines_superiores`.
- **Entonces** ningún valor de `bldcnt` ni `bldalpha` es alterado.

### Criterio 5: Suite Completa en Verde
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa`
- **Dado** el repositorio tras los cambios.
- **Cuando** se ejecuta la suite unitaria.
- **Entonces** termina con código de salida 0 y 0 errores.

---

## 6. Plan de Implementación

### Archivo 1: `src/ppu_nativa.rs`
- Reemplazar la lógica de `sanear_bldcnt_scanlines_superiores`:
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

### Archivo 2: `src/ppu_nativa_tests.rs`
- Actualizar e incorporar los tests correspondientes a los Criterios 1 a 4.

---

## 7. Comparativa Antes / Ahora

### `src/ppu_nativa.rs`
**Antes:**
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

**Ahora:**
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

---

## 8. Cómo lo Comprobaré

- **Automatizado:**
  - `cargo test --lib ppu_nativa` (100% de éxito).
  - *Cláusula "no detecta":* Un test que evalúe `bldcnt` y `bldalpha` acoplados no detectaría una degradación de brillo cuando `bldcnt == 0x3F68` pero `bldalpha` cambia dinámicamente; el Criterio 2 garantiza su cobertura aislada.
- **Empaquetado APK:**
  - `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

---

## 9. Revisión Humana

1. Instalar el nuevo APK en el dispositivo Android.
2. Observar la cinemática del dragón: el fuego debe mostrarse con brillo continuo e invariable durante la escritura del texto, sin parpadeos ni fluctuaciones sutiles.
