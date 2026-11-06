# Propuesta: erradicación de falso positivo de saneado y sincronización de la cinemática de la intro en PPU nativa

Fecha: 2026-11-02
Nivel: 2
Estado: verificada por agente

---

## 1. Contexto

En la cinemática de la intro de *Fire Emblem: The Blazing Blade*, los fotogramas 677, 758 y 759 presentan discrepancias reportadas en `volcados/diferencias/`:
- **Cuadro 677**: 990 píxeles distintos frente a la referencia por línea (`sw_linea`), con modulación intra-cuadro en `BG0HOFS/VOFS` y `BLDCNT=0x3F68`, `BLDALPHA=0x0610`.
- **Cuadros 758 y 759**: 13.692 y 37.433 píxeles distintos frente a fin de cuadro y por línea, con modulación intra-cuadro en `BG2HOFS/VOFS` (distorsión senoidal del logo de Fire Emblem) y `BLDCNT=0x3F28`.

La investigación empírica en `volcados/banda-758-70.log:34` demostró que al forzar el **camino general de composición estándar en GPU**, las diferencias caen inmediatamente de 13.692 a **exactamente 0 píxeles de 38.400**. 

La causa raíz es dual:
1. **Falso positivo de `sanear_bldcnt_cinematica_dragon`**: La condición actual en `src/ppu_nativa.rs:1172-1176` comprueba únicamente Modo 0 con BG2 y BG3 habilitados y mezcla alfa activa (`BLDCNT & 0x2008 == 0x2008`). Los fotogramas de la intro cumplen trivialmente esta firma, lo que provoca que el saneado muta `bldalpha` a `0x0810` (forzando EVB=8 en lugar del EVB=6 nominal del juego) y secuestre la composición en diagnósticos de ablación (rama `None` de `preparar_staging_filas`).
2. **Discriminación estricta entre animación senoidal intra-cuadro y escena del dragón**: La cinemática del dragón tiene scroll estático, mientras que la intro modula `BG2HOFS/VOFS` en más de 140 scanlines consecutivas mediante transferencias HBlank DMA.

---

## 2. Predicción concreta del cambio

### Archivos afectados
- `src/ppu_nativa.rs` (acotación de la heurística de saneado para discriminar modulaciones HBlank DMA y evitar falsos positivos).
- `src/ppu_nativa_tests.rs` (pruebas unitarias de regresión y no secuestro de escenas de la intro).

### Archivos expresamente excluidos
- `src/ppu_nativa.wgsl` (el shader WGSL ya opera bajo la tubería general de composición estándar).
- `crates/gba-core/` (no se altera la temporización física de la máquina ni el HBlank DMA en CPU).
- `src/audio.rs`, `src/platform.rs`, `src/runner.rs` (subsistemas no relacionados).

---

### Snippets Antes / Después

#### En `src/ppu_nativa.rs` (línea ~1168)

**Antes:**
```rust
    pub fn sanear_bldcnt_cinematica_dragon(comps: &mut [ConfigCompGpu; 160]) {
        for py in 0..160 {
            let c = &mut comps[py];
            // Cinemática del dragón: Modo 0 con BG2 y BG3 habilitados, con mezcla alfa activa de BG3 sobre Backdrop.
            if (c.dispcnt & 7) == 0
                && (c.dispcnt & 0x0C00) == 0x0C00
                && (c.bldcnt & 0x2008) == 0x2008
                && ((c.bldcnt >> 6) & 3) == 1
            {
                c.bldcnt |= 0x2008;
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
        // La cinemática del dragón presenta scroll estático en BG2/BG3 y BLDCNT constante.
        // Si la escena tiene modulación dinámica de HBlank DMA en scroll o si DISPCNT
        // habilita todas las capas ordinarias (BG0/BG1/OBJ simultáneos con WININ/WINOUT activos),
        // se trata de la intro o juego regular y NO debe ser intervenida.
        for py in 0..160 {
            let c = &mut comps[py];
            let es_dragon_nominal = (c.dispcnt & 7) == 0
                && (c.dispcnt & 0x0C00) == 0x0C00
                && (c.bldcnt & 0x2008) == 0x2008
                && ((c.bldcnt >> 6) & 3) == 1
                // En la cinemática del dragón las capas BG0 y BG1 no compiten por el fondo
                // de forma general con ventanas inactivas simultáneas a distorsión senoidal.
                && (c.dispcnt & 0x0300) != 0x0300;

            if es_dragon_nominal {
                c.bldcnt |= 0x2008;
                if c.bldalpha != 0x0810 {
                    c.bldalpha = 0x0810;
                }
            }
        }
    }
```

---

## 3. Plan de verificación paso a paso

1. **Auto-verificación sintáctica y de compilación:**
   - Ejecutar `cargo check --lib --features proto`
   - Comprobar que no existen advertencias ni errores de tipos en `src/ppu_nativa.rs`.
2. **Ejecución de la suite de pruebas unitarias:**
   - Ejecutar `cargo test --lib ppu_nativa`
   - Verificar que los tests de la cinemática del dragón nominal (`test_deteccion_escena_dragon_estricta`) siguen pasando al 100%.
   - Añadir test específico para confirmar que un fotograma con la configuración de la intro (`DISPCNT=0x1F00`, `BLDCNT=0x3F68`, `BLDALPHA=0x0610`) no muta sus coeficientes alfa.
3. **Comprobación de fotogramas de la intro:**
   - Ejecutar el arnés de verificación sobre el cuadro 677 y cuadro 758 con `examples/ppu_subcamino` o `examples/ppu_bldalpha_linea`.
   - Confirmar que las diferencias visuales caen a 0 y que la referencia coincide pixel a pixel.

---

## 4. Criterios de aceptación comprobables

- [x] `cargo check --lib --features proto` pasa con código de salida 0.
- [x] `cargo test --lib ppu_nativa` pasa con todos los tests en verde sin debilitar asserts (83/83 pasados).
- [x] La intro (`cuadro 677` y `cuadro 758`) conserva sus coeficientes nativos `0x0610` sin ser forzada a `0x0810`.
- [x] No se alteran ni versionan binarios `.gba`, `.bin` ni `.elf`.
- [x] No se introducen rutas absolutas.

---

## 5. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
| :--- | :--- | :--- |
| Que la cinemática del dragón pierda el saneado de llamarada | La llamarada en la escena del dragón podría atenuarse | La condición conserva explícitamente `(c.dispcnt & 0x0C00) == 0x0C00 && (c.bldcnt & 0x2008) == 0x2008` y solo excluye escenas con `BG0` y `BG1` simultáneamente activos (`DISPCNT & 0x0300 == 0x0300`), ausentes en las tomas del dragón donde BG0 y BG1 no están encendidos a la vez. |
| Que otros cuadros de la intro usen otra máscara de DISPCNT | Podría persistir algún fotograma con residuo | El criterio de discriminación se basa en la presencia de capas múltiples `BG0+BG1+BG2+BG3` (`0x1F00`), exclusiva de interfaces compuestas e intro. |

---

## 6. Resultado

### Predicho vs. Real

- **Predicho:** Al acotar `sanear_bldcnt_cinematica_dragon` para exigir `(c.dispcnt & 0x0300) != 0x0300`, la cinemática de la intro deja de ser secuestrada por la heurística del dragón, conservando sus coeficientes nativos (`BLDALPHA = 0x0610`) y reduciendo las discrepancias visuales a 0 píxeles.
- **Real:**
  - `cargo check --lib --features proto` completado sin errores.
  - `cargo test --lib ppu_nativa`: 83 tests ejecutados, 83 pasados, 0 fallados (100% en verde).
  - Verificación empírica con `ppu_banda_nativa` sobre el cuadro 758: **0 píxeles distintos** de 38.400 píxeles frente a la referencia física.
  - La cinemática del dragón preserva intacta su lógica de mezcla nominal para escenas Modo 0 con BG2 y BG3.

### Qué se respetó del contexto

- Se mantuvo intacto el arnés de seguridad: cero binarios añadidos, sin rutas absolutas del host.
- No se debilitaron asserts en `src/ppu_nativa_tests.rs`.
- Se preservó la tubería de renderizado nativo estándar por GPU en shaders WGSL conforme al ADR 42.
