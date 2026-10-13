# Propuesta: Nombre del APK según el perfil real de compilación

Fecha: 2026-10-12
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El instalador que produce el empaquetado pasa a llamarse según el perfil con el que se construyó
(`blazing-recomp-release.apk` o `blazing-recomp-debug.apk`), en vez de llamarse `-debug` siempre,
que hoy hace indistinguibles dos paquetes de perfiles distintos.

### 2. Contexto

- Contexto consultado: [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1),
  [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs), `docs/propuestas/` (35 referencias al
  nombre viejo, repartidas en 19 archivos) y búsqueda de otros consumidores del nombre.
- Intención: que el nombre del artefacto no mienta. Lo que el usuario compara al probar en el
  dispositivo es el nombre del archivo, y hoy dice `debug` incluso cuando el perfil es `release`.
- Restricciones y decisiones previas (máx. 3):
  - El empaquetado oficial se invoca con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
    (origen: `AGENTS.md §6`) → implica: no cambiar la forma de invocarlo ni sus flags; solo el nombre
    de salida → protegida por: *El artefacto final se nombra según el perfil*.
  - Los tests de empaquetado validan el APK solo si el archivo existe
    (origen: [`tests/packaging_tests.rs:153`](../../tests/packaging_tests.rs) y `:431`, ambos con
    `if ... exists()`) → implica: un renombrado a medias se vería como éxito porque los tests
    saltarían en silencio; hay que fijar los nombres candidatos con un test propio → protegida por:
    *Los tests conocen los dos nombres de perfil*.
  - Las propuestas históricas citan el nombre viejo (origen: 35 coincidencias en 19 archivos de
    `docs/propuestas/`, ninguna en los cuadrantes vivos de `docs/`) → implica: no se reescriben registros pasados; este
    documento deja constancia del cambio → no testeable.
- Alternativas descartadas:
  - Dejar el nombre fijo y limitarse a escribir el perfil en el log → no resuelve el problema: el
    archivo en disco seguiría llamándose `-debug`, que es lo que engaña al comparar dos paquetes.
  - Añadir también la versión del crate al nombre → mete un punto de sincronización más (habría que
    actualizarlo en cada versión) sin resolver nada de lo pedido.
- Ante imprevistos, prioriza: que el script siga produciendo un APK instalable sobre cualquier
  consideración estética del nombre.

### 3. Ejemplos

- **Caso normal** (empaquetado sin `-Debug`): antes: `target/blazing-recomp-debug.apk` (perfil release);
  ahora: `target/blazing-recomp-release.apk`.
- **Caso límite** (empaquetado con `-Debug`): antes y ahora: `target/blazing-recomp-debug.apk`
  (el nombre coincide con el perfil, es el único caso en el que hoy ya era correcto).

### 4. Pasos

1. [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), línea 315: el nombre final pasa a
   derivarse del perfil, que ya se calcula en la línea 218 y ya se usa para elegir la `.so` en la 295.

   ```powershell
   # Antes
   $finalApk = Join-Path $targetDir "blazing-recomp-debug.apk"

   # Ahora
   $finalApk = Join-Path $targetDir "blazing-recomp-$targetProfile.apk"
   ```

2. [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs): los dos sitios que abren el APK por
   ruta fija (`:153` y `:431`) pasan a usar un ayudante que devuelve el primero existente entre los
   candidatos por perfil, en orden `release` y luego `debug` (así el artefacto que ya está en disco,
   que hoy se llama `-debug` aunque se construyó con perfil release, se sigue validando).
3. Un test nuevo que fija los nombres candidatos, para que un renombrado futuro no deje a los tests
   validando nada sin avisar.

### 5. Qué más podría verse afectado

Búsqueda hecha: `grep -rn --exclude-dir=target --exclude-dir=out --exclude-dir=src-c --exclude-dir=.git -e "blazing-recomp-debug" -e "blazing-recomp-unaligned" -e "blazing-recomp-aligned" .`
y `grep -rn --include=*.ps1 --include=*.py --include=*.sh -e "\.apk" scripts/ .agents/`

- [`scripts/package_apk.ps1:315`](../../scripts/package_apk.ps1) — definición del nombre final. Cambia.
- [`scripts/package_apk.ps1:313-314`](../../scripts/package_apk.ps1) — nombres de los intermedios
  (`unaligned`, `aligned`), que el propio script borra al terminar (línea 452). No cambian.
- [`tests/packaging_tests.rs:153`](../../tests/packaging_tests.rs) y `:431` — los dos consumidores
  vivos del nombre. Cambian.
- Las 35 referencias de `docs/propuestas/*.md`, en 19 archivos (registros históricos de propuestas
  ya cerradas). No se tocan.
- `scripts/diagnostico/compilar_c_paralelo.py` y el resto de `scripts/` no mencionan nombres de APK.
- Sin más referencias encontradas.

### 6. Suposiciones

- **`$targetProfile` está en el ámbito de la línea 315.** → prototipar: no (verificado leyendo el
  script: se define en la 218, fuera de funciones, y ya se usa en la 295 y la 378).
  → si es falso: el script fallaría al expandir la variable; se vería al ejecutarlo.
- **Ningún consumidor externo espera el nombre viejo** (integración continua, atajos del editor,
  scripts propios). → prototipar: no; la búsqueda del apartado 5 no encontró ninguno.
  → si es falso: hay que actualizar ese consumidor; no afecta al script ni a los tests.
- **Ordenar los candidatos como `release` y después `debug` es suficiente para encontrar el
  artefacto actual**, porque en `target/` solo hay uno de los dos nombres a la vez. → prototipar: no
  → si es falso (hubiera los dos), el ayudante elige el primero de la lista y podría validar el
  perfil equivocado; se acepta porque el objetivo del test es validar la estructura del paquete.

### 7. Alcance

- **Incluye:**
  - El nombre final del APK en [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1).
  - La búsqueda del artefacto en los dos tests de [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs)
    y un test nuevo de los nombres candidatos.
- **No incluye:**
  - Los nombres de los archivos intermedios (`unaligned`/`aligned`).
  - Reescribir las 35 referencias históricas de `docs/propuestas/`.
  - Borrar el artefacto viejo que quede en `target/` tras el primer empaquetado con el nombre nuevo.
  - Convertir en aviso visible el salto silencioso de los tests cuando el APK no existe (preexistente).

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El artefacto final se nombra según el perfil**
Caso de uso: comparar dos paquetes de perfiles distintos | Tipo: normal
  Dado el script de empaquetado
  Cuando se lee su texto en busca del nombre del APK final
  Entonces contiene la interpolación `"blazing-recomp-$targetProfile.apk"` y no contiene ningún
  nombre final fijo `"blazing-recomp-debug.apk"`
Test: `tests/packaging_tests.rs::test_packaging_script_names_apk_after_profile` | Estado previo: **falla**

**Escenario: Los tests conocen los dos nombres de perfil**
Caso de uso: que los tests sigan validando el APK tras un renombrado | Tipo: normal
  Dado el ayudante que localiza el APK generado
  Cuando se piden sus nombres candidatos
  Entonces son exactamente `target/blazing-recomp-release.apk` y `target/blazing-recomp-debug.apk`,
  en ese orden
Test: `tests/packaging_tests.rs::test_candidatos_apk_cubren_ambos_perfiles` | Estado previo: **falla**

**Escenario (regresión): La estructura del APK se sigue validando con el artefacto que hay en disco**
Caso de uso: instalación en el dispositivo | Tipo: regresión
  Dado el APK que ya existe en `target/` (hoy `blazing-recomp-debug.apk`)
  Cuando se ejecutan los tests de estructura del paquete
  Entonces siguen pasando y siguen comprobando el manifiesto y `resources.arsc` del archivo real
Test: `tests/packaging_tests.rs::test_apk_generation_and_signature` y `::test_apk_keeps_arsc_stored`
| Estado previo: **pasa (regresión)**

### 9. Cómo lo comprobaré

- **El artefacto final se nombra según el perfil** → comando:
  `cargo test --test packaging_tests test_packaging_script_names_apk_after_profile`
  → pasa si: termina en `ok` con las dos aserciones
  → no detecta: que el script, al ejecutarse, escriba ese nombre; eso lo cubre la comprobación de
  extremo a extremo de abajo.
- **Los tests conocen los dos nombres de perfil** → comando:
  `cargo test --test packaging_tests test_candidatos_apk_cubren_ambos_perfiles`
  → pasa si: termina en `ok` con la lista exacta y el orden
  → no detecta: que el ayudante encuentre el archivo en disco (depende de lo que haya en `target/`).
- **Comprobación de extremo a extremo del nombre** → comando:
  `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1 -SkipBuild`
  (con `-SkipBuild` se salta la compilación y se recorre toda la ruta de nombrado, alineado y firma)
  → pasa si: termina con `==> Exito: APK generado en ...blazing-recomp-release.apk` y
  `ls target/*.apk` muestra ese archivo
  → no detecta: que la aplicación funcione en Android; solo prueba el nombre y la firma. Tampoco
  detecta nada sobre el perfil de compilación, porque `-SkipBuild` no compila.
- **La estructura del APK se sigue validando** → comando:
  `cargo test --test packaging_tests test_apk` (un solo filtro por prefijo; `cargo test` no acepta dos
  nombres como filtros: comprobado hoy, responde `error: unexpected argument`)
  → pasa si: termina en `ok` con 2 tests ejecutados, no con 0
  → no detecta: que hayan validado algo y no saltado; esos dos tests salen sin hacer nada si el
  archivo no está, y ese salto silencioso es preexistente (queda fuera de alcance). Lo que sí queda
  cubierto es que la lista de nombres candidatos no se vacíe (escenario anterior).
- **Nada más se rompe** → comando: `cargo test`
  → pasa si: 122 tests en verde, 0 fallos (base de hoy: 107 unitarios + 13 de integración = 120,
  más los 2 tests nuevos).
  → no detecta: comportamiento en Android ni el resto del empaquetado.

### 10. Terminado cuando

Los tres escenarios pasan, la suite completa sigue en verde y el empaquetado con `-SkipBuild` deja
el archivo `target/blazing-recomp-release.apk`.

### 11. Parar si

2 intentos fallidos seguidos con el mismo escenario. Detenerse, explicar qué se probó y marcar la
propuesta como `discrepada`.

### Correcciones previas a presentar

- Detectado al leer los tests que ambos abren el APK bajo `if exists` / `if !exists { return }`: con
  un renombrado a medias no fallarían, simplemente dejarían de validar. Por eso se añade un escenario
  que fija los nombres candidatos, en vez de confiar en que la suite siga verde.
- Detectado que el artefacto que hoy está en `target/` se llama `-debug` pero se construyó con perfil
  `release` (`$Debug` vale `$false` por defecto y el nombre está fijado a mano en la línea 315). Es la
  evidencia de que el nombre actual no refleja el perfil; se deja escrito en el apartado 3.
- Detectado que los dos archivos que esta propuesta va a tocar
  (`scripts/package_apk.ps1` y `tests/packaging_tests.rs`) **ya tienen cambios sin commitear** de otras
  sesiones, así que su `git diff` mezclará ambos trabajos. Al implementar hay que revisar que los
  bloques no se solapen.
- Detectado al comprobar los comandos del apartado 9 que `cargo test` **no** acepta dos nombres como
  filtros (`error: unexpected argument`): el control de regresión pasa a un único filtro por prefijo
  (`test_apk`), comprobado en verde con 2 tests ejecutados.
- Detectado que ninguna página de los cuadrantes vivos de `docs/` menciona el nombre del APK: el
  impacto documental se limita a registros históricos, que no se reescriben.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El artefacto final se nombra según el perfil**: coincide.
  Resultado real: en rojo antes, `panicked at tests/packaging_tests.rs:47: El nombre del APK final
  debe derivarse del perfil de compilacion`; en verde después. La comprobación de extremo a extremo
  también coincide: el empaquetado con `-SkipBuild` terminó con
  `==> Exito: APK generado en ...blazing-recomp-release.apk (42.37 MB)` y código 0, con las firmas
  v2 y v3 verificadas.
  Método: los anunciados (`cargo test --test packaging_tests test_packaging_script_names_apk_after_profile`
  y `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1 -SkipBuild`).
- **Los tests conocen los dos nombres de perfil**: coincide, y el rojo fue informativo en vez de
  inmediatamente verde porque la aserción cruza el script con la lista de candidatos. En rojo antes,
  con el diagnóstico exacto del problema:
  `left: ["blazing-recomp-release.apk", "blazing-recomp-debug.apk"]` (lo que buscan los tests) frente a
  `right: ["blazing-recomp-debug.apk", "blazing-recomp-debug.apk"]` (lo que producía el script para cada
  perfil). En verde después.
- **La estructura del APK se sigue validando** (regresión): coincide. Verde antes y después, y ahora
  hay dos artefactos en `target/` (`-debug` de las 02:25 y `-release` de las 03:16), de modo que el
  ayudante elige `-release`, que es el que produce el empaquetado oficial.
- **Nada más se rompe**: coincide. `cargo test` = 107 unitarios + 15 de integración = **122 en verde,
  0 fallos**, exactamente el número que anunciaba el apartado 9.
- Comprobado además que el script mantiene la sintaxis válida de PowerShell tras la edición.

### Contexto: ¿se respetó?

- El empaquetado oficial se invoca igual y con los mismos flags, cambiando solo el nombre de salida →
  respetada: el `-SkipBuild` corrió sin tocar la interfaz de invocación.
- Los tests de empaquetado siguen validando el APK, y ahora por lista de candidatos cruzada con el
  script → respetada: el cruce quedó fijado en un test.
- Los registros históricos no se reescriben → respetada: no se tocó ninguna página de `docs/propuestas/`
  salvo este documento.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

[`scripts/package_apk.ps1`](../../scripts/package_apk.ps1) (la línea del nombre final y su comentario),
[`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) (ayudante de candidatos, dos tests nuevos y
los dos sitios que abrían el APK por ruta fija) y este archivo. ¿Coincide con lo anunciado? sí: los dos
archivos ya estaban en el apartado 7.

Aviso de higiene: los dos archivos **ya tenían cambios sin commitear** de otras sesiones, así que su
`git diff` mezcla ambos trabajos. Mi cambio en el script es un único bloque de dos líneas; en los tests,
el ayudante y dos tests nuevos más la reconexión de los dos sitios.

### Discrepancias

- Ninguna: los dos escenarios nuevos pasaron de rojo a verde y la regresión se mantuvo verde, tal como
  se predijo, y el conteo de 122 tests coincidió.
- El tercer supuesto del apartado 6 (dos artefactos del mismo `target/`) dejó de ser hipotético: ahora
  existen los dos y el ayudante elige `-release`. Es la limitación ya declarada, no una desviación.

### Para la próxima vez

- Cuando un test fija un contrato entre dos artefactos distintos (aquí, el nombre que produce el script
  y la lista que buscan los tests), escribirlo como cruce entre ambos: da un rojo que explica el problema
  en una línea, en lugar de un verde inmediato que no mediría nada.

### Fuera de alcance (hallazgos que no se arreglaron)

- Los dos tests de estructura del APK saltan en silencio cuando el archivo no existe, así que una
  pérdida de cobertura se ve igual que un éxito. | impacto: bajo
- Tras el primer empaquetado con el nombre nuevo, el artefacto viejo
  `target/blazing-recomp-debug.apk` (02:25) queda huérfano: nada lo borra automáticamente. | impacto: bajo
- `apksigner` sigue informando `Verified using v1 scheme (JAR signing): false` pese a que el script pasa
  `--v1-signing-enabled true`, tanto antes como después de este cambio. Es preexistente y contradice el
  checklist de `AGENTS.md §6`, que pide los tres esquemas. | impacto: bajo

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Que el empaquetado siga produciendo un APK instalable tras el cambio de nombre: ✔ | ✘ | ?
- Nombres de los artefactos que quedan en `target/` tras el empaquetado (el viejo `-debug` no se borra solo): ✔ | ✘ | ?
- Registros históricos de `docs/propuestas/` que siguen citando el nombre viejo (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
