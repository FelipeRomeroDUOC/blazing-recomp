---
tipo: explicacion
verificado: si
---

# 🔥 Análisis de Rendimiento — Blazing Recomp

> Log analizado: `backend=OpenGL modo=INT present=AutoVsync alpha=Opaque reloj=747ns`
> Ventana: **122 segundos** de telemetría (portrait 1200×2000 → landscape 2000×1200)

> [!WARNING]
> **Estado (2026-10-13): este análisis es histórico y su conclusión ya no describe el port.** Se
> redactó con `modo=INT`, 13 funciones mapeadas en [`blazing.labels.toml`](../../blazing.labels.toml) y
> `latency=1`. Hoy el juego corre en AOT —el intérprete quedó reducido al BIOS, con 7.759 pasos en
> 256 s— y `present` mide 7,15 ms de media, no ~100 ms, así que la espera del compositor ya no es el
> cuello de botella. El documento se conserva como registro de aquella medición y de su método.
>
> Números vigentes:
> - **Android TV** (sesión 17): `ejec` 13,53 ms, con `ppu` 7,53 (fondos 3,21 + sprites 1,35 +
>   composición 2,96) y `apu` 1,52.
> - **Host** (arnés, 7.200 fotogramas, antes → después de la decisión 34): `ppu` 0,536 → 0,367 ms,
>   `compose` 0,157 → 0,071, `bg` 0,284 → 0,197, con `spr` y `apu` planos.
>
> Detalle en [`DECISIONES.md`](DECISIONES.md) §34 y en
> [`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](../propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md).

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

El juego tiene **17,207 funciones** según el comentario de [`recomp.rs:4`](../../src/recomp.rs#L4). Con solo 13 labels, la cobertura nativa es del **0.075%** — el intérprete ejecuta instrucción por instrucción todo lo demás.

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

| Acción | Prioridad | Impacto esperado |
|---|---|---|
| Subir `desired_maximum_frame_latency` a **2** (triple buffering real) | 🔴 Inmediata | `present` de ~100ms a ~16-33ms |
| Reactivar calibración (`CALIBRAR_PRESENTACION = true`) para medir | 🟡 Alta | Dato de `lat=2` vs `lat=3` + Mailbox vs Fifo |
| Probar Vulkan (`PREFERIR_OPENGL_ES = false`) junto con lat=2 | 🟡 Alta | Posible ~30% menos en `present` por fences explícitas |

> [!TIP]
> Solo con `desired_maximum_frame_latency: 2` el FPS debería saltar de 10 a ~30+ en portrait. Es un cambio de **una línea**.

### Eje 2: Ampliar la cobertura de labels (de 13 a ~500+ funciones)

| Acción | Prioridad | Impacto esperado |
|---|---|---|
| Ejecutar `gba-mapper` para generar el mapa completo de funciones | 🔴 Inmediata | `ejec` de ~12ms a ~3-4ms |
| Priorizar las ~50 funciones de hot-path (bucle principal, PPU, DMA) | 🟡 Alta | Reducción del intérprete al < 5% |
| Objetivo "0 intérprete" (17,207 funciones cubiertas) | 🟢 Medio plazo | `interprete=0` en telemetría |

### Eje 3: Optimizaciones secundarias

| Acción | Prioridad | Impacto esperado |
|---|---|---|
| Validar `alpha_mode = Opaque` llega efectivamente al compositor | 🟡 Alta | Evitar blend a pantalla completa en SurfaceFlinger |
| Escala de render dinámica como fallback para dispositivos gama baja | 🟢 Baja | Ya probado: 0.50 baja `present` a ~3ms pero pierde nitidez |

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


