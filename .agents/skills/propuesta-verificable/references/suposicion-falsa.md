# Cuando una suposición resulta falsa

Se aplica en cualquier momento en que algo que la propuesta daba por cierto se muestra falso. Puede pasar por varios caminos:

- una prueba rápida previa a la edición lo falsifica;
- un test falla y la causa es una suposición;
- lo descubres al leer código o al editar;
- la persona te lo dice.

No importa si la suposición estaba marcada para probar o no.

## La regla

Primero se evalúa la propuesta que existe. Después decide la persona. Solo entonces se escribe algo nuevo.

Mientras no haya evaluación escrita y decisión de la persona:

- no escribas una propuesta nueva;
- no sigas editando el proyecto;
- no deshagas lo ya editado.

Por qué: una suposición falsa casi nunca invalida toda la propuesta. Lo normal es que rompa una parte y deje el resto intacto. Si saltas a una propuesta nueva, la persona vuelve a leer todo, incluido lo que seguía valiendo, y pierde la forma de ver qué cambió de verdad. Además la alternativa que habías previsto ("Si es falso: …") es otra hipótesis. Puede apoyarse en suposiciones que tampoco has comprobado, o en una regla de negocio que solo conoce la persona.

## Pasos

Escribe todo en el bloque "Evaluación" del archivo de la propuesta. Ese bloque se puede añadir aunque la propuesta ya esté congelada, porque registra lo ocurrido y no reescribe lo predicho. Mira `assets/ejemplo-evaluacion.md` para calibrar el nivel de detalle; no copies su contenido.

**1. Establece el hecho.**
Qué se supuso y qué se observó, con la evidencia (comando y salida, o archivo y línea). Antes de darlo por falso, comprueba que la prueba no es la que falló: repite el comando o mira el dato desde otro ángulo. Anota qué hiciste. Si no puedes comprobarlo, dilo.

**2. Mira en qué estado está el proyecto.**
Con `git status` o listando archivos: qué archivos están editados y qué tests están escritos. Si no hay Git, compara con la lista de "Archivos" de la propuesta.

**3. Recorre la propuesta pieza por pieza.**
Lista cada paso, cada comprobación, cada regla y cada otra suposición. Marca cada una:

- **sigue valiendo:** nada de lo observado la toca.
- **hay que ajustar:** se mantiene la idea y cambia el cómo.
- **deja de valer:** ya no tiene sentido o es imposible.
- **no se sabe:** depende de algo que aún no se ha comprobado.

Cada marca lleva una frase con el motivo. No la rellenes por rutina: una pieza solo "sigue valiendo" si puedes explicar por qué la observación no la toca.

Cierra la lista con la línea "Resumen de la lista", contando las marcas de la lista que acabas de escribir (la suposición falsa no se cuenta entre las piezas). Usa esos mismos números en el mensaje del chat: no los estimes.

**4. Examina la alternativa prevista.**
Lee lo que decía "Si es falso" en esa suposición. Pregúntate de qué depende esa alternativa y si alguna de esas cosas está sin comprobar. Pruébala, si se puede de forma barata y fuera del proyecto, antes de recomendarla. Si lo falso toca una regla de negocio (cuánto cobrar, cómo redondear, qué hacer con un cliente inactivo), no elijas tú: es una pregunta para la persona.

**5. Propón opciones según el estado del proyecto.** Máximo tres, siempre con una recomendación y su razón.

Si todavía no editaste el proyecto:
- **A · Ajustar esta propuesta.** Cambias solo las piezas marcadas "hay que ajustar" o "deja de valer", el estado pasa a `modificada`, anotas el cambio en "Cambios tras pruebas previas" y esperas aprobación.
- **D · Parar.** Si lo observado pone en duda el objetivo mismo o falta información de la persona.

Si ya editaste:
- **B · Propuesta de corrección que hereda.** Una propuesta nueva y corta, con solo lo que cambia. Lo que sigue valiendo se cita por su id (S2, R1, C3) y no se vuelve a escribir. El código ya editado se conserva.
- **C · Deshacer y empezar de nuevo.** Solo si la mayoría de las piezas "deja de valer" o cambia el objetivo. Antes de deshacer, di exactamente qué se perdería.
- **D · Parar y preguntar.** Si lo falso es una decisión de la persona.

Una regla práctica para recomendar: si la mayoría de las piezas sigue valiendo y el objetivo no cambia, recomienda A o B; si el objetivo cambia o casi nada sirve, recomienda C o D. Escribe la razón con base en tu lista, no con la impresión de que "mejor rehacer".

**6. Preséntalo y espera.**
Mensaje en el chat, máximo 6 líneas:

```
Se rompió una suposición: [S# en una frase].
Sigue valiendo: [N de M piezas, según el Resumen de la lista]. Cambia: [cuáles, en pocas palabras].
Código: [editado: archivos] | sin editar.
Opciones: A [...] / B [...]. Recomiendo [letra] porque [razón].
Necesito de ti: elegir una opción. No escribo nada más hasta tu respuesta.
Archivo: [ruta], sección "Evaluación"
```

Anota la respuesta en "Decisión de la persona".

**7. Ejecuta lo elegido.**
- **A:** modifica la propuesta como se describe arriba y espera nueva aprobación.
- **B o C:** escribe la propuesta nueva con `Viene de:` y `Hereda sin cambios:` rellenos. "Para decidir" explica en pocas líneas qué cambia respecto a la anterior. En "Detalle" incluye solo suposiciones, reglas y comprobaciones nuevas o modificadas. No copies lo heredado.
- **D:** detente y espera la respuesta de la persona.

La propuesta original queda en estado `discrepada` y pasa a `cerrada` cuando se cierra su corrección.

## Qué no hacer

- No escribas la propuesta de corrección antes de evaluar. Es el error que esta regla existe para evitar.
- No des por buena la alternativa de "Si es falso" sin examinarla.
- No marques todo como "deja de valer" para justificar empezar de cero.
- No cambies un criterio de aceptación para que el test pase. Si crees que el criterio está mal, es una decisión de la persona: ponla como opción.
- No decidas por la persona qué hacer con el código ya editado.

## Para la próxima vez

Cierra con una línea: si la suposición no estaba marcada para probar, ¿habría sido barato probarla antes? Esa línea alimenta el criterio para marcar suposiciones en propuestas futuras.
