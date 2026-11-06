# Verificación posterior

Se aplica después de editar una propuesta de nivel 2. Desde que empieza la edición del proyecto, la propuesta original no se modifica nunca: es la hipótesis que se contrasta. (Antes de editar, una prueba previa sí puede cambiarla, con registro en "Cambios tras pruebas previas"; ver `references/prototipos.md`.) Si la reescribes para que parezca acertada, la persona pierde la única forma de medir si tus predicciones son buenas.

Sí puedes añadir los bloques "Resultado" y "Evaluación" del archivo: registran lo ocurrido y no reescriben lo predicho.

## 1. Verificación del agente

1. Ejecuta las comprobaciones con el comando que anunciaste (los tests de las comprobaciones y los tests existentes relacionados). Si usas otro método, anótalo en "Resultado" con el motivo; no lo cambies en silencio. Los tests ya existen desde antes de editar; si falta alguno, es una discrepancia: no lo escribas ahora para que pase. Registra el resultado real, no el esperado. Releer tu propio trabajo no cuenta como verificar: los modelos tienden a confiar demasiado en su primera respuesta si no ejecutan nada.
2. Compara comprobación por comprobación: coincide | difiere | no pudo comprobarse. Las que solo se pueden mirar a simple vista se registran como "no pudo comprobarse" y pasan a revisión humana.
3. Contrasta con las reglas del proyecto: ¿se respetó cada una (usa el test que la protege)? ¿Se tocó algo que alguna prohibía? ¿Surgió algún imprevisto y se aplicó la prioridad anunciada? ¿Reapareció una alternativa descartada? Una regla incumplida es una discrepancia.
4. Lista los archivos realmente modificados (con el listado de cambios de Git u otro comando) y compáralos con los anunciados. Indica si se tocó algo más.
5. Completa la sección "Resultado" del archivo: veredicto, una línea por comprobación, reglas, archivos, discrepancias con su clasificación y "Para la próxima vez" (una sola línea concreta).
6. No corrijas en silencio ni reescribas la propuesta.

## 2. Clasificación de discrepancias

- **Error de juicio:** el plan estaba mal planteado.
- **Error de implementación:** el plan era correcto, la ejecución no.
- **Suposición falsa:** algo que se dio por cierto resultó no serlo.
- **Información faltante:** la información no estaba al alcance del agente: solo la tenía la persona.
- **Omisión:** la información estaba al alcance (en el código, en archivos o en el contexto) y no se usó.
- **Imprevisto:** el plan era razonable y falló por algo que no se podía anticipar con lo disponible (el comportamiento de una librería, el entorno).

Desempate entre información faltante y omisión: ¿estaba esa información en algo que el agente tenía a su alcance? Si sí, omisión; si no, información faltante.

Qué anotar según el tipo (en "Para la próxima vez"):
- **Información faltante:** propón una línea para el archivo de contexto del proyecto, con su fuente. La persona la aprueba; nunca la escribas sin aprobación.
- **Omisión:** qué debiste leer o comprobar antes de afirmar.
- **Imprevisto:** la zona o herramienta donde ocurrió, para la próxima vez que se toque.
- **Suposición falsa:** si no se probó antes, ¿habría sido barato hacerlo?

## 3. Estado "discrepada"

Se activa cuando:
- El agente encuentra una discrepancia o una comprobación que no se cumple.
- Una suposición resulta falsa, por el camino que sea.
- La persona marca ✘ o detecta algo que el agente dio por bueno.
- El agente falla 2 veces seguidas la misma comprobación (detenerse, no seguir probando).

Al activarse, en este orden:
1. Cambia el estado a "discrepada". No puede pasar a "cerrada" mientras tanto.
2. **Escribe el bloque "Evaluación"** siguiendo `references/suposicion-falsa.md`: el hecho, el estado del proyecto, qué sigue valiendo y qué no, opciones y una recomendación.
3. **Presenta la evaluación en el chat y espera la decisión de la persona.** No escribas ninguna propuesta nueva ni sigas editando antes de recibirla.
4. Solo después, ejecuta lo elegido. Si es una propuesta de corrección, enlázala a la original con `Viene de:` y hereda lo que sigue valiendo en vez de copiarlo. No reescribas la original.
5. La original pasa a "cerrada" cuando se cierra su corrección.

## 4. Aviso en el chat

- Sin discrepancias: usa el formato de resumen final de `SKILL.md` (máximo 5 líneas) con "Coincide con lo predicho".
- Con discrepancias: usa el mensaje de `references/suposicion-falsa.md`.

## 5. Revisión humana

La sección "Revisión humana" del archivo la editan los dos, con roles distintos.

**Tú (el agente) SÍ debes editarla** para escribir lo que queda para la persona:
- Al presentar la propuesta, lo que solo ella puede mirar: comprobaciones visibles y reglas "solo tú", uno por línea.
- Al verificar, añade los que resultaron "no pudo comprobarse".
- Cada uno con las marcas de veredicto sin elegir, tal cual: `✔ | ✘ | ?`.
- Si no queda nada para ella, escribe "ninguno: todo se comprobó con tests" y cuáles tests. No inventes elementos para llenar la lista; una lista vacía es válida cuando todo se pudo comprobar.

**Tú NO puedes:** elegir una marca (✔, ✘ o ?), escribir la nota de la persona ni rellenar "Detectado solo por el humano". Evaluar tu propio trabajo en esta sección anula la revisión.

Formato del veredicto (lo elige la persona): por cada punto, una marca entre ✔ | ✘ | ? con nota opcional de una línea. A la persona le cuesta poner en palabras qué está mal, así que no le exijas descripciones precisas.

Se pide su revisión cuando:
- Hay discrepancias (confirma o corrige la clasificación).
- Hay comprobaciones visibles o "no pudo comprobarse".

Si su comentario es vago ("se ve raro", "algo no cuadra"):
1. Reformula con tu interpretación concreta (qué elemento, qué diferencia, cuánto) y pídele confirmar.
2. No edites hasta que confirme.

Métrica: si la persona abre una discrepancia sobre algo que el agente había dado por correcto, cuenta como "detectado solo por el humano". Anota qué comprobación habría podido detectarlo.

## 6. Estados y cierre

pendiente → aprobada → verificada por agente → revisada por humano → cerrada

- "discrepada" puede activarse desde cualquier etapa posterior a la edición, y obliga a escribir la evaluación antes de cualquier propuesta nueva.
- "modificada" y "descartada" ocurren antes de editar.
- Sin discrepancias ni puntos sin comprobar, el agente puede cerrar tras su verificación. Con alguno de los dos, no se cierra sin la revisión humana.
