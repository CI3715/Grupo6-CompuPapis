"use client";

import { dividirGasto, type ReglaDivision } from "@/lib/division";
import { useState } from "react";

type Miembro = {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  es_admin: boolean;
};

export default function VistaGasto({
  miembros,
  moneda,
}: {
  miembros: Miembro[];
  moneda: string;
}) {
  const [monto, setMonto] = useState("");
  const [concepto, setConcepto] = useState("");
  const [fecha, setFecha] = useState("");
  const [pagador, setPagador] = useState("");
  const [regla, setRegla] = useState<ReglaDivision>("iguales");
  const [participantes, setParticipantes] = useState<string[]>([]);
  const [valores, setValores] = useState<Record<string, string>>({});
  let partes: ReturnType<typeof dividirGasto> = [];
  let mensaje = "";
  if (monto || participantes.length) {
    try {
      partes = dividirGasto(
        monto,
        regla,
        miembros
          .filter((m) => participantes.includes(m.id))
          .map((m) => ({
            id: m.id,
            nombre_usuario: m.nombre_usuario,
            valor: valores[m.id] ?? "",
          })),
      );
    } catch (error: unknown) {
      mensaje = error instanceof Error ? error.message : "Revisa la división.";
    }
  }
  return (
    <section className="form-card social-wide">
      <h2 className="form-title">Vista previa de gasto</h2>
      <p className="social-muted">
        Calcula las partes antes de registrar el gasto.
      </p>
      <div className="form-row">
        <div>
          <label className="form-label" htmlFor="concepto-gasto">
            Concepto
          </label>
          <input
            id="concepto-gasto"
            className="form-control"
            value={concepto}
            onChange={(e) => setConcepto(e.target.value)}
          />
        </div>
        <div>
          <label className="form-label" htmlFor="fecha-gasto">
            Fecha
          </label>
          <input
            id="fecha-gasto"
            className="form-control"
            type="date"
            value={fecha}
            onChange={(e) => setFecha(e.target.value)}
          />
        </div>
        <div>
          <label className="form-label" htmlFor="pagador-gasto">
            Quién pagó
          </label>
          <select
            id="pagador-gasto"
            className="form-control"
            value={pagador}
            onChange={(e) => setPagador(e.target.value)}
          >
            <option value="">Selecciona un miembro</option>
            {miembros.map((m) => (
              <option key={m.id} value={m.id}>
                {m.nombre_visible}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="form-label" htmlFor="monto-gasto">
            Monto ({moneda})
          </label>
          <input
            className="form-control"
            id="monto-gasto"
            inputMode="decimal"
            placeholder="0.00"
            value={monto}
            onChange={(e) => setMonto(e.target.value)}
          />
        </div>
        <div>
          <label className="form-label" htmlFor="regla-gasto">
            División
          </label>
          <select
            className="form-control"
            id="regla-gasto"
            value={regla}
            onChange={(e) => setRegla(e.target.value as ReglaDivision)}
          >
            <option value="iguales">Partes iguales</option>
            <option value="porcentajes">Porcentajes</option>
            <option value="montos_fijos">Montos fijos</option>
          </select>
        </div>
      </div>
      <h3>Participantes</h3>
      {miembros.map((m) => (
        <div key={m.id} className="social-row">
          <label>
            <input
              type="checkbox"
              checked={participantes.includes(m.id)}
              onChange={(e) =>
                setParticipantes((prev) =>
                  e.target.checked
                    ? [...prev, m.id]
                    : prev.filter((id) => id !== m.id),
                )
              }
            />{" "}
            {m.nombre_visible}{" "}
            <span className="social-muted">@{m.nombre_usuario}</span>
          </label>
          {regla !== "iguales" && participantes.includes(m.id) && (
            <input
              className="form-control social-value"
              inputMode="decimal"
              aria-label={`${regla === "porcentajes" ? "Porcentaje" : "Monto"} de ${m.nombre_visible}`}
              placeholder="0.00"
              value={valores[m.id] ?? ""}
              onChange={(e) =>
                setValores((prev) => ({ ...prev, [m.id]: e.target.value }))
              }
            />
          )}
        </div>
      ))}
      {mensaje && (
        <p className="social-muted" role="status">
          {mensaje}
        </p>
      )}
      {partes.length > 0 && (
        <div>
          <h3>Partes</h3>
          {partes.map((p) => (
            <div className="social-row" key={p.id}>
              <span>
                @{p.nombre_usuario}{" "}
                {p.residuo && <small>(incluye 0,01 de redondeo)</small>}
              </span>
              <strong>
                {p.monto} {moneda}
              </strong>
            </div>
          ))}
        </div>
      )}
      <p className="social-muted">
        La vista previa aún no guarda gastos. El registro y los balances
        requieren completar el servidor.
      </p>
    </section>
  );
}
