# Propuesta: desglosar el hash de entrada y medir la salida real de la GPU (dragón)

Fecha: 2026-11-02
Nivel: 2 (más de 3 archivos y segundo intento seguido con el mismo problema)
Estado: verificada por agente

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cada línea `# hash` pasa a incluir una huella por componente de la entrada (registros por línea, VRAM, OAM, paleta de fin de cuadro, paleta por línea) y una huella **real** de la imagen que dibujó la GPU (`sal`, hoy siempre `00000000`). El script dice qué componente difiere primero y solo compara la salida cuando todas las entradas coinciden.

No arregla el parpadeo. Sirve para saber si el problema está en los datos que recibe la GPU o en lo que la GPU hace con ellos.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs` (`calcular_hash_entrada`), `scripts/diagnostico/comparar_hash_cuadros.py`, `src/platform.rs` (`leer_textura_juego_rgb`, bloque `# hash`), `src/bin/ppu_doble.rs`, log `telemetria/blazing-perf-36.txt` y `pc-hash.txt`.
- Intención: pasar de "no sé qué difiere" a "difiere X en el cuadro N". Éxito = el script nombra un componente, o dice que todas las entradas coinciden y la salida no.
- Hechos medidos:
  - `bld0/50/100/150` y `bg3_blend` coinciden en los 601 cuadros.
  - `ent` difiere desde `rel=0`, pero el patrón de repetición entre cuadros es idéntico en ambos (358 valores distintos, mismas repeticiones).
  - `sal` valía `00000000` en ambos: la salida de la GPU nunca se midió.
- Restricciones:
  - Sin la bandera, nada cambia → protegida por: test de bandera inactiva ya existente.
  - No tocar composición ni shader → protegida por: regresión de la suite.
- Alternativas descartadas:
  - Proponer ya un arreglo en el shader o en la sincronización de buffers sin medir.

### 3. Ejemplos

Línea generada:
```
# hash rel=0 frame=613 ent=A634CEDD sal=5C03E19A bld0=2168 bld50=2168 bld100=2168 bld150=2168 bg3_blend=160 hc=.. hv=.. ho=.. hp=.. hl=..
```

### 4. Pasos

1. En `calcular_huellas_componentes`: calcular huella por componente (`comps`, `vram`, `oam`, `paleta`, `paleta_lineas`).
2. En PC (`ppu_doble.rs`): `sal` = `fnv1a_32(&nat_rgb)`.
3. En Android (`platform.rs`): `sal` = `leer_textura_juego_rgb().map(fnv1a_32)`.
4. Ampliar formato con `hc hv ho hp hl`.
5. En `comparar_hash_cuadros.py`: leer campos enriquecidos y diagnosticar componente exacto.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/platform.rs`, `src/bin/ppu_doble.rs`, `scripts/diagnostico/comparar_hash_cuadros.py` y sus tests.

### 6. Suposiciones

- La lectura de la textura en Android no altera el ritmo ni oculta el parpadeo → prototipar: sí
- En Android, `game_texture` contiene la imagen final de `NativePpu` en ese punto del cuadro → prototipar: sí

### 7. Alcance

- Incluye: huellas por componente, `sal` real y script actualizado.
- No incluye: arreglo del parpadeo, cambios en shader o en presentación.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Cambiar un solo componente cambia solo su huella
Caso de uso: localizar la diferencia | Tipo: normal
  Dado un estado sintético con comps, VRAM, OAM, paleta y paleta por línea fijos
  Cuando se cambia un byte de la paleta por línea y se recalcula
  Entonces `hl` cambia, `hc`, `hv`, `ho` y `hp` no cambian, y `ent` cambia
Test: `ppu_nativa_tests::test_hash_por_componente_aisla_paleta_lineas`
Estado previo: pasa

Escenario: Las huellas son estables con entrada idéntica
Caso de uso: comparar dos plataformas | Tipo: límite
  Dado el mismo estado sintético dos veces
  Cuando se calculan las huellas
  Entonces las 6 huellas son iguales
Test: `ppu_nativa_tests::test_hash_por_componente_estable`
Estado previo: pasa

Escenario: El script nombra el componente que difiere
Caso de uso: diagnosticar | Tipo: normal
  Dados dos logs sintéticos iguales salvo `hv` en `rel=1`
  Cuando se ejecuta `comparar_hash_cuadros.py`
  Entonces imprime `rel=1: ENTRADA difiere en: VRAM`, termina con código 1 y no compara `sal`
Test: `scripts/diagnostico/test_comparar_hash_cuadros_componentes.py`
Estado previo: pasa

Escenario: Logs antiguos siguen siendo válidos
Caso de uso: no romper `blazing-perf-36.txt` | Tipo: regresión
  Dado un log con el formato anterior (sin `hc hv ho hp hl`)
  Cuando se ejecuta el script
  Entonces se comporta como antes y no falla
Test: casos existentes de `test_comparar_hash_cuadros.py`
Estado previo: pasa (regresión)

Escenario: La suite no se rompe
Caso de uso: no romper lo ya hecho | Tipo: regresión
  Dado `cargo test --lib` con 239 tests en verde
  Cuando se aplica el cambio
  Entonces siguen 239 o más en verde y 0 fallos
Test: suite existente
Estado previo: pasa (regresión, 241 pasados)

Escenario (visible): Una nueva medición en PC y Android da un veredicto
Caso de uso: diagnóstico real | Tipo: normal
  Dado el APK nuevo con la bandera y `ppu_doble --hash-cuadros`
  Cuando se reproduce la escena del dragón en ambos
  Entonces el script nombra un componente distinto o dice "entradas iguales, salida difiere"

### 9. Cómo lo comprobaré

- Aislamiento por componente y estabilidad → `cargo test --lib test_hash_por_componente` → pasa.
- Script componentes → `python scripts/diagnostico/test_comparar_hash_cuadros_componentes.py` → pasa con código 0.
- Regresión script → `python scripts/diagnostico/test_comparar_hash_cuadros.py` → pasa con código 0.
- Regresión general → `cargo test --lib` → 0 fallos (241 tests pasados).
- (visible) → revisión humana en hardware real.

### 10. Terminado cuando

Se cumplen todos los criterios y existe un veredicto del script con logs reales.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Escenario: Cambiar un solo componente cambia solo su huella: coincide
  Resultado real: Test `test_hash_por_componente_aisla_paleta_lineas` superado con código 0.
  Método: el anunciado (`cargo test --lib test_hash_por_componente`).

- Escenario: Las huellas son estables con entrada idéntica: coincide
  Resultado real: Test `test_hash_por_componente_estable` superado con código 0.
  Método: el anunciado (`cargo test --lib test_hash_por_componente`).

- Escenario: El script nombra el componente que difiere: coincide
  Resultado real: 5/5 pruebas pasadas en `test_comparar_hash_cuadros_componentes.py` con código 0.
  Método: el anunciado (`python scripts/diagnostico/test_comparar_hash_cuadros_componentes.py`).

- Escenario: Logs antiguos siguen siendo válidos: coincide
  Resultado real: Suite `test_comparar_hash_cuadros.py` superada al 100% con código 0.
  Método: el anunciado (`python scripts/diagnostico/test_comparar_hash_cuadros.py`).

- Escenario: La suite no se rompe: coincide
  Resultado real: 241 pasados, 0 fallados en la suite de librería de Rust.
  Método: el anunciado (`cargo test --lib`).

- Escenario (visible): Una nueva medición en PC y Android da un veredicto: pendiente de la nueva captura con hardware real y visor PC.

### Contexto: ¿se respetó?

- Restricciones respetadas: Shaders y pipeline de render intactos; todas las huellas y lecturas condicionadas a la bandera activa en runtime.
- Fat APK generado: `target/blazing-recomp-release.apk` (44.806.984 bytes).

### Archivos modificados

- `src/ppu_nativa.rs` (modificado)
- `src/ppu_nativa_tests.rs` (modificado)
- `src/platform.rs` (modificado)
- `src/bin/ppu_doble.rs` (modificado)
- `scripts/diagnostico/comparar_hash_cuadros.py` (modificado)
- `scripts/diagnostico/test_comparar_hash_cuadros_componentes.py` (creado)

---

## Revisión humana

- Una nueva medición en PC y Android da un veredicto (visible): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
