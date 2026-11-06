# Redacción legible de la propuesta

Se lee antes de escribir una propuesta de nivel 2 y antes de presentarla. Una propuesta que la persona no lee no cumple su función: la aprueba sin entenderla o la rechaza sin saber por qué. Que esté completa no basta; tiene que poder leerse en poco tiempo.

## Quién lee y con cuánto tiempo

Una persona que suele ser principiante, que no conoce el código por dentro y que tiene pocos minutos. Necesita tres cosas, en este orden:

1. Saber qué va a cambiar y qué verá ella.
2. Saber qué debe decidir o mirar.
3. Poder comprobar, si quiere, que lo que afirmas es cierto.

Por eso el archivo tiene dos partes. "Para decidir" cubre las dos primeras necesidades y se basta sola. "Detalle para verificar" cubre la tercera y se lee solo si la persona quiere revisar a fondo.

## Prueba de lectura antes de presentar

Lee solo "Para decidir", como si no supieras nada del proyecto.

- ¿Se entiende qué cambia y cómo se vería?
- ¿Se sabe qué hay que decidir, y con qué opciones?
- ¿Se sabe qué tendrá que mirar la persona con sus propios ojos?

Si alguna respuesta es no, reescribe esa parte. No lo arregles añadiendo texto en el detalle.

## Reglas

**1. La decisión va arriba.** Lo primero que aparece es qué cambia y qué se necesita de la persona. Las preguntas viven en el archivo, no solo en el chat: el chat se pierde y el archivo queda.

**2. "Para decidir" cabe en una pantalla.** Unas 20 líneas. Si no cabe, el cambio es demasiado grande: divídelo en dos propuestas, que es lo mismo que ya pide el tope de 3 o 4 comprobaciones.

**3. Cada línea cambia algo que harías.** Si no, sobra. Una regla del proyecto solo se lista si cambia lo que haces o lo que no tocas, con un máximo de 3. "Los tests se ejecutan con tal comando" es un dato del proyecto, no una regla de este cambio. La evidencia incluye solo lo que sostiene una afirmación del resumen o una decisión.

**3 bis. Cada dato aparece una vez.** El valor de un caso (por ejemplo 35.7) puede estar en el "Ejemplo" de arriba y en la comprobación correspondiente, porque el test lo necesita. No repitas el mismo caso en un tercer lugar. Por eso no existe una sección aparte de "cómo lo comprobaré": cada comprobación lleva su comando y su límite.

**4. El archivo final no contiene instrucciones.** La plantilla trae marcadores entre corchetes, guiones bajos y comentarios `<!-- -->` que son para ti. Antes de presentar, comprueba que no queda ninguno ni texto del estilo "Cada línea debe…" o "Reglas:". Las secciones fijas que serían idénticas en toda propuesta ("Terminado cuando", "Parar si") no se escriben: ya están en la skill.

**5. Una idea por línea, un dato por línea.** Una frase de unas 25 palabras como máximo. No encadenes varios campos con flechas, barras y paréntesis en una misma línea ("(origen: …) → implica: … → protegida por: …"). Cada campo va en su propia línea, como en la plantilla; solo se permite juntar dos campos cortos, como "Test" y "Estado previo". Una pregunta a la persona ocupa cuatro líneas: la pregunta, las opciones, qué usarás si no responde y qué cambia.

**6. Los nombres de archivos y funciones van en el detalle.** En "Para decidir" describe lo que verá quien usa la app. Si una afirmación sobre el código es necesaria arriba, escríbela en palabras y apunta a su evidencia: "el total lo usan dos pantallas (ver E1)". El origen completo (comando, archivo, línea) vive en "Evidencia". Así sigue siendo verificable sin ensuciar el texto.

**7. Sin jerga.** Si un término técnico es imprescindible, defínelo entre paréntesis la primera vez. Usa estas equivalencias en el texto que lee la persona:

| En lugar de | Escribe |
|---|---|
| regresión | algo que ya funcionaba y no debe cambiar |
| prototipo | prueba rápida fuera del proyecto |
| lógica compartida | código que usan otras partes |
| caso límite | caso extremo |
| refactorizar | reordenar el código sin cambiar lo que hace |
| endpoint | dirección a la que la app envía o pide datos |
| "mejora la robustez", "optimiza el flujo" | di qué cambia exactamente, con un ejemplo |

**8. Todo lo que afirmas se puede comprobar.** Una afirmación sobre el código lleva su evidencia en "Evidencia". Una cifra, un tiempo o un conteo solo aparece si salió de una medición real. "No verificado" es una respuesta válida. Si el valor exacto es el propio error (35.699999999999996), escríbelo tal cual.

**9. Las preguntas son fáciles de contestar.** Máximo 3. Cada una dice qué cambia según la respuesta y ofrece opciones, incluida "no sé". La persona debe poder contestar con una palabra.

**10. Los títulos dicen lo que hay debajo.** "Qué podría romperse" informa; "Riesgos" no. "Reglas del proyecto que respeto" informa; "Contexto" no.

## Mensaje en el chat al presentar

Cuatro líneas como máximo. No repitas la propuesta.

```
Propuesta: [ruta]
Cambia: [una frase]
Necesito de ti: [las preguntas numeradas, con sus opciones] | solo aprobar
Mirar tú: [N puntos que solo tú puedes comprobar] | nada
```

## Errores frecuentes

- Listar como regla algo que no cambia lo que haces.
- Pegar en el archivo los textos de ayuda de la plantilla.
- Escribir el mismo caso en el ejemplo, en la comprobación y en una tercera sección.
- Poner rutas y nombres de funciones en la primera pantalla.
- Dejar las preguntas solo en el chat.
- Llenar "Qué podría romperse" con riesgos genéricos. Si no encontraste ninguno, escribe lo que buscaste y que no hay resultados.
