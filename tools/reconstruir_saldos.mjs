#!/usr/bin/env node
/**
 * Herramienta independiente de Cuentas Claras (RNF-04).
 * Reconstruye cuentas, movimientos y saldos exactos a partir de un archivo
 * de exportación sin cifrar (.ndjson) descrito en docs/formato-exportacion.md,
 * usando aritmética decimal exacta con BigInt (sin float).
 *
 * Uso CLI:
 *   node tools/reconstruir_saldos.mjs ruta/al/respaldo.ndjson
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ESCALA = 8; // Soporta hasta 8 decimales exactos (igual que el dominio)
const FACTOR = 10n ** BigInt(ESCALA);

/** Convierte una cadena decimal (ej. "151.67", "-10.5", "0") a entero escalado a 10^8. */
export function parseDecimal(texto) {
  const limpio = String(texto).trim();
  const match = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(limpio);
  if (!match) {
    throw new Error(`Número decimal inválido: ${texto}`);
  }
  const signo = match[1] === "-" ? -1n : 1n;
  const entera = BigInt(match[2]);
  const fraccion = (match[3] ?? "").padEnd(ESCALA, "0").slice(0, ESCALA);
  return signo * (entera * FACTOR + BigInt(fraccion));
}

/** Formatea un entero escalado con la cantidad de decimales indicada. */
export function formatDecimal(valor, decimales = 2) {
  const negativo = valor < 0n;
  const abs = negativo ? -valor : valor;
  const divisor = 10n ** BigInt(ESCALA - decimales);
  const escalado = abs / divisor;
  const base = 10n ** BigInt(decimales);
  const ent = escalado / base;
  const frac = escalado % base;
  const cuerpo =
    decimales === 0
      ? ent.toString()
      : `${ent.toString()}.${frac.toString().padStart(decimales, "0")}`;
  return negativo && escalado > 0n ? `-${cuerpo}` : cuerpo;
}

/**
 * Reconstruye el libro completo a partir del contenido NDJSON sin cifrar.
 * Aplica el orden determinista (momento_registro, dispositivo_origen, id).
 */
export function reconstruirDesdeNdjson(contenido) {
  const lineas = contenido
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);

  if (lineas.length === 0) {
    throw new Error("El archivo de exportación está vacío.");
  }

  const encabezado = JSON.parse(lineas[0]);
  if (encabezado.formato !== "cuentas-claras-libro") {
    throw new Error(`Formato no reconocido: ${encabezado.formato}`);
  }
  if (encabezado.version !== 1) {
    throw new Error(`Versión de formato no soportada: ${encabezado.version}`);
  }

  const eventos = lineas.slice(1).map((l, i) => {
    try {
      return JSON.parse(l);
    } catch {
      throw new Error(`Línea ${i + 2} mal formada.`);
    }
  });

  if (eventos.length !== encabezado.cantidad_eventos) {
    throw new Error(
      `Cantidad de eventos no coincide: encabezado=${encabezado.cantidad_eventos}, reales=${eventos.length}`,
    );
  }

  // Deduplicar por id y ordenar determinísticamente: momento_registro, dispositivo_origen, id
  const vistos = new Map();
  for (const ev of eventos) {
    if (!vistos.has(ev.id)) {
      vistos.set(ev.id, ev);
    }
  }
  const ordenados = [...vistos.values()].sort((a, b) => {
    const tA = Date.parse(a.momento_registro);
    const tB = Date.parse(b.momento_registro);
    if (tA !== tB) return tA - tB;
    if (a.dispositivo_origen !== b.dispositivo_origen) {
      return a.dispositivo_origen < b.dispositivo_origen ? -1 : 1;
    }
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
  });

  const decimalesMoneda = new Map([
    ["USD", 2],
    ["VES", 2],
    ["EUR", 2],
    ["USDT", 2],
  ]);
  const cuentas = new Map();
  const tasas = new Map();
  let monedaBase = "USD";

  for (const ev of ordenados) {
    const d = ev.datos;
    switch (ev.tipo) {
      case "MonedaCreada": {
        decimalesMoneda.set(d.codigo, Number(d.decimales ?? 2));
        break;
      }
      case "MonedaBaseCambiada": {
        monedaBase = d.codigo;
        break;
      }
      case "TasaRegistrada": {
        tasas.set(d.id, d);
        break;
      }
      case "TasaEliminada": {
        tasas.delete(d.tasa_id);
        break;
      }
      case "CuentaCreada": {
        const saldoInicial = parseDecimal(d.saldo_inicial.cantidad);
        cuentas.set(d.cuenta_id, {
          id: d.cuenta_id,
          nombre: d.nombre,
          tipo: d.tipo,
          moneda: d.saldo_inicial.moneda,
          inicial: saldoInicial,
          ingresos: 0n,
          gastos: 0n,
          ajustes: 0n,
          movimientos: 0,
        });
        break;
      }
      case "MovimientoRegistrado": {
        const cuenta = cuentas.get(d.cuenta_id);
        if (!cuenta) continue;
        const cantidad = parseDecimal(d.monto?.cantidad ?? d.monto);
        const tipoMov = (d.tipo ?? d.tipo_movimiento ?? "").toLowerCase();
        if (tipoMov === "ingreso") {
          cuenta.ingresos += cantidad;
        } else {
          cuenta.gastos += cantidad;
        }
        cuenta.movimientos += 1;
        break;
      }
      case "MovimientoCorregido": {
        const cuenta = cuentas.get(d.cuenta_id);
        if (!cuenta) continue;
        const dif = parseDecimal(d.diferencia ?? d.monto_ajuste);
        cuenta.ajustes += dif;
        cuenta.movimientos += 1;
        break;
      }
      case "GastoDeGrupoAplicado": {
        const cuenta = cuentas.get(d.cuenta_id);
        if (!cuenta) continue;
        const cantidad = parseDecimal(d.monto.cantidad);
        cuenta.gastos += cantidad;
        cuenta.movimientos += 1;
        break;
      }
      case "CategoriaCreada":
      default:
        break;
    }
  }

  const resumenCuentas = [...cuentas.values()].map((c) => {
    const dec = decimalesMoneda.get(c.moneda) ?? 2;
    const saldo = c.inicial + c.ingresos - c.gastos + c.ajustes;
    return {
      id: c.id,
      nombre: c.nombre,
      tipo: c.tipo,
      moneda: c.moneda,
      saldo: formatDecimal(saldo, dec),
      movimientos: c.movimientos,
    };
  });

  return {
    version: encabezado.version,
    fechaExportacion: encabezado.fecha_exportacion,
    cantidadEventos: ordenados.length,
    monedaBase,
    tasasVigentes: tasas.size,
    cuentas: resumenCuentas,
  };
}

// Ejecución directa por CLI
const esEjecucionDirecta =
  process.argv[1] &&
  path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url));

if (esEjecucionDirecta) {
  const ruta = process.argv[2];
  if (!ruta) {
    console.error("Uso: node tools/reconstruir_saldos.mjs <archivo.ndjson>");
    process.exit(1);
  }
  const contenido = fs.readFileSync(ruta, "utf8");
  const resultado = reconstruirDesdeNdjson(contenido);
  console.log(JSON.stringify(resultado, null, 2));
}
