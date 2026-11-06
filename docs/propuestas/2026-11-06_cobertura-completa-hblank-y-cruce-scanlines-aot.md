# Propuesta: Cobertura Completa de HBlank y Cruce de Scanlines en AOT

Estado: discrepada
- **ID:** `2026-11-06_cobertura-completa-hblank-y-cruce-scanlines-aot`
- **Fecha:** `2026-11-06`
- **Nivel:** `2`
- **Estado:** `discrepada`
- **Viene de:** `2026-11-06_sincronia-irq-hblank-vcount-aot`
- **Hereda sin cambios:** Sincronía fina en `m.cpu.mode() == Mode::Irq` con `raster_irq_habilitado`, y paso a paso cuando `irq_video_esperando && m.cpu.flag(FLAG_I)`.

---

## 1. Para decidir

### Qué cambia
Se amplía la ventana de sincronía fina en `run_frame_native` para que abarque no solo la aproximación a HBlank, sino todo el período de HBlank activo (ciclos 1006 a 1232) y la frontera de inicio de la siguiente scanline (ciclos 0 a 80) mientras `DISPSTAT` tenga habilitadas interrupciones de raster (`HBlank` o `VCount`).

### Para qué
Eliminar los dos últimos parpadeos que persisten justo antes de los relámpagos al final de la cinemática del dragón. En la propuesta anterior, la ventana `(880..1060)` terminaba en el ciclo 1060; cuando la rutina de interrupción de HBlank terminaba y la CPU retornaba a modo sistema en el ciclo ~1150, la máquina seguía en HBlank pero la condición se apagaba, despachando un bloque C11 AOT en bulto que cruzaba el ciclo 1232 y desfasaba la modulación de brillo (`BLDY` / `BLDCNT`) de los relámpagos.

### Ejemplo
- **Antes:** `(880..1060).contains(&ciclo_linea)` — En el ciclo 1150 (aún en HBlank tras salir de la ISR), se ejecutaba un bloque C11 que arrollaba el ciclo 1232 (fin de línea) y los primeros 100 ciclos de la siguiente scanline.
- **Ahora:** `(ciclo_linea >= 860 || ciclo_linea < 80)` — Se mantiene el paso a paso (`m.step()`) durante toda la ventana de HBlank y el arranque de la siguiente scanline, impidiendo que bloques AOT desfasados modifiquen registros MMIO de video fuera de tiempo.

### Pasos en lenguaje de uso
1. El usuario inicia `ppu_doble` o el APK en la cinemática del dragón.
2. Durante las 160 scanlines activas con interrupciones de raster, cada HBlank y transición de línea se procesa con exactitud de ciclo.
3. La escena del dragón completa la animación del fuego y transiciona a los relámpagos sin ningún parpadeo visual en ninguno de los dos paneles.

### Qué podría romperse
Nada en la compatibilidad general: el paso fino solo se activa durante las líneas visibles cuando `DISPSTAT & 0x30 != 0` (escenas con efectos de raster activos como la del dragón), preservando el rendimiento nativo AOT en el resto del juego.

### Qué no se toca
No se modifican los shaders WGSL, ni el pipeline de audio, ni la lógica de decodificación C11 ni `load_native_blocks`.

---

## 2. Detalle para verificar

### Suposiciones
- **S1:** En `gba-core`, el período de HBlank se extiende de `1006` a `1232` ciclos y la siguiente línea inicia en `0`. Proteger `ciclo_linea >= 860 || ciclo_linea < 80` garantiza que ninguna instrucción C11 en bulto cruce la frontera del raster. *(Verificado en `crates/gba-core/src/mem.rs:46-53`).*

### Reglas que se respetan
- **R1:** Priorizar cobertura nativa AOT manteniendo la sincronía estricta de hardware exigida por AGENTS.md §1.
- **R2:** Toda prueba y cambio debe ser verificable en PC con `ppu_doble` antes de compilar APK.

### Comprobaciones
- **C1 (Normal):** Dado que se ejecuta `cargo test recomp`, cuando se corren las pruebas unitarias, entonces la suite pasa al 100% incluyendo la sincronización de cruce de scanlines.
- **C2 (Extrema):** Dado que `ciclo_linea` alcanza 1200 en `Mode::System` con `raster_irq_habilitado`, cuando `run_frame_native` evalúa la condición, entonces selecciona `m.step()` y no despacha un bloque C11 en bulto.
- **C3 (No debe cambiar):** La compilación de `ppu_doble` con `--features proto` compila sin errores ni advertencias nuevas.

### Archivos
- `src/recomp.rs`
- `src/recomp_tests.rs`

---

## 3. Revisión humana

| Criterio | Estado |
| :--- | :--- |
| Tests automáticos en verde | ✔ |
| Verificación visual en `ppu_doble` (0 parpadeos antes del relámpago) | ✘ |

---

## 4. Evaluación

- **Estado:** `discrepada`
- **Resultado:** `fallida`

### 4.1. Hecho observado vs. predicho
- **Suposición falsa (S1):** Se supuso que evaluar `ciclo_linea >= 860 || ciclo_linea < 80` en el bucle externo de Rust (`run_frame_native`) bastaba para impedir que bloques C11 AOT cruzaran la frontera de HBlank (`1006..1232`).
- **Hecho comprobado:** Al final de la escena del dragón (justo antes de los relámpagos, `frame >= 1110`), la CPU deja de estar en `halted` y ejecuta en `Mode::System` el megagrafo C11 `g_080ba7a5` (`0x080BD80E` / `0x080BD748`). Cuando `run_frame_native` invoca `f(&RT_API, mptr)` en el ciclo `80` (justo al salir de `ciclo_linea < 80`), el control entra en la función C11 y **no regresa al bucle de Rust durante cientos de ciclos**, atravesando por dentro de C11 el ciclo `860`, el `1006` (`HBLANK_FLAG_CYCLE`) y el `1232` (`LineEnd`), porque el 89,87% de los `goto L_...` en C11 son saltos hacia adelante sin `chain_gate`. Por eso ampliar `880..1060` a `>= 860 || < 80` en el bucle externo de Rust dejó los 2 parpadeos exactamente igual.

### 4.2. Estado del proyecto y recorrido de piezas
- **Archivos editados:** `src/recomp.rs` y `src/recomp_tests.rs` (19/19 tests en verde).
- **Pieza 1 (Sincronía heredada en `Mode::Irq` e `irq_video_esperando`):** **sigue valiendo** — eliminó los parpadeos de la primera mitad de la escena cuando la CPU sí pasa por `halted`.
- **Pieza 2 (Filtro parcial por `ciclo_linea` en `run_frame_native`):** **deja de valer** — cualquier hueco en `80..860` donde se despache `f(&RT_API, mptr)` durante líneas visibles con efectos de raster activos (`raster_irq_habilitado && !en_vblank`) permite que un grafo C11 largo secuestre la ejecución y cruce HBlank/VCount sin volver a Rust.
- **Pieza 3 (Comprobaciones C1 y C3):** **sigue valiendo** — la infraestructura de tests y compilación de `ppu_doble` se mantiene intacta.
- **Resumen de la lista:** 2 siguen valiendo, 0 hay que ajustar, 1 deja de valer.

### 4.3. Opciones
- **Opción B (Recomendada · Propuesta de corrección que hereda):** Mantener paso fino (`m.step()`) durante toda la línea visible (`!en_vblank`) siempre que `raster_irq_habilitado` (`(dispstat & 0x30) != 0 && m.bus.ime`) esté activo (y/o cortar en `rt_chain_gate`), mientras VBlank (`scanline >= 160`) y todas las escenas sin HBlank/VCount IRQ corren en AOT C11 nativo. Mientras la CPU está `halted` en líneas visibles, `m.step()` salta en 1 solo paso al siguiente evento (`skip_to_next_event`), y cuando no está `halted` (los cuadros previos al relámpago), ningún grafo C11 podrá cruzar HBlank ni retrasar la ISR ni un solo ciclo.
- **Opción C (Deshacer y empezar de nuevo):** Revertir `src/recomp.rs` y `src/recomp_tests.rs`. No se recomienda porque perderíamos el progreso ya validado en la escena del dragón.

### 4.4. Decisión de la persona
- **Opción elegida:** `Opción B` (Crear propuesta de corrección que hereda eliminando el hueco `80..860` en líneas visibles y cortando `rt_chain_gate` al entrar en scanlines visibles con raster IRQ activo).
