---
name: propuesta-verificable
description: Flujo de propuesta verificable antes de editar código. El agente predice qué va a cambiar (con ejemplos concretos), espera aprobación, edita solo lo aprobado y después compara lo predicho con lo que realmente pasó. Usar siempre que vayas a modificar, crear o borrar código en un proyecto, incluso si el cambio parece pequeño; también al corregir errores, refactorizar, agregar funciones, cambiar configuración o tocar base de datos, autenticación o pagos. Elige el nivel de detalle (0, 1 o 2) con criterios comprobables. No usar para explicar código, responder preguntas ni revisar sin editar.
---

# Propuesta verificable

Antes de editar código, escribe una propuesta que prediga qué va a pasar. Después de editar, compara lo predicho con lo real.

Por qué: explicar un plan paso a paso ayuda a encontrar errores en él antes de gastar una edición (es la idea del rubber duck debugging, aplicada antes de editar y no después). Además le da a la persona algo concreto que revisar, en vez de un plan vago que termina aprobando sin leer. Tu propuesta es una hipótesis: la persona la revisa antes, y tú la contrastas con la realidad después. Es también tu contrato: define qué puedes hacer y qué no, y te da criterio cuando aparece algo que no se previó.

Mantén el esfuerzo proporcional al riesgo: más detalle donde un error cuesta caro, nada donde no hace falta.

## 1. Elige el nivel

Usa criterios que puedas comprobar (contar archivos, buscar referencias), no tu impresión de si algo es riesgoso.

**Nivel 2** si se cumple CUALQUIERA:
- Toca una zona sensible. La lista está en el archivo de contexto del proyecto; si no existe, pídesela a la persona antes de seguir.
- Modifica más de 3 archivos, o crea o borra archivos.
- Cambia lógica compartida: cualquier función, clase o constante usada por otros archivos, sin importar cuántos sean (compruébalo buscando referencias).
- Es la primera vez que se modifica ese módulo desde su creación. Busca en `propuestas/` y en el historial de Git: si el módulo solo tiene su commit de creación y ninguna propuesta previa, cuenta como primera vez. La razón es que en esa etapa se toman las decisiones más grandes del proyecto (arquitectura, stack), y un error ahí sale caro más adelante.
- No hay un test que cubra el cambio y tampoco se puede ver al ejecutar la app.
- Es el segundo intento fallido seguido con el mismo problema.

**Nivel 0** solo si se cumplen TODAS: es texto, estilo o comentarios; es un solo archivo; no toca lógica.

**Nivel 1**: todo lo demás.

Si dudas entre dos niveles, aplica el más alto. Pasarte un poco de cuidado cuesta unos tokens; quedarte corto es justo lo que esta práctica intenta evitar.

## 2. Escribe la propuesta

- **Nivel 0:** no hay propuesta. Edita directamente.
- **Nivel 1:** en el chat. Una o dos frases, un ejemplo "antes: A, ahora: B", un test mínimo que lo compruebe y, en una línea, con qué herramienta o comando lo comprobarás. Si el proyecto no tiene infraestructura de tests y montarla no es barato, dilo en una línea en vez de montarla.
- **Nivel 2:** crea el archivo `propuestas/AAAA-MM-DD_nombre-corto.md` a partir de `assets/plantilla-propuesta.md`. Sigue las reglas de esa plantilla, en especial:
  - Criterios de aceptación como escenarios Dado/Cuando/Entonces. Cada uno nombra el caso de uso del que sale, su tipo (normal, límite, error o regresión) y el nombre del test. Cobertura mínima: uno normal, uno límite y, si el cambio afecta comportamiento existente, uno de regresión. Detalle en `references/diseno-de-tests.md`.
  - En la propuesta defines los tests (nombre y qué comprueban), pero todavía no los escribes. Los escribes después de la aprobación; ver la sección 4.
  - Cada restricción del bloque "Contexto" se protege con un escenario o se marca "no testeable".
  - Explica cómo vas a comprobar cada escenario (sección "Cómo lo comprobaré"): la herramienta o el comando concreto, qué resultado cuenta como pasar y qué NO detecta esa comprobación. La persona no debe tener que adivinar cómo se valida nada.
  - Incluye el bloque "Contexto": intención, restricciones con su origen y "→ implica", alternativas descartadas y criterio ante imprevistos. Cada línea debe cambiar algo que harías; si no, sobra.
  - En las suposiciones, marca cuáles se prototiparán (máximo 3) y por qué esas.
  - En el chat solo escribe 3 líneas: la ruta del archivo, qué cambia en una frase y cuántas suposiciones sin verificar hay (y cuántas llevan prototipo). No repitas la propuesta.

Antes de tu primera propuesta de nivel 2 en la conversación, mira `assets/ejemplo-propuesta.md` para calibrar cuánta concreción se espera. No copies su contenido.

### Preguntas antes de proponer (nivel 2)

Antes de escribir, revisa el archivo de contexto del proyecto, `docs/` (si existe) y el código. Si el plan depende de algo que no encuentras en ninguno de los tres, pregúntale a la persona. Lo que solo ella sabe no se puede deducir, y rellenarlo con suposiciones es lo que más errores ha causado.

- Máximo 3 preguntas. No preguntes lo que puedes encontrar tú ni lo que ya está en el archivo de contexto.
- Revisa cuatro categorías: restricciones, convenciones, cosas que no deben tocarse y comportamiento esperado en casos borde.
- Cada pregunta dice de qué decisión depende ("Cambia si el paso 3 usa DELETE o una marca de estado") y ofrece opciones para elegir, más una salida "no sé". La persona suele tener dificultad para redactar respuestas largas: que pueda contestar con una palabra.
- "No sé" pasa a "Suposiciones", con su "si es falso, el plan cambia así".
- Las respuestas entran al bloque "Contexto" con origen "respuesta humana". Si una respuesta es un hecho que seguirá valiendo más allá de este cambio, al cerrar la propuesta sugiere una línea para el archivo de contexto, con su origen, y espera la aprobación. Nunca escribas en ese archivo sin ella.
- Si una línea del archivo de contexto contradice el código, gana el código: repórtalo como discrepancia.

## 3. Autocomprobación antes de presentar

1. Compara la propuesta con lo que se pidió: ni añadir ni omitir.
2. Pasa cada ejemplo por los pasos propuestos y verifica que da el resultado que afirmas.
3. Contrasta lo que afirmas sobre el código con el código real (archivos leídos, referencias buscadas), no con tu memoria.
4. Comprueba que cada método de comprobación es real: que la herramienta o el comando existe y corre en este proyecto (por ejemplo, que el comando de tests se ejecuta). Si no puedes ejecutarlo, dilo en la propuesta en vez de asumir que funcionará.
5. Verifica que cada escenario tiene caso de uso, tipo y nombre de test, y que cada restricción tiene un escenario que la protege o está marcada "no testeable".
6. Si encuentras una contradicción, corrígela antes de mostrar la propuesta y anótalo en "Correcciones previas".

## 4. Aprobación y edición

Espera la aprobación y no edites nada antes. Edita solo lo aprobado.

Después de la aprobación, en este orden:
1. **Prototipos** de las suposiciones marcadas `prototipar: sí` (si las hay).
2. **Tests:** lee `references/diseno-de-tests.md` (y mira `assets/ejemplo-tests.py` si no tienes claro el estilo), escribe los tests de los escenarios, ejecútalos y anota el "Estado previo" de cada uno. Deben fallar por la razón correcta (los de regresión deben pasar).
3. **Edición** del proyecto.

Los prototipos van antes porque pueden cambiar el plan, y escribir tests de un plan que luego cambia es trabajo perdido. Los tests se escriben antes de editar y nunca después para que "pasen": un test hecho a medida del código no mide nada.

- **Fuera de alcance:** si encuentras algo que no está en la propuesta, no lo arregles. En nivel 2, anótalo en la sección "Fuera de alcance" del archivo. En niveles 0 y 1 no lo reportes, salvo que toque datos, seguridad o varios módulos (entonces va en "Requiere tu decisión").
- **Prototipos:** si alguna suposición dice `prototipar: sí`, lee `references/prototipos.md`. Si uno falsifica una suposición, detente.
- Los criterios de aceptación no se modifican sin aprobación. Si crees que uno está mal, propón el cambio por separado.
- **Parar si** fallas 2 veces seguidas el mismo criterio: detente, explica qué probaste, marca la propuesta como `discrepada` y espera la decisión de la persona. Seguir probando sin avisar es lo que hace perder tiempo.

## 5. Verifica después de editar

Para propuestas de nivel 2, lee `references/verificacion-posterior.md` y sigue su flujo: ejecución real, comparación con lo predicho, comprobación contra el bloque "Contexto", clasificación de discrepancias, estado `discrepada` y revisión humana.

Para nivel 1, ejecuta el test mínimo y di en el resumen si coincide con lo predicho.

### Sección "Revisión humana" del archivo (nivel 2)

Sí puedes y debes editar esta sección: es donde escribes lo que queda para la persona. Lo que no puedes es emitir el veredicto.

- Al presentar la propuesta, escribe ahí los escenarios marcados `(visible)` y las restricciones marcadas "no testeable", uno por línea.
- Al verificar, añade los que resultaron "no pudo comprobarse".
- Escribe cada uno con las marcas de veredicto sin elegir, tal cual: `✔ | ✘ | ?`. La persona elige una.
- Si no queda nada para ella, escribe "ninguno: todo se comprobó con tests" y cuáles tests. No inventes elementos para llenar la lista.

**No puedes:** elegir una marca (✔, ✘ o ?), escribir la nota de la persona ni rellenar la línea "Detectado solo por el humano". Que quien hizo el trabajo se califique a sí mismo le quita todo el sentido a la revisión.

## 6. Resumen final en el chat (máximo 5 líneas)

Se aplica a todos los niveles. Es corto a propósito: la persona suele saltarse los resúmenes largos.

```
Hecho: [una frase] + coincide | difiere de la propuesta.
Requiere tu decisión: [bloqueantes, riesgos serios, discrepancias,
  criterios "(visible)"] o "nada".
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

## 7. Pendientes

Cuando una propuesta de nivel 2 se cierre, copia sus hallazgos de "Fuera de alcance" a `propuestas/PENDIENTES.md` sin duplicar. El triaje solo se hace cuando la persona declara el cierre de una etapa ("cerremos etapa", "hagamos triaje"): ahí lee `references/triaje-pendientes.md`. Nunca lo inicies por tu cuenta. Si hay 10 o más pendientes abiertos, menciónalo en una línea del resumen final y nada más.

## Reglas de lenguaje y verificabilidad

La persona suele ser principiante y no puede juzgar jerga ni frases vagas, así que lo que escribas tiene que poder comprobarse.

- Sin jerga. Si un término técnico es imprescindible, defínelo entre paréntesis la primera vez.
- Prohibidas las frases sin contenido comprobable ("mejora la robustez", "optimiza el flujo"). Cada afirmación debe poder verificarse con un ejemplo o un archivo.
- Toda cifra o afirmación sobre el código indica su origen: archivo y función, o el comando que ejecutaste. Sin origen, no se escribe.
- "No verificado" es una respuesta válida. No rellenes con algo que suene plausible.
- No cuantifiques sin medir: porcentajes, tiempos y conteos solo si salieron de una medición real.
- Código en la propuesta solo en 1 o 2 partes críticas, siempre con su explicación al lado.
