"use client";

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import EncabezadoPantalla from "@/app/EncabezadoPantalla";
import { formatearMontoLocal } from "@/lib/money";
import type {
  ErrorDominio,
  FilaPatrimonio,
  Moneda,
  Patrimonio,
  PropsPantalla,
  ResumenCuentas,
  SubtotalPatrimonio,
  TasaUsada,
} from "@/lib/tipos";
import {
  formatearFecha,
  normalizarError,
  obtenerClaseBadgeMoneda,
} from "@/lib/utilidades";

export default function PantallaPatrimonio({ onVolver, onIr }: PropsPantalla) {
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [resumen, setResumen] = useState<ResumenCuentas | null>(null);
  const [fecha, setFecha] = useState("");
  const [version, setVersion] = useState(0);
  const [patrimonio, setPatrimonio] = useState<Patrimonio | null>(null);
  const [monedaBase, setMonedaBase] = useState("USD"); // USD por defecto
  const baseElegida = useRef(false); // true cuando el usuario ya eligió una base
  const [error, setError] = useState<ErrorDominio | null>(null);

  const [mostrarFormMoneda, setMostrarFormMoneda] = useState(false);
  const [codigo, setCodigo] = useState("");
  const [nombreMoneda, setNombreMoneda] = useState("");
  const [simboloMoneda, setSimboloMoneda] = useState("");
  const [decimales, setDecimales] = useState("2");
  const [errorMoneda, setErrorMoneda] = useState<ErrorDominio | null>(null);

  // Cada dato se carga por separado: si uno falla, el resto de la pantalla sigue visible.
  useEffect(() => {
    let activo = true;
    invoke<Moneda[]>("listar_monedas")
      .then((lista) => {
        if (activo) setMonedas(lista);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    invoke<ResumenCuentas>("listar_cuentas")
      .then((r) => {
        if (activo) setResumen(r);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    invoke<string>("fecha_local_actual")
      .then((hoy) => {
        if (activo) setFecha(hoy);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    return () => {
      activo = false;
    };
  }, []);

  useEffect(() => {
    if (!fecha) return;
    let activo = true;
    invoke<Patrimonio>("consultar_patrimonio", { fecha })
      .then((p) => {
        if (!activo) return;
        setPatrimonio(p);
        if (!baseElegida.current) setMonedaBase(p.monedaBase);
        setError(null);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    return () => {
      activo = false;
    };
  }, [fecha, version]);

  const cambiarBase = async (nueva: string) => {
    baseElegida.current = true;
    setMonedaBase(nueva);
    try {
      await invoke("cambiar_moneda_base", { codigo: nueva });
      setError(null);
      setVersion((v) => v + 1);
    } catch (err: unknown) {
      setError(normalizarError(err));
    }
  };

  const abrirFormMoneda = () => {
    setCodigo("");
    setNombreMoneda("");
    setSimboloMoneda("");
    setDecimales("2");
    setErrorMoneda(null);
    setMostrarFormMoneda(true);
  };

  const manejarCrearMoneda = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMoneda(null);
    try {
      await invoke("crear_moneda", {
        codigo,
        nombre: nombreMoneda,
        simbolo: simboloMoneda,
        decimales,
      });
      setMostrarFormMoneda(false);
      setMonedas(await invoke<Moneda[]>("listar_monedas"));
    } catch (err: unknown) {
      setErrorMoneda(normalizarError(err));
    }
  };

  // ---- Datos para las tablas: del backend si hay conversión, de listar_cuentas si no ----
  const conversionDisponible = patrimonio !== null;
  const base = patrimonio?.monedaBase ?? monedaBase;

  const obtenerSimbolo = (cod: string) =>
    monedas.find((m) => m.codigo === cod)?.simbolo ?? cod;

  const filas: FilaPatrimonio[] =
    patrimonio?.filas ??
    (resumen?.cuentas ?? []).map((c) => ({
      cuentaId: c.id,
      nombre: c.nombre,
      moneda: c.moneda,
      saldo: c.saldo.cantidad,
      tasa: null,
      inversa: false,
      enBase: null,
    }));

  const subtotales: SubtotalPatrimonio[] =
    patrimonio?.subtotales ?? resumen?.subtotales ?? [];

  const consolidado =
    patrimonio !== null &&
    patrimonio.total !== null &&
    patrimonio.faltantes.length === 0;

  const textoTasa = (
    moneda: string,
    tasa: TasaUsada | null | undefined,
    inversa: boolean | undefined,
  ) => {
    if (moneda === base) return "Moneda base";
    if (!conversionDisponible) return "Pendiente";
    if (!tasa) return "Falta tasa";
    return `1 ${tasa.origen} = ${tasa.valor} ${tasa.destino}${inversa ? " (inversa)" : ""} • ${formatearFecha(tasa.fecha)} • ${tasa.fuente}`;
  };

  const textoEnBase = (
    moneda: string,
    monto: string,
    enBase: string | null | undefined,
  ) => {
    if (moneda === base)
      return formatearMontoLocal(monto, obtenerSimbolo(base));
    if (enBase === null || enBase === undefined) return "—";
    return formatearMontoLocal(enBase, obtenerSimbolo(base));
  };

  return (
    <div className="app-wrapper">
      <main className="content-detalle">
        <EncabezadoPantalla
          titulo="Patrimonio"
          etiquetaIr="Tasas de cambio"
          onVolver={onVolver}
          onIr={onIr}
        />

        {error && (
          <div className="error-alert" role="alert">
            {error.mensaje}
          </div>
        )}

        {/* Moneda base, fecha de consulta y monedas propias */}
        <section className="form-card">
          <div className="form-row" style={{ marginBottom: 0 }}>
            <div>
              <label className="form-label">Moneda base</label>
              <select
                value={monedaBase}
                onChange={(e) => cambiarBase(e.target.value)}
                className="form-control"
              >
                {monedas.map((m) => (
                  <option key={m.codigo} value={m.codigo}>
                    {m.codigo} ({m.simbolo})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="form-label">Fecha de consulta</label>
              <input
                type="date"
                value={fecha}
                onChange={(e) => setFecha(e.target.value)}
                className="form-control"
              />
            </div>
          </div>

          {!mostrarFormMoneda && (
            <div className="form-actions" style={{ marginTop: "16px" }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={abrirFormMoneda}
              >
                + Nueva moneda
              </button>
            </div>
          )}

          {mostrarFormMoneda && (
            <div
              style={{
                marginTop: "20px",
                paddingTop: "20px",
                borderTop: "1px solid var(--border-subtle)",
              }}
            >
              <h3 className="form-title">Registrar moneda propia</h3>
              <p className="form-subtitle">
                Una vez creada podrás usarla en cuentas nuevas y en tasas.
              </p>

              {errorMoneda && (
                <div className="error-alert">
                  <strong>!</strong>
                  <div>
                    <strong>Revisa el campo «{errorMoneda.campo}»: </strong>
                    <span>{errorMoneda.mensaje}</span>
                  </div>
                </div>
              )}

              <form onSubmit={manejarCrearMoneda}>
                <div className="form-row">
                  <div>
                    <label className="form-label">Código (3 a 5 letras)</label>
                    <input
                      type="text"
                      value={codigo}
                      onChange={(e) => setCodigo(e.target.value.toUpperCase())}
                      placeholder="Ej: BTC"
                      className={`form-control ${errorMoneda?.campo === "codigo" ? "input-error" : ""}`}
                    />
                  </div>
                  <div>
                    <label className="form-label">Nombre</label>
                    <input
                      type="text"
                      value={nombreMoneda}
                      onChange={(e) => setNombreMoneda(e.target.value)}
                      placeholder="Ej: Bitcoin"
                      className={`form-control ${errorMoneda?.campo === "nombre" ? "input-error" : ""}`}
                    />
                  </div>
                  <div>
                    <label className="form-label">Símbolo</label>
                    <input
                      type="text"
                      value={simboloMoneda}
                      onChange={(e) => setSimboloMoneda(e.target.value)}
                      placeholder="Ej: ₿"
                      className={`form-control ${errorMoneda?.campo === "simbolo" ? "input-error" : ""}`}
                    />
                  </div>
                  <div>
                    <label className="form-label">Decimales (0 a 8)</label>
                    <input
                      type="text"
                      value={decimales}
                      onChange={(e) => setDecimales(e.target.value)}
                      placeholder="2"
                      className={`form-control ${errorMoneda?.campo === "decimales" ? "input-error" : ""}`}
                    />
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    type="button"
                    onClick={() => setMostrarFormMoneda(false)}
                    className="btn btn-ghost"
                  >
                    Cancelar
                  </button>
                  <button type="submit" className="btn btn-primary">
                    Guardar moneda
                  </button>
                </div>
              </form>
            </div>
          )}
        </section>

        {/* Total consolidado */}
        <div className="detalle-header">
          <div className="detalle-header-top">
            <div className="detalle-info">
              <h2 className="detalle-titulo">Total Consolidado</h2>
              <span className={obtenerClaseBadgeMoneda(base)}>{base}</span>
            </div>
            <div className="detalle-saldo">
              <p className="detalle-subtitulo">
                {fecha ? `Al ${formatearFecha(fecha)}` : "Patrimonio"}
              </p>
              <h2 className="saldo-gigante">
                {consolidado
                  ? formatearMontoLocal(
                      patrimonio?.total ?? "0",
                      obtenerSimbolo(base),
                    )
                  : "—"}
              </h2>
            </div>
          </div>
        </div>

        {patrimonio && !consolidado && (
          <div className="warning-alert" role="alert">
            <strong>No se puede mostrar el total consolidado.</strong>
            {patrimonio.faltantes.map((f) => (
              <p key={`${f.origen}-${f.destino}`}>
                Falta una tasa de {f.origen} a {f.destino} con fecha igual o
                anterior al {formatearFecha(patrimonio.fecha)}.
              </p>
            ))}
          </div>
        )}

        {!patrimonio && (
          <div className="warning-alert" role="status">
            El cálculo de conversión todavía no está disponible. Se muestran los
            saldos; las tasas y los montos en {base} aparecerán cuando responda{" "}
            <strong>consultar_patrimonio</strong>.
          </div>
        )}

        {/* Subtotales por moneda */}
        <section className="form-card">
          <h2 className="section-title">Subtotales por moneda</h2>
          {subtotales.length === 0 ? (
            <div className="empty-state">
              <p className="empty-title">Sin subtotales</p>
            </div>
          ) : (
            <div className="subtotals-grid">
              {subtotales.map((sub) => (
                <div key={sub.codigo} className="subtotal-card">
                  <div className="subtotal-header">
                    <span className="subtotal-label">Subtotal</span>
                    <span className={obtenerClaseBadgeMoneda(sub.codigo)}>
                      {sub.codigo}
                    </span>
                  </div>
                  <div className="subtotal-amount">
                    {formatearMontoLocal(sub.subtotal, sub.simbolo)}
                  </div>
                  <p
                    className="account-meta"
                    style={{ textTransform: "none", marginTop: "8px" }}
                  >
                    {textoTasa(sub.codigo, sub.tasa, sub.inversa)}
                  </p>
                  <p className="subtotal-label" style={{ marginTop: "4px" }}>
                    En {base}:{" "}
                    <strong>
                      {textoEnBase(sub.codigo, sub.subtotal, sub.enBase)}
                    </strong>
                  </p>
                </div>
              ))}
            </div>
          )}
        </section>

        {/* Cuentas */}
        <section className="form-card">
          <h2 className="section-title">Cuentas ({filas.length})</h2>
          {filas.length === 0 ? (
            <div className="empty-state">
              <p className="empty-title">No hay cuentas registradas</p>
            </div>
          ) : (
            <div className="accounts-list">
              {filas.map((f) => (
                <div key={f.cuentaId} className="account-card">
                  <div>
                    <div className="account-name-row">
                      <h3 className="account-name">{f.nombre}</h3>
                      <span className={obtenerClaseBadgeMoneda(f.moneda)}>
                        {f.moneda}
                      </span>
                    </div>
                    <p
                      className="account-meta"
                      style={{ textTransform: "none" }}
                    >
                      {textoTasa(f.moneda, f.tasa, f.inversa)}
                    </p>
                  </div>
                  <div style={{ display: "flex", gap: "24px" }}>
                    <div className="account-balance-box">
                      <span className="account-balance-label">Monto</span>
                      <span className="account-balance-value">
                        {formatearMontoLocal(f.saldo, obtenerSimbolo(f.moneda))}
                      </span>
                    </div>
                    <div className="account-balance-box">
                      <span className="account-balance-label">En {base}</span>
                      <span className="account-balance-value">
                        {textoEnBase(f.moneda, f.saldo, f.enBase)}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}
