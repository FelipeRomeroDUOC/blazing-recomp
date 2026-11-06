# Propuesta: activación definitiva de la composición multicapa de la cinemática del dragón

Fecha: 2026-10-31
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Corrige la máscara de detección de la cinemática del dragón en la PPU nativa para activar la composición multicapa incondicionalmente sin truncar las capas con prioridad compartida.

### 2. Contexto

- Contexto consultado:
  - Telemetría de usuario: Líneas `# capas frame=836..3246 bg0=ON(prio=2) bg1=ON(prio=3) bg2=ON(prio=3) bg3=ON(prio=3) comp=estandar correccion=INACTIVA`.
  - Volcados: `volcados/estado-cuadro-769.txt`, `volcados/atribucion-769.txt`, `volcados/diferencias/cuadro-5602.txt`.
  - Código fuente: `src/ppu_nativa.rs` (líneas 1162-1180), `src/ppu_nativa.wgsl` (líneas 554-615), `src/perf.rs` (líneas 580-640).
- Causa raíz aislada:
  1. **Máscara errónea `(dispcnt & 0x0E07) == 0x0C00`:** Se asumía que `BG1` (bit 9, `0x0200`) estaba apagado en la cinemática del dragón. La nueva telemetría reveló que en el juego real `BG1` está **encendido** (`bg1=ON`, bit 9 = 1) con prioridad 3. Al evaluar `(dispcnt & 0x0E07)`, el resultado era `0x0E00 != 0x0C00`, provocando que `es_escena_dragon` evaluara en **falso** en el 100 % de los fotogramas.
  2. **Truncamiento por desempate en la tubería estándar de 2 capas:** Al caer en la tubería estándar, con `BG0` (prioridad 2), `BG1` (prioridad 3), `BG2` (prioridad 3) y `BG3` (prioridad 3), los dos únicos slots (`primera` y `segunda`) quedaban copados por `BG0` y `BG1`. `BG3` (el fuego) y `BG2` (el dragón) se descartaban o alternaban bruscamente según la transparencia de `BG1` y las modulaciones HBlank, provocando los parpadeos continuos.
- Restricciones:
  - Mantener 60.0 FPS estables sin sobrecargar la GPU (`AGENTS.md` §1).
  - Incluir `BG1` en la jerarquía multicapa para no perder elementos visuales (humo/nubes) sin pisar `BG2` cuando `BG1` sea transparente.
  - No incrementar la versión del proyecto sin confirmación previa del usuario (`AGENTS.md` §3).

### 3. Criterios de aceptación (comprobables)

- Escenario [Regresión]: detección de la escena del dragón con BG1 encendido
  - Dado un estado de PPU donde `dispcnt` tiene Modo 0, BG2 y BG3 encendidos (`0x0C00`), y BG1 también encendido (`0x0E00`) con `prio2 == 3 && prio3 == 3` o `(bldcnt & 8) != 0`
  - Cuando se evalúa `es_escena_dragon` en WGSL, Rust o Telemetría
  - Entonces la condición evalúa en `true`, activando la composición multicapa y reportando `comp=multicapa_dragon correccion=ACTIVA`, y si es falso (ej. no es Modo 0 o BG2/BG3 no están activos), evalúa en `false` manteniendo la tubería estándar.
  - Test: `test_deteccion_escena_dragon_con_bg1_activo` en `src/ppu_nativa_tests.rs` (`cargo test -p blazing-recomp -- test_deteccion_escena_dragon_con_bg1_activo`).

- Escenario [Normal]: composición armónica de las 4 capas de fondo en la escena del dragón
  - Dado un píxel en la escena del dragón donde BG3 (fuego), BG2 (dragón), BG1 (humo) y BG0 (subtítulos) tienen contenido
  - Cuando se ejecuta `fs_comp` en GPU o `compone_pixel_jerarquica_dragon` en CPU
  - Entonces BG3 se mezcla sobre el backdrop, BG2 se dibuja sobre el fuego, BG1 se compone sobre BG2 si es opaco (`c1 != 0xFFFF`), BG0 se superpone con mezcla de diálogo, y los sprites se dibujan sobre todo lo anterior sin truncar capas por desempate.
  - Test: `test_composicion_jerarquica_dragon_cuatro_capas` en `src/ppu_nativa_tests.rs` (`cargo test -p blazing-recomp -- test_composicion_jerarquica_dragon_cuatro_capas`).

- Escenario [Límite]: transparencia de BG1 y control de ventana en la escena del dragón
  - Dado un píxel en la escena del dragón donde BG1 es transparente (`c1 == 0xFFFF`) o `(control & 2) == 0`
  - Cuando se ejecuta la composición jerárquica en GPU o CPU
  - Entonces el color del dragón (BG2) permanece visible sin ser oscurecido ni reemplazado por BG1, y si es falso (`c1 != 0xFFFF` y `(control & 2) != 0`), BG1 se superpone sobre BG2.
  - Test: `test_composicion_dragon_bg1_transparente` en `src/ppu_nativa_tests.rs` (`cargo test -p blazing-recomp -- test_composicion_dragon_bg1_transparente`).

### 4. Qué código va a cambiar (ejemplos concretos)

- Archivos:
  - `src/ppu_nativa.wgsl`
  - `src/ppu_nativa.rs`
  - `src/perf.rs`
  - `src/ppu_nativa_tests.rs`

Ejemplo antes/ahora en `src/ppu_nativa.wgsl`:
```wgsl
// ANTES (exigía que BG1 fuera 0 con máscara 0x0E07 == 0x0C00):
let es_escena_dragon = (cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u || (cfg_comp.bg_prio2 == 2u && cfg_comp.bg_prio3 == 3u));

// AHORA (Modo 0, BG2 y BG3 activos, sin importar BG1, y prioridad 3 o mezcla de fuego):
let es_escena_dragon = (cfg_comp.dispcnt & 7u) == 0u 
    && (cfg_comp.dispcnt & 0x0C00u) == 0x0C00u 
    && ((cfg_comp.bg_prio2 == 3u && cfg_comp.bg_prio3 == 3u) || (cfg_comp.bldcnt & 8u) != 0u);
```

Y dentro de `if (es_escena_dragon)`:
```wgsl
        // 2b. Capa BG1 (humo/nubes sobre dragón y fuego si está visible)
        if ((control & 2u) != 0u) {
            let c1 = color_capa(upy, 1u, upx, upy);
            if (c1 != 0xFFFFu) {
                color_acc = c1;
            }
        }
```

Ejemplo antes/ahora en `src/ppu_nativa.rs` (`compone_pixel_jerarquica_dragon`):
```rust
// ANTES (sin c1):
pub fn compone_pixel_jerarquica_dragon(
    c0: u16, c2: u16, c3: u16, obj_color: u16, obj_visible: bool, obj_semi: bool,
    control: u16, bldcnt: u16, eva: u32, evb: u32, evy: u32, backdrop: u16,
) -> u16

// AHORA (con c1):
pub fn compone_pixel_jerarquica_dragon(
    c0: u16, c1: u16, c2: u16, c3: u16, obj_color: u16, obj_visible: bool, obj_semi: bool,
    control: u16, bldcnt: u16, eva: u32, evb: u32, evy: u32, backdrop: u16,
) -> u16 {
    ...
    // 2. Dragón (BG2 sobre fuego)
    if (control & 4) != 0 && c2 != TRANSPARENTE {
        color_acc = c2;
    }
    // 2b. Humo/nubes (BG1 sobre dragón si es visible)
    if (control & 2) != 0 && c1 != TRANSPARENTE {
        color_acc = c1;
    }
    ...
}
```

Ejemplo antes/ahora en `src/perf.rs`:
```rust
// ANTES:
let es_dragon = (dispcnt & 0x0E07) == 0x0C00 && (bldcnt != 0 || (prio2 == 2 && prio3 == 3));

// AHORA:
let es_dragon = (dispcnt & 7) == 0 && (dispcnt & 0x0C00) == 0x0C00 && ((prio2 == 3 && prio3 == 3) || (bldcnt & 8) != 0);
```

### 5. Plan paso a paso

1. Modificar la condición `es_escena_dragon` en `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs` (`sanear_bldcnt_cinematica_dragon`) y `src/perf.rs`.
2. Añadir la evaluación de `c1` en `es_escena_dragon` en `src/ppu_nativa.wgsl` y en `compone_pixel_jerarquica_dragon` en `src/ppu_nativa.rs`.
3. Actualizar las llamadas existentes a `compone_pixel_jerarquica_dragon` en `src/ppu_nativa_tests.rs` pasando `TRANSPARENTE` para `c1`.
4. Añadir las pruebas unitarias: `test_deteccion_escena_dragon_con_bg1_activo`, `test_composicion_jerarquica_dragon_cuatro_capas` y `test_composicion_dragon_bg1_transparente`.
5. Ejecutar la suite de pruebas unitarias (`cargo test -p blazing-recomp`).
6. Generar el paquete Fat APK (`scripts/package_apk.ps1`).

---

## Predicción vs Realidad (se completa tras ejecutar)

### 1. Qué se predijo que cambiaría
- Activación efectiva de `es_escena_dragon` en WGSL y telemetría (`comp=multicapa_dragon correccion=ACTIVA`).
- Inclusión de `BG1` en la jerarquía de composición.
- Erradicación de la alternancia y truncamiento de capas a 60 FPS estables.

### 2. Qué cambió realmente
- `src/ppu_nativa.wgsl` & `src/ppu_nativa.rs`: Condición `es_escena_dragon` corregida para aceptar `BG1` activo (`(dispcnt & 7) == 0 && (dispcnt & 0x0C00) == 0x0C00 && ((prio2 == 3 && prio3 == 3) || (bldcnt & 8) != 0)`).
- `src/ppu_nativa.wgsl` & `src/ppu_nativa.rs`: Incorporado `c1` (`BG1`) entre `BG2` y `BG0` en la jerarquía bottom-up.
- `src/perf.rs`: Detección en telemetría alineada para reportar `comp=multicapa_dragon correccion=ACTIVA` ante la cinemática.
- `src/ppu_nativa_tests.rs`: Actualizadas las llamadas a `compone_pixel_jerarquica_dragon` y añadidos 3 tests nuevos (`test_deteccion_escena_dragon_con_bg1_activo`, `test_composicion_jerarquica_dragon_cuatro_capas`, `test_composicion_dragon_bg1_transparente`). Todos los 78 tests de `ppu_nativa`, 17 de `pipeline` y 10 de `perf` pasados con 0 errores.
- Fat APK empaquetado: `target/blazing-recomp-release.apk` (42.73 MB).

### 3. Explicación de las discrepancias
- Ninguna discrepancia frente a lo predicho.
