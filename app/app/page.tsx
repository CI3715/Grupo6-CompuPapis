"use client";

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

import { formatearMontoLocal } from "@/lib/money";

// --- INTERFACES ---
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

interface Categoria {
  nombre: string;
  tipo: "ingreso" | "gasto";
}

interface Movimiento {
  id: string;
  fecha: string;
  tipo: "ingreso" | "gasto" | "ajuste";
  categoria: string;
  nota?: string;
  monto: string;
  montoVigente: string;
  corregido: boolean;
  esAjuste: boolean;
  referenciaId?: string;
}

interface DetalleCuenta {
  saldoActual: string;
  composicion: {
    inicial: string;
    ingresos: string;
    gastos: string;
    ajustes: string;
  };
  movimientos: Movimiento[];
}

// --- FUNCIONES UTILERÍA ---
interface EvaluacionMovimiento {
  dejaSaldoNegativo: boolean;
  saldoResultante: string;
}

function normalizarError(error: unknown): ErrorDominio {
  if (
    typeof error === "object" &&
    error !== null &&
    "campo" in error &&
    "mensaje" in error &&
    typeof error.campo === "string" &&
    typeof error.mensaje === "string"
  ) {
    return { campo: error.campo, mensaje: error.mensaje };
  }
  return {
    campo: "general",
    mensaje:
      "No se pudo completar la operación. Abre la aplicación con Tauri e inténtalo de nuevo.",
  };
}

async function leerDatos() {
  const [resumen, monedas, categorias] = await Promise.all([
    invoke<ResumenCuentas>("listar_cuentas"),
    invoke<Moneda[]>("listar_monedas"),
    invoke<Categoria[]>("listar_categorias"),
  ]);
  return { resumen, monedas, categorias };
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
  // --- ESTADOS CUENTAS ---
  const [resumen, setResumen] = useState<ResumenCuentas>({
    cuentas: [],
    subtotales: [],
  });
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [mostrarFormulario, setMostrarFormulario] = useState(false);

  const [nombre, setNombre] = useState("");
  const [tipo, setTipo] = useState("efectivo");
  const [moneda, setMoneda] = useState("");
  const [saldoInicial, setSaldoInicial] = useState("0.00");
  const [error, setError] = useState<ErrorDominio | null>(null);

  // --- ESTADOS DETALLE Y MOVIMIENTOS (ENTREGA 3) ---
  const [cuentaSeleccionadaId, setCuentaSeleccionadaId] = useState<
    string | null
  >(null);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuenta | null>(
    null,
  );

  const [mostrarFormMovimiento, setMostrarFormMovimiento] = useState(false);
  const [tipoMovimiento, setTipoMovimiento] = useState<"gasto" | "ingreso">(
    "gasto",
  );
  const [montoMovimiento, setMontoMovimiento] = useState("");
  const [fechaMovimiento, setFechaMovimiento] = useState("");
  const [categoriaMovimiento, setCategoriaMovimiento] = useState("");
  const [notaMovimiento, setNotaMovimiento] = useState("");
  const [errorMovimiento, setErrorMovimiento] = useState<ErrorDominio | null>(
    null,
  );
  const [advertenciaSaldoNegativo, setAdvertenciaSaldoNegativo] =
    useState(false);

  // Nueva categoría personalizada
  const [mostrarNuevaCategoria, setMostrarNuevaCategoria] = useState(false);
  const [nombreNuevaCategoria, setNombreNuevaCategoria] = useState("");

  // Estado para corregir movimiento (RF-LIB-06)
  const [movimientoACorregir, setMovimientoACorregir] =
    useState<Movimiento | null>(null);
  const [montoCorrecto, setMontoCorrecto] = useState("");
  const [errorCorreccion, setErrorCorreccion] = useState<ErrorDominio | null>(
    null,
  );

  const operacionEnCurso = useRef(false);
  const solicitudDetalle = useRef(0);
  const [guardando, setGuardando] = useState(false);
  const [estadoServidor, setEstadoServidor] = useState("");
  const [probandoServidor, setProbandoServidor] = useState(false);

  const probarServidor = async () => {
    setProbandoServidor(true);
    try {
      const ping = await invoke<{
        estado: string;
        mensaje: string;
        version: string;
        timestamp: string;
        latencia_ms: number;
      }>("ping_servidor");
      setEstadoServidor(
        `${ping.estado}: ${ping.mensaje} · v${ping.version} · ${ping.timestamp} · ${ping.latencia_ms} ms`,
      );
    } catch {
      setEstadoServidor(
        "Servidor no disponible. Puedes seguir usando tus cuentas sin conexión.",
      );
    } finally {
      setProbandoServidor(false);
    }
  };

  // --- CARGA INICIAL ---
  const cargarDatos = async () => {
    try {
      const datos = await leerDatos();
      setResumen(datos.resumen);
      setMonedas(datos.monedas);
      setCategorias(datos.categorias);
    } catch (err: unknown) {
      setError(normalizarError(err));
    }
  };

  useEffect(() => {
    let activo = true;
    leerDatos()
      .then((datos) => {
        if (!activo) return;
        setResumen(datos.resumen);
        setMonedas(datos.monedas);
        setCategorias(datos.categorias);
      })
      .catch((err: unknown) => {
        if (activo) setError(normalizarError(err));
      });
    return () => {
      activo = false;
    };
  }, []);

  // Redimensionar ventana Tauri dinámicamente
  useEffect(() => {
    const ajustarTamanoVentana = async () => {
      try {
        const numCuentas = resumen.cuentas.length;
        const anchoBase = cuentaSeleccionadaId ? 1120 : 620;
        const altoBase = 560;
        const crecimientoAlto = 80;
        const maximoAlto = 900;
        const nuevoAlto = Math.min(
          altoBase + numCuentas * crecimientoAlto,
          maximoAlto,
        );

        const appWindow = getCurrentWindow();
        await appWindow.setSize(new LogicalSize(anchoBase, nuevoAlto));
      } catch (err) {
        console.error("Error al redimensionar la ventana:", err);
      }
    };

    ajustarTamanoVentana();
  }, [resumen.cuentas.length, cuentaSeleccionadaId]);

  const obtenerSimbolo = (codigo: string) =>
    monedas.find((m) => m.codigo === codigo)?.simbolo ?? codigo;

  // --- MANEJADORES DE CUENTAS ---
  const manejarCrearCuenta = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await invoke("crear_cuenta", { nombre, tipo, moneda, saldoInicial });
      setNombre("");
      setSaldoInicial("0.00");
      setMoneda("");
      setMostrarFormulario(false);
      await cargarDatos();
    } catch (err: unknown) {
      setError(normalizarError(err));
    }
  };

  const seleccionarCuenta = async (cuentaId: string) => {
    if (operacionEnCurso.current) return;
    const solicitud = ++solicitudDetalle.current;
    setCuentaSeleccionadaId(cuentaId);
    setDetalleCuenta(null);
    setMostrarFormulario(false);
    setMostrarFormMovimiento(false);
    setMovimientoACorregir(null);
    setError(null);
    setErrorMovimiento(null);

    try {
      const det = await invoke<DetalleCuenta>("obtener_detalle_cuenta", {
        cuentaId,
      });
      if (solicitud === solicitudDetalle.current) setDetalleCuenta(det);
    } catch (err: unknown) {
      if (solicitud === solicitudDetalle.current)
        setErrorMovimiento(normalizarError(err));
    }
  };

  // --- MANEJADORES DE MOVIMIENTOS Y CATEGORÍAS ---
  const abrirFormularioMovimiento = async (tipoMov: "gasto" | "ingreso") => {
    try {
      setFechaMovimiento(await invoke<string>("fecha_local_actual"));
    } catch (err: unknown) {
      setErrorMovimiento(normalizarError(err));
      return;
    }
    setTipoMovimiento(tipoMov);
    setMontoMovimiento("");

    const primeraCat = categorias.find((c) => c.tipo === tipoMov)?.nombre ?? "";
    setCategoriaMovimiento(primeraCat);
    setNotaMovimiento("");
    setErrorMovimiento(null);
    setAdvertenciaSaldoNegativo(false);
    setMovimientoACorregir(null);
    setMostrarFormMovimiento(true);
  };

  const manejarCrearCategoria = async () => {
    setErrorMovimiento(null);
    try {
      const nueva = await invoke<Categoria>("crear_categoria", {
        nombre: nombreNuevaCategoria,
        tipo: tipoMovimiento,
      });
      const resCategorias = await invoke<Categoria[]>("listar_categorias");
      setCategorias(resCategorias);
      setCategoriaMovimiento(nueva.nombre);
      setNombreNuevaCategoria("");
      setMostrarNuevaCategoria(false);
    } catch (err: unknown) {
      setErrorMovimiento(normalizarError(err));
    }
  };

  const manejarRegistrarMovimiento = async (
    confirmadoSaldoNegativo = false,
  ) => {
    if (operacionEnCurso.current) return;
    if (!cuentaSeleccionadaId || !detalleCuenta) return;
    setErrorMovimiento(null);

    operacionEnCurso.current = true;
    setGuardando(true);
    try {
      const evaluacion = await invoke<EvaluacionMovimiento>(
        "evaluar_movimiento",
        {
          cuentaId: cuentaSeleccionadaId,
          tipo: tipoMovimiento,
          monto: montoMovimiento,
        },
      );
      if (evaluacion.dejaSaldoNegativo && !confirmadoSaldoNegativo) {
        setAdvertenciaSaldoNegativo(true);
        return;
      }
      const nuevoDetalle = await invoke<DetalleCuenta>("registrar_movimiento", {
        cuentaId: cuentaSeleccionadaId,
        tipo: tipoMovimiento,
        monto: montoMovimiento,
        fecha: fechaMovimiento,
        categoria: categoriaMovimiento,
        nota: notaMovimiento || null,
      });
      setDetalleCuenta(nuevoDetalle);
      setMostrarFormMovimiento(false);
      setAdvertenciaSaldoNegativo(false);
      await cargarDatos();
    } catch (err: unknown) {
      setErrorMovimiento(normalizarError(err));
    } finally {
      operacionEnCurso.current = false;
      setGuardando(false);
    }
  };

  // --- MANEJADOR DE CORRECCIÓN (RF-LIB-06) ---
  const iniciarCorreccion = (mov: Movimiento) => {
    setMostrarFormMovimiento(false);
    setErrorCorreccion(null);
    setMovimientoACorregir(mov);
    setMontoCorrecto(mov.montoVigente);
  };

  const manejarCorregirMovimiento = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!cuentaSeleccionadaId || !movimientoACorregir) return;
    setErrorCorreccion(null);

    try {
      const nuevoDetalle = await invoke<DetalleCuenta>("corregir_movimiento", {
        cuentaId: cuentaSeleccionadaId,
        movimientoId: movimientoACorregir.id,
        montoCorrecto,
      });
      setDetalleCuenta(nuevoDetalle);
      setMovimientoACorregir(null);
      setMontoCorrecto("");
      await cargarDatos();
    } catch (err: unknown) {
      setErrorCorreccion(normalizarError(err));
    }
  };

  const cuentaSeleccionada = resumen.cuentas.find(
    (c) => c.id === cuentaSeleccionadaId,
  );
  const categoriasFiltradas = categorias.filter(
    (c) => c.tipo === tipoMovimiento,
  );

  // --- RENDER ---
  return (
    <div
      className={`app-wrapper ${cuentaSeleccionadaId ? "layout-split" : "layout-single"}`}
    >
      {/* PANEL IZQUIERDO: LISTA DE CUENTAS Y FORMULARIO */}
      <aside className="sidebar-cuentas">
        <header className="header">
          <div>
            <span className="header-tag">Cuentas Claras • Local-First</span>
            <h1 className="header-title">Mis cuentas</h1>
          </div>
          {!cuentaSeleccionadaId && (
            <button
              onClick={() => {
                setError(null);
                setMostrarFormulario(!mostrarFormulario);
              }}
              className={`btn ${mostrarFormulario ? "btn-secondary" : "btn-primary"}`}
            >
              {mostrarFormulario ? "Cancelar" : "+ Nueva cuenta"}
            </button>
          )}
        </header>

        <details>
          <summary>Conexión con el servidor · Entrega 1</summary>
          <button
            type="button"
            className="btn btn-ghost"
            disabled={probandoServidor}
            onClick={probarServidor}
          >
            {probandoServidor ? "Comprobando…" : "Probar conexión"}
          </button>
          {estadoServidor && <p role="status">{estadoServidor}</p>}
        </details>

        {error && !mostrarFormulario && (
          <div className="error-alert" role="alert">
            {error.mensaje}
          </div>
        )}
        {errorMovimiento && !mostrarFormMovimiento && (
          <div className="error-alert" role="alert">
            Revisa el campo «{errorMovimiento.campo}»: {errorMovimiento.mensaje}
          </div>
        )}

        {mostrarFormulario && (
          <section className="form-card">
            <h2 className="form-title">Registrar nueva cuenta</h2>
            <p className="form-subtitle">
              Los datos se guardan únicamente en este dispositivo. La moneda no
              podrá cambiarse después de creada.
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
                  className={`form-control ${error?.campo === "nombre" ? "input-error" : ""}`}
                />
              </div>

              <div className="form-row">
                <div>
                  <label className="form-label">Tipo de cuenta</label>
                  <select
                    value={tipo}
                    onChange={(e) => setTipo(e.target.value)}
                    className={`form-control ${error?.campo === "tipo" ? "input-error" : ""}`}
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
                    className={`form-control ${error?.campo === "moneda" ? "input-error" : ""}`}
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
                  <label className="form-label">Saldo inicial</label>
                  <input
                    type="text"
                    value={saldoInicial}
                    onChange={(e) => setSaldoInicial(e.target.value)}
                    placeholder="0.00"
                    className={`form-control ${error?.campo === "saldo_inicial" ? "input-error" : ""}`}
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

        <section className="section-block">
          <h2 className="section-title">
            Cuentas registradas ({resumen.cuentas.length})
          </h2>

          {resumen.cuentas.length === 0 ? (
            <div className="empty-state">
              <p className="empty-title">
                Todavía no tienes cuentas registradas
              </p>
              <p className="empty-description">
                Crea tu primera cuenta de efectivo, banco o billetera digital
                para empezar a llevar el control en este dispositivo.
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
                <div
                  key={c.id}
                  className={`account-card ${c.id === cuentaSeleccionadaId ? "account-card-active" : ""}`}
                  onClick={() => seleccionarCuenta(c.id)}
                  style={{ cursor: "pointer" }}
                >
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
                        obtenerSimbolo(c.moneda),
                      )}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </aside>

      {/* PANEL DERECHO: DETALLE DE LA CUENTA Y MOVIMIENTOS */}
      {cuentaSeleccionadaId && cuentaSeleccionada && detalleCuenta && (
        <main className="content-detalle">
          <button
            className="btn btn-ghost"
            onClick={() => setCuentaSeleccionadaId(null)}
          >
            <span style={{ fontSize: "24px", lineHeight: "0.8" }}>
              ← Volver
            </span>
          </button>

          <header className="detalle-header">
            <div className="detalle-header-top">
              <div className="detalle-info">
                <h2 className="detalle-titulo">{cuentaSeleccionada.nombre}</h2>
                <div
                  style={{ display: "flex", gap: "8px", alignItems: "center" }}
                >
                  <span
                    className={obtenerClaseBadgeMoneda(
                      cuentaSeleccionada.moneda,
                    )}
                  >
                    {cuentaSeleccionada.moneda}
                  </span>
                  <span className="account-meta">
                    ({cuentaSeleccionada.tipo})
                  </span>
                </div>
              </div>

              <div className="detalle-saldo">
                <p className="detalle-subtitulo">Saldo Actual</p>
                <h1 className="saldo-gigante">
                  {formatearMontoLocal(
                    detalleCuenta.saldoActual,
                    obtenerSimbolo(cuentaSeleccionada.moneda),
                  )}
                </h1>
              </div>
            </div>
          </header>

          {/* Composición del saldo (Sección 2.3) */}
          <section className="section-block">
            <h3 className="section-title">Composición del saldo</h3>
            <div className="subtotals-grid">
              <div className="subtotal-card">
                <span className="subtotal-label">Saldo inicial</span>
                <div className="subtotal-amount">
                  {formatearMontoLocal(
                    detalleCuenta.composicion.inicial,
                    obtenerSimbolo(cuentaSeleccionada.moneda),
                  )}
                </div>
              </div>
              <div className="subtotal-card">
                <span className="subtotal-label">Total ingresos</span>
                <div
                  className="subtotal-amount"
                  style={{ color: "var(--text-success)" }}
                >
                  {formatearMontoLocal(
                    detalleCuenta.composicion.ingresos,
                    obtenerSimbolo(cuentaSeleccionada.moneda),
                    true,
                  )}
                </div>
              </div>
              <div className="subtotal-card">
                <span className="subtotal-label">Total gastos</span>
                <div
                  className="subtotal-amount"
                  style={{ color: "var(--text-danger)" }}
                >
                  {formatearMontoLocal(
                    `-${detalleCuenta.composicion.gastos}`,
                    obtenerSimbolo(cuentaSeleccionada.moneda),
                  )}
                </div>
              </div>
              <div className="subtotal-card">
                <span className="subtotal-label">Total ajustes</span>
                <div
                  className="subtotal-amount"
                  style={{ color: "var(--text-ajust)" }}
                >
                  {formatearMontoLocal(
                    detalleCuenta.composicion.ajustes,
                    obtenerSimbolo(cuentaSeleccionada.moneda),
                    true,
                  )}
                </div>
              </div>
            </div>
          </section>

          {/* Botones de Acción de Movimientos */}
          <div
            className="acciones-movimientos"
            style={{ marginBottom: "20px" }}
          >
            <h3
              className="section-title"
              style={{ margin: 0, alignSelf: "center" }}
            >
              Historial de Movimientos ({detalleCuenta.movimientos.length})
            </h3>
            <div style={{ display: "flex", gap: "10px" }}>
              <button
                type="button"
                onClick={() => abrirFormularioMovimiento("gasto")}
                className="btn btn-danger"
                style={{ border: "1px solid var(--error-border)" }}
              >
                - Registrar Gasto
              </button>
              <button
                type="button"
                onClick={() => abrirFormularioMovimiento("ingreso")}
                className="btn btn-success"
                style={{ border: "1px solid var(--text-success)" }}
              >
                + Registrar Ingreso
              </button>
            </div>
          </div>

          {/* Formulario Registrar Gasto / Ingreso */}
          {mostrarFormMovimiento && (
            <section className="form-card">
              <h3 className="form-title">
                Registrar {tipoMovimiento === "gasto" ? "Gasto" : "Ingreso"} en{" "}
                {cuentaSeleccionada.moneda}
              </h3>
              <p className="form-subtitle">
                La moneda del movimiento es siempre la de la cuenta (
                {cuentaSeleccionada.moneda}).
              </p>

              {errorMovimiento && (
                <div className="error-alert">
                  <strong>!</strong>
                  <div>
                    <strong>Revisa el campo «{errorMovimiento.campo}»: </strong>
                    <span>{errorMovimiento.mensaje}</span>
                  </div>
                </div>
              )}

              {/* Advertencia de Saldo Negativo (RF-LIB-04) */}
              {advertenciaSaldoNegativo && (
                <div className="warning-alert">
                  <div>
                    <strong>Advertencia de saldo negativo: </strong>
                    <span>
                      Este gasto dejará el saldo de la cuenta en negativo.
                      ¿Deseas confirmarlo de todas formas?
                    </span>
                  </div>
                  <div
                    style={{ display: "flex", gap: "8px", marginTop: "10px" }}
                  >
                    <button
                      type="button"
                      className="btn btn-primary"
                      disabled={guardando}
                      onClick={() => manejarRegistrarMovimiento(true)}
                    >
                      Sí, confirmar gasto
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => setAdvertenciaSaldoNegativo(false)}
                    >
                      Cancelar
                    </button>
                  </div>
                </div>
              )}

              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  void manejarRegistrarMovimiento();
                }}
              >
                <div className="form-row">
                  <div>
                    <label className="form-label">Tipo</label>
                    <select
                      value={tipoMovimiento}
                      onChange={(e) => {
                        const nuevoTipo = e.target.value as "gasto" | "ingreso";
                        setTipoMovimiento(nuevoTipo);
                        const primera =
                          categorias.find((c) => c.tipo === nuevoTipo)
                            ?.nombre ?? "";
                        setCategoriaMovimiento(primera);
                        setAdvertenciaSaldoNegativo(false);
                      }}
                      className="form-control"
                    >
                      <option value="gasto">Gasto</option>
                      <option value="ingreso">Ingreso</option>
                    </select>
                  </div>

                  <div>
                    <label className="form-label">
                      Monto ({cuentaSeleccionada.moneda})
                    </label>
                    <input
                      type="text"
                      value={montoMovimiento}
                      onChange={(e) => {
                        setMontoMovimiento(e.target.value);
                        setAdvertenciaSaldoNegativo(false);
                      }}
                      placeholder="0.00"
                      className={`form-control ${errorMovimiento?.campo === "monto" ? "input-error" : ""}`}
                    />
                  </div>

                  <div>
                    <label className="form-label">Fecha</label>
                    <input
                      type="date"
                      value={fechaMovimiento}
                      onChange={(e) => setFechaMovimiento(e.target.value)}
                      className={`form-control ${errorMovimiento?.campo === "fecha" ? "input-error" : ""}`}
                    />
                  </div>
                </div>

                <div className="form-row">
                  <div>
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <label className="form-label">Categoría</label>
                      <button
                        type="button"
                        onClick={() =>
                          setMostrarNuevaCategoria(!mostrarNuevaCategoria)
                        }
                        style={{
                          background: "none",
                          border: "none",
                          color: "var(--brand-primary)",
                          fontSize: "12px",
                          fontWeight: 600,
                          cursor: "pointer",
                        }}
                      >
                        {mostrarNuevaCategoria ? "Cerrar" : "+ Nueva categoría"}
                      </button>
                    </div>
                    <select
                      value={categoriaMovimiento}
                      onChange={(e) => setCategoriaMovimiento(e.target.value)}
                      className={`form-control ${errorMovimiento?.campo === "categoria" ? "input-error" : ""}`}
                    >
                      <option value="">Seleccione categoría...</option>
                      {categoriasFiltradas.map((cat) => (
                        <option
                          key={`${cat.tipo}-${cat.nombre}`}
                          value={cat.nombre}
                        >
                          {cat.nombre}
                        </option>
                      ))}
                    </select>

                    {mostrarNuevaCategoria && (
                      <div
                        style={{
                          display: "flex",
                          gap: "8px",
                          marginTop: "8px",
                        }}
                      >
                        <input
                          type="text"
                          value={nombreNuevaCategoria}
                          onChange={(e) =>
                            setNombreNuevaCategoria(e.target.value)
                          }
                          placeholder="Nombre de nueva categoría"
                          className="form-control"
                        />
                        <button
                          type="button"
                          onClick={manejarCrearCategoria}
                          className="btn btn-secondary"
                        >
                          Añadir
                        </button>
                      </div>
                    )}
                  </div>

                  <div>
                    <label className="form-label">Nota (opcional)</label>
                    <input
                      type="text"
                      value={notaMovimiento}
                      onChange={(e) => setNotaMovimiento(e.target.value)}
                      placeholder="Ej: Mercado del sábado"
                      className="form-control"
                    />
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    type="button"
                    onClick={() => setMostrarFormMovimiento(false)}
                    className="btn btn-ghost"
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    className="btn btn-primary"
                    disabled={guardando}
                  >
                    Guardar movimiento
                  </button>
                </div>
              </form>
            </section>
          )}

          {/* Formulario de Corrección de Movimiento (RF-LIB-06) */}
          {movimientoACorregir && (
            <section className="form-card">
              <h3 className="form-title">Corregir movimiento</h3>
              <p className="form-subtitle">
                El movimiento original ({movimientoACorregir.categoria} del{" "}
                {movimientoACorregir.fecha}) no se borrará: se registrará un
                ajuste por la diferencia. Si el movimiento no debió existir,
                escribe <strong>0</strong>.
              </p>

              {errorCorreccion && (
                <div className="error-alert">
                  <strong>!</strong>
                  <div>
                    <strong>Revisa el campo «{errorCorreccion.campo}»: </strong>
                    <span>{errorCorreccion.mensaje}</span>
                  </div>
                </div>
              )}

              <form onSubmit={manejarCorregirMovimiento}>
                <div className="form-group">
                  <label className="form-label">
                    Monto correcto ({cuentaSeleccionada.moneda}) — Valor vigente
                    actual: {movimientoACorregir.montoVigente}
                  </label>
                  <input
                    type="text"
                    value={montoCorrecto}
                    onChange={(e) => setMontoCorrecto(e.target.value)}
                    placeholder="0.00"
                    className={`form-control ${errorCorreccion?.campo === "monto" ? "input-error" : ""}`}
                  />
                </div>

                <div className="form-actions">
                  <button
                    type="button"
                    onClick={() => setMovimientoACorregir(null)}
                    className="btn btn-ghost"
                  >
                    Cancelar
                  </button>
                  <button
                    type="submit"
                    className="btn btn-primary"
                    disabled={guardando}
                  >
                    Registrar ajuste de corrección
                  </button>
                </div>
              </form>
            </section>
          )}

          {/* Lista de Movimientos (Sección 2.3) */}
          <section className="lista-movimientos">
            {detalleCuenta.movimientos.length === 0 ? (
              <div className="empty-state">
                No hay movimientos registrados en esta cuenta.
              </div>
            ) : (
              <div className="accounts-list">
                {detalleCuenta.movimientos.map((mov) => {
                  const esPositivo = !mov.monto.startsWith("-");
                  return (
                    <div key={mov.id} className="account-card">
                      <div>
                        <div className="account-name-row">
                          <span className="account-name">{mov.categoria}</span>
                          <span
                            className="badge"
                            style={{ textTransform: "uppercase" }}
                          >
                            {mov.tipo}
                          </span>
                          {mov.corregido && (
                            <span className="badge badge-corregido">
                              Corregido (vigente:{" "}
                              {formatearMontoLocal(
                                mov.montoVigente,
                                obtenerSimbolo(cuentaSeleccionada.moneda),
                              )}
                              )
                            </span>
                          )}
                          {mov.esAjuste && (
                            <span className="badge badge-ajuste">
                              Ajuste de #{mov.referenciaId?.slice(0, 8)}
                            </span>
                          )}
                        </div>
                        <p className="account-meta">
                          Fecha: {mov.fecha} {mov.nota ? `• ${mov.nota}` : ""} •
                          ID: #{mov.id.slice(0, 8)}
                        </p>
                      </div>

                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: "16px",
                        }}
                      >
                        <div className="account-balance-box">
                          <span
                            className="account-balance-value"
                            style={{
                              color: mov.esAjuste
                                ? "var(--text-ajust)"
                                : esPositivo
                                  ? "var(--text-success)"
                                  : "var(--text-danger)",
                            }}
                          >
                            {formatearMontoLocal(
                              mov.monto,
                              obtenerSimbolo(cuentaSeleccionada.moneda),
                              true,
                            )}
                          </span>
                        </div>

                        {/* Solo los movimientos originales se pueden corregir; no existe opción de editar ni eliminar (INV-04) */}
                        {!mov.esAjuste && (
                          <button
                            type="button"
                            onClick={() => iniciarCorreccion(mov)}
                            className="btn btn-ghost"
                            style={{
                              border: "1px solid var(--border-subtle)",
                              padding: "6px 12px",
                              fontSize: "12px",
                            }}
                          >
                            Corregir
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        </main>
      )}
    </div>
  );
}
