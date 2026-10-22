"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

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

interface Movimiento {
  id: string;
  fecha: string;
  tipo: "ingreso" | "gasto" | "ajuste";
  categoria: string;
  nota?: string;
  monto: string;
  corregido: boolean;
  esAjuste: boolean;
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
function formatearMontoLocal(cantidadStr: string, simbolo: string): string {
  const num = Number(cantidadStr);
  const formateado = new Intl.NumberFormat("es-VE", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(Math.abs(num));
  return `${num < 0 ? "- " : ""}${simbolo} ${formateado}`;
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

export default function MisCuentasPage() {
  // --- ESTADOS ---
  const [resumen, setResumen] = useState<ResumenCuentas>({ cuentas: [], subtotales: [] });
  const [monedas, setMonedas] = useState<Moneda[]>([]);
  const [mostrarFormulario, setMostrarFormulario] = useState(false);
  
  const [nombre, setNombre] = useState("");
  const [tipo, setTipo] = useState("efectivo");
  const [moneda, setMoneda] = useState(""); // Estado inicial vacío para el placeholder
  const [saldoInicial, setSaldoInicial] = useState("0.00");
  const [error, setError] = useState<ErrorDominio | null>(null);

  const [cuentaSeleccionadaId, setCuentaSeleccionadaId] = useState<string | null>(null);
  const [detalleCuenta, setDetalleCuenta] = useState<DetalleCuenta | null>(null);

  // --- EFECTOS ---
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

  // Efecto para redimensionar la ventana de Tauri dinámicamente
  useEffect(() => {
    const ajustarTamanoVentana = async () => {
      try {
        const numCuentas = resumen.cuentas.length;
        // Si hay una cuenta seleccionada, la ventana debe ser más ancha para acomodar la vista de detalles
        const anchoBase = cuentaSeleccionadaId ? 1100 : 600; 
        const altoBase = 500;  

        const crecimientoAlto = 80; 
        const maximoAncho = 1400;
        const maximoAlto = 900;

        const nuevoAlto = Math.min(altoBase + (numCuentas * crecimientoAlto), maximoAlto);
        
        const appWindow = getCurrentWindow();
        await appWindow.setSize(new LogicalSize(anchoBase, nuevoAlto));
      } catch (error) {
        console.error("Error al redimensionar la ventana:", error);
      }
    };

    if (resumen.cuentas) {
      ajustarTamanoVentana();
    }
  }, [resumen.cuentas.length, cuentaSeleccionadaId]); 

  // --- MANEJADORES ---
  const obtenerSimbolo = (codigo: string) => monedas.find((m) => m.codigo === codigo)?.simbolo ?? codigo;

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
    } catch (err: any) {
      setError(err as ErrorDominio);
    }
  };

  const seleccionarCuenta = async (cuentaId: string) => {
    setCuentaSeleccionadaId(cuentaId);
    setMostrarFormulario(false); 
    setError(null);

    const saldoActualReal = cuentaSeleccionada ? cuentaSeleccionada.saldo.cantidad : "0.00";
    const inicialReal = cuentaSeleccionada ? cuentaSeleccionada.saldo.cantidad : "0.00";
    
    // Simulación de los datos para la Entrega 3 mientras se conecta con invoke("obtener_detalle_cuenta")
    setDetalleCuenta({
      saldoActual: saldoActualReal,
      composicion: {
        inicial: inicialReal,
        ingresos: "0",
        gastos: "0",
        ajustes: "0"
      },
      movimientos: [] // Aquí irán los movimientos mockeados
    });
  };

  const cuentaSeleccionada = resumen.cuentas.find(c => c.id === cuentaSeleccionadaId);

  // --- RENDER ---
  return (
    <div className={`app-wrapper ${cuentaSeleccionadaId ? 'layout-split' : 'layout-single'}`}>
      
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
                    <option value="" disabled hidden>Seleccione...</option>
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

        {/* Subtotales */}
        
          <section className="section-block">
            <h2 className="section-title">Subtotales por moneda</h2>
            <div className="subtotals-grid">
              {resumen.subtotales.map((sub) => (
                <div key={sub.codigo} className="subtotal-card">
		  <span className="subtotal-label"></span>
                  <span className={obtenerClaseBadgeMoneda(sub.codigo)}>{sub.codigo}</span>
                  <div className="subtotal-amount">
                    {formatearMontoLocal(sub.subtotal, sub.simbolo)}
                  </div>
                </div>
              ))}
            </div>
          </section>
       

        <section className="section-block">
          <h2 className="section-title">Cuentas registradas ({resumen.cuentas.length})</h2>
          
          {resumen.cuentas.length === 0 ? (
            <div className="empty-state">
              <p className="empty-title">Todavía no tienes cuentas registradas</p>
	      <p className="empty-description">
              Crea tu primera cuenta de efectivo, banco o billetera digital para empezar a llevar el control en este dispositivo.
            </p>
              <button onClick={() => setMostrarFormulario(true)} className="btn btn-primary">
                Crear mi primera cuenta
              </button>
            </div>
          ) : (
            <div className="accounts-list">
              {resumen.cuentas.map((c) => (
                <div 
                  key={c.id} 
                  className={`account-card ${c.id === cuentaSeleccionadaId ? 'account-card-active' : ''}`}
                  onClick={() => seleccionarCuenta(c.id)}
                  style={{ cursor: 'pointer' }}
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
          )}
        </section>
      </aside>

      {/* PANEL DERECHO: DETALLE DE LA CUENTA */}
      {cuentaSeleccionadaId && cuentaSeleccionada && detalleCuenta && (
        <main className="content-detalle">
          
          <button className="btn btn-ghost" onClick={() => setCuentaSeleccionadaId(null)}>
            <span style={{ fontSize: "28px", lineHeight: "0.8" }}>←</span>
          </button>
          
          <header className="detalle-header">
            <div className="detalle-header-top">
                {/* Izquierda: Nombre de la cuenta */}
                <div className="detalle-info">
                  <h2 className="detalle-titulo">{cuentaSeleccionada.nombre}</h2>
                  <span className={obtenerClaseBadgeMoneda(cuentaSeleccionada.moneda)}>
                    {cuentaSeleccionada.moneda}
                  </span>
                </div>

                {/* Derecha: Saldo actual */}
                <div className="detalle-saldo">
                  <p className="detalle-subtitulo">Saldo Actual</p>
                  <h1 className="saldo-gigante">
                    {formatearMontoLocal(
                      cuentaSeleccionada.saldo.cantidad, 
                      obtenerSimbolo(cuentaSeleccionada.moneda)
                    )}
                  </h1>
                </div>
              </div>
          </header>

          <section className="composicion-saldo">
            <h3 className="section-title">Detalle del saldo</h3>
            <div className="subtotal-grid">
              <div className="subtotal-card">
                <div className="acciones-movimientos">
                  <span className="subtotal-amount">Inicial</span> 
                  <span className="subtotal-amount">{cuentaSeleccionada.saldo.cantidad}</span>
                </div>
              </div>
              <div className="subtotal-card">
                <div className="acciones-movimientos">
                  <span className="subtotal-amount" style={{color: "var(--text-success)"}}>Ingresos</span> 
                  <span className="subtotal-amount" style={{color: "var(--text-success)"}}>{detalleCuenta.composicion.ingresos}</span>
                </div>
              </div>
              <div className="subtotal-card">
                <div className="acciones-movimientos">
                  <span className="subtotal-amount" style={{color: "var(--text-danger)"}}>Gastos</span> 
                  <span className="subtotal-amount" style={{color: "var(--text-danger)"}}>{detalleCuenta.composicion.gastos}</span>
                </div>
              </div>
              <div className="subtotal-card">
                <div className="acciones-movimientos">
                  <span className="subtotal-amount" style={{color: "var(--text-ajust)"}}>Ajustes</span> 
                  <span className="subtotal-amount" style={{color: "var(--text-ajust)"}}>{detalleCuenta.composicion.ajustes}</span>
                </div>
              </div>
            </div>
          </section>
          
          <h3 className="section-title">Historial de Movimientos</h3>
          <section className="lista-movimientos">
            <div className="acciones-movimientos">
              <button className="btn btn-danger">- Registrar Gasto</button>
              <button className="btn btn-success">+ Registrar Ingreso</button>
            </div>
            {detalleCuenta.movimientos.length === 0 ? (
              <div className="empty-state">No hay movimientos registrados.</div>
            ) : (
              <div className="movimientos-container">
                {/* Aquí irá el .map de los movimientos  */}
              </div>
            )}
          </section>
        </main>
      )}
    </div>
  );
}