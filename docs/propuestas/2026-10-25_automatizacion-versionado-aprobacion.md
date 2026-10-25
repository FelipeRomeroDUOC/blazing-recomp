# Propuesta: Automatización de Versionado y Compuerta de Aprobación Previa

Fecha: 2026-10-25
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El sistema contará con una herramienta canónica para sincronizar las versiones de la aplicación de forma atómica en todos los manifiestos y documentos, requiriendo siempre la aprobación explícita y previa de la persona antes de aplicar cualquier incremento tras validar una función o corrección.

### 2. Contexto

- Contexto consultado: `docs/explicacion/roadmap-y-versionado.md`, `Cargo.toml`, `android/AndroidManifest.xml`, `pack.toml`, `CHANGELOG.md`, `docs/BACKLOG.md`, `tests/packaging_tests.rs`.
- Intención: Eliminar discrepancias silenciosas entre `Cargo.toml`, `AndroidManifest.xml` y la documentación, asegurando que el incremento de versión solo ocurra cuando la persona esté segura de que la solución o feature está probada y lista.
- Restricciones y decisiones previas:
  - Fórmula matemática de versión Android: $\text{versionCode} = \text{MAJOR} \times 10000 + \text{MINOR} \times 100 + \text{PATCH}$ (origen: `docs/explicacion/roadmap-y-versionado.md`) → implica: `versionCode` debe calcularse determinísticamente a partir del SemVer textual → protegida por: `Escenario: Validación matemática de versionCode y sincronización de manifiestos`
  - Compuerta humana obligatoria (origen: respuesta humana, 2026-10-25) → implica: ningún script ni agente incrementará la versión en disco sin previa aprobación explícita de la persona → protegida por: `Escenario: Modo interactivo requiere confirmación explícita`
  - Idempotencia y modo seguro (origen: `AGENTS.md` §2) → implica: debe soportar comprobación de sincronización (`--check`) y simulación de cambios (`--dry-run`) sin alterar archivos → protegida por: `Escenario: Verificación de coherencia e informe dry-run sin modificaciones`
- Alternativas descartadas:
  - Edición manual de 6 archivos dispersos → descartada porque induce olvidos y rompe tests de empaquetado.
  - Incremento automático desasistido al cerrar cada tarea → descartada porque la persona debe evaluar empíricamente la estabilidad de la corrección antes del bump.
- Ante imprevistos, prioriza: Coherencia unánime y confirmación humana sobre rapidez de ejecución.

### 3. Ejemplos

- Caso normal: Tras validar una corrección, se propone `0.0.3` → `0.0.4` (`versionCode = 4`). La persona aprueba y `bump_version.py` actualiza atómicamente `Cargo.toml`, `AndroidManifest.xml`, `pack.toml`, `CHANGELOG.md`, `BACKLOG.md` y documentos de referencia.
- Caso límite: Si un archivo estuviese desalineado (ej. `pack.toml` en `0.0.2` y `Cargo.toml` en `0.0.3`), el comando `--check` reporta el error y `--patch` rehúsa avanzar hasta que se subsane la discrepancia.

### 4. Pasos

1. Crear script canónico `scripts/bump_version.py` con subcomandos `--check`, `--dry-run`, `--patch`, `--minor`, `--major`, `--version <SEMVER>`, `--mensaje <TEXTO>` y `--si`.
2. Implementar compuerta interactiva por defecto (`¿Desea aplicar el incremento a X.Y.Z? [s/N]`), exigiendo confirmación de la persona a menos que se use la bandera explícita `--si`.
3. Crear suite de tests de integración en `tests/version_sync_tests.rs` para garantizar en `cargo test` que todos los manifiestos coinciden y cumplen la fórmula SemVer.
4. Actualizar `AGENTS.md` (§3 Reglas Técnicas y §5 Comandos Canónicos) formalizando la regla de versión y el comando canónico.
5. Actualizar `docs/explicacion/roadmap-y-versionado.md` documentando la herramienta de sincronización.

### 5. Qué más podría verse afectado

- Manifiestos: `Cargo.toml`, `android/AndroidManifest.xml`, `pack.toml`.
- Documentos: `CHANGELOG.md`, `docs/BACKLOG.md`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/explicacion/roadmap-y-versionado.md`.
- Reglas operativas: `AGENTS.md`.

### 6. Suposiciones

- La biblioteca estándar de Python (`tomllib`, `xml.etree.ElementTree`, `re`, `argparse`) es suficiente para analizar y verificar todos los manifiestos sin dependencias externas pesadas → prototipar: sí → si es falso: usar dependencias externas → resultado del prototipo: verificado con éxito en Python 3.14 estándar | no detecta: ninguno.

### 7. Alcance

- Incluye: Script canónico `scripts/bump_version.py`, modo de verificación `--check`, simulación `--dry-run`, compuerta interactiva de confirmación, test de integración Rust `tests/version_sync_tests.rs`, actualización de `AGENTS.md` y documentación Diátaxis.
- No incluye: Creación de tags de Git (`git tag`), publicación en Google Play Store ni modificación de la lógica de emulación.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Validación matemática de versionCode y sincronización de manifiestos
Caso de uso: Verificación de estado de versión en el repositorio | Tipo: normal
  Dado que los archivos `Cargo.toml`, `android/AndroidManifest.xml`, `pack.toml` y `docs/BACKLOG.md` declaran la versión actual
  Cuando se ejecuta `python scripts/bump_version.py --check`
  Entonces el script termina con código de salida 0 y reporta que todos los manifiestos están sincronizados en `0.0.3` con `versionCode = 3`.
Test: `tests/version_sync_tests.rs::test_version_manifests_are_synchronized`
Estado previo: falla (el test aún no existe)

Escenario: Verificación de coherencia e informe dry-run sin modificaciones
Caso de uso: Previsualización de un incremento antes de confirmarlo | Tipo: límite
  Dado el repositorio con versión `0.0.3`
  Cuando se ejecuta `python scripts/bump_version.py --dry-run --patch`
  Entonces muestra la propuesta de cambio hacia `0.0.4` (`versionCode = 4`) sin modificar ningún archivo en el disco.
Test: `tests/version_sync_tests.rs::test_bump_version_dry_run_leaves_files_untouched`
Estado previo: falla (el test aún no existe)

Escenario: Modo interactivo requiere confirmación explícita
Caso de uso: Protección contra incrementos accidentales o desasistidos | Tipo: error
  Dado que se invoca `python scripts/bump_version.py --patch` sin la opción `--si` y con entrada no afirmativa
  Cuando el usuario responde que no o no confirma
  Entonces la ejecución se cancela inmediatamente con código de error o advertencia y los archivos permanecen inalterados.
Test: `tests/version_sync_tests.rs::test_bump_version_aborts_without_confirmation`
Estado previo: falla (el test aún no existe)

Escenario: Integración en suite de tests previene regresiones de versión
Caso de uso: Ejecución de la suite completa de desarrollo | Tipo: regresión
  Dado el repositorio en cualquier commit de trabajo
  Cuando se ejecuta `cargo test --test version_sync_tests`
  Entonces los tests compilan y aprueban la relación unánime de versión y la regla matemática de Android.
Test: `tests/version_sync_tests.rs`
Estado previo: falla (el test aún no existe)

### 9. Cómo lo comprobaré

- Validación matemática de versionCode y sincronización de manifiestos → herramienta o comando: `python scripts/bump_version.py --check` → pasa si: código de salida 0 y salida confirmando sincronía → no detecta: inconsistencias en archivos no monitoreados.
- Verificación de coherencia e informe dry-run sin modificaciones → herramienta o comando: `python scripts/bump_version.py --dry-run --patch` → pasa si: no hay cambios en `git status` y reporta `0.0.4` / `4` → no detecta: bloqueos de permisos de archivo.
- Modo interactivo requiere confirmación explícita → herramienta o comando: `python scripts/bump_version.py --patch` simulando entrada `n` → pasa si: aborta con mensaje de cancelación y no altera archivos → no detecta: bugs en emuladores de terminal especiales.
- Integración en suite de tests previene regresiones de versión → herramienta o comando: `cargo test --test version_sync_tests` → pasa si: todos los tests pasan en verde al 100% → no detecta: fallos en entornos sin Cargo.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y la suite de tests pasa al 100%.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno; el prototipo con `tomllib` y `xml.etree.ElementTree` confirmó la viabilidad sin dependencias externas.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Validación matemática de versionCode y sincronización de manifiestos: coincide
  Resultado real: `python scripts/bump_version.py --check` retornó 0 confirmando sincronía unánime en versión `0.0.3` (`versionCode = 3`).
  Método: el anunciado
- Verificación de coherencia e informe dry-run sin modificaciones: coincide
  Resultado real: `python scripts/bump_version.py --dry-run --patch` proyectó el incremento a `0.0.4` (`versionCode = 4`) listando los 7 archivos a actualizar sin alterar el disco.
  Método: el anunciado
- Modo interactivo requiere confirmación explícita: coincide
  Resultado real: al enviar `n\n` en la entrada estándar sin `--si`, el script canceló la operación con código 1 y no modificó ningún archivo.
  Método: el anunciado
- Integración en suite de tests previene regresiones de versión: coincide
  Resultado real: `cargo test --test version_sync_tests` pasó al 100% (4/4 tests en 0.45s).
  Método: el anunciado

### Contexto: ¿se respetó?

- Fórmula matemática de versión Android → respetada
- Compuerta humana obligatoria → respetada
- Idempotencia y modo seguro → respetada
- ¿Se tocó algo que alguna restricción prohibía? no
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos
- ¿Reapareció alguna alternativa descartada? no

### Archivos modificados

- `scripts/bump_version.py` (creado)
- `tests/version_sync_tests.rs` (creado)
- `AGENTS.md` (regla técnica 6 y comandos canónicos)
- `docs/explicacion/roadmap-y-versionado.md` (sección de automatización)
- `docs/propuestas/2026-10-25_automatizacion-versionado-aprobacion.md` (creado y completado)

¿Coincide con lo anunciado? sí

### Discrepancias

Ninguna.

### Para la próxima vez

Mantener los tests de sincronización de versiones como parte obligatoria del pre-commit gatekeeper para evitar regresiones silenciosas en los manifiestos de empaquetado.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Discrepancia (solo si el estado es "discrepada")

(No aplica)

---

## Revisión humana

- Validación de la compuerta humana: confirmación previa requerida antes de aplicar cambios de versión: ✔
- Detectado solo por el humano: Las fórmulas matemáticas requerían soporte tipográfico semántico de LaTeX en la previsualización y las viñetas omitían el primer carácter por un cálculo de prefijo rígido en `generar_preview_propuesta.py`.
