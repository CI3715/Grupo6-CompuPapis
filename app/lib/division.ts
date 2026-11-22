export type ReglaDivision = "iguales" | "porcentajes" | "montos_fijos";
export type ParticipanteDivision = {
  id: string;
  nombre_usuario: string;
  valor: string;
};
function centesimas(valor: string): bigint {
  if (!/^\d+(\.\d{1,2})?$/.test(valor))
    throw new Error("Introduce valores no negativos con hasta dos decimales.");
  const [entero, decimal = ""] = valor.split(".");
  return BigInt(entero) * BigInt("100") + BigInt(decimal.padEnd(2, "0"));
}
function mostrar(valor: bigint): string {
  return `${valor / BigInt("100")}.${String(valor % BigInt("100")).padStart(2, "0")}`;
}
export function dividirGasto(
  monto: string,
  regla: ReglaDivision,
  participantes: ParticipanteDivision[],
) {
  const total = centesimas(monto);
  if (total <= BigInt("0") || total > BigInt("999999999999"))
    throw new Error("Introduce un monto positivo dentro del límite permitido.");
  if (!participantes.length)
    throw new Error("Selecciona al menos un participante.");
  const orden = [...participantes].sort((a, b) =>
    a.nombre_usuario < b.nombre_usuario
      ? -1
      : a.nombre_usuario > b.nombre_usuario
        ? 1
        : 0,
  );
  const valores = orden.map((p) =>
    regla === "iguales" ? BigInt("1") : centesimas(p.valor),
  );
  const suma = valores.reduce((a, b) => a + b, BigInt("0"));
  if (regla === "porcentajes" && suma !== BigInt("10000"))
    throw new Error(
      `Los porcentajes suman ${mostrar(suma)}; ${suma < BigInt("10000") ? "faltan" : "sobran"} ${mostrar(suma < BigInt("10000") ? BigInt("10000") - suma : suma - BigInt("10000"))}.`,
    );
  if (regla === "montos_fijos" && suma !== total)
    throw new Error(
      `Los montos suman ${mostrar(suma)}; ${suma < total ? "faltan" : "sobran"} ${mostrar(suma < total ? total - suma : suma - total)}.`,
    );
  const partes = valores.map((valor) =>
    regla === "montos_fijos"
      ? valor
      : (total * valor) /
        (regla === "iguales" ? BigInt(orden.length) : BigInt("10000")),
  );
  let residuo = total - partes.reduce((a, b) => a + b, BigInt("0"));
  return orden.map((p, i) => {
    const extra = residuo > BigInt("0");
    if (extra) residuo--;
    return {
      ...p,
      monto: mostrar(partes[i] + (extra ? BigInt("1") : BigInt("0"))),
      residuo: extra,
    };
  });
}
