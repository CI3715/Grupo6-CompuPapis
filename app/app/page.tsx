"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

import AppShell from "@/app/AppShell";
import PantallaPatrimonio from "@/app/patrimonio";
import PantallaRespaldo from "@/app/respaldo";
import DiagnosticoLibro from "@/app/diagnostico-libro";
import PantallaSocial from "@/app/social";
import PantallaTasas from "@/app/tasas";
import { useSincronizacion } from "@/app/sincronizacion";
import { formatearMontoLocal } from "@/lib/money";
import type {
  Categoria,
  DetalleCuenta,
  ErrorDominio,
  EstadoConfigSyncPersonal,
  InformeSyncPersonal,
  Moneda,
  Movimiento,
  Pantalla,
  ResumenCuentas,
} from "@/lib/tipos";
import { normalizarError, obtenerClaseBadgeMoneda } from "@/lib/utilidades";
import { AuthScreens, UsuarioSesion } from "./authSection";

function IconoTipoCuenta({ tipo }: { tipo: string }) {
  const normalizado = tipo.toLowerCase();
  if (normalizado.includes("banco")) {
    return (
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <line x1="3" x2="21" y1="22" y2="22" />
        <line x1="6" x2="6" y1="18" y2="11" />
        <line x1="10" x2="10" y1="18" y2="11" />
        <line x1="14" x2="14" y1="18" y2="11" />
        <line x1="18" x2="18" y1="18" y2="11" />
        <polygon points="12 2 20 7 4 7" />
      </svg>
    );
  }
  if (normalizado.includes("billetera")) {
    return (
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <rect width="14" height="20" x="5" y="2" rx="2" ry="2" />
        <path d="M12 18h.01" />
      </svg>
    );
  }
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect width="20" height="12" x="2" y="6" rx="2" />
      <circle cx="12" cy="12" r="2" />
      <path d="M6 12h.01M18 12h.01" />
    </svg>
  );
}

// --- FUNCIONES UTILERÍA ---
interface EvaluacionMovimiento {
  dejaSaldoNegativo: boolean;
  saldoResultante: string;
}

async function leerDatos() {
  const [resumen, monedas, categorias] = await Promise.all([
    invoke<ResumenCuentas>("listar_cuentas"),
    invoke<Moneda[]>("listar_monedas"),
    invoke<Categoria[]>("listar_categorias"),
  ]);
  return { resumen, monedas, categorias };
}

export default function MisCuentasPage() {
  // Entrega 5: Autenticación y manejo de sesión
  const [usuarioSesion, setUsuarioSesion] = useState<UsuarioSesion | null>(
    null,
  );
  const [sesionVencida, setSesionVencida] = useState(false);
  const [vistaAuth, setVistaAuth] = useState<
    "ninguna" | "login" | "registro" | "perfil"
  >("ninguna");

  useEffect(() => {
    const vencimiento = usuarioSesion?.vencimiento ?? usuarioSesion?.expira_en;
    if (!vencimiento) return;
    const tiempo = Date.parse(vencimiento);
    if (!Number.isFinite(tiempo)) return;
    const timer = window.setTimeout(
      () => {
        void invoke("cerrar_sesion").catch(() => undefined);
        setUsuarioSesion(null);
        setSesionVencida(true);
        setVistaAuth("ninguna");
      },
      Math.max(0, tiempo - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [usuarioSesion]);

  const marcarSesionVencida = useCallback(() => {
    void invoke("cerrar_sesion").catch(() => undefined);
    setUsuarioSesion(null);
    setSesionVencida(true);
    setVistaAuth("ninguna");
  }, []);

  // --- NAVEGACIÓN ---
  const [pantalla, setPantalla] = useState<Pantalla>("cuentas");

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
  const [informeSyncPersonal, setInformeSyncPersonal] =
    useState<InformeSyncPersonal | null>(null);

  const manejarCerrarSesion = async () => {
    try {
      await invoke("cerrar_sesion");
    } catch {
      // Incluso sin red, borra la sesión en pantalla
    } finally {
      setUsuarioSesion(null);
      setSesionVencida(false);
      setVistaAuth("ninguna");
    }
  };

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
    // Entrega 8 (Sección 2.5.3): sincroniza al abrir la app si hay un destino configurado
    invoke<EstadoConfigSyncPersonal>("obtener_config_sync_personal")
      .then((cfg) => {
        if (!activo || !cfg.configurado) return;
        return invoke<InformeSyncPersonal>("sincronizar_personal_ahora").then(
          (inf) => {
            if (!activo) return;
            setInformeSyncPersonal(inf);
            if (inf.recibidos > 0) {
              return leerDatos().then((datos) => {
                if (!activo) return;
                setResumen(datos.resumen);
                setMonedas(datos.monedas);
                setCategorias(datos.categorias);
              });
            }
          },
        );
      })
      .catch(() => {
        // Sin conexión o sin Tauri: las cuentas personales siguen operando normalmente
      });
    return () => {
      activo = false;
    };
  }, []);

  const alSincronizar = async () => {
    await cargarDatos();
    if (cuentaSeleccionadaId) {
      try {
        setDetalleCuenta(
          await invoke<DetalleCuenta>("obtener_detalle_cuenta", {
            cuentaId: cuentaSeleccionadaId,
          }),
        );
      } catch (err: unknown) {
        setErrorMovimiento(normalizarError(err));
      }
    }
  };

  const sync = useSincronizacion({
    sesion: Boolean(usuarioSesion),
    usuarioId: usuarioSesion?.id ?? null,
    onSesionVencida: marcarSesionVencida,
    onSincronizado: alSincronizar,
  });

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
    if (mov.esGastoDeGrupo) return;
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

  const irAPantalla = (destino: Pantalla) => {
    setPantalla(destino);
    setVistaAuth("ninguna");
    if (destino === "cuentas") {
      setCuentaSeleccionadaId(null);
      void cargarDatos();
    }
  };

  let contenidoPantalla: React.ReactNode;

  if (vistaAuth !== "ninguna") {
    contenidoPantalla = (
      <div className="page-view">
        <AuthScreens
          vista={vistaAuth}
          usuario={usuarioSesion}
          onCambiarVista={setVistaAuth}
          onAutenticado={(u) => {
            setUsuarioSesion(u);
            setSesionVencida(false);
          }}
          onCerrarSesion={manejarCerrarSesion}
          onSesionVencida={marcarSesionVencida}
        />
      </div>
    );
  } else if (pantalla === "social") {
    contenidoPantalla = (
      <PantallaSocial
        key={usuarioSesion?.id ?? "sin-sesion"}
        usuarioId={usuarioSesion?.id ?? ""}
        volver={() => irAPantalla("cuentas")}
        sesion={Boolean(usuarioSesion)}
        sesionVencida={sesionVencida}
        onSesionVencida={marcarSesionVencida}
        cuentas={resumen.cuentas}
        sincronizar={sync.sincronizar}
        ultimaSync={sync.ultimaSync}
        alCambiarLibro={cargarDatos}
        iniciarSesion={() => {
          setPantalla("cuentas");
          setVistaAuth("login");
        }}
      />
    );
  } else if (pantalla === "patrimonio") {
    contenidoPantalla = (
      <PantallaPatrimonio
        onVolver={() => irAPantalla("cuentas")}
        onIr={() => irAPantalla("tasas")}
      />
    );
  } else if (pantalla === "tasas") {
    contenidoPantalla = (
      <PantallaTasas
        onVolver={() => irAPantalla("cuentas")}
        onIr={() => irAPantalla("patrimonio")}
      />
    );
  } else if (pantalla === "respaldo") {
    contenidoPantalla = (
      <PantallaRespaldo
        alCambiarLibro={cargarDatos}
        informeInicial={informeSyncPersonal}
      />
    );
  } else if (cuentaSeleccionadaId && cuentaSeleccionada && detalleCuenta) {
    contenidoPantalla = (
      <main className="page-view">
        <div className="detalle-back">
          <button
            type="button"
            className="btn btn-ghost btn-sm"
            onClick={() => setCuentaSeleccionadaId(null)}
          >
            ← Volver a cuentas
          </button>
        </div>

        {/* Tarjeta Hero de la Cuenta */}
        <section className="hero-card" aria-label="Resumen de cuenta">
          <div className="hero-top">
            <div>
              <span className="hero-eyebrow">Detalle de cuenta</span>
              <h1 className="hero-title">{cuentaSeleccionada.nombre}</h1>
              <div className="hero-meta-row">
                <span className="hero-badge">{cuentaSeleccionada.moneda}</span>
                <span className="hero-meta-text">
                  {cuentaSeleccionada.tipo}
                </span>
              </div>
            </div>

            <div className="hero-balance-block">
              <span className="hero-eyebrow">Saldo actual</span>
              <p className="hero-amount">
                {formatearMontoLocal(
                  detalleCuenta.saldoActual,
                  obtenerSimbolo(cuentaSeleccionada.moneda),
                )}
              </p>
            </div>
          </div>
        </section>

        {errorMovimiento && !mostrarFormMovimiento && (
          <div className="error-alert" role="alert">
            Revisa el campo «{errorMovimiento.campo}»: {errorMovimiento.mensaje}
          </div>
        )}

        {/* Composición del saldo (Sección 2.3) */}
        <section className="section-block">
          <h2 className="section-title">Composición del saldo</h2>
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
              <div className="subtotal-amount amount-positive">
                {formatearMontoLocal(
                  detalleCuenta.composicion.ingresos,
                  obtenerSimbolo(cuentaSeleccionada.moneda),
                  true,
                )}
              </div>
            </div>
            <div className="subtotal-card">
              <span className="subtotal-label">Total gastos</span>
              <div className="subtotal-amount amount-negative">
                {formatearMontoLocal(
                  `-${detalleCuenta.composicion.gastos}`,
                  obtenerSimbolo(cuentaSeleccionada.moneda),
                )}
              </div>
            </div>
            <div className="subtotal-card">
              <span className="subtotal-label">Total ajustes</span>
              <div className="subtotal-amount amount-adjust">
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
        <div className="section-header-bar">
          <h2 className="section-title">
            Historial de movimientos ({detalleCuenta.movimientos.length})
          </h2>
          <div className="acciones-grupo">
            <button
              type="button"
              onClick={() => abrirFormularioMovimiento("gasto")}
              className="btn btn-danger btn-sm"
            >
              − Registrar gasto
            </button>
            <button
              type="button"
              onClick={() => abrirFormularioMovimiento("ingreso")}
              className="btn btn-success btn-sm"
            >
              + Registrar ingreso
            </button>
          </div>
        </div>

        {/* Formulario Registrar Gasto / Ingreso */}
        {mostrarFormMovimiento && (
          <section className="form-card" aria-label="Registrar movimiento">
            <h3 className="form-title">
              Registrar {tipoMovimiento === "gasto" ? "gasto" : "ingreso"} en{" "}
              {cuentaSeleccionada.moneda}
            </h3>
            <p className="form-subtitle">
              La moneda del movimiento es siempre la de la cuenta (
              {cuentaSeleccionada.moneda}).
            </p>

            {errorMovimiento && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <div>
                    <strong>Revisa el campo «{errorMovimiento.campo}»: </strong>
                    <span>{errorMovimiento.mensaje}</span>
                  </div>
                </div>
              </div>
            )}

            {/* Advertencia de Saldo Negativo (RF-LIB-04) */}
            {advertenciaSaldoNegativo && (
              <div className="warning-alert" role="alert">
                <div>
                  <strong>Advertencia de saldo negativo: </strong>
                  <span>
                    Este gasto dejará el saldo de la cuenta en negativo. ¿Deseas
                    confirmarlo de todas formas?
                  </span>
                </div>
                <div className="warning-alert-actions">
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    disabled={guardando}
                    onClick={() => manejarRegistrarMovimiento(true)}
                  >
                    Sí, confirmar gasto
                  </button>
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
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
                  <label className="form-label" htmlFor="mov-tipo">
                    Tipo
                  </label>
                  <select
                    id="mov-tipo"
                    value={tipoMovimiento}
                    onChange={(e) => {
                      const nuevoTipo = e.target.value as "gasto" | "ingreso";
                      setTipoMovimiento(nuevoTipo);
                      const primera =
                        categorias.find((c) => c.tipo === nuevoTipo)?.nombre ??
                        "";
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
                  <label className="form-label" htmlFor="mov-monto">
                    Monto ({cuentaSeleccionada.moneda})
                  </label>
                  <input
                    id="mov-monto"
                    type="text"
                    inputMode="decimal"
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
                  <label className="form-label" htmlFor="mov-fecha">
                    Fecha
                  </label>
                  <input
                    id="mov-fecha"
                    type="date"
                    value={fechaMovimiento}
                    onChange={(e) => setFechaMovimiento(e.target.value)}
                    className={`form-control ${errorMovimiento?.campo === "fecha" ? "input-error" : ""}`}
                  />
                </div>
              </div>

              <div className="form-row">
                <div>
                  <div className="form-label-row">
                    <label className="form-label" htmlFor="mov-categoria">
                      Categoría
                    </label>
                    <button
                      type="button"
                      className="btn-link"
                      onClick={() =>
                        setMostrarNuevaCategoria(!mostrarNuevaCategoria)
                      }
                    >
                      {mostrarNuevaCategoria ? "Cerrar" : "+ Nueva categoría"}
                    </button>
                  </div>
                  <select
                    id="mov-categoria"
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
                    <div className="inline-input-row">
                      <input
                        type="text"
                        aria-label="Nombre de nueva categoría"
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
                        className="btn btn-secondary btn-sm"
                      >
                        Añadir
                      </button>
                    </div>
                  )}
                </div>

                <div>
                  <label className="form-label" htmlFor="mov-nota">
                    Nota (opcional)
                  </label>
                  <input
                    id="mov-nota"
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
          <section className="form-card" aria-label="Corregir movimiento">
            <h3 className="form-title">Corregir movimiento</h3>
            <p className="form-subtitle">
              El movimiento original ({movimientoACorregir.categoria} del{" "}
              {movimientoACorregir.fecha}) no se borrará: se registrará un
              ajuste por la diferencia. Si el movimiento no debió existir,
              escribe <strong>0</strong>.
            </p>

            {errorCorreccion && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <div>
                    <strong>Revisa el campo «{errorCorreccion.campo}»: </strong>
                    <span>{errorCorreccion.mensaje}</span>
                  </div>
                </div>
              </div>
            )}

            <form onSubmit={manejarCorregirMovimiento}>
              <div className="form-group">
                <label className="form-label" htmlFor="monto-correcto">
                  Monto correcto ({cuentaSeleccionada.moneda}) — Valor vigente
                  actual: {movimientoACorregir.montoVigente}
                </label>
                <input
                  id="monto-correcto"
                  type="text"
                  inputMode="decimal"
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
              <p className="empty-title">Sin movimientos aún</p>
              <p className="empty-description">
                No hay movimientos registrados en esta cuenta.
              </p>
            </div>
          ) : (
            <div className="accounts-list">
              {detalleCuenta.movimientos.map((mov) => {
                const esPositivo = !mov.monto.startsWith("-");
                const claseMonto = mov.esAjuste
                  ? "amount-adjust"
                  : esPositivo
                    ? "amount-positive"
                    : "amount-negative";
                const claseIcono = mov.esAjuste
                  ? "movement-icon-adjust"
                  : esPositivo
                    ? "movement-icon-positive"
                    : "movement-icon-negative";
                const simboloIcono = mov.esAjuste
                  ? "⇄"
                  : esPositivo
                    ? "↑"
                    : "↓";
                return (
                  <article key={mov.id} className="account-card">
                    <div className="account-card-leading">
                      <span
                        className={`movement-icon ${claseIcono}`}
                        aria-hidden="true"
                      >
                        {simboloIcono}
                      </span>
                      <div className="account-card-info">
                        <div className="account-name-row">
                          <span className="account-name">{mov.categoria}</span>
                          <span className="badge badge-uppercase">
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
                          {mov.esGastoDeGrupo && (
                            <span className="badge badge-grupo">
                              Gasto de grupo
                            </span>
                          )}
                        </div>
                        <p className="account-meta text-normal">
                          Fecha: {mov.fecha}
                          {mov.nota ? ` • ${mov.nota}` : ""} • ID: #
                          {mov.id.slice(0, 8)}
                        </p>
                      </div>
                    </div>

                    <div className="movement-right">
                      <div className="account-balance-box">
                        <span className={`account-balance-value ${claseMonto}`}>
                          {formatearMontoLocal(
                            mov.monto,
                            obtenerSimbolo(cuentaSeleccionada.moneda),
                            true,
                          )}
                        </span>
                      </div>

                      {/* Solo los movimientos originales se pueden corregir; no existe opción de editar ni eliminar (INV-04) */}
                      {!mov.esAjuste && !mov.esGastoDeGrupo && (
                        <button
                          type="button"
                          onClick={() => iniciarCorreccion(mov)}
                          className="btn btn-secondary btn-xs"
                        >
                          Corregir
                        </button>
                      )}
                    </div>
                  </article>
                );
              })}
            </div>
          )}
        </section>
      </main>
    );
  } else {
    contenidoPantalla = (
      <main className="page-view">
        {/* Tarjeta Hero Resumen de Cuentas estilo Cashea */}
        <section className="hero-card" aria-label="Resumen general de cuentas">
          <div className="hero-top">
            <div>
              <span className="hero-eyebrow">Cuentas Claras • Local-First</span>
              <h1 className="hero-title">Mis cuentas</h1>
              <p className="hero-subtitle">
                {resumen.cuentas.length === 0
                  ? "Crea tu primera cuenta para empezar a controlar tus saldos."
                  : `${resumen.cuentas.length} ${resumen.cuentas.length === 1 ? "cuenta registrada" : "cuentas registradas"} en este dispositivo`}
              </p>
            </div>
            <button
              type="button"
              onClick={() => {
                setError(null);
                setMostrarFormulario(!mostrarFormulario);
              }}
              className="btn btn-hero"
            >
              {mostrarFormulario ? "Cancelar" : "+ Nueva cuenta"}
            </button>
          </div>

          {resumen.subtotales.length > 0 && (
            <div className="hero-pills" aria-label="Subtotales por moneda">
              {resumen.subtotales.map((sub) => (
                <div key={sub.codigo} className="hero-chip">
                  <span className="hero-chip-label">
                    <span>Subtotal</span>
                    <span>{sub.codigo}</span>
                  </span>
                  <span className="hero-chip-value">
                    {formatearMontoLocal(sub.subtotal, sub.simbolo)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>

        {error && !mostrarFormulario && (
          <div className="error-alert" role="alert">
            {error.mensaje}
          </div>
        )}
        {error?.campo === "almacenamiento" && !mostrarFormulario && (
          <DiagnosticoLibro onRespaldar={() => irAPantalla("respaldo")} />
        )}
        {errorMovimiento && !mostrarFormMovimiento && (
          <div className="error-alert" role="alert">
            Revisa el campo «{errorMovimiento.campo}»: {errorMovimiento.mensaje}
          </div>
        )}

        {mostrarFormulario && (
          <section className="form-card" aria-label="Registrar nueva cuenta">
            <h2 className="form-title">Registrar nueva cuenta</h2>
            <p className="form-subtitle">
              Los datos se guardan únicamente en este dispositivo. La moneda no
              podrá cambiarse después de creada.
            </p>

            {error && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <div>
                    <strong>Revisa el campo «{error.campo}»: </strong>
                    <span>{error.mensaje}</span>
                  </div>
                </div>
              </div>
            )}

            <form onSubmit={manejarCrearCuenta}>
              <div className="form-group">
                <label className="form-label" htmlFor="cuenta-nombre">
                  Nombre de la cuenta
                </label>
                <input
                  id="cuenta-nombre"
                  type="text"
                  value={nombre}
                  onChange={(e) => setNombre(e.target.value)}
                  placeholder="Ej: Cartera, Cuenta nómina, Billetera dólares"
                  className={`form-control ${error?.campo === "nombre" ? "input-error" : ""}`}
                />
              </div>

              <div className="form-row">
                <div>
                  <label className="form-label" htmlFor="cuenta-tipo">
                    Tipo de cuenta
                  </label>
                  <select
                    id="cuenta-tipo"
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
                  <label className="form-label" htmlFor="cuenta-moneda">
                    Moneda
                  </label>
                  <select
                    id="cuenta-moneda"
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
                  <label className="form-label" htmlFor="cuenta-saldo-inicial">
                    Saldo inicial
                  </label>
                  <input
                    id="cuenta-saldo-inicial"
                    type="text"
                    inputMode="decimal"
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
                type="button"
                onClick={() => setMostrarFormulario(true)}
                className="btn btn-primary"
              >
                Crear mi primera cuenta
              </button>
            </div>
          ) : (
            <div className="accounts-grid">
              {resumen.cuentas.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className="account-card account-card-clickable"
                  onClick={() => seleccionarCuenta(c.id)}
                >
                  <div className="account-card-leading">
                    <span className="account-avatar">
                      <IconoTipoCuenta tipo={c.tipo} />
                    </span>
                    <div className="account-card-info">
                      <div className="account-name-row">
                        <span className="account-name">{c.nombre}</span>
                        <span className={obtenerClaseBadgeMoneda(c.moneda)}>
                          {c.moneda}
                        </span>
                      </div>
                      <span className="account-meta">{c.tipo}</span>
                    </div>
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
                </button>
              ))}
            </div>
          )}
        </section>
      </main>
    );
  }

  return (
    <AppShell
      pantallaActual={pantalla}
      onCambiarPantalla={irAPantalla}
      usuario={usuarioSesion}
      sesionVencida={sesionVencida}
      vistaAuth={vistaAuth}
      onCambiarVistaAuth={setVistaAuth}
      onCerrarSesion={manejarCerrarSesion}
      sincronizacion={sync}
      estadoServidor={estadoServidor}
      probandoServidor={probandoServidor}
      onProbarServidor={probarServidor}
    >
      {contenidoPantalla}
    </AppShell>
  );
}
