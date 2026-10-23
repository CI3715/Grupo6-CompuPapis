import test from "node:test";
import assert from "node:assert/strict";
import { formatearMontoLocal } from "../lib/money.ts";

test("conserva centavos más allá de la precisión de Number", () => {
  assert.equal(
    formatearMontoLocal("9007199254740992.01", "$"),
    "$ 9.007.199.254.740.992,01",
  );
  assert.equal(
    formatearMontoLocal("79228162514264337593543950335", "$"),
    "$ 79.228.162.514.264.337.593.543.950.335,00",
  );
});
test("formato local, signos y cero", () => {
  assert.equal(formatearMontoLocal("1250.50", "Bs."), "Bs. 1.250,50");
  assert.equal(formatearMontoLocal("-50.00", "$"), "- $ 50,00");
  assert.equal(formatearMontoLocal("450.00", "$", true), "+ $ 450,00");
  assert.equal(formatearMontoLocal("-0.00", "USDT", true), "USDT 0,00");
  assert.equal(formatearMontoLocal("0.1", "€"), "€ 0,10");
});
test("no redondea ni interpreta texto decimal inválido", () => {
  for (const valor of ["10.555", "NaN", "1e3", "", "1,25"]) {
    assert.throws(() => formatearMontoLocal(valor, "$"));
  }
});
