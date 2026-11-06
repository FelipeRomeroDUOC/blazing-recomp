# Ejemplo de evaluación tras una suposición falsa (ilustrativo)

Nota: ejemplo inventado, continuación de `ejemplo-propuesta.md` ya editada. Solo calibra estilo y nivel de detalle. No copies su contenido.

---

## Evaluación

Abierta por: agente · Fecha: 2026-11-06

**Qué se supuso:** S1 · Todas las fechas guardadas están escritas como AAAA-MM-DD.

**Qué se observó:** al abrir el pedido 9 de los datos de ejemplo, la pantalla falla con `ValueError: time data '20-11-2026' does not match format '%Y-%m-%d'` — evidencia: `grep -n "20-11-2026" datos/pedidos_ejemplo.json` → línea 31.

**¿Está bien comprobado?** Llamé a la función sola con "20-11-2026" y da el mismo error, así que el fallo viene del dato y no de la pantalla.

**Estado del proyecto ahora:** editados app/entregas.py, app/ui.py y tests/test_entregas.py; sin confirmar en Git. Los 4 tests pasan, porque ninguno usa ese formato de fecha.

**Qué sigue valiendo y qué no:**
Paso 1 · hay que ajustar — la función debe aceptar también el formato con guiones.
Paso 2 · sigue valiendo — la pantalla sigue usando la función nueva.
Paso 3 · sigue valiendo — los reportes no se tocaron.
C1, C2 · siguen valiendo — pasan con el código actual.
C3, C4 · siguen valiendo — pasan con el código actual.
R1, R2 · siguen valiendo — nada de lo observado las toca.
S2 · sigue valiendo — no depende del formato.
Falta una comprobación: un pedido con fecha "20-11-2026".

**Resumen de la lista:** 9 sigue valiendo · 1 hay que ajustar · 0 deja de valer · 0 no se sabe · total 10

**Opciones:**
A · No disponible: la propuesta ya está editada y congelada.
B · Propuesta de corrección que hereda S2, R1, R2, C1 a C4 y cambia solo el paso 1, con una comprobación nueva. — implica: se conserva todo el código ya editado.
C · Deshacer y empezar de nuevo — implica: se pierde lo que ya funciona; no lo recomiendo porque 9 de 10 piezas siguen valiendo.
D · Parar y preguntar primero cómo se leen las fechas con guiones.

**Recomiendo:** B, cuando respondas la pregunta de abajo, porque solo cambia una pieza y el resto sigue valiendo.

**La alternativa prevista en "Si es falso" depende de:** que "20-11-2026" sea día-mes-año. Con "05-11-2026" no se puede saber si es el 5 de noviembre o el 11 de mayo, y eso solo lo sabes tú.

**Decisión de la persona:** se anota aquí lo que responda en el chat.

---

Mensaje que se envía en el chat:

```
Se rompió una suposición: no todas las fechas guardadas son AAAA-MM-DD (el pedido 9 trae 20-11-2026).
Sigue valiendo: 9 de 10 piezas. Cambia: el paso 1 y falta una comprobación.
Código: editado (app/entregas.py, app/ui.py, tests).
Opciones: B corrección corta que hereda lo que sirve / C rehacer todo. Recomiendo B porque casi todo sigue valiendo.
Necesito de ti: ¿"05-11-2026" es 5 de noviembre? Sí / No (es 11 de mayo) / no sé. No escribo nada más hasta tu respuesta.
Archivo: propuestas/2026-11-05_fecha-entrega.md, sección "Evaluación"
```
