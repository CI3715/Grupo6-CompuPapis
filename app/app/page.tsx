"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  saldo: { cantidad: string; moneda: string };
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
  composicion: { inicial: string; ingresos: string; gastos: string; ajustes: string };
  movimientos: Movimiento[];
}

interface TasaCambio {
  id: string;
  origen: string;
  destino: string;
  valor: string;
  fecha: string;
  fuente: string;
}

interface TasaAplicadaInfo {
  par: string;
  descripcion: string;
  valor: string;
  fecha: string;
  fuente: string;
  esInversa: boolean;
}

interface CuentaPatrimonioVista {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  simbolo: string;
  saldoOriginal: string;
  enMonedaBase?: string;
  tasaUsada?: TasaAplicadaInfo;
}

interface InformePatrimonio {
  fechaConsulta: string;
  monedaBase: string;
  simboloBase: string;
  cuentas: CuentaPatrimonioVista[];
  subtotales: SubtotalMoneda[];
  totalConsolidado?: string;
  tasasEmpleadas: TasaAplicadaInfo[];
  avisosTasasFaltantes: string[];
}

interface UsuarioPublico {
  id: string;
  nombreUsuario: string;
  nombreVisible: string;
  correo: string;
  fechaAlta: string;
}

interface EstadoSesionApp {
  activa: boolean;
  vencida: boolean;
  mensajeAviso?: string;
  usuario?: UsuarioPublico;
}

function formatearMontoLocal(cantidadStr: string, simbolo: string, mostrarSignoPositivo = false): string {
  const num = Number(cantidadStr);
  const formateado = new Intl.NumberFormat("es-VE", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(Math.abs(num));
  if (num < 0) return `- ${simbolo} ${formateado}`;
  if (mostrarSignoPositivo && num > 0) return `+ ${simbolo} ${formateado}`;
  return `${simbolo} ${formateado}`;
}

function obtenerHoyLocal(): string {
  const hoy = new Date();
  return `${hoy.getFullYear()}-${String(hoy.getMonth() + 1).padStart(2, "0")}-${String(hoy.getDate()).padStart(2, "0")}`;
}

function obtenerClaseBadgeMoneda(codigo: string): string {
  switch (codigo.toUpperCase()) {
    case "USD": return "badge badge-usd";
    case "VES": return "badge badge-ves";
    case "EUR": return "badge badge-eur";
    case "USDT": return "badge badge-usdt";
    default: return "badge";
  }
}

export default function CuentasClarasApp() {
  const [vistaActiva, setVistaActiva] = useState<"cuentas" | "tasas" | "patrimonio" | "auth">("cuentas");

  // --- ESTADO SESIÓN (ENTREGA 5) ---
  const [usuarioSesion, setUsuarioSesion] = useState<UsuarioPublico | null>(null);
  const [avisoSesion, setAvisoSesion] = useState<string | null>(null);
  const [modoAuth, setModoAuth] = useState<"login" | "registro">("login");
  const [regUsuario, setRegUsuario] = useState("");
  const [regVisible, setRegVisible] = useState("");
  const [regCorreo, setRegCorreo] = useState("");
  const [regContrasena, setRegContrasena] = useState("");
  const [loginCorreo, setLoginCorreo] = useState("");
  const [loginContrasena, setLoginContrasena] = useState("");
  const [errorAuth, setErrorAuth] = useState<ErrorDominio | null>(null);
  const [mensajeExitoAuth, setMensajeExitoAuth] = useState<string | null>(null);

  // --- ESTADO CUENTAS Y MOVIMIENTOS (ENTREGAS 2 Y 3) ---
  const [resumen, setResumen] = useState<ResumenCuentas>({ cuentas: [], subtotales: [] });
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [mostrarFormularioCuenta, setMostrarFormularioCuenta] = useState(false);

  const [nombreCuenta, setNombreCuenta] = useState("");
  const [tipoCuenta, setTipoCuenta] = useState("efectivo");
  const [monedaCuenta, setMonedaCuenta] = useState("");
  const [saldoInicial, setSaldoInicial] = useState("0.00");
  const [errorCuenta, setErrorCuenta] = useState<ErrorDominio | null>(null);

  const [cuentaSeleccionadaId, setCuentaSeleccionadaId] = useState<string | null>(null);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuenta | null>(null);

  const [mostrarFormMovimiento, setMostrarFormMovimiento] = useState(false);
  const [tipoMovimiento, setTipoMovimiento] = useState<"gasto" | "ingreso">("gasto");
  const [montoMovimiento, setMontoMovimiento] = useState("");
  const [fechaMovimiento, setFechaMovimiento] = useState(obtenerHoyLocal());
  const [categoriaMovimiento, setCategoriaMovimiento] = useState("");
  const [notaMovimiento, setNotaMovimiento] = useState("");
  const [errorMovimiento, setErrorMovimiento] = useState<ErrorDominio | null>(null);
  const [advertenciaSaldoNegativo, setAdvertenciaSaldoNegativo] = useState(false);

  const [movimientoACorregir, setMovimientoACorregir] = useState<Movimiento | null>(null);
  const [montoCorrecto, setMontoCorrecto] = useState("");
  const [errorCorreccion, setErrorCorreccion] = useState<ErrorDominio | null>(null);

  // --- ESTADO TASAS, MONEDAS PROPIAS Y PATRIMONIO (ENTREGA 4) ---
  const [tasas, setTasas] = useState<TasaCambio[]>([]);
  const [tasaOrigen, setTasaOrigen] = useState("USD");
  const [tasaDestino, setTasaDestino] = useState("VES");
  const [tasaValor, setTasaValor] = useState("");
  const [tasaFecha, setTasaFecha] = useState(obtenerHoyLocal());
  const [tasaFuente, setTasaFuente] = useState("");
  const [errorTasa, setErrorTasa] = useState<ErrorDominio | null>(null);

  const [nuevaMonCodigo, setNuevaMonCodigo] = useState("");
  const [nuevaMonNombre, setNuevaMonNombre] = useState("");
  const [nuevaMonSimbolo, setNuevaMonSimbolo] = useState("");
  const [nuevaMonDecimales, setNuevaMonDecimales] = useState("2");
  const [errorMonedaPropia, setErrorMonedaPropia] = useState<ErrorDominio | null>(null);

  const [patrimonio, setPatrimonio] = useState<InformePatrimonio | null>(null);
  const [fechaPatrimonio, setFechaPatrimonio] = useState(obtenerHoyLocal());

  const cargarTodoLocal = async () => {
    try {
      const [resCuentas, resMonedas, resCategorias, resTasas, resPat] = await Promise.all([
        invoke<ResumenCuentas>("listar_cuentas"),
        invoke<Moneda[]>("listar_monedas"),
        invoke<Categoria[]>("listar_categorias"),
        invoke<TasaCambio[]>("listar_tasas"),
        invoke<InformePatrimonio>("consultar_patrimonio", { fechaConsulta: fechaPatrimonio }),
      ]);
      setResumen(resCuentas);
      setMonedas(resMonedas);
      setCategorias(resCategorias);
      setTasas(resTasas);
      setPatrimonio(resPat);
    } catch {
      // Operación local silenciosa
    }
  };

  const verificarSesionGuardada = async () => {
    try {
      const estado = await invoke<EstadoSesionApp>("obtener_usuario_actual");
      if (estado.activa && estado.usuario) {
        setUsuarioSesion(estado.usuario);
        setAvisoSesion(null);
      } else if (estado.vencida && estado.mensajeAviso) {
        setUsuarioSesion(null);
        setAvisoSesion(estado.mensajeAviso);
      }
    } catch {
      // Sin conexión el usuario local sigue operando con normalidad (RNF-17)
    }
  };

  useEffect(() => {
    cargarTodoLocal();
    verificarSesionGuardada();
  }, []);

  const obtenerSimbolo = (codigo: string) =>
    monedas.find((m) => m.codigo === codigo)?.simbolo ?? codigo;

  // --- MANEJADORES CUENTAS Y MOVIMIENTOS ---
  const manejarCrearCuenta = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorCuenta(null);
    try {
      await invoke("crear_cuenta", {
        nombre: nombreCuenta,
        tipo: tipoCuenta,
        moneda: monedaCuenta,
        saldoInicial,
      });
      setNombreCuenta("");
      setSaldoInicial("0.00");
      setMonedaCuenta("");
      setMostrarFormularioCuenta(false);
      await cargarTodoLocal();
    } catch (err: any) {
      setErrorCuenta(err as ErrorDominio);
    }
  };

  const seleccionarCuenta = async (cuentaId: string) => {
    setCuentaSeleccionadaId(cuentaId);
    setMostrarFormularioCuenta(false);
    setMostrarFormMovimiento(false);
    setMovimientoACorregir(null);
    const det = await invoke<DetalleCuenta>("obtener_detalle_cuenta", { cuentaId });
    setDetalleCuenta(det);
  };

  const manejarRegistrarMovimiento = async (e: React.FormEvent, confirmadoNegativo = false) => {
    e.preventDefault();
    if (!cuentaSeleccionadaId || !detalleCuenta) return;
    setErrorMovimiento(null);

    const montoNum = Number(montoMovimiento.trim().replace(",", "."));
    const saldoActualNum = Number(detalleCuenta.saldoActual);
    if (
      tipoMovimiento === "gasto" &&
      !isNaN(montoNum) &&
      montoNum > 0 &&
      saldoActualNum - montoNum < 0 &&
      !confirmadoNegativo
    ) {
      setAdvertenciaSaldoNegativo(true);
      return;
    }

    try {
      const nuevoDet = await invoke<DetalleCuenta>("registrar_movimiento", {
        cuentaId: cuentaSeleccionadaId,
        tipo: tipoMovimiento,
        monto: montoMovimiento,
        fecha: fechaMovimiento,
        categoria: categoriaMovimiento,
        nota: notaMovimiento || null,
      });
      setDetalleCuenta(nuevoDet);
      setMostrarFormMovimiento(false);
      setAdvertenciaSaldoNegativo(false);
      await cargarTodoLocal();
    } catch (err: any) {
      setErrorMovimiento(err as ErrorDominio);
    }
  };

  const manejarCorregirMovimiento = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!cuentaSeleccionadaId || !movimientoACorregir) return;
    setErrorCorreccion(null);
    try {
      const nuevoDet = await invoke<DetalleCuenta>("corregir_movimiento", {
        cuentaId: cuentaSeleccionadaId,
        movimientoId: movimientoACorregir.id,
        montoCorrecto,
      });
      setDetalleCuenta(nuevoDet);
      setMovimientoACorregir(null);
      await cargarTodoLocal();
    } catch (err: any) {
      setErrorCorreccion(err as ErrorDominio);
    }
  };

  // --- MANEJADORES TASAS Y PATRIMONIO (ENTREGA 4) ---
  const manejarRegistrarTasa = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorTasa(null);
    try {
      await invoke("registrar_tasa", {
        origen: tasaOrigen,
        destino: tasaDestino,
        valor: tasaValor,
        fecha: tasaFecha,
        fuente: tasaFuente,
      });
      setTasaValor("");
      setTasaFuente("");
      await cargarTodoLocal();
    } catch (err: any) {
      setErrorTasa(err as ErrorDominio);
    }
  };

  const manejarEliminarTasa = async (tasaId: string) => {
    await invoke("eliminar_tasa", { tasaId });
    await cargarTodoLocal();
  };

  const manejarCrearMonedaPropia = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMonedaPropia(null);
    try {
      await invoke("crear_moneda", {
        codigo: nuevaMonCodigo,
        nombre: nuevaMonNombre,
        simbolo: nuevaMonSimbolo,
        decimales: Number(nuevaMonDecimales),
      });
      setNuevaMonCodigo("");
      setNuevaMonNombre("");
      setNuevaMonSimbolo("");
      await cargarTodoLocal();
    } catch (err: any) {
      setErrorMonedaPropia(err as ErrorDominio);
    }
  };

  const manejarCambiarMonedaBase = async (nuevaBase: string) => {
    await invoke("cambiar_moneda_base", { monedaBase: nuevaBase });
    const resPat = await invoke<InformePatrimonio>("consultar_patrimonio", { fechaConsulta: fechaPatrimonio });
    setPatrimonio(resPat);
  };

  const manejarCambiarFechaPatrimonio = async (nuevaFecha: string) => {
    setFechaPatrimonio(nuevaFecha);
    const resPat = await invoke<InformePatrimonio>("consultar_patrimonio", { fechaConsulta: nuevaFecha });
    setPatrimonio(resPat);
  };

  // --- MANEJADORES AUTENTICACIÓN (ENTREGA 5) ---
  const manejarRegistroUsuario = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorAuth(null);
    setMensajeExitoAuth(null);
    try {
      const creado = await invoke<UsuarioPublico>("registrar_usuario", {
        nombreUsuario: regUsuario,
        nombreVisible: regVisible,
        correo: regCorreo,
        contrasena: regContrasena,
      });
      setMensajeExitoAuth(`Cuenta @${creado.nombreUsuario} creada con éxito. Ahora puedes iniciar sesión.`);
      setRegContrasena("");
      setLoginCorreo(creado.correo);
      setModoAuth("login");
    } catch (err: any) {
      setErrorAuth(err as ErrorDominio);
    }
  };

  const manejarInicioSesion = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorAuth(null);
    setMensajeExitoAuth(null);
    try {
      const u = await invoke<UsuarioPublico>("iniciar_sesion", {
        correo: loginCorreo,
        contrasena: loginContrasena,
      });
      setUsuarioSesion(u);
      setAvisoSesion(null);
      setLoginContrasena("");
    } catch (err: any) {
      setErrorAuth(err as ErrorDominio);
    }
  };

  const manejarCerrarSesion = async () => {
    await invoke("cerrar_sesion");
    setUsuarioSesion(null);
    setAvisoSesion(null);
  };

  const cuentaSeleccionada = resumen.cuentas.find((c) => c.id === cuentaSeleccionadaId);

  return (
    <div>
      {/* BARRA SUPERIOR GLOBAL DE NAVEGACIÓN Y SESIÓN */}
      <nav
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          padding: "12px 24px",
          backgroundColor: "#FFFFFF",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
          <button
            className={`btn ${vistaActiva === "cuentas" ? "btn-primary" : "btn-ghost"}`}
            onClick={() => setVistaActiva("cuentas")}
          >
            Mis cuentas
          </button>
          <button
            className={`btn ${vistaActiva === "tasas" ? "btn-primary" : "btn-ghost"}`}
            onClick={() => {
              setCuentaSeleccionadaId(null);
              setVistaActiva("tasas");
            }}
          >
            Tasas y Monedas
          </button>
          <button
            className={`btn ${vistaActiva === "patrimonio" ? "btn-primary" : "btn-ghost"}`}
            onClick={() => {
              setCuentaSeleccionadaId(null);
              cargarTodoLocal();
              setVistaActiva("patrimonio");
            }}
          >
            Patrimonio
          </button>
        </div>

        <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
          {usuarioSesion ? (
            <>
              <span style={{ fontSize: "13px", fontWeight: 600 }}>
                Sesión: {usuarioSesion.nombreVisible} (@{usuarioSesion.nombreUsuario})
              </span>
              <button
                className="btn btn-ghost"
                onClick={() => {
                  verificarSesionGuardada();
                  setVistaActiva("auth");
                }}
              >
                Mi perfil
              </button>
              <button className="btn btn-secondary" onClick={manejarCerrarSesion}>
                Cerrar sesión
              </button>
            </>
          ) : (
            <button
              className="btn btn-secondary"
              onClick={() => {
                setErrorAuth(null);
                setVistaActiva("auth");
              }}
            >
              Iniciar sesión
            </button>
          )}
        </div>
      </nav>

      {/* AVISO SI EL TOKEN VENCIÓ (Sección 2.4 Entrega 5) */}
      {avisoSesion && (
        <div className="warning-alert" style={{ margin: "16px 24px" }}>
          <strong>Aviso de sesión: </strong> {avisoSesion}.{" "}
          <button
            onClick={() => setVistaActiva("auth")}
            style={{ textDecoration: "underline", background: "none", border: "none", cursor: "pointer", fontWeight: 700 }}
          >
            Iniciar sesión ahora
          </button>
        </div>
      )}

      {/* ========================================================= */}
      {/* VISTA 1: MIS CUENTAS Y MOVIMIENTOS (ENTREGAS 2 Y 3)       */}
      {/* ========================================================= */}
      {vistaActiva === "cuentas" && (
        <div className={`app-wrapper ${cuentaSeleccionadaId ? "layout-split" : "layout-single"}`}>
          <aside className="sidebar-cuentas">
            <header className="header">
              <div>
                <span className="header-tag">Cuentas Claras • Local-First</span>
                <h1 className="header-title">Mis cuentas</h1>
              </div>
              {!cuentaSeleccionadaId && (
                <button
                  onClick={() => {
                    setErrorCuenta(null);
                    setMostrarFormularioCuenta(!mostrarFormularioCuenta);
                  }}
                  className={`btn ${mostrarFormularioCuenta ? "btn-secondary" : "btn-primary"}`}
                >
                  {mostrarFormularioCuenta ? "Cancelar" : "+ Nueva cuenta"}
                </button>
              )}
            </header>

            {mostrarFormularioCuenta && (
              <section className="form-card">
                <h2 className="form-title">Registrar nueva cuenta</h2>
                {errorCuenta && (
                  <div className="error-alert">
                    <strong>!</strong>
                    <div>
                      <strong>Revisa el campo «{errorCuenta.campo}»: </strong>
                      <span>{errorCuenta.mensaje}</span>
                    </div>
                  </div>
                )}
                <form onSubmit={manejarCrearCuenta}>
                  <div className="form-group">
                    <label className="form-label">Nombre de la cuenta</label>
                    <input
                      type="text"
                      value={nombreCuenta}
                      onChange={(e) => setNombreCuenta(e.target.value)}
                      className={`form-control ${errorCuenta?.campo === "nombre" ? "input-error" : ""}`}
                    />
                  </div>
                  <div className="form-row">
                    <div>
                      <label className="form-label">Tipo</label>
                      <select
                        value={tipoCuenta}
                        onChange={(e) => setTipoCuenta(e.target.value)}
                        className="form-control"
                      >
                        <option value="efectivo">Efectivo</option>
                        <option value="banco">Banco</option>
                        <option value="billetera digital">Billetera digital</option>
                      </select>
                    </div>
                    <div>
                      <label className="form-label">Moneda</label>
                      <select
                        value={monedaCuenta}
                        onChange={(e) => setMonedaCuenta(e.target.value)}
                        className={`form-control ${errorCuenta?.campo === "moneda" ? "input-error" : ""}`}
                      >
                        <option value="">Seleccione...</option>
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
                        className={`form-control ${errorCuenta?.campo === "saldo_inicial" ? "input-error" : ""}`}
                      />
                    </div>
                  </div>
                  <div className="form-actions">
                    <button type="submit" className="btn btn-primary">
                      Guardar cuenta
                    </button>
                  </div>
                </form>
              </section>
            )}

            {resumen.subtotales.length > 0 && (
              <section className="section-block">
                <h2 className="section-title">Subtotales por moneda</h2>
                <div className="subtotals-grid">
                  {resumen.subtotales.map((sub) => (
                    <div key={sub.codigo} className="subtotal-card">
                      <span className={obtenerClaseBadgeMoneda(sub.codigo)}>{sub.codigo}</span>
                      <div className="subtotal-amount">
                        {formatearMontoLocal(sub.subtotal, sub.simbolo)}
                      </div>
                    </div>
                  ))}
                </div>
              </section>
            )}

            <section className="section-block">
              <h2 className="section-title">Cuentas registradas ({resumen.cuentas.length})</h2>
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
                        <span className={obtenerClaseBadgeMoneda(c.moneda)}>{c.moneda}</span>
                      </div>
                      <p className="account-meta">Tipo: {c.tipo}</p>
                    </div>
                    <div className="account-balance-box">
                      <span className="account-balance-label">Saldo actual</span>
                      <span className="account-balance-value">
                        {formatearMontoLocal(c.saldo.cantidad, obtenerSimbolo(c.moneda))}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </aside>

          {cuentaSeleccionadaId && cuentaSeleccionada && detalleCuenta && (
            <main className="content-detalle">
              <button className="btn btn-ghost" onClick={() => setCuentaSeleccionadaId(null)}>
                ← Volver
              </button>
              <header className="detalle-header">
                <div className="detalle-header-top">
                  <div className="detalle-info">
                    <h2 className="detalle-titulo">{cuentaSeleccionada.nombre}</h2>
                    <span className={obtenerClaseBadgeMoneda(cuentaSeleccionada.moneda)}>
                      {cuentaSeleccionada.moneda}
                    </span>
                  </div>
                  <div className="detalle-saldo">
                    <p className="detalle-subtitulo">Saldo Actual</p>
                    <h1 className="saldo-gigante">
                      {formatearMontoLocal(detalleCuenta.saldoActual, obtenerSimbolo(cuentaSeleccionada.moneda))}
                    </h1>
                  </div>
                </div>
              </header>

              <div className="acciones-movimientos" style={{ marginBottom: "16px" }}>
                <button
                  className="btn btn-danger"
                  onClick={() => {
                    setTipoMovimiento("gasto");
                    setCategoriaMovimiento(categorias.find((c) => c.tipo === "gasto")?.nombre ?? "");
                    setMostrarFormMovimiento(true);
                  }}
                >
                  - Registrar Gasto
                </button>
                <button
                  className="btn btn-success"
                  onClick={() => {
                    setTipoMovimiento("ingreso");
                    setCategoriaMovimiento(categorias.find((c) => c.tipo === "ingreso")?.nombre ?? "");
                    setMostrarFormMovimiento(true);
                  }}
                >
                  + Registrar Ingreso
                </button>
              </div>

              {mostrarFormMovimiento && (
                <section className="form-card">
                  {errorMovimiento && (
                    <div className="error-alert">
                      <strong>Campo «{errorMovimiento.campo}»:</strong> {errorMovimiento.mensaje}
                    </div>
                  )}
                  {advertenciaSaldoNegativo && (
                    <div className="warning-alert">
                      <p>Este gasto dejará el saldo en negativo. ¿Confirmar?</p>
                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={(e) => manejarRegistrarMovimiento(e as any, true)}
                      >
                        Confirmar de todas formas
                      </button>
                    </div>
                  )}
                  <form onSubmit={(e) => manejarRegistrarMovimiento(e, false)}>
                    <div className="form-row">
                      <input
                        type="text"
                        placeholder="Monto"
                        value={montoMovimiento}
                        onChange={(e) => setMontoMovimiento(e.target.value)}
                        className="form-control"
                      />
                      <input
                        type="date"
                        value={fechaMovimiento}
                        onChange={(e) => setFechaMovimiento(e.target.value)}
                        className="form-control"
                      />
                      <select
                        value={categoriaMovimiento}
                        onChange={(e) => setCategoriaMovimiento(e.target.value)}
                        className="form-control"
                      >
                        {categorias
                          .filter((c) => c.tipo === tipoMovimiento)
                          .map((c) => (
                            <option key={c.nombre} value={c.nombre}>
                              {c.nombre}
                            </option>
                          ))}
                      </select>
                      <input
                        type="text"
                        placeholder="Nota opcional"
                        value={notaMovimiento}
                        onChange={(e) => setNotaMovimiento(e.target.value)}
                        className="form-control"
                      />
                    </div>
                    <button type="submit" className="btn btn-primary">
                      Guardar movimiento
                    </button>
                  </form>
                </section>
              )}

              {movimientoACorregir && (
                <section className="form-card">
                  <h3 className="form-title">Corregir movimiento (vigente: {movimientoACorregir.montoVigente})</h3>
                  {errorCorreccion && (
                    <div className="error-alert">
                      <strong>Campo «{errorCorreccion.campo}»:</strong> {errorCorreccion.mensaje}
                    </div>
                  )}
                  <form onSubmit={manejarCorregirMovimiento}>
                    <input
                      type="text"
                      value={montoCorrecto}
                      onChange={(e) => setMontoCorrecto(e.target.value)}
                      className="form-control"
                      style={{ marginBottom: "12px" }}
                    />
                    <button type="submit" className="btn btn-primary">
                      Registrar ajuste
                    </button>
                  </form>
                </section>
              )}

              <div className="accounts-list">
                {detalleCuenta.movimientos.map((mov) => (
                  <div key={mov.id} className="account-card">
                    <div>
                      <strong>{mov.categoria}</strong> ({mov.tipo}) — {mov.fecha}
                      {mov.corregido && <span className="badge badge-corregido" style={{ marginLeft: 8 }}>Corregido</span>}
                      {mov.esAjuste && <span className="badge badge-ajuste" style={{ marginLeft: 8 }}>Ajuste</span>}
                    </div>
                    <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
                      <strong>
                        {formatearMontoLocal(mov.monto, obtenerSimbolo(cuentaSeleccionada.moneda), true)}
                      </strong>
                      {!mov.esAjuste && (
                        <button
                          className="btn btn-ghost"
                          onClick={() => {
                            setMovimientoACorregir(mov);
                            setMontoCorrecto(mov.montoVigente);
                          }}
                        >
                          Corregir
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </main>
          )}
        </div>
      )}

      {/* ========================================================= */}
      {/* VISTA 2: TASAS DE CAMBIO Y MONEDAS PROPIAS (ENTREGA 4)    */}
      {/* ========================================================= */}
      {vistaActiva === "tasas" && (
        <main className="app-container">
          <h1 className="header-title" style={{ marginBottom: "20px" }}>
            Tasas de Cambio y Monedas Propias
          </h1>

          <section className="form-card">
            <h2 className="form-title">Registrar tasa de cambio manual</h2>
            <p className="form-subtitle">
              1 unidad de la moneda de origen equivale al valor indicado en la moneda de destino.
            </p>
            {errorTasa && (
              <div className="error-alert">
                <strong>Revisa el campo «{errorTasa.campo}»: </strong> {errorTasa.mensaje}
              </div>
            )}
            <form onSubmit={manejarRegistrarTasa}>
              <div className="form-row">
                <div>
                  <label className="form-label">Moneda origen</label>
                  <select value={tasaOrigen} onChange={(e) => setTasaOrigen(e.target.value)} className="form-control">
                    {monedas.map((m) => (
                      <option key={m.codigo} value={m.codigo}>{m.codigo}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="form-label">Moneda destino</label>
                  <select value={tasaDestino} onChange={(e) => setTasaDestino(e.target.value)} className="form-control">
                    {monedas.map((m) => (
                      <option key={m.codigo} value={m.codigo}>{m.codigo}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="form-label">Valor (hasta 8 decimales)</label>
                  <input
                    type="text"
                    value={tasaValor}
                    onChange={(e) => setTasaValor(e.target.value)}
                    placeholder="Ej: 887.00"
                    className="form-control"
                  />
                </div>
                <div>
                  <label className="form-label">Fecha</label>
                  <input
                    type="date"
                    value={tasaFecha}
                    onChange={(e) => setTasaFecha(e.target.value)}
                    className="form-control"
                  />
                </div>
                <div>
                  <label className="form-label">Fuente</label>
                  <input
                    type="text"
                    value={tasaFuente}
                    onChange={(e) => setTasaFuente(e.target.value)}
                    placeholder="Ej: BCV, casa de cambio"
                    className="form-control"
                  />
                </div>
              </div>
              <button type="submit" className="btn btn-primary">Guardar tasa</button>
            </form>
          </section>

          <section className="section-block">
            <h2 className="section-title">Historial de tasas registradas ({tasas.length})</h2>
            <div className="accounts-list">
              {tasas.map((t) => (
                <div key={t.id} className="account-card">
                  <div>
                    <strong>1 {t.origen} = {t.valor} {t.destino}</strong>
                    <p className="account-meta">Fecha: {t.fecha} • Fuente: {t.fuente}</p>
                  </div>
                  <button className="btn btn-danger" onClick={() => manejarEliminarTasa(t.id)}>
                    Eliminar
                  </button>
                </div>
              ))}
            </div>
          </section>

          <section className="form-card">
            <h2 className="form-title">Añadir moneda propia al catálogo (RF-CAM-01)</h2>
            {errorMonedaPropia && (
              <div className="error-alert">
                <strong>Campo «{errorMonedaPropia.campo}»: </strong> {errorMonedaPropia.mensaje}
              </div>
            )}
            <form onSubmit={manejarCrearMonedaPropia}>
              <div className="form-row">
                <input
                  type="text"
                  placeholder="Código (3-5 mayúsculas, ej: COP)"
                  value={nuevaMonCodigo}
                  onChange={(e) => setNuevaMonCodigo(e.target.value)}
                  className="form-control"
                />
                <input
                  type="text"
                  placeholder="Nombre (ej: Peso colombiano)"
                  value={nuevaMonNombre}
                  onChange={(e) => setNuevaMonNombre(e.target.value)}
                  className="form-control"
                />
                <input
                  type="text"
                  placeholder="Símbolo (ej: COL$)"
                  value={nuevaMonSimbolo}
                  onChange={(e) => setNuevaMonSimbolo(e.target.value)}
                  className="form-control"
                />
                <input
                  type="number"
                  min={0}
                  max={8}
                  value={nuevaMonDecimales}
                  onChange={(e) => setNuevaMonDecimales(e.target.value)}
                  className="form-control"
                />
              </div>
              <button type="submit" className="btn btn-primary">Añadir moneda</button>
            </form>
          </section>
        </main>
      )}

      {/* ========================================================= */}
      {/* VISTA 3: PATRIMONIO CONSOLIDADO (ENTREGA 4)               */}
      {/* ========================================================= */}
      {vistaActiva === "patrimonio" && patrimonio && (
        <main className="app-container">
          <header className="header">
            <div>
              <span className="header-tag">Informe Multimoneda • RF-INF-01</span>
              <h1 className="header-title">Patrimonio Consolidado</h1>
            </div>
            <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
              <div>
                <label className="form-label">Moneda base</label>
                <select
                  value={patrimonio.monedaBase}
                  onChange={(e) => manejarCambiarMonedaBase(e.target.value)}
                  className="form-control"
                >
                  {monedas.map((m) => (
                    <option key={m.codigo} value={m.codigo}>{m.codigo} ({m.simbolo})</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="form-label">Fecha de consulta</label>
                <input
                  type="date"
                  value={fechaPatrimonio}
                  onChange={(e) => manejarCambiarFechaPatrimonio(e.target.value)}
                  className="form-control"
                />
              </div>
            </div>
          </header>

          {/* Aviso de tasa faltante (INV-02, RF-CAM-04) */}
          {patrimonio.avisosTasasFaltantes.length > 0 ? (
            <div className="error-alert">
              <div>
                <strong>No se puede consolidar el total en {patrimonio.monedaBase}:</strong>
                <ul style={{ marginLeft: "18px", marginTop: "6px" }}>
                  {patrimonio.avisosTasasFaltantes.map((aviso, i) => (
                    <li key={i}>{aviso}</li>
                  ))}
                </ul>
              </div>
            </div>
          ) : (
            <section className="form-card">
              <span className="subtotal-label">Total consolidado en {patrimonio.monedaBase}</span>
              <div className="saldo-gigante" style={{ marginTop: "8px" }}>
                {formatearMontoLocal(patrimonio.totalConsolidado ?? "0.00", patrimonio.simboloBase)}
              </div>
              {patrimonio.tasasEmpleadas.length > 0 && (
                <div style={{ marginTop: "14px", fontSize: "13px", color: "var(--text-muted)" }}>
                  <strong>Tasas empleadas:</strong>
                  <ul style={{ marginLeft: "18px" }}>
                    {patrimonio.tasasEmpleadas.map((t) => (
                      <li key={t.par}>{t.descripcion}</li>
                    ))}
                  </ul>
                </div>
              )}
            </section>
          )}

          <section className="section-block">
            <h2 className="section-title">Subtotales por moneda</h2>
            <div className="subtotals-grid">
              {patrimonio.subtotales.map((sub) => (
                <div key={sub.codigo} className="subtotal-card">
                  <span className={obtenerClaseBadgeMoneda(sub.codigo)}>{sub.codigo}</span>
                  <div className="subtotal-amount">{formatearMontoLocal(sub.subtotal, sub.simbolo)}</div>
                </div>
              ))}
            </div>
          </section>

          <section className="section-block">
            <h2 className="section-title">Detalle por cuenta</h2>
            <div className="accounts-list">
              {patrimonio.cuentas.map((c) => (
                <div key={c.id} className="account-card">
                  <div>
                    <strong>{c.nombre}</strong> ({c.moneda})
                    {c.tasaUsada && <p className="account-meta">Tasa: {c.tasaUsada.descripcion}</p>}
                  </div>
                  <div style={{ textAlign: "right" }}>
                    <div>Saldo: <strong>{formatearMontoLocal(c.saldoOriginal, c.simbolo)}</strong></div>
                    {c.enMonedaBase && (
                      <div className="account-meta">
                        En {patrimonio.monedaBase}: {formatearMontoLocal(c.enMonedaBase, patrimonio.simboloBase)}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </section>
        </main>
      )}

      {/* ========================================================= */}
      {/* VISTA 4: REGISTRO, INICIO DE SESIÓN Y PERFIL (ENTREGA 5)  */}
      {/* ========================================================= */}
      {vistaActiva === "auth" && (
        <main className="app-container" style={{ maxWidth: "520px" }}>
          {usuarioSesion ? (
            <section className="form-card">
              <span className="header-tag">Sesión Activa • GET /usuarios/yo</span>
              <h2 className="form-title" style={{ marginTop: "6px" }}>
                {usuarioSesion.nombreVisible}
              </h2>
              <p className="form-subtitle">@{usuarioSesion.nombreUsuario}</p>
              <div style={{ fontSize: "14px", marginBottom: "20px" }}>
                <p><strong>Correo:</strong> {usuarioSesion.correo}</p>
                <p><strong>ID:</strong> {usuarioSesion.id}</p>
                <p><strong>Fecha de alta:</strong> {usuarioSesion.fechaAlta}</p>
              </div>
              <button className="btn btn-secondary" onClick={manejarCerrarSesion}>
                Cerrar sesión en este dispositivo
              </button>
            </section>
          ) : (
            <section className="form-card">
              <div style={{ display: "flex", gap: "8px", marginBottom: "16px" }}>
                <button
                  type="button"
                  className={`btn ${modoAuth === "login" ? "btn-primary" : "btn-ghost"}`}
                  onClick={() => {
                    setErrorAuth(null);
                    setModoAuth("login");
                  }}
                >
                  Iniciar sesión
                </button>
                <button
                  type="button"
                  className={`btn ${modoAuth === "registro" ? "btn-primary" : "btn-ghost"}`}
                  onClick={() => {
                    setErrorAuth(null);
                    setModoAuth("registro");
                  }}
                >
                  Registrarse
                </button>
              </div>

              {mensajeExitoAuth && (
                <div className="warning-alert" style={{ borderColor: "#059669", color: "#065F46", backgroundColor: "#ECFDF5" }}>
                  {mensajeExitoAuth}
                </div>
              )}

              {errorAuth && (
                <div className="error-alert">
                  <div>
                    <strong>Revisa «{errorAuth.campo}»: </strong>
                    <span>{errorAuth.mensaje}</span>
                  </div>
                </div>
              )}

              {modoAuth === "login" ? (
                <form onSubmit={manejarInicioSesion}>
                  <div className="form-group">
                    <label className="form-label">Correo electrónico</label>
                    <input
                      type="email"
                      value={loginCorreo}
                      onChange={(e) => setLoginCorreo(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "correo" ? "input-error" : ""}`}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Contraseña</label>
                    <input
                      type="password"
                      value={loginContrasena}
                      onChange={(e) => setLoginContrasena(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "contrasena" ? "input-error" : ""}`}
                    />
                  </div>
                  <button type="submit" className="btn btn-primary" style={{ width: "100%" }}>
                    Entrar
                  </button>
                </form>
              ) : (
                <form onSubmit={manejarRegistroUsuario}>
                  <div className="form-group">
                    <label className="form-label">Nombre de usuario (3-30 minúsculas, dígitos, . o _)</label>
                    <input
                      type="text"
                      value={regUsuario}
                      onChange={(e) => setRegUsuario(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "nombre_usuario" ? "input-error" : ""}`}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Nombre visible</label>
                    <input
                      type="text"
                      value={regVisible}
                      onChange={(e) => setRegVisible(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "nombre_visible" ? "input-error" : ""}`}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Correo electrónico</label>
                    <input
                      type="email"
                      value={regCorreo}
                      onChange={(e) => setRegCorreo(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "correo" ? "input-error" : ""}`}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Contraseña (mínimo 10 caracteres)</label>
                    <input
                      type="password"
                      value={regContrasena}
                      onChange={(e) => setRegContrasena(e.target.value)}
                      className={`form-control ${errorAuth?.campo === "contrasena" ? "input-error" : ""}`}
                    />
                  </div>
                  <button type="submit" className="btn btn-primary" style={{ width: "100%" }}>
                    Crear cuenta en el servidor
                  </button>
                </form>
              )}
            </section>
          )}
        </main>
      )}
    </div>
  );
}