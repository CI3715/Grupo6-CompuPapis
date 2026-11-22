# ADR 011 - Gastos compartidos y división exacta

## Estado

Aceptada.

## Contexto

La entrega 6 requiere persistir gastos de grupo, repartir céntimos sin pérdida y calcular balances que sumen cero. Los datos financieros personales permanecen en el dispositivo.

## Decisión

Los grupos y gastos compartidos se almacenan en PostgreSQL. El cliente envía importes y porcentajes como texto decimal; el servidor valida la membresía y recalcula las partes con céntimos enteros, distribuyendo el residuo en orden de nombre de usuario. Cada gasto y sus partes se guardan en una transacción; una consulta con instantánea coherente obtiene gastos y balances. La migración 011 registra la regla y la marca de residuo. Para datos anteriores, ambos campos quedan desconocidos cuando no pueden reconstruirse.

Las rutas requieren sesión. Solo un miembro aceptado puede consultar o registrar gastos; una persona ajena recibe la misma respuesta que ante un grupo inexistente. Las cuentas, saldos personales, movimientos y tasas no se envían al servidor.

## Alternativas consideradas

- Usar coma flotante: descartado por errores de precisión monetaria.
- Guardar las partes calculadas por el cliente: descartado porque el servidor debe validarlas y calcularlas.
- Inventar las marcas de residuos de datos históricos: descartado porque no se puede reconstruir su origen.

## Consecuencias

Las operaciones compartidas requieren conexión y PostgreSQL. Una división inválida no se guarda; los datos históricos incoherentes impiden mostrar balances hasta corregirse. La vista previa del cliente no sustituye la validación del servidor.
