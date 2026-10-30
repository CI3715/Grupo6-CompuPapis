"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import EncabezadoPantalla from "@/app/EncabezadoPantalla";
import type { ErrorDominio, Moneda, PropsPantalla, Tasa } from "@/lib/tipos";
import { formatearFecha, normalizarError } from "@/lib/utilidades";

export default function PantallaTasas({ onVolver, onIr }: PropsPantalla) {
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [tasas, setTasas] = useState<Tasa[]>([]);
  const [error, setError] = useState<ErrorDominio | null>(null);

  const [mostrarFormTasa, setMostrarFormTasa] = useState(false);
  const [origen, setOrigen] = useState("");
  const [destino, setDestino] = useState("");
  const [valor, setValor] = useState("");
  const [fecha, setFecha] = useState("");
  const [fuente, setFuente] = useState("");
  const [errorTasa, setErrorTasa] = useState<ErrorDominio | null>(null);

  const cargarTasas = async () => {
    try {
      setTasas(await invoke<Tasa[]>("listar_tasas"));
    } catch (err: unknown) {
      setError(normalizarError(err));
    }
  };

  // Las monedas y las tasas se cargan por separado: si una falla, la otra sigue funcionando.
  useEffect(() => {
    let activo = true;
    invoke<Moneda[]>("listar_monedas")
      .then((lista) => {
        if (activo) setMonedas(lista);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    invoke<Tasa[]>("listar_tasas")
      .then((lista) => {
        if (activo) setTasas(lista);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    return () => {
      activo = false;
    };
  }, []);

  const abrirFormTasa = async () => {
    try {
      setFecha(await invoke<string>("fecha_local_actual"));
    } catch (err: unknown) {
      setError(normalizarError(err));
      return;
    }
    setOrigen("");
    setDestino("");
    setValor("");
    setFuente("");
    setErrorTasa(null);
    setMostrarFormTasa(true);
  };

  const manejarRegistrarTasa = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorTasa(null);
    try {
      await invoke("registrar_tasa", { origen, destino, valor, fecha, fuente });
      setMostrarFormTasa(false);
      await cargarTasas();
    } catch (err: unknown) {
      setErrorTasa(normalizarError(err));
    }
  };

  const eliminarTasa = async (tasaId: string) => {
    setError(null);
    try {
      await invoke("eliminar_tasa", { tasaId });
      await cargarTasas();
    } catch (err: unknown) {
      setError(normalizarError(err));
    }
  };

  const grupos = tasas.reduce<Record<string, Tasa[]>>((acc, t) => {
    (acc[`${t.origen} → ${t.destino}`] ??= []).push(t);
    return acc;
  }, {});

  return (
    <div className="app-wrapper">
      <main className="content-detalle">
        <EncabezadoPantalla
          titulo="Tasas de cambio"
          etiquetaIr="Patrimonio"
          onVolver={onVolver}
          onIr={onIr}
        />

        {error && (
          <div className="error-alert" role="alert">
            {error.mensaje}
          </div>
        )}

        <div className="acciones-movimientos" style={{ marginBottom: "20px" }}>
          <h3
            className="section-title"
            style={{ margin: 0, alignSelf: "center" }}
          >
            Historial por par ({tasas.length})
          </h3>
          <div style={{ display: "flex", gap: "10px" }}>
            <button
              type="button"
              className="btn btn-primary"
              onClick={abrirFormTasa}
            >
              + Nueva tasa
            </button>
          </div>
        </div>

        {mostrarFormTasa && (
          <section className="form-card">
            <h3 className="form-title">Registrar tasa de cambio</h3>
            <p className="form-subtitle">
              Valor: cuántas unidades de la moneda destino equivalen a 1 unidad
              de la moneda origen. Se guarda únicamente en este dispositivo.
            </p>

            {errorTasa && (
              <div className="error-alert">
                <strong>!</strong>
                <div>
                  <strong>Revisa el campo «{errorTasa.campo}»: </strong>
                  <span>{errorTasa.mensaje}</span>
                </div>
              </div>
            )}

            <form onSubmit={manejarRegistrarTasa}>
              <div className="form-row">
                <div>
                  <label className="form-label">Moneda origen</label>
                  <select
                    value={origen}
                    onChange={(e) => setOrigen(e.target.value)}
                    className={`form-control ${errorTasa?.campo === "origen" ? "input-error" : ""}`}
                  >
                    <option value="" disabled hidden>
                      Seleccione...
                    </option>
                    {monedas.map((m) => (
                      <option key={m.codigo} value={m.codigo}>
                        {m.codigo} ({m.simbolo})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="form-label">Moneda destino</label>
                  <select
                    value={destino}
                    onChange={(e) => setDestino(e.target.value)}
                    className={`form-control ${errorTasa?.campo === "destino" ? "input-error" : ""}`}
                  >
                    <option value="" disabled hidden>
                      Seleccione...
                    </option>
                    {monedas.map((m) => (
                      <option key={m.codigo} value={m.codigo}>
                        {m.codigo} ({m.simbolo})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="form-label">Valor</label>
                  <input
                    type="text"
                    value={valor}
                    onChange={(e) => setValor(e.target.value)}
                    placeholder="0.00"
                    className={`form-control ${errorTasa?.campo === "valor" ? "input-error" : ""}`}
                  />
                </div>
              </div>

              <div className="form-row">
                <div>
                  <label className="form-label">Fecha</label>
                  <input
                    type="date"
                    value={fecha}
                    onChange={(e) => setFecha(e.target.value)}
                    className={`form-control ${errorTasa?.campo === "fecha" ? "input-error" : ""}`}
                  />
                </div>
                <div>
                  <label className="form-label">Fuente</label>
                  <input
                    type="text"
                    value={fuente}
                    onChange={(e) => setFuente(e.target.value)}
                    placeholder="Ej: BCV, casa de cambio, estimación propia"
                    className={`form-control ${errorTasa?.campo === "fuente" ? "input-error" : ""}`}
                  />
                </div>
              </div>

              <div className="form-actions">
                <button
                  type="button"
                  onClick={() => setMostrarFormTasa(false)}
                  className="btn btn-ghost"
                >
                  Cancelar
                </button>
                <button type="submit" className="btn btn-primary">
                  Guardar tasa
                </button>
              </div>
            </form>
          </section>
        )}

        {tasas.length === 0 ? (
          <div className="empty-state">
            <p className="empty-title">Todavía no hay tasas registradas</p>
            <p className="empty-description">
              Registra una tasa con su fecha y fuente para poder consolidar tu
              patrimonio en una sola moneda.
            </p>
          </div>
        ) : (
          Object.entries(grupos).map(([par, lista]) => (
            <section key={par} className="section-block">
              <h2 className="section-title">{par}</h2>
              <div className="accounts-list">
                {lista.map((t) => (
                  <div key={t.id} className="account-card">
                    <div>
                      <div className="account-name-row">
                        <span className="account-name">
                          1 {t.origen} = {t.valor} {t.destino}
                        </span>
                      </div>
                      <p
                        className="account-meta"
                        style={{ textTransform: "none" }}
                      >
                        Fecha: {formatearFecha(t.fecha)} • Fuente: {t.fuente}
                      </p>
                    </div>
                    <button
                      type="button"
                      className="btn btn-danger"
                      style={{ border: "1px solid var(--error-border)" }}
                      onClick={() => eliminarTasa(t.id)}
                    >
                      Eliminar
                    </button>
                  </div>
                ))}
              </div>
            </section>
          ))
        )}
      </main>
    </div>
  );
}
