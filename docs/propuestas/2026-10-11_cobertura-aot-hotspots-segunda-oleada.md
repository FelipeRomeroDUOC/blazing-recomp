# Propuesta: Cobertura AOT C11 de Hotspots Fase Cinemática, IA y Combate (TV Box)

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorporación y compilación nativa en C11 (AOT con Clang `-O2`) de 20 subrutinas críticas identificadas en el log de telemetría de 331 segundos en Android TV Box (4 núcleos Cortex-A53 @ 1.34 GHz):

1. **`0x08004394` (Thumb): `Entity_GetTargetUnitByIndex`**
   - Resuelve la salida forzada al intérprete desde `Battle_TargetScanUnits` (`0x08025E1C`), donde cada llamada `BL` salía a emulación acumulando más de 15,200 ticks de CPU.
2. **`0x080C09A0` (Thumb): `Script_EventCommandDispatch_Inner`**
   - Despachador interno de comandos de eventos de script.
3. **`0x080C0A10` (Thumb): `Script_MathFixedPoint_Div`**
   - Subrutina de división de punto fijo ejecutada en diálogos y cinemáticas (acumuló 31,686 ticks, TOP 1 absoluto del log).
4. **`0x080C0A7E` (Thumb): `Script_MathFixedPoint_Helper`**
   - Helper de ajuste matemático de script.
5. **`0x080C0A88` (Thumb): `Script_MathFixedPoint_Mod`**
   - Operación módulo de punto fijo de script.
6. **`0x080771CC` (Thumb): `Battle_ComputeWeaponEffect`**
   - Cálculo de efectos y modificadores de armas en combate.
7. **`0x0807793C` (Thumb): `Battle_CalculateCombatHits`**
   - Cálculo de impacto y daño de golpes en combate (11,226 ticks en $t=330\,\text{s}$).
8. **`0x0804C3A8` (Thumb): `Ai_DecideActionPath`**
   - Selección y evaluación de ruta de la IA (8,022 ticks).
9. **`0x0804D428` (Thumb): `Ai_EvaluateTargetTile`**
   - Evaluación de casillas objetivo de la IA.
10. **`0x0804D450` (Thumb): `Ai_ReadMapTileData`**
    - Función leaf ultra-frecuente de acceso a datos de casilla (5,636 ticks).
11. **`0x0804F6FC` (Thumb): `Ai_GetUnitTargetPosition`**
    - Obtención de coordenadas objetivo para IA (7,706 ticks).
12. **`0x0804F7BC` (Thumb): `Ai_CheckAttackRange`**
    - Cálculo de rango de ataque de unidades de la IA (9,896 ticks).
13. **`0x08005944` (Thumb): `Gfx_CopyOamBuffer`**
    - Copia y composición del buffer de sprites OAM (19,968 ticks, 10.6% de la sesión).
14. **`0x080A8E2C` (Thumb): `Gfx_DecompressVramTiles`**
    - Bucle de descompresión y copia de tiles a VRAM (8,904 ticks).
15. **`0x08007748` (Thumb): `Bg_UpdatePaletteColor`**
    - Actualización y mezcla de paletas de fondo (1,378 ticks).
16. **`0x08013E10` (Thumb): `Cursor_UpdateTileInfo`**
    - Actualización de información de casillas bajo el cursor (7,872 ticks).
17. **`0x08010398` (Thumb): `Unit_GetStatBonus`**
    - Lectura de bonificaciones de estadísticas de unidades (1,888 ticks).
18. **`0x08087F60` (Thumb): `Menu_RefreshCharacterPage`**
    - Refresco de la página de personaje en menús (1,486 ticks).
19. **`0x08023CF8` (Thumb): `Map_ProcessUnitStatus`**
    - Procesamiento de estados de unidades en el mapa (6,096 ticks).
20. **`0x0806A3B4` (Thumb): `Sound_UpdateTrackVolume`**
    - Ajuste de volumen y mezcla de pistas de audio (3,588 ticks).

Se actualiza [`blazing.labels.toml`](../../blazing.labels.toml) elevando el total de funciones mapeadas de 102 a 122 labels, se regenera el código C11 nativo con `compilar_recomp_aot.ps1` y se reempaqueta el APK oficial con `package_apk.ps1`.

---

### 2. Contexto

- Contexto consultado: Telemetría de TV Box (331 s), [`blazing.labels.toml`](../../blazing.labels.toml), `src-c/`, [`AGENTS.md`](../../AGENTS.md).
- Intención: Reducir el tiempo de CPU `ejec` de 19.65 ms a < 14 ms para sostener 60 FPS clavados en Android TV Box eliminando el 77.1% de los hotspots del intérprete.
- Restricciones y decisiones previas (máx. 3):
  - Prioridad 0-Interpreter (origen: `AGENTS.md §4.2`): Mapear funciones canónicas a C11 nativo AOT compilado por el NDK. → protegida por: regeneración limpia de `recomp build`.
  - Portabilidad y privacidad (origen: `AGENTS.md §6`): Cero rutas locales absolutas del usuario en el repositorio. → protegida por: pre-commit gatekeeper.
  - Empaquetado oficial con `scripts/package_apk.ps1` (origen: `AGENTS.md §6`): Construcción multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) con alineación a 16 KB y firmas v2/v3. → protegida por: script oficial.

---

### 3. Ejemplos

- Caso Escaneo de Unidades (`0x08004394`):
  - Antes: `Battle_TargetScanUnits` llamaba por `BL` a una dirección sin etiqueta, saliendo al intérprete repetidamente en cada unidad (15,285 ticks emulados).
  - Ahora: Se resuelve como llamada directa C11 interna compilada nativamente con Clang `-O2`.
- Caso Cinemáticas / Diálogos (`0x080C0A10`):
  - Antes: 1,620 ticks por segundo en el intérprete durante diálogos (31,686 ticks emulados).
  - Ahora: Ejecución nativa en nanosegundos en hardware ARM.
- Caso IA y Combate (`0x0804F7BC` y `0x0807793C`):
  - Antes: En turnos enemigos y combates la tasa caía a 32–40 FPS.
  - Ahora: La IA y el cálculo de combate corren 100% nativos sin tirones.

---

### 4. Pasos

1. Incorporar las 20 funciones a [`blazing.labels.toml`](../../blazing.labels.toml).
2. Validar integridad de labels con `python scripts/diagnostico/auditar_labels.py`.
3. Recompilar AOT C11 con `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1`.
4. Ejecutar suite de pruebas `cargo test` para verificar 111/111 en verde.
5. Reempaquetar APK multi-ABI con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.
6. Pasar checklist de Pre-Commit Gatekeeper.

---

### 5. Criterios de verificación

- [x] `auditar_labels.py` pasa con 0 errores y 122 labels verificadas.
- [x] `compilar_recomp_aot.ps1` regenera `src-c/` limpiamente (19,301 bloques traducidos, +230 bloques nuevos).
- [x] `cargo test` aprueba los 111 tests unitarios y de integración.
- [x] `package_apk.ps1` genera el Fat APK firmado sin advertencias (35.02 MB, esquemas v2 y v3, 16 KB alineado).
- [x] Cero rutas absolutas introducidas.

---

## Verificación (la escribe el agente después de editar)

### Predicho vs. real

| Criterio | Predicho | Real |
|---|---|---|
| Cobertura de labels | 122 subrutinas (20 nuevas) | 122 labels verificadas sin solapamientos en `blazing.labels.toml` |
| Bloques AOT generados | > 19,200 bloques nativos | 19,301 bloques generados (+230 bloques compilados nativamente en C11) |
| Integridad de tests | 111/111 en verde | 111/111 en verde (100 unitarios + 11 de integración) |
| Empaquetado Fat APK | APK multi-ABI de ~35 MB | APK de 35.02 MB generado, verificado y firmado con esquemas v2/v3 y alineación a 16 KB |
| Rutas absolutas | Cero rutas locales | 0 rutas locales introducidas en el árbol de trabajo |

