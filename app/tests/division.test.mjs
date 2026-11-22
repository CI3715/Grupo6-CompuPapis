import assert from "node:assert/strict";
import { dividirGasto } from "../lib/division.ts";
const participantes = ["carla", "ana", "beto"].map((nombre_usuario) => ({
  id: nombre_usuario,
  nombre_usuario,
  valor: "",
}));
const partes = dividirGasto("100.00", "iguales", participantes);
assert.deepEqual(
  partes.map((p) => p.monto),
  ["33.34", "33.33", "33.33"],
);
assert.equal(partes[0].residuo, true);
const porcentajes = participantes.map((p) => ({
  ...p,
  valor: p.id === "carla" ? "33.34" : "33.33",
}));
assert.deepEqual(
  dividirGasto("10.00", "porcentajes", porcentajes).map((p) => p.monto),
  ["3.34", "3.33", "3.33"],
);
assert.throws(
  () =>
    dividirGasto("10", "porcentajes", [{ ...participantes[0], valor: "95" }]),
  /faltan 5.00/,
);
assert.throws(
  () =>
    dividirGasto("10", "montos_fijos", [{ ...participantes[0], valor: "11" }]),
  /sobran 1.00/,
);
assert.equal(
  dividirGasto("9999999999.99", "iguales", [participantes[0]])[0].monto,
  "9999999999.99",
);
console.log("Vista previa de división: pruebas aprobadas.");
