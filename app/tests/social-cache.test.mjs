import test from "node:test";
import assert from "node:assert/strict";
import {
  claveCacheGrupos,
  guardarCacheSocial,
  leerCacheSocial,
} from "../lib/social-cache.ts";

test("la copia de grupos y detalles está separada por usuario", () => {
  const datos = new Map();
  const almacen = {
    setItem: (k, v) => datos.set(k, v),
    getItem: (k) => datos.get(k) ?? null,
  };
  guardarCacheSocial(() => almacen, claveCacheGrupos("ana"), {
    grupos: [{ id: "viaje" }],
  });
  guardarCacheSocial(() => almacen, claveCacheGrupos("ana", "viaje"), {
    miembros: ["ana", "beto"],
  });
  assert.deepEqual(
    leerCacheSocial(() => almacen, claveCacheGrupos("ana")),
    { grupos: [{ id: "viaje" }] },
  );
  assert.deepEqual(
    leerCacheSocial(() => almacen, claveCacheGrupos("ana", "viaje")),
    { miembros: ["ana", "beto"] },
  );
  assert.equal(
    leerCacheSocial(() => almacen, claveCacheGrupos("beto")),
    null,
  );
  assert.equal(
    leerCacheSocial(() => almacen, claveCacheGrupos("beto", "viaje")),
    null,
  );
});

test("una copia ilegible o almacenamiento inaccesible no rompe la pantalla", () => {
  const ilegible = {
    getItem: () => "{",
    setItem: () => {
      throw Error("sin espacio");
    },
  };
  assert.equal(
    leerCacheSocial(() => ilegible, "x"),
    null,
  );
  assert.doesNotThrow(() => guardarCacheSocial(() => ilegible, "x", {}));
  assert.equal(
    leerCacheSocial(() => {
      throw Error("sin acceso");
    }, "x"),
    null,
  );
});
