# Propuesta: prueba de hipótesis de sincronización GPU en subida de VRAM en Android

Fecha: 2026-11-02
Nivel: 2 (más de 3 archivos, prueba de hipótesis experimental con impacto en sincronización GPU)
Estado: verificada por agente

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se introduce una bandera de diagnóstico experimental (`sync-vram.flag`) que, cuando está presente en `filesDir` o Descargas de Android, fuerza una sincronización explícita de la cola de la GPU (`device.poll(wgpu::PollType::Wait)`) justo después de que `subir_buffers_lineas` actualice la VRAM y antes de que el shader de composición grabe sus pases de render.

Esta prueba no altera el pipeline por defecto: sin la bandera, el código opera exactamente igual que hasta ahora. Su único propósito es **comprobar o refutar experimentalmente la hipótesis** de si las 113 discrepancias de salida en Android (`sal_android != sal_pc`) son causadas por una condición de carrera / asincronía en la subida de VRAM en controladores Vulkan móviles.

### 2. Contexto

- Contexto consultado: `src/platform.rs` (bucle de render, `subir_buffers_lineas`, `grabar_pases`), `src/ppu_nativa.rs` (actualización de `vram_buffer`), telemetría `blazing-perf-37.txt` y `pc-hash.txt`.
- Intención: Probar empíricamente si la asincronía en la subida de VRAM es la causa de que `sal` difiera en 113 fotogramas.
  - Éxito experimental = con `sync-vram.flag`, `comparar_hash_cuadros.py` reporta que las discrepancias de `sal` caen a 0 (hipótesis confirmada).
  - Falsación = las discrepancias persisten igual (hipótesis refutada; se descarta la sincronización de staging y se investiga precisión/shader).
- Hechos comprobados previamente:
  - En los 601 fotogramas de la cinemática, las entradas a la GPU (`hc`, `hv`, `ho`, `hp`, `hl`, `bld0..150`, `bg3_blend`) son 100% idénticas entre Android y PC (0 discrepancias de CPU).
  - En 488 fotogramas (81.2%), la GPU de Android produce un resultado idéntico bit a bit a PC.
  - En 113 fotogramas (18.8%), `sal` difiere. El 69.9% de estas discrepancias coinciden con fotogramas donde la VRAM muta (`hv` cambia).
- Restricciones:
  - Sin la bandera `sync-vram.flag`, el rendimiento y el flujo de ejecución no sufren ningún bloqueo (`poll` es omitido).
  - No modificar la lógica de render ni los shaders WGSL en esta fase.

### 3. Ejemplos

- Caso normal (sin bandera `sync-vram.flag`):
  - `subir_buffers_lineas` actualiza buffers vía `queue.write_buffer`.
  - El render pass se graba y envía en el mismo frame sin esperar (`device.poll` no se llama).
- Caso experimental (con `sync-vram.flag` activa):
  - `subir_buffers_lineas` actualiza buffers.
  - Si la VRAM cambió en ese cuadro, se ejecuta `device.poll(wgpu::PollType::Wait)`.
  - Se graba el pase de render garantizando que la memoria de tiles de fuego esté asentada.

### 4. Pasos

1. En `src/platform.rs`:
   - Añadir la constante `NOMBRE_BANDERA_SYNC_VRAM: &str = "sync-vram.flag";`
   - Añadir el campo `sync_vram_activo: bool` en `AppState` (default `false`).
   - Reevaluar periódicamente `sync_vram_activo` comprobando si existe `sync-vram.flag` en los directorios de telemetría/Descargas (`resolve_perf_candidates`).
   - En el bucle de renderizado donde se invoca `nativa.subir_buffers_lineas`: si `sync_vram_activo` es true, invocar `self.device.poll(wgpu::PollType::Wait)`.
2. En `scripts/diagnostico/comparar_hash_cuadros.py`:
   - Añadir al resumen final el conteo total de discrepancias de salida detectadas sobre el total de cuadros analizados.
3. Tests y verificación:
   - Crear test unitario en `src/ppu_nativa_tests.rs` que verifique que el flag de sincronización experimental esté inactivo por defecto.
   - Ejecutar la suite completa de tests de Rust (`cargo test --lib`).
   - Compilar y empaquetar el Fat APK de release mediante `package_apk.ps1`.

### 5. Qué más podría verse afectado

- `src/platform.rs`, `src/ppu_nativa_tests.rs`, `scripts/diagnostico/comparar_hash_cuadros.py`.
- No afecta partidas guardadas ni el renderizado en PC.

### 6. Suposiciones

- `device.poll(wgpu::PollType::Wait)` fuerza a Vulkan en Android a completar las transferencias de búfer pendientes antes de que el CommandEncoder continúe → prototipar: sí (al ejecutar en el dispositivo con la bandera).
  → si es falso (las discrepancias no cambian): la hipótesis queda refutada y se procede a aislar el shader WGSL.
- `device.poll` una vez por cuadro no degrada el framerate por debajo de 60 FPS → prototipar: sí (monitoreado en `blazing-perf`).

### 7. Alcance

- Incluye: bandera experimental `sync-vram.flag`, `device.poll` condicional, test unitario y generación de Fat APK.
- No incluye: cambios permanentes en la arquitectura de renderizado hasta tener el resultado empírico.

### 8. Criterios de aceptación

Escenario: Bandera de sincronización inactiva por defecto
Caso de uso: ejecución normal | Tipo: límite
  Dado que no existe el archivo `sync-vram.flag`
  Cuando se inicializa `AppState`
  Entonces `sync_vram_activo` vale `false`
Test: `ppu_nativa_tests::test_sync_vram_inactivo_por_defecto`
Estado previo: pasa

Escenario: La suite completa permanece en verde sin regresiones
Caso de uso: no romper funcionalidad existente | Tipo: regresión
  Dado el código actual con 241 tests pasando
  Cuando se introduce la bandera experimental
  Entonces `cargo test --lib` reporta 242 tests pasando y 0 fallos
Test: suite completa de Rust
Estado previo: pasa (regresión, 242 pasados)

Escenario (visible): Prueba de falsación de hipótesis en hardware real
Caso de uso: diagnóstico experimental | Tipo: normal
  Dado el APK nuevo con `sync-vram.flag` en la carpeta Descargas
  Cuando se ejecuta la cinemática del dragón y se compara con `pc-hash.txt`
  Entonces `comparar_hash_cuadros.py` indica si las 113 discrepancias de salida desaparecieron o persisten

### 9. Cómo lo comprobaré

- Bandera inactiva: `cargo test --lib test_sync_vram_inactivo_por_defecto` (vía Test Runner) → pasa (1 passed).
- Regresión: `cargo test --lib` (vía Test Runner) → 242 pasados, 0 fallos.
- (visible): Análisis de la salida de `comparar_hash_cuadros.py` con el nuevo log de Android frente a `pc-hash.txt`.

### 10. Terminado cuando

Se ejecuten los tests unitarios, se compile el APK y se obtenga la comparación empírica con hardware real.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Escenario: Bandera de sincronización inactiva por defecto: coincide
  Resultado real: Test `test_sync_vram_inactivo_por_defecto` superado (1/1 passed).
  Método: el anunciado (`cargo test --lib test_sync_vram`).

- Escenario: La suite completa permanece en verde sin regresiones: coincide
  Resultado real: 242 pasados, 0 fallados en la suite de Rust.
  Método: el anunciado (`cargo test --lib`).

- Escenario (visible): Prueba de falsación de hipótesis en hardware real: pendiente de prueba humana en el móvil.

### Contexto: ¿se respetó?

- Restricciones respetadas: `device.poll` solo se activa si existe `sync-vram.flag`; sin ella, el rendimiento y código son idénticos.
- Fat APK generado: `target/blazing-recomp-release.apk` (44.815.685 bytes).

### Archivos modificados

- `src/platform.rs` (modificado)
- `src/ppu_nativa_tests.rs` (modificado)
- `scripts/diagnostico/comparar_hash_cuadros.py` (modificado)

---

## Revisión humana

- Prueba de falsación de hipótesis en hardware real (visible): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
