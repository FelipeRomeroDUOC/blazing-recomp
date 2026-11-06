---
name: propuesta-verificable
description: Flujo de propuesta verificable antes de editar código. El agente predice qué va a cambiar (con ejemplos concretos), espera aprobación, edita solo lo aprobado y después compara lo predicho con lo que realmente pasó. Usar siempre que vayas a modificar, crear o borrar código en un proyecto, incluso si el cambio parece pequeño; también al corregir errores, refactorizar, agregar funciones, cambiar configuración o tocar base de datos, autenticación o pagos. Usar también cuando una suposición de una propuesta resulta falsa o un criterio falla: primero se evalúa la propuesta vigente y decide la persona, antes de escribir otra. Elige el nivel de detalle (0, 1 o 2) con criterios comprobables. No usar para explicar código, responder preguntas ni revisar sin editar.
---

# Propuesta verificable

Antes de editar código, escribe una propuesta que prediga qué va a pasar. Después de editar, compara lo predicho con lo real.

Por qué: explicar un plan paso a paso ayuda a encontrar errores en él antes de gastar una edición (es la idea del rubber duck debugging, aplicada antes de editar y no después). Además le da a la persona algo concreto que revisar, en vez de un plan vago que termina aprobando sin leer. Tu propuesta es una hipótesis: la persona la revisa antes, y tú la contrastas con la realidad después. Es también tu contrato: define qué puedes hacer y qué no, y te da criterio cuando aparece algo que no se previó.

Mantén el esfuerzo proporcional al riesgo: más detalle donde un error cuesta caro, nada donde no hace falta.

## Tres reglas que mandan

1. **La propuesta se escribe para que se lea.** La persona suele ser principiante y tiene poco tiempo. Lo que debe decidir va arriba y cabe en una pantalla; el detalle va debajo. Lee `references/redaccion-legible.md` antes de escribir una de nivel 2.
2. **El archivo final no contiene instrucciones de la plantilla.** Sin corchetes, sin comentarios, sin textos de ayuda.
3. **Si una suposición resulta falsa, primero evalúas la propuesta que existe; después decide la persona; solo entonces se escribe algo nuevo.** Lee `references/suposicion-falsa.md`. Escribir otra propuesta de inmediato hace que la persona relea todo y oculta qué cambió de verdad.

### Diagnóstico previo y prevención de bucles de parches (al corregir errores)

Antes de redactar una propuesta para corregir un fallo, aplica estas 4 reglas:
1. **Bisección desde el primer minuto:** Discierne primero todas las capas desde donde puede venir el fallo (`Origen/Lógica -> Transporte/Sincronía -> Transformación -> Salida`) y haz una comprobación para aislar en qué frontera nace la discrepancia, en vez de parchar directamente el síntoma visible.
2. **Cero "saneamientos" heurísticos:** No propongas parches basados en combinaciones específicas de una pantalla, cuadro o escena para forzar estados artificiales. La corrección debe respetar las reglas generales del hardware o del subsistema.
3. **Un solo fallo por hipótesis (*1-Strike Rule*):** Si una propuesta anterior sobre el mismo problema quedó como `discrepada`, está prohibido proponer una variante de la misma hipótesis en el mismo lugar sin antes presentar evidencia nueva que explique por qué falló el intento previo y descarte esa capa.
4. **Reproducción local en PC:** Siempre que el fallo no sea exclusivo del entorno Android, busca reproducirlo y verificarlo localmente en PC antes de proponer ediciones a ciegas.
5. **Principio de absolución previa:** Plantea la prueba mínima que demuestre si el componente sospechoso es inocente (ej. ¿el componente de referencia o software falla igual con la misma entrada?). Si falla igual, queda absuelto: descártalo y no toques ese archivo.
6. **Pruebas de humo deterministas en PC:** Favorece comprobaciones rápidas o scripts headless locales en PC en lugar de depender de iteraciones completas de empaquetado de APK para validar hipótesis.

## 1. Elige el nivel

Usa criterios que puedas comprobar (contar archivos, buscar referencias), no tu impresión de si algo es riesgoso.

**Nivel 2** si se cumple CUALQUIERA:
- Toca una zona sensible de **Blazing Recomp**: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/audio.rs`, `src/recomp.rs`, `src/pipeline.rs`, `src/runner.rs`, `../gba-recomp/crates/gba-core/**`, `Cargo.toml`, `pack.toml`, `android/AndroidManifest.xml` o `.agents/hooks/*.py`.
- Modifica más de 3 archivos, o crea o borra archivos.
- Cambia lógica compartida: cualquier función, clase o constante usada por otros archivos, sin importar cuántos sean (compruébalo buscando referencias).
- Es la primera vez que se modifica ese módulo desde su creación. Consulta el resumen de propuestas e historial de Git ya precalculado en tu `<SYSTEM_MESSAGE>` inicial.
- No hay un test que cubra el cambio y tampoco se puede ver al ejecutar la app.
- Es el segundo intento fallido seguido con el mismo problema.

> **Invariantes del validador automático (`scripts/validar_propuesta_y_skills.py`):**
> No abras `assets/plantilla-propuesta.md` ni `references/` a menos que haya una discrepancia (`suposicion-falsa.md`): el post-hook valida automáticamente que la propuesta Nivel 2 tenga `## Para decidir`, `## Detalle para verificar` (con máx. 3 suposiciones `S1..S3`, máx. 3 reglas `R1..R3` y `### Comprobaciones` con `Dado que` / `Cuando` / `Entonces`), `## Revisión humana` con `✔ | ✘ | ?` sin elegir o `ninguno`, y cero restos de plantilla (`[nombre corto]`, `[por rellenar]`, `<!-- PLANTILLA`).

**Nivel 0** solo si se cumplen TODAS: es texto, estilo o comentarios; es un solo archivo; no toca lógica.

**Nivel 1**: todo lo demás.

Si dudas entre dos niveles, aplica el más alto. Pasarte un poco de cuidado cuesta unos tokens; quedarte corto es justo lo que esta práctica intenta evitar.

## 2. Escribe la propuesta

- **Nivel 0:** no hay propuesta. Edita directamente.
- **Nivel 1:** en el chat. Una o dos frases, un ejemplo "antes: A, ahora: B", un test mínimo que lo compruebe y, en una línea, con qué herramienta o comando lo comprobarás. Si el proyecto no tiene infraestructura de tests y montarla no es barato, dilo en una línea en vez de montarla.
- **Nivel 2:** crea el archivo `propuestas/AAAA-MM-DD_nombre-corto.md` a partir de `assets/plantilla-propuesta.md`. Antes de la primera de la conversación, lee `references/redaccion-legible.md` y mira `assets/ejemplo-propuesta.md` para calibrar longitud y estilo. No copies su contenido.

Qué lleva el archivo:
- **"Para decidir":** qué cambia, para qué, un ejemplo, los pasos en lenguaje de quien usa la app, qué podría romperse, qué no se toca, y qué necesitas de la persona (preguntas con opciones y lo que solo ella puede mirar).
- **"Detalle para verificar":** suposiciones (marca cuáles se prueban antes de editar, máximo 3, y por qué esas), reglas del proyecto que respetas (máximo 3, solo las que cambian lo que haces; cada una con su fuente y protegida por una comprobación o marcada "solo tú"), comprobaciones, archivos y evidencia.
- **Comprobaciones** como escenarios Dado/Cuando/Entonces. Cada una nombra su situación real, su tipo (normal, extremo, entrada inválida o no debe cambiar), el nombre del test, el comando exacto y qué no cubre. Cobertura mínima: una normal, una extrema y, si el cambio afecta comportamiento existente, una de "no debe cambiar". Detalle en `references/diseno-de-tests.md`.
- En la propuesta defines los tests (nombre y qué comprueban), pero todavía no los escribes. Los escribes después de la aprobación; ver la sección 4.
- Cada línea debe cambiar algo que harías. Si no, sobra.
- No escribas las secciones "Terminado cuando" ni "Parar si": son iguales en toda propuesta y ya están en esta skill.

En el chat escribe como máximo 4 líneas: la ruta, qué cambia en una frase, qué necesitas de la persona (o "solo aprobar") y cuántos puntos tendrá que mirar ella. No repitas la propuesta.

### Preguntas antes de proponer (nivel 2)

Antes de escribir, revisa el archivo de contexto del proyecto, `docs/` (si existe) y el código. Si el plan depende de algo que no encuentras en ninguno de los tres, pregúntale a la persona. Lo que solo ella sabe no se puede deducir, y rellenarlo con suposiciones es lo que más errores ha causado.

- Máximo 3 preguntas. No preguntes lo que puedes encontrar tú ni lo que ya está en el archivo de contexto.
- Revisa cuatro categorías: restricciones, convenciones, cosas que no deben tocarse y comportamiento esperado en casos extremos.
- Cada pregunta dice qué cambia según la respuesta ("Cambia si el paso 3 borra o solo marca como inactivo") y ofrece opciones para elegir, más una salida "no sé". La persona suele tener dificultad para redactar respuestas largas: que pueda contestar con una palabra.
- "No sé" pasa a "Suposiciones", con su "si es falso, el plan cambia así".
- Las respuestas entran a "Reglas del proyecto" con fuente "respuesta humana". Si una respuesta es un hecho que seguirá valiendo más allá de este cambio, al cerrar la propuesta sugiere una línea para el archivo de contexto, con su fuente, y espera la aprobación. Nunca escribas en ese archivo sin ella.
- Si una línea del archivo de contexto contradice el código, gana el código: repórtalo como discrepancia.

## 3. Autocomprobación antes de presentar

1. Compara la propuesta con lo que se pidió: ni añadir ni omitir.
2. Pasa cada ejemplo por los pasos propuestos y verifica que da el resultado que afirmas.
3. Contrasta lo que afirmas sobre el código con el código real (archivos leídos, referencias buscadas), no con tu memoria.
4. Comprueba que cada comando de comprobación existe y corre en este proyecto. Si no puedes ejecutarlo, dilo en la propuesta en vez de asumir que funcionará.
5. Verifica que cada comprobación tiene situación real, tipo y nombre de test, y que cada regla tiene una comprobación que la protege o dice "solo tú".
6. Haz la prueba de lectura de `references/redaccion-legible.md`: lee solo "Para decidir" y comprueba que no queda ningún corchete ni texto de la plantilla.
7. Si encuentras una contradicción, corrígela antes de mostrar la propuesta y anótalo en "Antes de presentar, corregí".

## 4. Aprobación y edición

Espera la aprobación y no edites nada antes. Edita solo lo aprobado.

Después de la aprobación, en este orden:
1. **Pruebas previas** de las suposiciones marcadas "se prueba antes de editar" (si las hay).
2. **Tests:** lee `references/diseno-de-tests.md` (y mira `assets/ejemplo-tests.py` si no tienes claro el estilo), escribe los tests de las comprobaciones, ejecútalos y anota el "Estado previo" de cada uno. Deben fallar por la razón correcta (los de "no debe cambiar" deben pasar).
3. **Edición** del proyecto.

Las pruebas previas van antes porque pueden cambiar el plan, y escribir tests de un plan que luego cambia es trabajo perdido. Los tests se escriben antes de editar y nunca después para que "pasen": un test hecho a medida del código no mide nada.

- **Fuera de alcance:** si encuentras algo que no está en la propuesta, no lo arregles. En nivel 2, anótalo en "Fuera de alcance" del archivo. En niveles 0 y 1 no lo reportes, salvo que toque datos, seguridad o varios módulos (entonces va en "Requiere tu decisión").
- **Pruebas previas:** si alguna suposición dice "se prueba antes de editar", lee `references/prototipos.md`.
- Las comprobaciones no se modifican sin aprobación. Si crees que una está mal, propón el cambio por separado.
- **Parar y evaluar si:**
  - una suposición resulta falsa, por el camino que sea;
  - fallas 2 veces seguidas la misma comprobación.

  Detente, marca la propuesta como `discrepada` y sigue `references/suposicion-falsa.md` (con `assets/ejemplo-evaluacion.md` como modelo). Seguir probando o escribir otra propuesta sin avisar es lo que hace perder tiempo.

## 5. Verifica después de editar

Para propuestas de nivel 2, lee `references/verificacion-posterior.md` y sigue su flujo: ejecución real, comparación con lo predicho, comprobación contra las reglas, clasificación de discrepancias, estado `discrepada` y revisión humana.

Para nivel 1, ejecuta el test mínimo y di en el resumen si coincide con lo predicho.

### Sección "Revisión humana" del archivo (nivel 2)

Sí puedes y debes editar esta sección: es donde escribes lo que queda para la persona. Lo que no puedes es emitir el veredicto.

- Al presentar la propuesta, escribe ahí lo que solo ella puede mirar (comprobaciones visibles y reglas "solo tú"), uno por línea.
- Al verificar, añade los puntos que resultaron "no pudo comprobarse".
- Escribe cada uno con las marcas de veredicto sin elegir, tal cual: `✔ | ✘ | ?`. La persona elige una.
- Si no queda nada para ella, escribe "ninguno: todo se comprobó con tests" y cuáles tests. No inventes elementos para llenar la lista.

**No puedes:** elegir una marca (✔, ✘ o ?), escribir la nota de la persona ni rellenar la línea "Detectado solo por el humano". Que quien hizo el trabajo se califique a sí mismo le quita todo el sentido a la revisión.

## 6. Resumen final en el chat (máximo 5 líneas)

Se aplica a todos los niveles. Es corto a propósito: la persona suele saltarse los resúmenes largos.

```
Hecho: [una frase] + coincide | difiere de la propuesta.
Requiere tu decisión: [bloqueantes, riesgos serios, discrepancias,
  puntos que solo tú puedes mirar] o "nada".
Fuera de alcance: [N hallazgos anotados en el archivo] o "ninguno".
Archivo: [ruta]
```

- Niveles 0 y 1: solo "Hecho" y "Requiere tu decisión".
- Nivel 2: las cuatro líneas.
- No narres el proceso: di qué quedó, no cómo llegaste.
- No propongas mejoras ni "podríamos también..." en el resumen.
- De los hallazgos fuera de alcance solo da el conteo, nunca la lista.
- Excepción: si un hallazgo toca datos, seguridad o varios módulos, sube a "Requiere tu decisión".
- Si no cabe en 5 líneas, el resto va al archivo.
- Si hay una discrepancia, el mensaje es el de `references/suposicion-falsa.md`, no este.

## 7. Pendientes

Cuando una propuesta de nivel 2 se cierre, copia sus hallazgos de "Fuera de alcance" a `propuestas/PENDIENTES.md` sin duplicar. El triaje solo se hace cuando la persona declara el cierre de una etapa ("cerremos etapa", "hagamos triaje"): ahí lee `references/triaje-pendientes.md`. Nunca lo inicies por tu cuenta. Si hay 10 o más pendientes abiertos, menciónalo en una línea del resumen final y nada más.

## Reglas de verificabilidad

Los detalles de redacción están en `references/redaccion-legible.md`. Lo esencial:

- Toda afirmación sobre el código tiene su evidencia (archivo y línea, o el comando que ejecutaste) en la sección "Evidencia". Sin evidencia, no se escribe.
- "No verificado" es una respuesta válida. No rellenes con algo que suene plausible.
- No cuantifiques sin medir: porcentajes, tiempos y conteos solo si salieron de una medición real.
- Código en la propuesta solo en 1 o 2 partes críticas, siempre con su explicación al lado.
- Sin jerga en lo que lee la persona; define el término técnico imprescindible la primera vez.
