# ADR 009 - Patrimonio histórico y precisión decimal

## Estado

Implementada en la Entrega 4; documentada retrospectivamente según el código actual.

## Contexto

El patrimonio consolida cuentas en distintas monedas para una fecha civil. El uso de punto flotante o de tasas inadecuadas puede producir importes imprecisos o totales engañosos.

## Decisión

- Proyectar el saldo local a la fecha solicitada: excluir cuentas anteriores a su creación e incluir únicamente movimientos y ajustes aplicables hasta esa fecha. Una corrección posterior no altera un saldo anterior.
- Usar la moneda base y el catálogo actuales, sin reescribir eventos ni reconstruir preferencias históricas. La moneda base predeterminada es USD.
- Transportar importes y tasas como texto decimal y calcular en Rust con `rust_decimal`. La conversión inversa divide directamente por la tasa; el total se redondea una sola vez con midpoint nearest-even. Un desbordamiento devuelve error.
- Si falta una tasa, devolver los saldos, subtotales y pares faltantes con `total: null`; no inventar conversiones ni presentar un total parcial como consolidado. La selección de tasas sigue el [ADR 008](008-tasas-eventos-e-historial.md).

## Alternativas consideradas

- `f64` o `Number` para cálculos monetarios: pueden perder precisión.
- Tasas cruzadas automáticas o un total parcial: alterarían el contrato y podrían ocultar datos faltantes.
- Recalcular movimientos al cambiar la moneda base: modificaría el libro personal.

## Consecuencias

La consulta es local y las tasas usadas son explícitas. Los importes sin tasa requieren completar la información antes de obtener el total. Las preferencias actuales no representan necesariamente las que existían en la fecha consultada. Ver [Entrega 4: backend local](../entrega-4-backend.md) y `app/src-tauri/src/domain.rs`.
