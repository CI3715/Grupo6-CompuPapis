"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

import { formatearMontoLocal } from "@/lib/money";
import type {
  ErrorDominio,
  FilaPatrimonio,
  Moneda,
  Patrimonio,
  PropsPantalla,
  SubtotalPatrimonio,
  TasaUsada,
} from "@/lib/tipos";
import {
  formatearFecha,
  normalizarError,
  obtenerClaseBadgeMoneda,
} from "@/lib/utilidades";

export default function PantallaPatrimonio({ onIr }: PropsPantalla) {
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [fecha, setFecha] = useState("");
  const [version, setVersion] = useState(0);
  const [resultado, setResultado] = useState<{
    fecha: string;
    version: number;
    valor: Patrimonio;
  } | null>(null);
  const [cambiandoBase, setCambiandoBase] = useState(false);
  const patrimonio =
    !cambiandoBase &&
    resultado?.fecha === fecha &&
    resultado.version === version
      ? resultado.valor
      : null;
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
        setResultado({ fecha, version, valor: p });
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
    if (nueva === monedaBase || cambiandoBase) return;
    setCambiandoBase(true);
    try {
      await invoke("cambiar_moneda_base", { codigo: nueva });
      baseElegida.current = true;
      setMonedaBase(nueva);
      setError(null);
      setVersion((v) => v + 1);
    } catch (err: unknown) {
      setError(normalizarError(err));
    } finally {
      setCambiandoBase(false);
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

  // Decimales de la moneda; undefined mientras el catálogo no ha cargado.
  const decimalesDe = (cod: string) => {
    const d = monedas.find((m) => m.codigo === cod)?.decimales;
    return d === undefined ? undefined : Number(d);
  };

  const filas: FilaPatrimonio[] = patrimonio?.filas ?? [];
  const subtotales: SubtotalPatrimonio[] = patrimonio?.subtotales ?? [];

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
    // enBase llega exacto desde el dominio: se redondea solo al mostrarlo.
    return formatearMontoLocal(
      enBase,
      obtenerSimbolo(base),
      false,
      decimalesDe(base),
    );
  };

  return (
    <main className="page-view">
      {/* Tarjeta Hero Total Consolidado estilo Cashea */}
      <section className="hero-card" aria-label="Total consolidado">
        <div className="hero-top">
          <div>
            <span className="hero-eyebrow">Cuentas Claras • Patrimonio</span>
            <h1 className="hero-title">Total consolidado</h1>
            <div className="hero-meta-row">
              <span className="hero-badge">{base}</span>
              <span className="hero-meta-text">
                {fecha ? `Al ${formatearFecha(fecha)}` : "Fecha actual"}
              </span>
            </div>
          </div>
          <div className="hero-balance-block">
            <span className="hero-eyebrow">Patrimonio en {base}</span>
            <p className="hero-amount">
              {consolidado
                ? formatearMontoLocal(
                    patrimonio?.total ?? "0",
                    obtenerSimbolo(base),
                  )
                : "—"}
            </p>
          </div>
        </div>
        <div className="hero-pills">
          <button
            type="button"
            className="btn btn-hero btn-sm"
            onClick={abrirFormMoneda}
          >
            + Nueva moneda
          </button>
          <button type="button" className="btn btn-hero btn-sm" onClick={onIr}>
            Tasas de cambio →
          </button>
        </div>
      </section>

      {error && (
        <div className="error-alert" role="alert">
          {error.mensaje}
        </div>
      )}

      {/* Moneda base, fecha de consulta y monedas propias */}
      <section className="form-card" aria-label="Filtros de patrimonio">
        <div className="form-row form-row-flush">
          <div>
            <label className="form-label" htmlFor="patrimonio-moneda-base">
              Moneda base
            </label>
            <select
              id="patrimonio-moneda-base"
              value={monedaBase}
              disabled={cambiandoBase}
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
            <label className="form-label" htmlFor="patrimonio-fecha">
              Fecha de consulta
            </label>
            <input
              id="patrimonio-fecha"
              type="date"
              value={fecha}
              onChange={(e) => setFecha(e.target.value)}
              className="form-control"
            />
          </div>
        </div>

        {mostrarFormMoneda && (
          <div className="form-divider">
            <h2 className="form-title">Registrar moneda propia</h2>
            <p className="form-subtitle">
              Una vez creada podrás usarla en cuentas nuevas y en tasas.
            </p>

            {errorMoneda && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <div>
                    <strong>Revisa el campo «{errorMoneda.campo}»: </strong>
                    <span>{errorMoneda.mensaje}</span>
                  </div>
                </div>
              </div>
            )}

            <form onSubmit={manejarCrearMoneda}>
              <div className="form-row">
                <div>
                  <label className="form-label" htmlFor="moneda-codigo">
                    Código (3 a 5 letras)
                  </label>
                  <input
                    id="moneda-codigo"
                    type="text"
                    value={codigo}
                    onChange={(e) => setCodigo(e.target.value.toUpperCase())}
                    placeholder="Ej: BTC"
                    className={`form-control ${errorMoneda?.campo === "codigo" ? "input-error" : ""}`}
                  />
                </div>
                <div>
                  <label className="form-label" htmlFor="moneda-nombre">
                    Nombre
                  </label>
                  <input
                    id="moneda-nombre"
                    type="text"
                    value={nombreMoneda}
                    onChange={(e) => setNombreMoneda(e.target.value)}
                    placeholder="Ej: Bitcoin"
                    className={`form-control ${errorMoneda?.campo === "nombre" ? "input-error" : ""}`}
                  />
                </div>
                <div>
                  <label className="form-label" htmlFor="moneda-simbolo">
                    Símbolo
                  </label>
                  <input
                    id="moneda-simbolo"
                    type="text"
                    value={simboloMoneda}
                    onChange={(e) => setSimboloMoneda(e.target.value)}
                    placeholder="Ej: ₿"
                    className={`form-control ${errorMoneda?.campo === "simbolo" ? "input-error" : ""}`}
                  />
                </div>
                <div>
                  <label className="form-label" htmlFor="moneda-decimales">
                    Decimales (0 a 8)
                  </label>
                  <input
                    id="moneda-decimales"
                    type="text"
                    inputMode="numeric"
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
          {error
            ? "No se pudo consultar el patrimonio para esta fecha."
            : "Consultando patrimonio…"}
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
                <p className="account-meta text-normal mt-sm">
                  {textoTasa(sub.codigo, sub.tasa, sub.inversa)}
                </p>
                <p className="subtotal-label mt-xs">
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
              <article key={f.cuentaId} className="account-card">
                <div className="account-card-info">
                  <div className="account-name-row">
                    <h3 className="account-name">{f.nombre}</h3>
                    <span className={obtenerClaseBadgeMoneda(f.moneda)}>
                      {f.moneda}
                    </span>
                  </div>
                  <p className="account-meta text-normal">
                    {textoTasa(f.moneda, f.tasa, f.inversa)}
                  </p>
                </div>
                <div className="patrimonio-balances">
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
              </article>
            ))}
          </div>
        )}
      </section>
    </main>
  );
}
