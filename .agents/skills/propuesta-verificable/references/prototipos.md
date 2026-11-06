# Pruebas previas de suposiciones (prototipos)

Se usan después de la aprobación y antes de editar el proyecto. Comprobar una suposición barata evita editar sobre una base falsa. Se hace después de aprobar, y no antes de proponer, para no llenar el contexto con detalles de pruebas antes de escribir la propuesta y para no gastar tokens en planes que la persona rechaza.

## Qué se prueba

Solo las suposiciones marcadas "Se prueba antes de editar: sí". Cada una debe cumplir las tres condiciones:
- Se puede probar de forma aislada (cómo responde una librería, qué devuelve un comando, si un formato de datos es el esperado).
- Si fuera falsa, el plan cambiaría.
- Probarla es barato.

Máximo 3 por propuesta, cada prueba pequeña y con un solo objetivo. Para lo visual, renderiza y mide aparte antes de integrarlo.

## Reglas de seguridad

- Fuera del proyecto: usa una carpeta temporal. No toques archivos del repositorio ni crees directorios nuevos dentro de él.
- Sin datos reales, credenciales ni acciones con efectos externos (red, base de datos real, instalaciones que alteren el entorno del proyecto).
- Se descarta al terminar. Si quieres reutilizar algo, entra al proyecto como código normal y pasa por los tests reales.

## Relación con los tests

Las pruebas previas van antes de escribir los tests, porque pueden cambiar el plan. Una prueba previa responde "¿el entorno se comporta como supongo?" y se descarta; un test responde "¿mi cambio cumple lo prometido?" y se queda en el proyecto. Si una prueba revela un comportamiento que vale la pena proteger, añádelo como comprobación nueva: eso modifica la propuesta y pide nueva aprobación.

## Registro

En la suposición, una línea con el resultado y qué no cubre (en qué se diferencia la prueba del entorno real). No guardes logs ni código en el archivo: solo la conclusión.

## Si la prueba confirma la suposición

No te detengas. Anota el resultado y sigue.

## Si la prueba la falsifica

Detente antes de editar el proyecto y sigue `references/suposicion-falsa.md`. En resumen:

1. Registra el resultado en esa suposición.
2. Evalúa la propuesta pieza por pieza y escribe la evaluación en el archivo. No reescribas el plan todavía.
3. Examina la alternativa que decía "Si es falso": es otra hipótesis, no una decisión tomada.
4. Presenta opciones con una recomendación y espera la respuesta de la persona.
5. Si la decisión es ajustar la propuesta (opción A), pasa el estado a `modificada`, anota qué decía antes, qué dice ahora y por qué en "Cambios tras pruebas previas", y espera nueva aprobación. Si la evaluación concluye que el plan no necesita cambiar, dilo en el chat y espera confirmación.

La propuesta puede modificarse así solo antes de empezar a editar. Una vez que empieza la edición del proyecto, queda congelada.

## Si el contexto quedó pesado

Si tras las pruebas la conversación está muy cargada, la propuesta ya está en el archivo: sugiere continuar en una conversación nueva leyéndola.
