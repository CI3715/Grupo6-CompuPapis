"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core"; // Si usas Tauri v1: "@tauri-apps/api/tauri"

interface Moneda {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
}

interface Cuenta {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  saldo: {
    cantidad: string;
    moneda: string;
  };
}

interface SubtotalMoneda {
  codigo: string;
  simbolo: string;
  subtotal: string;
}

interface ResumenCuentas {
  cuentas: Cuenta[];
  subtotales: SubtotalMoneda[];
}

interface ErrorDominio {
  campo: string;
  mensaje: string;
}

function formatearMontoLocal(cantidadStr: string, simbolo: string): string {
  const num = Number(cantidadStr);
  const formateado = new Intl.NumberFormat("es-VE", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
  return `${simbolo} ${formateado}`;
}

function obtenerClaseBadgeMoneda(codigo: string): string {
  switch (codigo.toUpperCase()) {
    case "USD":
      return "badge badge-usd";
    case "VES":
      return "badge badge-ves";
    case "EUR":
      return "badge badge-eur";
    case "USDT":
      return "badge badge-usdt";
    default:
      return "badge";
  }
}

export default function MisCuentasPage() {
  const [resumen, setResumen] = useState<ResumenCuentas>({
    cuentas: [],
    subtotales: [],
  });
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [mostrarFormulario, setMostrarFormulario] = useState(false);

  const [nombre, setNombre] = useState("");
  const [tipo, setTipo] = useState("efectivo");
  const [moneda, setMoneda] = useState("VES");
  const [saldoInicial, setSaldoInicial] = useState("0.00");
  const [error, setError] = useState<ErrorDominio | null>(null);

  const cargarDatos = async () => {
    try {
      const [resCuentas, resMonedas] = await Promise.all([
        invoke<ResumenCuentas>("listar_cuentas"),
        invoke<Moneda[]>("listar_monedas"),
      ]);
      setResumen(resCuentas);
      setMonedas(resMonedas);
    } catch (err) {
      setError({
        campo: "general",
        mensaje: "No se pudieron cargar las cuentas desde el almacenamiento local.",
      });
    }
  };

  useEffect(() => {
    cargarDatos();
  }, []);

  const obtenerSimbolo = (codigo: string) =>
    monedas.find((m) => m.codigo === codigo)?.simbolo ?? codigo;

  const manejarCrearCuenta = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await invoke("crear_cuenta", {
        nombre,
        tipo,
        moneda,
        saldoInicial,
      });
      setNombre("");
      setSaldoInicial("0.00");
      setMostrarFormulario(false);
      await cargarDatos();
    } catch (err: any) {
      setError(err as ErrorDominio);
    }
  };

  return (
    <main className="app-container">
      {/* Encabezado Principal */}
      <header className="header">
        <div>
          <span className="header-tag">Cuentas Claras • Local-First</span>
          <h1 className="header-title">Mis cuentas</h1>
        </div>

        <button
          onClick={() => {
            setError(null);
            setMostrarFormulario(!mostrarFormulario);
          }}
          className={`btn ${mostrarFormulario ? "btn-secondary" : "btn-primary"}`}
        >
          {mostrarFormulario ? "Cancelar" : "+ Nueva cuenta"}
        </button>
      </header>

      {/* Formulario de Creación de Cuenta */}
      {mostrarFormulario && (
        <section className="form-card">
          <h2 className="form-title">Registrar nueva cuenta</h2>
          <p className="form-subtitle">
            Los datos se guardan únicamente en este dispositivo. La moneda no podrá cambiarse después de creada.
          </p>

          {error && (
            <div className="error-alert">
              <strong>!</strong>
              <div>
                <strong>Revisa el campo «{error.campo}»: </strong>
                <span>{error.mensaje}</span>
              </div>
            </div>
          )}

          <form onSubmit={manejarCrearCuenta}>
            <div className="form-group">
              <label className="form-label">Nombre de la cuenta</label>
              <input
                type="text"
                value={nombre}
                onChange={(e) => setNombre(e.target.value)}
                placeholder="Ej: Cartera, Cuenta nómina, Billetera dólares"
                className={`form-control ${
                  error?.campo === "nombre" ? "input-error" : ""
                }`}
              />
            </div>

            <div className="form-row">
              <div>
                <label className="form-label">Tipo de cuenta</label>
                <select
                  value={tipo}
                  onChange={(e) => setTipo(e.target.value)}
                  className={`form-control ${
                    error?.campo === "tipo" ? "input-error" : ""
                  }`}
                >
                  <option value="efectivo">Efectivo</option>
                  <option value="banco">Banco</option>
                  <option value="billetera digital">Billetera digital</option>
                </select>
              </div>

              <div>
                <label className="form-label">Moneda</label>
                <select
                  value={moneda}
                  onChange={(e) => setMoneda(e.target.value)}
                  className={`form-control ${
                    error?.campo === "moneda" ? "input-error" : ""
                  }`}
                >
                  {monedas.map((m) => (
                    <option key={m.codigo} value={m.codigo}>
                      {m.codigo} — {m.nombre} ({m.simbolo})
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="form-label">Saldo inicial</label>
                <input
                  type="text"
                  value={saldoInicial}
                  onChange={(e) => setSaldoInicial(e.target.value)}
                  placeholder="0.00"
                  className={`form-control ${
                    error?.campo === "saldo_inicial" ? "input-error" : ""
                  }`}
                />
              </div>
            </div>

            <div className="form-actions">
              <button
                type="button"
                onClick={() => {
                  setError(null);
                  setMostrarFormulario(false);
                }}
                className="btn btn-ghost"
              >
                Cancelar
              </button>
              <button type="submit" className="btn btn-primary">
                Guardar cuenta
              </button>
            </div>
          </form>
        </section>
      )}

      {/* Subtotales por Moneda */}
      {resumen.subtotales.length > 0 && (
        <section className="section-block">
          <h2 className="section-title">Subtotales por moneda</h2>
          <div className="subtotals-grid">
            {resumen.subtotales.map((sub) => (
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
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Listado de Cuentas o Estado Vacio */}
      <section className="section-block">
        <h2 className="section-title">
          Cuentas registradas ({resumen.cuentas.length})
        </h2>

        {resumen.cuentas.length === 0 ? (
          <div className="empty-state">
            <p className="empty-title">Todavía no tienes cuentas registradas</p>
            <p className="empty-description">
              Crea tu primera cuenta de efectivo, banco o billetera digital para empezar a llevar el control en este dispositivo.
            </p>
            <button
              onClick={() => setMostrarFormulario(true)}
              className="btn btn-primary"
            >
              Crear mi primera cuenta
            </button>
          </div>
        ) : (
          <div className="accounts-list">
            {resumen.cuentas.map((c) => (
              <div key={c.id} className="account-card">
                <div>
                  <div className="account-name-row">
                    <h3 className="account-name">{c.nombre}</h3>
                    <span className={obtenerClaseBadgeMoneda(c.moneda)}>
                      {c.moneda}
                    </span>
                  </div>
                  <p className="account-meta">Tipo: {c.tipo}</p>
                </div>

                <div className="account-balance-box">
                  <span className="account-balance-label">Saldo actual</span>
                  <span className="account-balance-value">
                    {formatearMontoLocal(
                      c.saldo.cantidad,
                      obtenerSimbolo(c.moneda)
                    )}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </main>
  );
}