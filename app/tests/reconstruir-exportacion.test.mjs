import assert from "node:assert/strict";
import test from "node:test";
import { reconstruirDesdeNdjson } from "../../tools/reconstruir_saldos.mjs";

test("reconstruye saldos exactos desde una exportación NDJSON sin cifrar (RNF-04)", () => {
  // Simula una exportación real con cuenta, ingreso, gasto, corrección, gasto de grupo y tasa
  // Los eventos se colocan intencionalmente desordenados para verificar el orden determinista.
  const eventos = [
    {
      id: "ev-5",
      tipo: "GastoDeGrupoAplicado",
      momento_registro: "2026-11-24T00:00:00Z",
      dispositivo_origen: "servidor",
      datos: {
        movimiento_id: "mov-grupo-1",
        cuenta_id: "cta-usd",
        evento_servidor_id: "srv-1",
        grupo_id: "grp-1",
        gasto_id: "gst-1",
        concepto: "Viaje · Hotel",
        fecha: "2026-11-24",
        monto: { cantidad: "33.33", moneda: "USD" },
        incluye_centimos_redondeo: false,
      },
    },
    {
      id: "ev-4",
      tipo: "MovimientoCorregido",
      momento_registro: "2026-11-23T15:00:00Z",
      dispositivo_origen: "laptop",
      datos: {
        ajuste_id: "aj-1",
        movimiento_original_id: "mov-2",
        cuenta_id: "cta-usd",
        monto_correcto: { cantidad: "25.00", moneda: "USD" },
        diferencia: "-5.00",
        fecha: "2026-11-23",
        nota: "Corrección de gasto",
      },
    },
    {
      id: "ev-1",
      tipo: "CuentaCreada",
      momento_registro: "2026-11-20T10:00:00Z",
      dispositivo_origen: "laptop",
      datos: {
        cuenta_id: "cta-usd",
        nombre: "Efectivo USD",
        tipo: "efectivo",
        saldo_inicial: { cantidad: "151.67", moneda: "USD" },
      },
    },
    {
      id: "ev-2",
      tipo: "MovimientoRegistrado",
      momento_registro: "2026-11-21T12:00:00Z",
      dispositivo_origen: "telefono",
      datos: {
        movimiento_id: "mov-1",
        cuenta_id: "cta-usd",
        tipo: "ingreso",
        monto: { cantidad: "50.00", moneda: "USD" },
        fecha: "2026-11-21",
        categoria: "Ventas",
        nota: null,
      },
    },
    {
      id: "ev-3",
      tipo: "MovimientoRegistrado",
      momento_registro: "2026-11-22T14:00:00Z",
      dispositivo_origen: "laptop",
      datos: {
        movimiento_id: "mov-2",
        cuenta_id: "cta-usd",
        tipo: "gasto",
        monto: { cantidad: "20.00", moneda: "USD" },
        fecha: "2026-11-22",
        categoria: "Alimentación",
        nota: "Almuerzo",
      },
    },
  ];

  const encabezado = {
    formato: "cuentas-claras-libro",
    version: 1,
    fecha_exportacion: "2026-11-26T22:00:00Z",
    cantidad_eventos: eventos.length,
  };

  const ndjson =
    [JSON.stringify(encabezado), ...eventos.map((e) => JSON.stringify(e))].join(
      "\n",
    ) + "\n";

  const resultado = reconstruirDesdeNdjson(ndjson);

  assert.equal(resultado.version, 1);
  assert.equal(resultado.cantidadEventos, 5);
  assert.equal(resultado.cuentas.length, 1);
  // 151.67 + 50.00 - 20.00 - 5.00 (ajuste) - 33.33 (grupo) = 143.34
  assert.equal(resultado.cuentas[0].nombre, "Efectivo USD");
  assert.equal(resultado.cuentas[0].saldo, "143.34");
  assert.equal(resultado.cuentas[0].movimientos, 4);
});
