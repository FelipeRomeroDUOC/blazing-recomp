# Propuesta: Desacoplar Sprites OBJ en Guarda de PPU y Acotar Telemetría a la Cinemática del Dragón

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / PPU Nativa y Telemetría de Pipeline)
Subsistemas afectados: `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/pipeline_tests.rs`

---

## 1. Resumen Ejecutivo

Esta propuesta erradica definitivamente los parpadeos intermitentes en la cinemática del dragón y elimina el piso de falsos positivos en la telemetría de rendimiento.
La causa raíz de los parpadeos aislados reside en que la guarda de saneamiento en `src/ppu_nativa.rs` exigía que los sprites estuvieran apagados (`(c.dispcnt & 0x1E07) == 0x0C00`); cada vez que la ROM activa sprites OBJ (la flecha titilante del diálogo esperando el botón A o partículas de chispas con `DISPCNT = 0x1D00` o `0x1C00`), la guarda fallaba por 1 único fotograma, apagando súbitamente el fuego de fondo (BG3).
La solución:
1. Desacopla el bit de sprites (`OBJ`, bit 12) en la guarda de `src/ppu_nativa.rs` mediante la máscara `(c.dispcnt & 0x0E07) == 0x0C00`, habilitando las 4 combinaciones posibles con y sin subtítulos/sprites y garantizando que la llamarada mantenga su transparencia aditiva de forma ininterrumpida.
2. Condiciona `MonitorRasterDragon` en `src/pipeline.rs` para que solo compute anomalías cuando la cinemática del dragón esté efectivamente activa en el bus I/O (`(dispcnt_l50 & 0x0E07) == 0x0C00`), preservando la lectura real de `bldcnt_linea50` y erradicando los 60 falsos positivos por segundo en menús, fundidos y pantallas de inicio.

---

## 2. Contexto

- **Intención:** Eliminar los microparpadeos intermitentes de 16 ms en la cinemática del prólogo y lograr que la telemetría `# dragon_anomalias` mida con precisión quirúrgica el estado real del renderizado.
- **Restricciones:**
  - *Restricción 1 (Inocuidad en combate y mapas):* La modificación de la guarda no debe alterar bajo ninguna circunstancia escenas de combate (donde `OBJ` y ventanas están activos) ni mapas de juego (donde `BG1` está habilitado). Origen: Estabilidad visual del juego → Implica que la máscara debe exigir estrictamente `BG1 == 0`, `BG2 == 1`, `BG3 == 1` y Modo 0 (`(dispcnt & 0x0E07) == 0x0C00`).
  - *Restricción 2 (Presupuesto de 60 FPS):* Las evaluaciones de máscaras a nivel de línea y en el monitor deben ser operaciones bit a bit puras en registros locales sin asignaciones dinámicas. Origen: `AGENTS.md` §1 → Implica coste de tiempo $\approx 0\text{ ns}$ por fotograma.
  - *Restricción 3 (Contrato de Telemetría):* El formato de salida de `blazing-perf.txt` debe mantenerse idéntico al actual. Origen: `telemetria_contrato_tests.rs` → Implica reutilizar la estructura `AnomaliasDragonAcumuladas`.
- **Alternativas Descartadas:**
  - *Descartada 1:* Forzar el saneamiento en el shader WGSL (`fs_comp`). Rechazada porque WGSL debe mantenerse neutral y conforme a GBATek; el saneamiento de registros corruptos pertenece al CPU/Staging en Rust.
  - *Descartada 2:* Interceptar la subrutina de texto `0x080CA402` en C11. Rechazada porque modificar funciones reescribe lógica AOT del juego con riesgo de desincronización de eventos de guión.
- **Criterio ante imprevistos:** Si tras corregir la máscara se detectase alguna escena no deseada con `(dispcnt & 0x0E07) == 0x0C00`, se incorporará la verificación de prioridades `c.bg_prio[3] == 3 && c.bg_prio[2] == 2` como segundo factor de exclusión.

---

## 3. Suposiciones

1. En la cinemática del dragón, `DISPCNT` puede alternar dinámicamente entre `0x0C00` (sin texto ni sprites), `0x0D00` (con texto en BG0), `0x1C00` (con sprites sin texto) y `0x1D00` (con texto y sprites simultáneos). En los 4 casos, `(dispcnt & 0x0E07)` arroja exactamente `0x0C00`.
   - *Si es falso:* Se examinará si la ROM activa modos afines (Modo 1 o 2).
   - *Prototipar:* no.
2. En los menús, logos y mapas de juego ordinarios, `(dispcnt & 0x0E07)` nunca es igual a `0x0C00` porque `BG1` (`0x0200`) está encendido o el modo de vídeo no es 0.
   - *Si es falso:* Se añadirá la guarda de prioridades de capas `c.bg_prio[3] == 3`.
   - *Prototipar:* no.
3. El cursor de diálogo parpadeante en FE7 utiliza la capa de sprites (`OBJ`, bit 12 de `DISPCNT`).
   - *Si es falso:* El parpadeo obedecería exclusivamente a eventos de VCount; la solución seguirá siendo válida al cubrir ambos casos.
   - *Prototipar:* no.

---

## 4. Alcance

### Dentro del Alcance
- Modificar la máscara de comprobación en `sanear_bldcnt_cinematica_dragon` en `src/ppu_nativa.rs` de `0x1E07` a `0x0E07`.
- Filtrar la comprobación de `MonitorRasterDragon::analizar_lineas` en `src/pipeline.rs` para que retorne `hay_anomalia: false` fuera de la cinemática (`(dispcnt_l50 & 0x0E07) != 0x0C00`), manteniendo `bldcnt_linea50: bldcnt_l50`.
- Crear y ampliar pruebas unitarias en `src/ppu_nativa_tests.rs` que verifiquen el saneamiento con sprites activos (`0x1D00`, `0x1C00`), con texto (`0x0D00`) y la inocuidad con capas de mapa (`0x1E00`).
- Actualizar las pruebas unitarias en `src/pipeline_tests.rs` configurando los fixtures con `dispcnt = 0x0D00` para pruebas de anomalías del dragón y verificando la ausencia de falsos positivos en menús y logos.

### Fuera del Alcance
- Modificar el shader WGSL (`src/ppu_nativa.wgsl`).
- Modificar el formato de archivo de telemetría o el compositor de presentación.

---

## 5. Criterios de Aceptación

### Criterio 1: Saneamiento de Fuego con Sprites y Cursor de Diálogo Activos
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_permite_sprites_obj_y_cursor`
- **Dado** una scanline con `dispcnt = 0x1D00` (Modo 0, BG0, BG2, BG3 y OBJ activos) y `bldcnt = 0x3F61` (caja de diálogo activa).
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** `bldcnt` pasa a ser `0x3F69` (coexistencia de BG0 y BG3 como 1st targets) y `bldalpha` se fija en `0x0810`.

### Criterio 2: Saneamiento de Fuego con Sprites Aislados sin Diálogo
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_permite_sprites_aislados`
- **Dado** una scanline con `dispcnt = 0x1C00` (Modo 0, BG2, BG3 y OBJ activos, sin BG0) y `bldcnt = 0x3F61`.
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** `bldcnt` se sanea a `0x3F69` y `bldalpha == 0x0810`.

### Criterio 3: Inocuidad Absoluta en Mapas con BG1 Habilitado
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** scanlines con `dispcnt = 0x1E00` (Modo 0, BG1, BG2, BG3 y OBJ activos, típico de mapas de terreno).
- **Cuando** se ejecuta `sanear_bldcnt_cinematica_dragon`.
- **Entonces** ningún valor de `bldcnt` ni `bldalpha` sufre alteración.

### Criterio 4: Supresión de Falsos Positivos en Telemetría Fuera del Dragón
- **Tipo:** límite
- **Test:** `cargo test --lib pipeline::tests::test_telemetria_dragon_ignora_escenas_ordinarias`
- **Dado** un volcado I/O de fotograma de menús o logos con `dispcnt = 0x1000` o `0x0000` y `lineas_blend = 0`.
- **Cuando** `MonitorRasterDragon::analizar_lineas` evalúa el fotograma.
- **Entonces** `hay_anomalia` devuelve `false` y no se acumulan errores.

### Criterio 5: Detección Genuina de Anomalías Durante la Cinemática
- **Tipo:** normal
- **Test:** `cargo test --lib pipeline::tests::test_telemetria_dragon_detecta_anomalia_bldcnt`
- **Dado** un fotograma con `dispcnt = 0x0D00` (cinemática activa) donde `bldcnt_l50 = 0x3F61` y `lineas_blend = 0`.
- **Cuando** `MonitorRasterDragon::analizar_lineas` evalúa el fotograma.
- **Entonces** `hay_anomalia` devuelve `true`.

---

## 6. Cómo lo comprobaré

1. **Pruebas unitarias de PPU:**
   `cargo test --lib ppu_nativa::tests`
   - *Pasa si:* 100% de los tests pasan, incluyendo los nuevos escenarios con `0x1D00` y `0x1C00`.
   - *No detecta:* Errores de sincronización visual en tiempo real en dispositivo móvil.
2. **Pruebas unitarias de Pipeline:**
   `cargo test --lib pipeline::tests`
   - *Pasa si:* Los tests del monitor pasan sin falsos positivos en pantallas sin cinemática y con fixtures actualizados.
   - *No detecta:* Caídas de framerate por GPU.
3. **Compilación y empaquetado Fat APK:**
   `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
   - *Pasa si:* El APK se compila sin errores y queda listo para validación en dispositivo.

---

## 7. Plan de Implementación

### Paso 1: Modificar `src/ppu_nativa.rs`
Ajustar la máscara de detección en `sanear_bldcnt_cinematica_dragon`:

```rust
// ANTES (src/ppu_nativa.rs:1044)
if (c.dispcnt & 0x1E07) == 0x0C00 {

// AHORA
if (c.dispcnt & 0x0E07) == 0x0C00 {
```

### Paso 2: Actualizar `src/ppu_nativa_tests.rs`
Añadir pruebas unitarias que certifiquen el saneamiento cuando `OBJ` está encendido (`0x1D00` y `0x1C00`) y confirmar la exclusión de `BG1` (`0x1E00`).

### Paso 3: Modificar `src/pipeline.rs`
En `MonitorRasterDragon::analizar_lineas`:

```rust
// ANTES (src/pipeline.rs:744-758)
let bldcnt_l50 = u16::from_le_bytes([lineas[50][0x50], lineas[50][0x51]]);
let mut lineas_blend = 0u32;
for py in 0..100 {
...
let hay_anomalia =
    lineas_blend == 0 || ((bldcnt_l50 >> 6) & 3 != 1) || ((bldcnt_l50 & 8) == 0);

// AHORA
let dispcnt_l50 = u16::from_le_bytes([lineas[50][0x00], lineas[50][0x01]]);
let bldcnt_l50 = u16::from_le_bytes([lineas[50][0x50], lineas[50][0x51]]);
let es_escena_dragon = (dispcnt_l50 & 0x0E07) == 0x0C00;
if !es_escena_dragon {
    return DiagnosticoDragon {
        hay_anomalia: false,
        bldcnt_linea50: bldcnt_l50,
        bldcnt_fin,
        lineas_blend: 0,
        tiene_io_lineas: true,
    };
}
let mut lineas_blend = 0u32;
for py in 0..100 {
...
let hay_anomalia =
    lineas_blend == 0 || ((bldcnt_l50 >> 6) & 3 != 1) || ((bldcnt_l50 & 8) == 0);
```

### Paso 4: Actualizar `src/pipeline_tests.rs`
- Actualizar los fixtures de `test_telemetria_dragon_detecta_anomalia_bldcnt` y `test_telemetria_dragon_silenciosa_en_fotograma_normal` asignando `dispcnt = 0x0D00` en la línea 50 para simular la escena del dragón activa.
- Añadir `test_telemetria_dragon_ignora_escenas_ordinarias` con `dispcnt = 0x1000` o `0x0000` para verificar que devuelve `hay_anomalia: false`.

---

## 8. Revisión Humana

- [ ] Criterio 1: Desaparición total del parpadeo intermitente en la cinemática del dragón en el dispositivo físico `✔ | ✘ | ?`.
- [ ] Criterio 2: La telemetría `# dragon_anomalias` ya no emite 60 fallos por segundo en menús ni pantallas de carga `✔ | ✘ | ?`.
- [ ] Criterio 3: Los combates y mapas regulares se visualizan exactamente igual sin alteraciones gráficas `✔ | ✘ | ?`.


---

## 9. Verificación Posterior

- **Fecha de Verificación:** 2026-10-31
- **Resultado:** Verificada con éxito en tests unitarios locales (217 tests pasando) y empaquetado Fat APK completado sin advertencias críticas.
- **Comparación con lo predicho:**
  - *Predicho:* Desacoplar `OBJ` mediante `(c.dispcnt & 0x0E07) == 0x0C00` permite mantener el saneamiento del fuego (`0x3F69`) con sprites y diálogo activos (`0x1D00` y `0x1C00`). *Real:* Confirmado por `test_sanear_bldcnt_permite_sprites_obj_y_cursor` y `test_sanear_bldcnt_permite_sprites_aislados`.
  - *Predicho:* Mapas ordinarios con `BG1` (`0x1E00`) quedan 100% inalterados. *Real:* Confirmado por `test_sanear_bldcnt_no_altera_otras_escenas`.
  - *Predicho:* `MonitorRasterDragon::analizar_lineas` devuelve `hay_anomalia: false` fuera de la cinemática sin emitir falsos positivos continuos. *Real:* Confirmado por `test_telemetria_dragon_ignora_escenas_ordinarias`.
- **Artefacto producido:** `target/blazing-recomp-release.apk` (42.72 MB).
