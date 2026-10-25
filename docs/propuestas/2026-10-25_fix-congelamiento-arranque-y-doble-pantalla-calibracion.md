# Propuesta: Eliminar congelamiento de pantalla en el arranque y ventana doble de calibración

Fecha: 2026-10-25
Nivel: 2
Estado: cerrada (verificada en hardware por el usuario el 2026-10-25: arranque inmediato y sin doble pantalla)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Al abrir la aplicación en el televisor u otros dispositivos, el juego se muestra desde el primer fotograma sin quedarse pegado en la pantalla negra inicial (`60 FPS · INT · OpenGL`) y ya no aparece una segunda pantalla pequeña superpuesta en la esquina inferior izquierda durante los primeros segundos.

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs) (líneas 218-235, 892-944, 1711-1806, 2128-2392), [`src/present.rs`](../../src/present.rs) (líneas 283-313), [`src/overlay_tests.rs`](../../src/overlay_tests.rs) (líneas 385-431), [`docs/PENDIENTES.md`](../PENDIENTES.md) (línea 45), [`docs/propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md`](2026-10-14_reactivar-la-calibracion-de-presentacion.md) y las dos fotografías del televisor aportadas por el usuario.
- Intención: Erradicar los dos fallos visuales de arranque en el televisor y otros dispositivos:
  1. **Pantalla negra congelada al iniciar (`60 FPS · INT · OpenGL`):** En `android_main`, `request_storage_permissions()` se invoca en el arranque antes de entrar al bucle de eventos (`platform.rs:2131`), incluso cuando la ROM ya es accesible o en dispositivos Android TV. Además, en `InitWindow` (`platform.rs:2221`), `state.redraw()` dibuja inmediatamente un primer cuadro vacío (`placeholder_rgba` negro con `is_native = false`, mostrando `INT`) en el contexto EGL/wgpu recién creado antes de que la transición de ventana del sistema termine de asentarse, dejando el búfer de `SurfaceFlinger` trabado hasta que se pausa y reanuda la app (`Pause` → `Resume`).
  2. **Segunda pantalla en la esquina inferior izquierda por 3 segundos (`59 FPS · AOT · OpenGL`):** Con `CALIBRAR_PRESENTACION: bool = true` (`platform.rs:233`), el tramo 3 de `PresentCalibration` (`present.rs:288-294`) fuerza `scale = 0.5` (`ESCALA_REDUCIDA`) durante 3 segundos (`t = 9..12 s`) y llama a `surface.configure()` con la mitad de resolución (`640x360` en un panel `1280x720`). En el driver OpenGL ES (`wgpu::Backends::GL`), reconfigurar la superficie EGL no altera el tamaño físico del `ANativeWindow`, por lo que `glViewport(0, 0, 640, 360)` dibuja el cuadro completo (con su propio badge `59 FPS · AOT · OpenGL`) en el cuadrante inferior izquierdo de OpenGL mientras el resto del búfer conserva el fotograma anterior a pantalla completa. Además, si ocurre un `WindowResized` durante ese tramo, `Compositor::render` (`platform.rs:934-943`) no restauraba `self.scale` si el tamaño físico `(w, h)` no había cambiado.
- Restricciones y decisiones previas (máx. 3):
  - Nitidez nativa y cierre de la sonda de calibración (origen: [`docs/PENDIENTES.md`](../PENDIENTES.md) §5 línea 45 y [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](2026-10-14_reactivar-la-calibracion-de-presentacion.md)) → implica: poner `CALIBRAR_PRESENTACION: bool = false` en `src/platform.rs` y actualizar su guardián en `src/overlay_tests.rs` con el motivo documentado → protegida por: `test_decisiones_de_presentacion_declaradas`.
  - No pedir diálogos de permisos al sistema si la ROM ya se puede leer o estamos en Android TV sin actividad de ajustes (origen: flujo no bloqueante de arranque en [`src/platform.rs`](../../src/platform.rs)) → implica: escanear primero los directorios de búsqueda y llamar a `request_storage_permissions()` únicamente si `scan_and_load_game` falla, evitando diálogos transparentes o pérdidas de foco que congelan `SurfaceFlinger` en el arranque → protegida por: `test_arranque_no_pide_permisos_antes_de_escanear_rom`.
  - Evitar bloqueo de `SurfaceFlinger` por fotograma negro prematuro en `InitWindow` y sincronizar la superficie al reanudar (origen: ciclo de vida `NativeActivity` en [`src/platform.rs`](../../src/platform.rs)) → implica: en `InitWindow`, si el `worker` ya está en marcha pero aún no ha entregado el primer `latest_frame`, no presentar el cuadro negro `INT` prematuro (esperar unos milisegundos al primer cuadro real del `worker`), y al recibir `Resume` o `WindowResized` forzar la reconfiguración limpia del swapchain → protegida por: `test_init_window_y_resume_sincronizan_swapchain`.
- Alternativas descartadas:
  - Mantener `CALIBRAR_PRESENTACION = true` pero quitar solo el tramo de `ESCALA_REDUCIDA` → descartada porque los demás tramos de calibración también reconfiguran el swapchain 4 veces durante los primeros 12 segundos provocando tirones y reinicios de superficie innecesarios, y `docs/PENDIENTES.md` ya estipula apagar `CALIBRAR_PRESENTACION`.
- Ante imprevistos, prioriza: estabilidad de la superficie de vídeo en el arranque por encima de sondas diagnósticas de arranque.

### 3. Ejemplos

- **Caso normal (arranque en TV con ROM presente):**
  - Antes: al abrir la app, `request_storage_permissions()` se lanza a ciegas antes de buscar la ROM y `InitWindow` presenta un cuadro negro con `60 FPS · INT · OpenGL`; en el televisor la pantalla se queda congelada en negro mientras el juego corre por debajo, obligando a salir y volver a entrar. Luego, a los 9 segundos de iniciar, la calibración baja `scale` a `0.50` y en OpenGL ES aparece una segunda pantalla en la esquina inferior izquierda durante 3 segundos.
  - Ahora: la app busca primero la ROM; como la encuentra, no dispara `requestPermissions` ni `startActivity` en el arranque, espera al primer fotograma real del motor antes de presentar en `InitWindow`, y con `CALIBRAR_PRESENTACION = false` mantiene escala `1.0` constante de principio a fin sin segunda pantalla.
- **Caso límite (arranque sin ROM en el dispositivo):**
  - Antes: solicita permisos antes de comprobar si la ROM ya estaba en la carpeta propia `Android/data/com.blazingrecomp.game/files/`.
  - Ahora: comprueba primero todas las rutas (incluida `external_data_path` que no requiere permisos); solo si no encuentra la ROM solicita permisos de almacenamiento y dibuja en pantalla la guía instructiva (`BLAZING RECOMP - GUIA`).

### 4. Pasos

1. En [`src/platform.rs`](../../src/platform.rs), cambiar `const CALIBRAR_PRESENTACION: bool = false;` y actualizar su comentario explicando que la sonda en TV con OpenGL ES provocaba el artefacto de doble pantalla en el cuadrante inferior izquierdo al reducir el swapchain a `0.5x`.
2. En [`src/platform.rs`](../../src/platform.rs), reordenar el arranque en `android_main`: ejecutar primero `scan_and_load_game(&search_dirs)` y llamar a `p.request_storage_permissions()` únicamente en la rama `Err` (cuando la ROM no se pudo encontrar), evitando que Android intercepte el foco de la ventana al iniciar cuando la ROM ya es legible.
3. En [`src/platform.rs`](../../src/platform.rs), asegurar la estabilidad de la superficie en `InitWindow`, `WindowResized` y `Resume`:
   - Añadir un método `Compositor::reconfigure_surface(&mut self, window_physical: (u32, u32))` que recalcule `(w, h)` y `scale = calcular_escala_swapchain(w, h)` y llame a `self.surface.configure(&self.device, &self.config)` e invalide `ultimo_game_params`, `ultimo_hud_params` y `ultimo_overlay_globals`.
   - En `InitWindow`, si `state.worker.is_some()` y `state.latest_frame.is_none()`, esperar hasta obtener el primer fotograma del worker (con un tope de seguridad de 100 ms) antes de llamar a `state.redraw()`, evitando presentar el cuadro negro falso `INT` en frío.
   - En `WindowResized`, `RedrawNeeded` y `Resume`, invocar `reconfigure_surface` si ya existe el compositor para que el swapchain no quede pegado en un búfer huérfano tras cambios de foco del sistema.
4. En [`src/overlay_tests.rs`](../../src/overlay_tests.rs), actualizar `test_decisiones_de_presentacion_declaradas` para verificar `const CALIBRAR_PRESENTACION: bool = false;` y añadir los tests de regresión para el orden de permisos y la reconfiguración de superficie.
5. Sincronizar [`docs/PENDIENTES.md`](../PENDIENTES.md), [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y cerrar la propuesta [`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](2026-10-14_reactivar-la-calibracion-de-presentacion.md).

### 5. Qué más podría verse afectado

Búsqueda realizada con `grep -rn "CALIBRAR_PRESENTACION" src/ docs/ tests/` y revisión de ciclo de vida en `src/platform.rs`:
- [`src/platform.rs`](../../src/platform.rs) — constante `CALIBRAR_PRESENTACION`, método `reconfigure_surface`, orden de `request_storage_permissions` y eventos `InitWindow` / `WindowResized` / `Resume`.
- [`src/overlay_tests.rs`](../../src/overlay_tests.rs) — guardián `test_decisiones_de_presentacion_declaradas` y nuevos tests de blindaje.
- [`docs/PENDIENTES.md`](../PENDIENTES.md), [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y [`docs/propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md`](2026-10-14_reactivar-la-calibracion-de-presentacion.md).

### 6. Suposiciones

- En OpenGL ES sobre Android (`wgpu::Backends::GL`), reconfigurar `wgpu::Surface` a `0.5x` del tamaño de `ANativeWindow` reduce el `glViewport` a la esquina inferior izquierda `(0, 0, rw, rh)` sin cambiar el tamaño físico del búfer de la ventana, lo que explica al 100% la segunda imagen superpuesta con su propio contador `59 FPS · AOT · OpenGL` durante los 3 segundos de la sonda → prototipar: no (la evidencia fotográfica coincide exactamente con el tramo 3 de `PresentCalibration` a `scale = 0.5`).
  → si es falso: habría otro punto donde se modifica `self.scale`, pero `grep` en `src/` confirma que solo `PresentCalibration` asigna `ESCALA_REDUCIDA` (`0.5`).
- Llamar a `request_storage_permissions()` en el arranque e inyectar un cuadro negro `INT` en `InitWindow` antes de que el hilo de emulación entregue el primer fotograma provoca que el compositor del televisor congele el búfer inicial hasta hacer `Pause`/`Resume` → prototipar: no (exige el televisor físico).
  → si es falso: la reconfiguración explícita de la superficie en `Resume` y `WindowResized` igualmente recupera el swapchain sin intervención del usuario.

### 7. Alcance

- Incluye:
  - Desactivar `CALIBRAR_PRESENTACION` (`false`) para eliminar el cambio de escala a `0.5x` (segunda pantalla en esquina inferior izquierda).
  - Posponer `request_storage_permissions()` únicamente a cuando no se encuentre la ROM tras escanear el almacenamiento.
  - Esperar al primer fotograma real (máx. 100 ms) en `InitWindow` y reconfigurar explícitamente la superficie en `WindowResized` y `Resume`.
  - Actualizar tests guardianes y documentación asociada.
- No incluye:
  - Eliminar el módulo `src/present.rs` ni sus tests unitarios (`src/present_tests.rs`).
  - Modificar shaders WGSL ni la PPU nativa (`src/ppu_nativa.rs`).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El guardián exige la calibración de arranque desactivada
Caso de uso: Arrancar el juego sin el barrido que reduce la escala a la mitad en el segundo 9 | Tipo: normal
  Dado el código fuente de `src/platform.rs`
  Cuando se ejecuta `test_decisiones_de_presentacion_declaradas`
  Entonces comprueba que `const CALIBRAR_PRESENTACION: bool = false;` está presente en `src/platform.rs`
Test: `src/overlay_tests.rs::test_decisiones_de_presentacion_declaradas`
Estado previo: falla (actualmente exige `true`)

Escenario: El arranque escanea la ROM antes de solicitar permisos de almacenamiento al sistema
Caso de uso: Abrir la app cuando la ROM ya es accesible sin que el sistema congele la ventana por diálogos de permisos | Tipo: límite
  Dado el flujo de inicialización en `src/platform.rs`
  Cuando se inspecciona el orden de llamadas en `android_main`
  Entonces `scan_and_load_game` se invoca antes que `request_storage_permissions`, y esta última solo se ejecuta si la carga inicial de la ROM devuelve error
Test: `src/overlay_tests.rs::test_arranque_no_pide_permisos_antes_de_escanear_rom`
Estado previo: falla

Escenario: InitWindow espera el primer cuadro real y Resume reconfigura la superficie
Caso de uso: Iniciar la app en el televisor sin quedarse pegado en el cuadro negro INT inicial | Tipo: regresión
  Dado el manejo de eventos `MainEvent::InitWindow`, `MainEvent::WindowResized` y `MainEvent::Resume` en `src/platform.rs`
  Cuando se crea o reanuda la ventana nativa
  Entonces `InitWindow` aguarda el primer fotograma del worker antes de dibujar y `Resume`/`WindowResized` invocan `reconfigure_surface`
Test: `src/overlay_tests.rs::test_init_window_y_resume_sincronizan_swapchain`
Estado previo: falla

Escenario (visible): Arranque limpio y sin doble pantalla en el televisor
Caso de uso: Abrir la app desde cero en el televisor y observar los primeros 20 segundos | Tipo: normal
  Dado el APK compilado con estos cambios instalado en el televisor
  Cuando se inicia la aplicación desde cero
  Entonces el juego se muestra en movimiento sin quedarse congelado en negro y durante los primeros 20 segundos no aparece la segunda pantalla pequeña en la esquina inferior izquierda
Test: revisión humana en el televisor
Estado previo: falla en el televisor

### 9. Cómo lo comprobaré

- El guardián exige la calibración de arranque desactivada → comando: `cargo test --lib overlay::tests::test_decisiones_de_presentacion_declaradas` → pasa si: el test compila y devuelve `ok` → no detecta: comportamiento del driver OpenGL en el televisor.
- El arranque escanea la ROM antes de solicitar permisos de almacenamiento al sistema → comando: `cargo test --lib overlay::tests::test_arranque_no_pide_permisos_antes_de_escanear_rom` → pasa si: devuelve `ok` → no detecta: políticas propietarias del firmware del TV.
- InitWindow espera el primer cuadro real y Resume reconfigura la superficie → comando: `cargo test --lib overlay::tests::test_init_window_y_resume_sincronizan_swapchain` → pasa si: devuelve `ok` → no detecta: tiempos reales de respuesta de `SurfaceFlinger`.
- Arranque limpio y sin doble pantalla en el televisor `(visible)` → revisión humana: abrir la app en la tele tras instalar el nuevo APK y verificar que arranca sin pantalla negra congelada y sin la ventana inferior izquierda a los 9-12 segundos → no detecta: otros dispositivos no probados.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación, la suite completa (`cargo test`) y el validador de documentación (`validar_docs.py`) pasan en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Ninguna.

---

## Resultado (lo completa el agente después de editar)

`Estado: verificada por agente` (los escenarios con código pasan al 100%; el visual en televisor queda para la revisión humana).

### Predicho vs. real

- **El guardián exige la calibración de arranque desactivada**: coincide.
  Resultado real: `test_decisiones_de_presentacion_declaradas` falló como se predijo antes del cambio y pasó en verde tras fijar `CALIBRAR_PRESENTACION = false`.
  Método: el anunciado (`cargo test --lib overlay::tests::test_decisiones_de_presentacion_declaradas`).
- **El arranque escanea la ROM antes de solicitar permisos de almacenamiento al sistema**: coincide.
  Resultado real: el test `test_arranque_no_pide_permisos_antes_de_escanear_rom` verifica que `scan_and_load_game` precede a `request_storage_permissions`, evitando interrupciones en la ventana cuando la ROM ya es accesible.
  Método: el anunciado (`cargo test --lib overlay::tests::test_arranque_no_pide_permisos_antes_de_escanear_rom`).
- **InitWindow espera el primer cuadro real y Resume reconfigura la superficie**: coincide.
  Resultado real: el test `test_init_window_y_resume_sincronizan_swapchain` verifica la existencia y uso de `reconfigure_surface` en `InitWindow`, `WindowResized` y `Resume`.
  Método: el anunciado (`cargo test --lib overlay::tests::test_init_window_y_resume_sincronizan_swapchain`).
- **Arranque limpio y sin doble pantalla en el televisor `(visible)`**: no pudo comprobarse con código (exige el televisor físico con el nuevo APK).

### Contexto: ¿se respetó?

- Nitidez nativa y cierre de la sonda de calibración → respetada: `CALIBRAR_PRESENTACION` quedó en `false`, eliminando la escala `0.5x`.
- No pedir diálogos de permisos si la ROM ya es legible → respetada: `p.request_storage_permissions()` solo se llama en la rama de error de carga.
- Sincronización limpia de la superficie en el ciclo de vida → respetada: `InitWindow` espera el primer cuadro del juego y `reconfigure_surface` refresca el swapchain al reanudar.
- Imprevistos: ninguno.

### Archivos modificados

- [`src/platform.rs`](../../src/platform.rs): constante `CALIBRAR_PRESENTACION = false`, método `reconfigure_surface`, espera de cuadro en `InitWindow` y orden de permisos en `android_main`.
- [`src/overlay_tests.rs`](../../src/overlay_tests.rs): guardián actualizado y dos nuevos tests de ciclo de vida y orden de escaneo.
- [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md): regenerado con `scripts/extraer_referencias.py`.
- [`docs/PENDIENTES.md`](../PENDIENTES.md), [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md) y [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md): sincronizados con el estado de la calibración.
- [`docs/propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md`](2026-10-14_reactivar-la-calibracion-de-presentacion.md): marcada como `cerrada`.
- Esta propuesta.

### Discrepancias

- Ninguna.

### Para la próxima vez

- No mantener sondas activas que modifiquen la escala de la superficie en caliente cuando el backend de producción es OpenGL ES, salvo que se gestione explícitamente el reescalado del viewport del sistema.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

---

## Revisión humana

- Escenario `(visible)` Arranque limpio y sin doble pantalla en el televisor: ✔ (confirmado por el usuario en hardware: arranca directo sin congelamiento y sin artefactos de pantalla)
- Detectado solo por el humano: necesidad de crear el compositor inmediatamente en Resume si la ventana nativa ya existía tras otorgar permisos.
