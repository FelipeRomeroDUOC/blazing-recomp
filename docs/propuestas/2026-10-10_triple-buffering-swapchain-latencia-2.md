# Propuesta: Forzar Triple Buffering Explícito en Swapchain Vulkan (Latencia = 2)

Fecha: 2026-10-10
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. **Configuración de `desired_maximum_frame_latency: 2` en [`src/platform.rs`](../../src/platform.rs):**
   - En `Compositor::new()`, se eleva `desired_maximum_frame_latency` de `1` a `2`.
   - En el backend Vulkan de `wgpu-hal`, la cantidad mínima de imágenes solicitadas en `VkSwapchainCreateInfoKHR.minImageCount` se calcula como `max(capabilities.minImageCount, desired_maximum_frame_latency + 1)`.
   - Al pasar de 1 a 2, `wgpu` solicita al menos `capabilities.minImageCount + 1` (3 imágenes en lugar de 2), acotado por `maxImageCount` si este es mayor que cero.
   - Esto evita que los controladores Vulkan en Android degraden `VK_PRESENT_MODE_MAILBOX_KHR` a un bloqueo equivalente a doble búfer, eliminando el retraso inerte de ~50 ms en `present` y permitiendo alcanzar los 60 FPS estables.
2. **Detección robusta de modo en cabecera de telemetría en [`src/platform.rs`](../../src/platform.rs):**
   - En `ensure_perf_file()`, si el primer informe (`t=1s`) llega antes del primer fotograma (`latest_frame.is_none()`), se evalúa la disponibilidad de la tabla AOT mediante `crate::recomp::load_native_blocks().is_some()`, evitando que la cabecera etiquete espuriamente `modo=INT` al arrancar.
3. **Actualización de pruebas en [`src/overlay_tests.rs`](../../src/overlay_tests.rs):**
   - Se actualiza `test_compositor_frame_latency_is_one` a `test_compositor_frame_latency_triple_buffering`, asegurando que el compositor declare de forma invariable `desired_maximum_frame_latency: 2`.

### 2. Contexto

- Contexto consultado: Último log de telemetría (`backend=Vulkan modo=INT present=Mailbox alpha=Inherit reloj=496ns`, con 18 FPS en portrait y 28 FPS en landscape, donde `present` acumula 50-54 ms de bloqueo), [`src/platform.rs`](../../src/platform.rs), [`src/present.rs`](../../src/present.rs), [`src/overlay_tests.rs`](../../src/overlay_tests.rs), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md).
- Intención: Eliminar el bloqueo inerte de ~50 ms en `present` causado por la falta de una tercera imagen en el swapchain Vulkan (`minImageCount = 2`), garantizando que siempre exista un fotograma disponible para escribir en GPU mientras el anterior es procesado por SurfaceFlinger.
- Restricciones y decisiones previas (máx. 3):
  - Calibración de presentación desactivada (`CALIBRAR_PRESENTACION: false`): La app debe arrancar inmediatamente con la configuración óptima sin demoras de 12-15s en el arranque.
  - Backend Vulkan preferente (`PREFERIR_OPENGL_ES: false`): Mantener Vulkan como API principal.
  - Portabilidad y tests en verde (origen: `AGENTS.md §4, §6`): Mantener 107+ tests en verde y cero rutas locales absolutas.

### 3. Ejemplos

- Caso normal (Renderizado en pantalla Portrait 1200×2000):
  - *Antes:* Con `desired_maximum_frame_latency: 1`, el swapchain tiene 2 imágenes (`minImageCount = 2`). Al llamar a `present`, ambas imágenes quedan retenidas por SurfaceFlinger y la cola de presentación. La CPU/GPU se bloquea durante 50–54 ms esperando una imagen libre. Rendimiento limitado a 18 FPS.
  - *Ahora:* Con `desired_maximum_frame_latency: 2`, el swapchain tiene 3 imágenes (`minImageCount = 3`, triple buffering real). Siempre hay un búfer libre disponible de inmediato para escribir; `present` deja de bloquear a la CPU/GPU (baja a <3 ms). El juego corre fluido a 60 FPS.
- Caso normal (Cabecera de telemetría al arrancar):
  - *Antes:* En `t=1s`, `latest_frame` es `None` y la cabecera escribe `# backend=Vulkan modo=INT` a pesar de que el código AOT nativo se cargó correctamente.
  - *Ahora:* `ensure_perf_file()` comprueba la presencia de bloques nativos y rotula correctamente `# backend=Vulkan modo=AOT`.

### 4. Pasos

1. Modificar [`src/platform.rs`](../../src/platform.rs):
   - En `Compositor::new()`, cambiar `desired_maximum_frame_latency: 1` a `2`.
   - En `ensure_perf_file()`, verificar `crate::recomp::load_native_blocks().is_some()` cuando `latest_frame` no esté presente aún.
2. Modificar [`src/overlay_tests.rs`](../../src/overlay_tests.rs):
   - Renombrar y actualizar la prueba a `test_compositor_frame_latency_triple_buffering`, validando `desired_maximum_frame_latency: 2`.
3. Ejecutar `cargo test` para verificar que la suite completa pase en verde.
4. Sincronizar referencias técnicas con `python scripts/extraer_referencias.py` y validar Diátaxis.
5. Reempaquetar el APK oficial con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- Uso de memoria GPU: Se asigna 1 imagen adicional de framebuffer en VRAM (para 1200×2000 en RGBA8 son ~9.6 MB de VRAM), lo cual es completamente trivial para cualquier dispositivo moderno.
- Fluidez y latencia de presentación: Desaparición del bloqueo artificial en el hilo de renderizado.

### 6. Suposiciones

- Los controladores Vulkan en Android respetan `desired_maximum_frame_latency: 2` creando al menos 3 imágenes cuando `capabilities.minImageCount <= 2` y `maxImageCount >= 3` (comportamiento estándar de Vulkan en Android).

### 7. Alcance

- Incluye:
  - Cambio a triple buffering (`desired_maximum_frame_latency: 2`) en `SurfaceConfiguration`.
  - Corrección de etiqueta `modo=AOT` en la cabecera inicial de telemetría.
  - Actualización de pruebas unitarias correspondientes.
- No incluye:
  - Re-activación del barrido dinámico de calibración (`CALIBRAR_PRESENTACION` permanece en `false`).
  - Reducción forzada de la escala de renderizado (se preserva 100% la nitidez nativa).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_compositor_frame_latency_triple_buffering
Caso de uso: Configuración de swapchain para triple buffering | Tipo: normal
  Dado el archivo `src/platform.rs`
  Cuando se inspecciona la inicialización del `Compositor`
  Entonces `desired_maximum_frame_latency` es exactamente 2.
Test: `src/overlay_tests.rs::test_compositor_frame_latency_triple_buffering`
Estado previo: falla (actualmente espera 1)

### 9. Cómo lo comprobaré

- `cargo test --lib overlay::tests::test_compositor_frame_latency_triple_buffering`
  → pasa si: el resultado es `ok`.
- Suite completa `cargo test`
  → pasa si: los 107+ tests pasan en verde.

### 10. Terminado cuando

Todos los criterios de aceptación están en verde, el APK se compila y firma limpiamente y las validaciones de Diátaxis y portabilidad están al 100%.

## Predicho vs. Real (lo escribe el agente al cerrar la propuesta)

### Criterios de aceptación
- [x] test_compositor_frame_latency_triple_buffering: Predicho: verde tras configurar `desired_maximum_frame_latency: 2` en `Compositor::new`. Real: verde en `src/overlay_tests.rs`.

### Archivos modificados
- `src/platform.rs`: `desired_maximum_frame_latency: 2` (triple buffering explícito) y `ensure_perf_file` con comprobación AOT en arranque.
- `src/overlay_tests.rs`: prueba unitaria `test_compositor_frame_latency_triple_buffering`.

### Desviaciones
- Ninguna respecto al alcance acordado.

### Fuera de alcance (hallazgos que no se arreglaron)
- Ninguno.
