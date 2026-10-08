# Prototipos de suposiciones

Se usan después de la aprobación y antes de editar el proyecto. Comprobar una suposición barata evita editar sobre una base falsa. Se hace después de aprobar, y no antes de proponer, para no llenar el contexto con detalles de pruebas antes de escribir la propuesta y para no gastar tokens en planes que la persona rechaza.

## Qué se prototipa

Solo las suposiciones marcadas `prototipar: sí`. Cada una debe cumplir las tres condiciones:
- Se puede probar de forma aislada (cómo responde una librería, qué devuelve un comando, si un formato de datos es el esperado).
- Si fuera falsa, el plan cambiaría.
- Probarla es barato.

Máximo 3 por propuesta, cada prototipo pequeño y con un solo objetivo. Para lo visual, renderiza y mide aparte antes de integrarlo.

## Reglas de seguridad

- Fuera del proyecto: usa una carpeta temporal. No toques archivos del repositorio ni crees directorios nuevos dentro de él.
- Sin datos reales, credenciales ni acciones con efectos externos (red, base de datos real, instalaciones que alteren el entorno del proyecto).
- Se descarta al terminar. Si quieres reutilizar algo, entra al proyecto como código normal y pasa por los tests reales.

## Relación con los tests

Los prototipos van antes de escribir los tests, porque pueden cambiar el plan. Un prototipo responde "¿el entorno se comporta como supongo?" y se descarta; un test responde "¿mi cambio cumple lo prometido?" y se queda en el proyecto. Si un prototipo revela un comportamiento que vale la pena proteger, añádelo como escenario nuevo: eso modifica la propuesta y pide nueva aprobación.

## Registro

En la suposición, una línea con el resultado y "no detecta" (en qué se diferencia el prototipo del entorno real). No guardes logs ni código en el archivo: solo la conclusión.

## Si el prototipo confirma la suposición

No te detengas. Anota el resultado y sigue.

## Si el prototipo la falsifica

Detente siempre antes de editar el proyecto.
1. Registra el resultado en esa suposición.
2. Si el plan debe cambiar: modifica la propuesta, pasa el estado a `modificada`, anota en "Cambios tras prototipos" qué decía antes, qué dice ahora y por qué, y espera nueva aprobación. Esto aplica aunque el cambio sea el que ya habías declarado en "si es falso".
3. Si el plan no necesita cambiar, avisa en el chat y espera confirmación.

La propuesta puede modificarse así solo antes de empezar a editar. Una vez que empieza la edición del proyecto, queda congelada.

## Si el contexto quedó pesado

Si tras los prototipos la conversación está muy cargada, la propuesta ya está en el archivo: sugiere continuar en una conversación nueva leyéndola.
