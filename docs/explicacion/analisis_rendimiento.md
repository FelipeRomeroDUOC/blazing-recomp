---
tipo: explicacion
verificado: si
---

# 🔥 Análisis de Rendimiento — Blazing Recomp

> Log analizado: `backend=OpenGL modo=INT present=AutoVsync alpha=Opaque reloj=747ns`
> Ventana: **122 segundos** de telemetría (portrait 1200×2000 → landscape 2000×1200)

> [!WARNING]
> **Estado del análisis:** Este documento es un registro histórico del diagnóstico inicial del proyecto y su conclusión ya no describe el estado actual del port. Se redactó cuando el juego operaba en `modo=INT` (95%+ en el intérprete de respaldo) con solo 13 funciones mapeadas y renderizado de scanlines por CPU en pantalla completa.
>
> **Evolución y métricas consolidadas:**
> - **Línea base inicial (Modo INT, 2026-10-09):** ~10 FPS en portrait (105 ms/frame) y ~16 FPS en landscape por saturación del intérprete y cuello de botella en `present`.
> - **Recompilación AOT (Sesión 17, 2026-10-13):** Con 17.663 bloques en C11, la CPU bajó a 13.53 ms, pero el rasterizador por software (`ppu.rs`) absorbía 7.53 ms en CPU.
> - **PPU Nativa en GPU (Sesión 30, 2026-10-23):** Con la eliminación definitiva de la PPU software y adopción de `NativePpu` en `wgpu` ([ADR 42](DECISIONES.md#42-eliminación-definitiva-de-ppushim-software-en-favor-de-pipeline-gpu-directo-nativeppu)), el tiempo medio de dibujo se redujo a **11.26 ms** en Android TV (TV Stick), alcanzando **59.7 FPS sostenidos** a lo largo de 400 segundos continuos de juego.
>
> El detalle de las fases y decisiones que hicieron posible esta optimización puede consultarse en [`DECISIONES.md`](DECISIONES.md) (ADRs 24 al 46).

---

## Contexto: Medición en Dispositivo Real

El log analizado muestra **10 FPS sostenidos en portrait** y ~16-17 FPS en landscape. El objetivo fundacional del port es 60 FPS estables. La telemetría capturada permite desglosar con precisión microsegundo a microsegundo dónde se disipa el tiempo de ejecución.

| # | Cuello de botella | Impacto estimado | Evidencia |
|---|---|---|---|
| **1** | `present` domina `dibujo` (~96-101 ms portrait / ~55-57 ms landscape) | **⬛ Crítico** | `dibujo ≈ present` en todo el log; `subida` + `codificacion` < 7 ms |
| **2** | `modo=INT` — 95%+ de la ejecución cae al intérprete | **⬛ Crítico** | ~30,000 pasos/frame intérprete vs ~300 bloques nativos |
| **3** | `backend=OpenGL` — camino de presentación más lento | **🟧 Moderado** | En portrait: de 10 a 14-17 FPS al rotar a landscape (menos píxeles) |

---

## Idea central: Anatomía de los Tres Cuellos de Botella

### 1. `present` — El Compositor del Sistema Devora el Fotograma

#### Datos

```text
Portrait  (1200×2000):  dibujo ≈ 105 ms/frame, present ≈ 98–101 ms
Landscape (2000×1200):  dibujo ≈  61 ms/frame, present ≈  55–57 ms
```

**El 93-96% de `dibujo` es `present`**, es decir, tiempo bloqueado en `frame.present()` esperando al compositor de pantalla de Android (SurfaceFlinger). Nuestro trabajo propio (`subida` ≈ 1 ms + `codificacion` ≈ 4-5 ms) solo cuesta ~6 ms → estaríamos a 60 FPS si `present` no bloqueara.

#### Causa raíz

En [`platform.rs:291`](../../src/platform.rs#L291):
```rust
desired_maximum_frame_latency: 1,
```

Con latencia 1, `wgpu-hal` pide `min_image_count = latencia + 1 = 2` al driver. Con solo 2 imágenes del swapchain:

- El modo `Mailbox` **degenera en doble búfer bloqueante**: no hay una tercera imagen donde renderizar mientras SurfaceFlinger consume la anterior.
- `present` se bloquea hasta que SurfaceFlinger libera un búfer, y su pipeline interno escala **linealmente con los píxeles de la superficie** (1200×2000 = 2.4M px → ~100 ms; 2000×1200 = 2.4M px en landscape → ~60 ms por diferente path de rotación del compositor del SoC).

Esto se confirma en el comentario del propio [`present.rs:10-15`](../../src/present.rs#L10-L15):
> `desired_maximum_frame_latency: 1` deja el swapchain con 2 imágenes: Mailbox no puede reemplazar un fotograma pendiente (necesita una tercera imagen) y `queue_present` degenera en bloqueo hasta que el compositor libera un búfer.

#### Evidencia adicional

El `present_mode` del log dice `AutoVsync`, que es `Fifo` estándar — confirma que ni siquiera se consiguió Mailbox. En Fifo con 2 búferes, cada `present` espera un ciclo completo de VSync + composición de SurfaceFlinger.

> [!IMPORTANT]
> Aquel arranque tenía la calibración desactivada ([`CALIBRAR_PRESENTACION`](../../src/platform.rs#L152)) y congelaba `latency=1` + `AutoVsync`, el peor escenario para este hardware.
>
> **Actualizado (2026-10-14):** la app fija `latency=2` desde el 2026-10-10 y el barrido está **activo**
> mientras dure el experimento
> [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](../propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md).

---

### 2. `modo=INT` — El Intérprete Hace el 95%+ del Trabajo

#### Datos del log

```text
Régimen estable (t=20–35s):  nativos ≈ 592/frame,  interprete ≈ 1,779/frame
Picos (t=14s):               nativos ≈ 1,281/frame, interprete ≈ 57,159/frame
Estado-mapa (t=50–80s):      nativos ≈ 300/frame,  interprete ≈ 30,000/frame
```

El encabezado del log confirma `modo=INT` (modo intérprete). Solo **13 funciones** están mapeadas en [`blazing.labels.toml`](../../blazing.labels.toml):

| Función | Dirección |
|---|---|
| `Reset_Handler` | `0x08000000` |
| `IntrMain` / `_Loop` / `_SetSp` | `0x080000b4-c8` |
| `BranchToThumb_Entry` | `0x080000ec` |
| `EventEngine_ContextRestore/PopRegisters` | `0x08000a3e-40` |
| `Sound_M4A_Hook` | `0x08000a90` |
| `Font_PutChar` / `Text_Init` | `0x08000bca-cc` |
| `Battle_MainLoop` | `0x08018844` |
| `Unit_InitStats` | `0x08024410` |
| `RNG_Advance` | `0x080283c8` |

El juego contaba en la fecha de este diagnóstico inicial con una estimación preliminar de **17.207 funciones** (posteriormente ampliada y consolidada en las 18.483 funciones canónicas actuales documentadas en [`blazing.labels.toml`](../../blazing.labels.toml)). Con solo 13 labels activas en ese momento, la cobertura nativa era del **0.075%** — el intérprete ejecutaba instrucción por instrucción todo lo demás.

#### Impacto en `ejec`

```text
ejec ≈ 10–14 ms promedio (con picos de 50+ ms en transiciones)
```

A 60 FPS necesitamos que `ejec` + `color` + nuestro `dibujo` propio (~6ms) quepan en 16.67 ms. Con `ejec` ya en ~12 ms **solo por el intérprete**, queda margen cero. Con bloques AOT nativos, `ejec` debería bajar a ~2-4 ms.

---

### 3. `backend=OpenGL` — Camino de Presentación Subóptimo

El encabezado confirma `backend=OpenGL`. La constante [`PREFERIR_OPENGL_ES: true`](../../src/platform.rs#L137) fuerza OpenGL ES antes que Vulkan como **experimento de presentación**.

Vulkan ofrece:
- Sincronización explícita de fences (menor bloqueo en `present`)
- Mejor triple buffering real con Mailbox
- Menor overhead de composición en drivers modernos

Sin embargo, la medición previa en tablet ya demostró que el cuello de botella de `present` es la composición de SurfaceFlinger, no el backend en sí. El impacto real de cambiar a Vulkan probablemente sea **moderado** (~10-20%) sin resolver la latencia del swapchain.

---

### Tabla Temporal Completa del Log

```text
Fase              │ Portrait (ms)   │ Landscape (ms)   │ Budget 60fps (ms)
──────────────────┼─────────────────┼──────────────────┼──────────────────
ejec (CPU)        │  10–14 avg      │  11–13 avg       │  ≤ 8
color (LCD sim)   │   0.5–1.5 avg   │   0.5–0.7 avg    │  ≤ 1
subida (GPU)      │   0.7–1.5 avg   │   0.7–1.5 avg    │  ≤ 1
codificacion      │   3.8–5.3 avg   │   3.8–5.2 avg    │  ≤ 2
present (VSync)   │  96–101 avg     │  55–57 avg       │  ≤ 5
──────────────────┼─────────────────┼──────────────────┼──────────────────
TOTAL dibujo      │ 103–106 avg     │  60–62 avg       │  ≤ 8
TOTAL frame       │ ~115 ms         │  ~73 ms          │  ≤ 16.67
FPS real          │  10             │  16–17           │  60
```

---

## Alternativas y compromisos en la Optimización

### Eje 1: Resolver el bloqueo de `present` (de ~100ms a <5ms)

| Acción | Estado / Resolución | Impacto obtenido |
|---|---|---|
| Subir `desired_maximum_frame_latency` a **2** (triple buffering real) | ✅ Completada (2026-10-10) | Redujo `present` de ~100ms a ~16-33ms desbloqueando SurfaceFlinger. |
| Reactivar calibración (`CALIBRAR_PRESENTACION = true`) para medir | ✅ Completada (2026-10-14) | Permitió seleccionar el `PresentMode` idóneo por dispositivo. |
| Probar Vulkan (`PREFERIR_OPENGL_ES = false`) junto con lat=2 | ✅ Completada (ADR 42) | Vulkan es el backend preferente; sincronización explícita por fences. |

> [!NOTE]
> La latencia 2 y el triple buffering quedaron consolidados en producción desde el 2026-10-10 en [`src/platform.rs`](../../src/platform.rs).

### Eje 2: Ampliar la cobertura de labels (de 13 a ~500+ funciones)

| Acción | Estado / Resolución | Impacto obtenido |
|---|---|---|
| Cosecha y mapeo exhaustivo de funciones | ✅ Completada (2026-10-11) | Se cosecharon 18.483 funciones con [`scripts/diagnostico/`](../../scripts/diagnostico/). |
| Priorizar funciones de hot-path (bucle principal, PPU, DMA) | ✅ Completada (2026-10-12) | El intérprete se redujo a menos del 1% en escenas tácticas. |
| Objetivo "0 intérprete" (18.483 funciones cubiertas) | 🟡 En curso | Totalidad de funciones en ROM cubiertas; pendiente IWRAM ([`docs/PENDIENTES.md`](../PENDIENTES.md)). |

### Eje 3: Optimizaciones secundarias

| Acción | Estado / Resolución | Impacto obtenido |
|---|---|---|
| Validar `alpha_mode = Opaque` llega efectivamente al compositor | ✅ Completada (2026-10-12) | Se eliminó el blending redundante a pantalla completa en el OS. |
| Escala de render dinámica como fallback para dispositivos gama baja | ✅ Descartada (ADR 27) | Descartada por borrosidad; la aceleración GPU nativa resolvió el cuello de botella. |

---

### Proyección de FPS Tras Correcciones

```text
Escenario                        │ ejec  │ color │ dibujo propio │ present │ TOTAL  │ FPS
─────────────────────────────────┼───────┼───────┼───────────────┼─────────┼────────┼─────
Actual (portrait)                │ 12    │ 1.0   │ 6             │ 98      │ ~117   │ 10
+ lat=2 (triple buffer)          │ 12    │ 1.0   │ 6             │ ~16     │ ~35    │ 28
+ lat=2 + labels AOT 500+        │  4    │ 0.5   │ 6             │ ~16     │ ~26.5  │ 37
+ lat=2 + AOT + Vulkan Mailbox   │  3    │ 0.5   │ 5             │ ~5      │ ~13.5  │ 60 ✅
```

> [!CAUTION]
> Estas son estimaciones basadas en la telemetría disponible. Los valores reales dependen del SoC, driver GPU y versión de Android del dispositivo. La única forma de validar es medir con `blazing-perf.txt` tras cada cambio.

---

## Conclusión

El rendimiento está **dominado por la espera del compositor de pantalla** (93% del tiempo de frame), no por la carga de trabajo del port. El cambio de mayor impacto inmediato es **subir la latencia del swapchain a 2** (`desired_maximum_frame_latency: 2`) — es un cambio de una sola línea en [`platform.rs:291`](../../src/platform.rs#L291). Junto con la ampliación progresiva de labels y la vuelta a Vulkan, el objetivo de 60 FPS es alcanzable.


