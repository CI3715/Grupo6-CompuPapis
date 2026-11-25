"use client";

import VistaGasto from "@/app/VistaGasto";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Cuenta, GastoEnCola, PendienteAplicar } from "@/lib/tipos";
import {
  claveCacheGrupos,
  guardarCacheSocial,
  leerCacheSocial,
} from "@/lib/social-cache";
import { obtenerClaseBadgeMoneda } from "@/lib/utilidades";

type Amigo = {
  id: string;
  usuario_id: string;
  nombre_usuario: string;
  nombre_visible: string;
};
type Solicitud = {
  id: string;
  emisor_id: string;
  destinatario_id: string;
  nombre_usuario: string;
  nombre_visible: string;
};
type Amistades = {
  amigos: Amigo[];
  enviadas: Solicitud[];
  recibidas: Solicitud[];
};
type Grupo = {
  id: string;
  nombre: string;
  descripcion: string | null;
  moneda: string;
  actividad: string | null;
  es_admin: boolean;
};
type Miembro = {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  es_admin: boolean;
};
type Parte = {
  usuario_id: string;
  nombre_usuario: string;
  monto: string;
  recibio_residuo: boolean | null;
};
type Gasto = {
  id: string;
  pagado_por: string;
  concepto: string;
  monto: string;
  fecha: string;
  partes: Parte[];
};
type Balance = {
  usuario_id: string;
  nombre_visible: string;
  nombre_usuario: string;
  pagado: string;
  corresponde: string;
  balance: string;
};
type Detalle = Omit<Grupo, "es_admin"> & {
  miembros: Miembro[];
  gastos: Gasto[];
  balances: Balance[];
};
type Grupos = { grupos: Grupo[]; invitaciones: Grupo[] };
type Usuario = { nombre_usuario: string; nombre_visible: string };
type Seccion = "amigos" | "grupos";
const llamar = <T,>(accion: string, datos: object = {}) =>
  invoke<T>("operacion_social", { accion, datos });
const errorTexto = (error: unknown) =>
  typeof error === "object" && error !== null && "mensaje" in error
    ? String(error.mensaje)
    : "No se pudo completar la operación.";

export default function PantallaSocial({
  usuarioId,
  volver,
  sesion,
  iniciarSesion,
  sesionVencida,
  onSesionVencida,
  cuentas,
  sincronizar,
  ultimaSync,
  alCambiarLibro,
}: {
  usuarioId: string;
  volver: () => void;
  sesion: boolean;
  iniciarSesion: () => void;
  sesionVencida: boolean;
  onSesionVencida: () => void;
  cuentas: Cuenta[];
  sincronizar: () => Promise<void>;
  ultimaSync: string | null;
  alCambiarLibro: () => Promise<void>;
}) {
  const [seccion, setSeccion] = useState<Seccion>("amigos");
  const [amistades, setAmistades] = useState<Amistades | null>(null);
  const [grupos, setGrupos] = useState<Grupos | null>(null);
  const [detalle, setDetalle] = useState<Detalle | null>(null);
  const [busqueda, setBusqueda] = useState("");
  const [encontrado, setEncontrado] = useState<Usuario | null>(null);
  const [nombre, setNombre] = useState("");
  const [descripcion, setDescripcion] = useState("");
  const [actividad, setActividad] = useState("");
  const [moneda, setMoneda] = useState("USD");
  const [seleccion, setSeleccion] = useState("");
  const [error, setError] = useState("");
  const [aviso, setAviso] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [cargandoInicial, setCargandoInicial] = useState(true);
  const [cuentaGrupo, setCuentaGrupo] = useState("");
  const [pendientes, setPendientes] = useState<PendienteAplicar[]>([]);
  const [cola, setCola] = useState<GastoEnCola[]>([]);
  const [confirmarDatosAnteriores, setConfirmarDatosAnteriores] =
    useState(false);

  const claveCache = claveCacheGrupos(usuarioId);

  const cargar = useCallback(async () => {
    const [a, g] = await Promise.all([
      llamar<Amistades>("listar_amistades"),
      llamar<Grupos>("listar_grupos"),
    ]);
    setAmistades(a);
    setGrupos(g);
    guardarCacheSocial(() => localStorage, claveCache, g);
  }, [claveCache]);
  useEffect(() => {
    if (!sesion) return;
    let activo = true;
    Promise.all([
      llamar<Amistades>("listar_amistades"),
      llamar<Grupos>("listar_grupos"),
    ])
      .then(([a, g]) => {
        if (activo) {
          setAmistades(a);
          setGrupos(g);
          guardarCacheSocial(() => localStorage, claveCache, g);
        }
      })
      .catch((e: unknown) => {
        if (activo) {
          if (
            typeof e === "object" &&
            e !== null &&
            "campo" in e &&
            e.campo === "sesion"
          )
            onSesionVencida();
          else {
            const guardados = leerCacheSocial<Grupos>(
              () => localStorage,
              claveCache,
            );
            if (guardados) setGrupos(guardados);
            setError(errorTexto(e));
          }
        }
      })
      .finally(() => {
        if (activo) setCargandoInicial(false);
      });
    return () => {
      activo = false;
    };
  }, [sesion, onSesionVencida, claveCache]);
  const cargarLocal = useCallback(async () => {
    const [p, c] = await Promise.all([
      invoke<PendienteAplicar[]>("listar_pendientes_aplicar"),
      invoke<GastoEnCola[]>("listar_cola_gastos"),
    ]);
    setPendientes(p);
    setCola(c);
  }, []);
  // Se vuelve a leer cada vez que termina una sincronización
  useEffect(() => {
    if (!sesion) return;
    let activo = true;
    queueMicrotask(() => {
      if (activo) void cargarLocal().catch(() => undefined);
    });
    return () => {
      activo = false;
    };
  }, [sesion, ultimaSync, cargarLocal]);

  async function ejecutar(accion: () => Promise<void>, mensaje = "") {
    setOcupado(true);
    setError("");
    setAviso("");
    try {
      await accion();
      if (mensaje) setAviso(mensaje);
    } catch (e: unknown) {
      if (
        typeof e === "object" &&
        e !== null &&
        "campo" in e &&
        e.campo === "sesion"
      )
        onSesionVencida();
      else setError(errorTexto(e));
    } finally {
      setOcupado(false);
    }
  }

  async function abrir(id: string) {
    const claveDetalle = claveCacheGrupos(usuarioId, id);
    try {
      const datos = await llamar<Detalle>("detalle_grupo", { id });
      guardarCacheSocial(() => localStorage, claveDetalle, datos);
      setDetalle(datos);
    } catch (error) {
      if (
        typeof error === "object" &&
        error !== null &&
        "campo" in error &&
        error.campo === "sesion"
      )
        throw error;
      const guardado = leerCacheSocial<Detalle>(
        () => localStorage,
        claveDetalle,
      );
      if (!guardado) throw error;
      setDetalle(guardado);
      setAviso(
        "Mostrando datos guardados del grupo. Los cambios se enviarán al recuperar la conexión.",
      );
    }
    setSeleccion("");
    setCuentaGrupo(
      (await invoke<string | null>("obtener_cuenta_grupo", { grupoId: id })) ??
        "",
    );
  }

  async function elegirCuenta(grupoId: string, cuentaId: string) {
    await invoke("elegir_cuenta_grupo", { grupoId, cuentaId });
    setCuentaGrupo(cuentaId);
    await cargarLocal();
    await alCambiarLibro();
  }

  const admin =
    detalle && grupos?.grupos.find((g) => g.id === detalle.id)?.es_admin;
  const nombreGrupo = (id: string) =>
    grupos?.grupos.find((g) => g.id === id)?.nombre ?? "Grupo";
  return (
    <main className="app-container social-page">
      <header className="header">
        <div>
          <span className="header-tag">Cuentas Claras • Compartido</span>
          <h1 className="header-title">Amigos y grupos</h1>
        </div>
        <button className="btn btn-ghost" type="button" onClick={volver}>
          ← Cuentas
        </button>
      </header>
      {!sesion ? (
        <section className="form-card">
          <h2 className="form-title">
            {sesionVencida ? "Tu sesión venció" : "Comparte con tus amigos"}
          </h2>
          <p className="social-muted">
            Inicia sesión para gestionar amistades y grupos. Tus cuentas
            personales siguen disponibles sin conexión.
          </p>
          <button
            className="btn btn-primary"
            type="button"
            onClick={iniciarSesion}
          >
            Iniciar sesión
          </button>
        </section>
      ) : (
        <>
          <nav
            className="social-tabs"
            role="tablist"
            aria-label="Secciones compartidas"
          >
            <button
              type="button"
              role="tab"
              aria-selected={seccion === "amigos"}
              className={`btn btn-sm ${seccion === "amigos" ? "btn-primary" : "btn-ghost"}`}
              onClick={() => {
                setSeccion("amigos");
                setDetalle(null);
              }}
            >
              Amigos
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={seccion === "grupos"}
              className={`btn btn-sm ${seccion === "grupos" ? "btn-primary" : "btn-ghost"}`}
              onClick={() => setSeccion("grupos")}
            >
              Grupos
            </button>
          </nav>
          {error && (
            <div className="error-alert" role="alert">
              <p>{error}</p>
              {error.includes("sin usuario identificado") && (
                <div>
                  <label className="checkbox-row">
                    <input
                      type="checkbox"
                      checked={confirmarDatosAnteriores}
                      onChange={(e) =>
                        setConfirmarDatosAnteriores(e.target.checked)
                      }
                    />
                    <span>
                      Confirmo que los datos anteriores de sincronización
                      pertenecen a mi usuario actual.
                    </span>
                  </label>
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm mt-sm"
                    disabled={!confirmarDatosAnteriores || ocupado}
                    onClick={() =>
                      void ejecutar(async () => {
                        await invoke("vincular_datos_anteriores");
                        setConfirmarDatosAnteriores(false);
                        await cargar();
                        await cargarLocal();
                        await sincronizar();
                      }, "Datos anteriores vinculados.")
                    }
                  >
                    Vincular mis datos anteriores
                  </button>
                </div>
              )}
              {!amistades && !grupos && !cargandoInicial && (
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  disabled={ocupado}
                  onClick={() => void ejecutar(cargar)}
                >
                  Reintentar
                </button>
              )}
            </div>
          )}
          {aviso && (
            <p role="status" className="social-notice">
              {aviso}
            </p>
          )}
          {seccion === "grupos" && pendientes.length > 0 && (
            <section className="form-card">
              <h2 className="form-title">Partes pendientes de aplicar</h2>
              <p className="social-notice" role="status">
                Estas partes llegaron de tus grupos, pero aún no elegiste la
                cuenta donde aplicarlas. Abre el grupo y elige una cuenta: se
                aplicarán en cuanto la elijas.
              </p>
              {pendientes.map((p) => (
                <div className="social-row" key={p.id}>
                  <div>
                    <strong>
                      {nombreGrupo(p.grupo_id)} · {p.concepto}
                    </strong>
                    <p className="social-muted">
                      {p.fecha}
                      {p.recibio_residuo ? " · incluye 0,01 de redondeo" : ""}
                    </p>
                  </div>
                  <strong>
                    {p.monto_parte} {p.moneda}
                  </strong>
                </div>
              ))}
            </section>
          )}
          {seccion === "grupos" && cola.length > 0 && (
            <section className="form-card">
              <h2 className="form-title">Gastos en cola</h2>
              {cola.map((g) =>
                g.estado === "rechazado" ? (
                  <div className="error-alert" role="alert" key={g.clave}>
                    <div>
                      <strong>No se pudo enviar «{g.concepto}»: </strong>
                      <span>
                        {g.motivo ?? "El servidor rechazó la operación."}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      disabled={ocupado}
                      onClick={() =>
                        void ejecutar(async () => {
                          await invoke("descartar_rechazo", { clave: g.clave });
                          await cargarLocal();
                        })
                      }
                    >
                      Entendido
                    </button>
                  </div>
                ) : (
                  <div className="social-row" key={g.clave}>
                    <div>
                      <strong>{g.concepto}</strong>
                      <p className="social-muted">Pendiente de envío</p>
                    </div>
                    <strong>{g.monto}</strong>
                  </div>
                ),
              )}
            </section>
          )}
          {seccion === "amigos" ? (
            <div className="social-grid">
              <section className="form-card">
                <h2 className="form-title">Buscar amigos</h2>
                <p className="social-muted">
                  Busca el nombre de usuario exacto.
                </p>
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void ejecutar(async () => {
                      setEncontrado(
                        await llamar<Usuario>("buscar_usuario", {
                          usuario: busqueda.trim(),
                        }),
                      );
                    });
                  }}
                >
                  <label className="form-label" htmlFor="buscar-amigo">
                    Nombre de usuario
                  </label>
                  <div className="social-inline">
                    <input
                      id="buscar-amigo"
                      className="form-control"
                      disabled={ocupado}
                      value={busqueda}
                      onChange={(e) => {
                        setBusqueda(e.target.value);
                        setEncontrado(null);
                      }}
                      required
                    />
                    <button className="btn btn-primary" disabled={ocupado}>
                      Buscar
                    </button>
                  </div>
                </form>
                {encontrado && (
                  <div className="social-row">
                    <div>
                      <strong>{encontrado.nombre_visible}</strong>
                      <p className="social-muted">
                        @{encontrado.nombre_usuario}
                      </p>
                    </div>
                    <button
                      className="btn btn-secondary"
                      disabled={ocupado}
                      onClick={() =>
                        void ejecutar(async () => {
                          await llamar("invitar_amistad", {
                            nombre_usuario: encontrado.nombre_usuario,
                          });
                          setEncontrado(null);
                          await cargar();
                        }, "Solicitud enviada.")
                      }
                    >
                      Invitar
                    </button>
                  </div>
                )}
              </section>
              <section className="form-card">
                <h2 className="form-title">Mis amigos</h2>
                {!amistades ? (
                  <p className="social-muted">
                    {cargandoInicial
                      ? "Cargando amistades…"
                      : "No se pudieron cargar las amistades."}
                  </p>
                ) : amistades.amigos.length === 0 ? (
                  <p className="social-muted">
                    Aún no tienes amigos agregados.
                  </p>
                ) : (
                  amistades.amigos.map((a) => (
                    <div className="social-row" key={a.id}>
                      <div>
                        <strong>{a.nombre_visible}</strong>
                        <p className="social-muted">@{a.nombre_usuario}</p>
                      </div>
                      <button
                        className="btn btn-danger"
                        disabled={ocupado}
                        onClick={() =>
                          void ejecutar(async () => {
                            await llamar("eliminar_amistad", { id: a.id });
                            await cargar();
                          }, "Amistad eliminada.")
                        }
                      >
                        Eliminar
                      </button>
                    </div>
                  ))
                )}
              </section>
              <section className="form-card">
                <h2 className="form-title">Solicitudes recibidas</h2>
                {cargandoInicial ? (
                  <p className="social-muted">Cargando solicitudes…</p>
                ) : !amistades ? (
                  <p className="social-muted">
                    No se pudieron cargar las solicitudes.
                  </p>
                ) : amistades.recibidas.length ? (
                  amistades.recibidas.map((s) => (
                    <div className="social-row" key={s.id}>
                      <span>
                        {s.nombre_visible}{" "}
                        <span className="social-muted">
                          @{s.nombre_usuario}
                        </span>
                      </span>
                      <div className="social-actions social-actions-columna">
                        <button
                          className="btn btn-primary"
                          disabled={ocupado}
                          onClick={() =>
                            void ejecutar(async () => {
                              await llamar("responder_amistad", {
                                id: s.id,
                                aceptar: true,
                              });
                              await cargar();
                            }, "Solicitud aceptada.")
                          }
                        >
                          Aceptar
                        </button>
                        <button
                          className="btn btn-ghost"
                          disabled={ocupado}
                          onClick={() =>
                            void ejecutar(async () => {
                              await llamar("responder_amistad", {
                                id: s.id,
                                aceptar: false,
                              });
                              await cargar();
                            }, "Solicitud rechazada.")
                          }
                        >
                          Rechazar
                        </button>
                      </div>
                    </div>
                  ))
                ) : (
                  <p className="social-muted">
                    No tienes solicitudes recibidas.
                  </p>
                )}
              </section>
              <section className="form-card">
                <h2 className="form-title">Solicitudes enviadas</h2>
                {cargandoInicial ? (
                  <p className="social-muted">Cargando solicitudes…</p>
                ) : !amistades ? (
                  <p className="social-muted">
                    No se pudieron cargar las solicitudes.
                  </p>
                ) : amistades.enviadas.length ? (
                  amistades.enviadas.map((s) => (
                    <div className="social-row" key={s.id}>
                      <span>
                        {s.nombre_visible}{" "}
                        <span className="social-muted">
                          @{s.nombre_usuario}
                        </span>
                      </span>
                      <span className="social-muted">Pendiente</span>
                    </div>
                  ))
                ) : (
                  <p className="social-muted">
                    No tienes solicitudes pendientes.
                  </p>
                )}
              </section>
            </div>
          ) : detalle ? (
            <div className="social-grid">
              <section className="form-card social-wide">
                <div className="detalle-back">
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
                    onClick={() => setDetalle(null)}
                  >
                    ← Grupos
                  </button>
                </div>
                <div className="account-name-row">
                  <h2 className="form-title">{detalle.nombre}</h2>
                  <span className={obtenerClaseBadgeMoneda(detalle.moneda)}>
                    {detalle.moneda}
                  </span>
                </div>
                <p className="social-muted">
                  {detalle.descripcion || "Sin descripción"}
                  {detalle.actividad ? ` · ${detalle.actividad}` : ""}
                </p>
                <h3 className="social-subheading">Miembros</h3>
                {detalle.miembros.map((m) => (
                  <div className="social-row" key={m.id}>
                    <span>
                      {m.nombre_visible}{" "}
                      <span className="social-muted">@{m.nombre_usuario}</span>
                    </span>
                    {m.es_admin && (
                      <span className="badge badge-grupo">Administrador</span>
                    )}
                  </div>
                ))}
              </section>
              {admin && (
                <section className="form-card">
                  <h2 className="form-title">Invitar a un amigo</h2>
                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      void ejecutar(async () => {
                        await llamar("invitar_grupo", {
                          id: detalle.id,
                          usuario_id: seleccion,
                        });
                        setSeleccion("");
                      }, "Invitación enviada.");
                    }}
                  >
                    <label className="form-label" htmlFor="amigo-grupo">
                      Amigo
                    </label>
                    <select
                      className="form-control"
                      id="amigo-grupo"
                      value={seleccion}
                      required
                      onChange={(e) => setSeleccion(e.target.value)}
                    >
                      <option value="">Selecciona un amigo</option>
                      {amistades?.amigos
                        .filter(
                          (a) =>
                            !detalle.miembros.some(
                              (m) => m.id === a.usuario_id,
                            ),
                        )
                        .map((a) => (
                          <option key={a.id} value={a.usuario_id}>
                            {a.nombre_visible} (@{a.nombre_usuario})
                          </option>
                        ))}
                    </select>
                    <button
                      className="btn btn-primary social-submit"
                      disabled={ocupado}
                    >
                      Enviar invitación
                    </button>
                  </form>
                </section>
              )}
              <section className="form-card">
                <h2 className="form-title">Cuenta para mis partes</h2>
                <p className="social-muted">
                  Tus partes de los gastos de este grupo se aplicarán en la
                  cuenta que elijas. Solo se ofrecen cuentas en {detalle.moneda}
                  . Si la cambias, afecta solo a las partes que lleguen después.
                </p>
                <label className="form-label" htmlFor="cuenta-grupo">
                  Cuenta
                </label>
                <select
                  className="form-control"
                  id="cuenta-grupo"
                  value={cuentaGrupo}
                  disabled={ocupado}
                  onChange={(e) => {
                    const cuentaId = e.target.value;
                    if (!cuentaId) return;
                    void ejecutar(
                      () => elegirCuenta(detalle.id, cuentaId),
                      "Cuenta del grupo guardada.",
                    );
                  }}
                >
                  <option value="">Selecciona una cuenta</option>
                  {cuentas
                    .filter((c) => c.moneda === detalle.moneda)
                    .map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.nombre} ({c.moneda})
                      </option>
                    ))}
                </select>
                {!cuentas.some((c) => c.moneda === detalle.moneda) && (
                  <p className="social-muted">
                    No tienes cuentas en {detalle.moneda}. Crea una en Mis
                    cuentas para poder recibir tus partes.
                  </p>
                )}
              </section>
              <VistaGasto
                key={detalle.id}
                miembros={detalle.miembros}
                moneda={detalle.moneda}
                registrar={async (datos) => {
                  let resultado: { pendiente?: boolean };
                  try {
                    resultado = await llamar<{ pendiente?: boolean }>(
                      "registrar_gasto",
                      { id: detalle.id, ...datos },
                    );
                  } catch (e: unknown) {
                    if (
                      typeof e === "object" &&
                      e !== null &&
                      "campo" in e &&
                      e.campo === "sesion"
                    )
                      onSesionVencida();
                    throw e;
                  }
                  void cargarLocal().catch(() => undefined);
                  void sincronizar();
                  if (!resultado.pendiente) {
                    try {
                      const actualizado = await llamar<Detalle>(
                        "detalle_grupo",
                        {
                          id: detalle.id,
                        },
                      );
                      setDetalle((actual) =>
                        actual?.id === detalle.id ? actualizado : actual,
                      );
                    } catch {
                      setError(
                        "El gasto se guardó, pero no se pudo actualizar el grupo. Vuelve a abrirlo para consultar los datos.",
                      );
                    }
                  }
                  return resultado;
                }}
              />
              <section className="form-card social-wide">
                <h2 className="form-title">Gastos</h2>
                {detalle.gastos.length === 0 ? (
                  <p className="social-muted">
                    Todavía no hay gastos registrados.
                  </p>
                ) : (
                  detalle.gastos.map((g) => (
                    <div className="social-row" key={g.id}>
                      <div>
                        <strong>{g.concepto}</strong>
                        <p className="social-muted">
                          {g.fecha} · Pagó{" "}
                          {detalle.miembros.find((m) => m.id === g.pagado_por)
                            ?.nombre_visible ?? "un miembro"}
                        </p>
                        {g.partes.map((p) => (
                          <p className="social-muted" key={p.usuario_id}>
                            @{p.nombre_usuario}: {p.monto} {detalle.moneda}
                            {p.recibio_residuo === true
                              ? " · incluye 0,01 de redondeo"
                              : ""}
                          </p>
                        ))}
                      </div>
                      <strong>
                        {g.monto} {detalle.moneda}
                      </strong>
                    </div>
                  ))
                )}
              </section>
              <section className="form-card social-wide">
                <h2 className="form-title">Balances</h2>
                <p className="social-muted">
                  Positivo: le deben. Negativo: debe.
                </p>
                {detalle.balances.map((b) => {
                  const esNegativo = b.balance.startsWith("-");
                  const esCero = Number(b.balance) === 0;
                  const claseBalance = esCero
                    ? ""
                    : esNegativo
                      ? "amount-negative"
                      : "amount-positive";
                  return (
                    <div className="social-row" key={b.usuario_id}>
                      <div>
                        <strong>{b.nombre_visible}</strong>
                        <p className="social-muted">
                          Pagó {b.pagado} · Le corresponde {b.corresponde}
                        </p>
                      </div>
                      <strong className={claseBalance}>
                        {b.balance} {detalle.moneda}
                      </strong>
                    </div>
                  );
                })}
              </section>
            </div>
          ) : (
            <div className="social-grid">
              <section className="form-card">
                <h2 className="form-title">Crear grupo</h2>
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void ejecutar(async () => {
                      await llamar("crear_grupo", {
                        nombre: nombre.trim(),
                        descripcion: descripcion.trim() || null,
                        actividad: actividad.trim() || null,
                        moneda,
                      });
                      setNombre("");
                      setDescripcion("");
                      setActividad("");
                      await cargar();
                    }, "Grupo creado.");
                  }}
                >
                  <label className="form-label" htmlFor="nombre-grupo">
                    Nombre
                  </label>
                  <input
                    id="nombre-grupo"
                    className="form-control"
                    value={nombre}
                    required
                    onChange={(e) => setNombre(e.target.value)}
                  />
                  <label
                    className="form-label social-label"
                    htmlFor="descripcion-grupo"
                  >
                    Descripción (opcional)
                  </label>
                  <input
                    id="descripcion-grupo"
                    className="form-control"
                    value={descripcion}
                    onChange={(e) => setDescripcion(e.target.value)}
                  />
                  <label
                    className="form-label social-label"
                    htmlFor="actividad-grupo"
                  >
                    Fecha o actividad (opcional)
                  </label>
                  <input
                    id="actividad-grupo"
                    className="form-control"
                    placeholder="Por ejemplo, viaje en marzo"
                    value={actividad}
                    onChange={(e) => setActividad(e.target.value)}
                  />
                  <label
                    className="form-label social-label"
                    htmlFor="moneda-grupo"
                  >
                    Moneda (código de tres letras)
                  </label>
                  <input
                    id="moneda-grupo"
                    className="form-control"
                    value={moneda}
                    required
                    pattern="[A-Z]{3}"
                    maxLength={3}
                    onChange={(e) => setMoneda(e.target.value.toUpperCase())}
                  />
                  <button
                    className="btn btn-primary social-submit"
                    disabled={ocupado}
                  >
                    Crear grupo
                  </button>
                </form>
              </section>
              <section className="form-card">
                <h2 className="form-title">Mis grupos</h2>
                {!grupos ? (
                  <p className="social-muted">
                    {cargandoInicial
                      ? "Cargando grupos…"
                      : "No se pudieron cargar los grupos."}
                  </p>
                ) : grupos.grupos.length ? (
                  grupos.grupos.map((g) => (
                    <div className="social-row" key={g.id}>
                      <div>
                        <div className="account-name-row">
                          <strong>{g.nombre}</strong>
                          <span className={obtenerClaseBadgeMoneda(g.moneda)}>
                            {g.moneda}
                          </span>
                        </div>
                        <p className="social-muted">
                          {g.es_admin ? "Administrador" : "Miembro"}
                        </p>
                      </div>
                      <button
                        type="button"
                        className="btn btn-secondary btn-sm"
                        disabled={ocupado}
                        onClick={() => void ejecutar(() => abrir(g.id))}
                      >
                        Ver grupo
                      </button>
                    </div>
                  ))
                ) : (
                  <p className="social-muted">
                    Todavía no perteneces a ningún grupo.
                  </p>
                )}
              </section>
              <section className="form-card social-wide">
                <h2 className="form-title">Invitaciones a grupos</h2>
                {cargandoInicial ? (
                  <p className="social-muted">Cargando invitaciones…</p>
                ) : !grupos ? (
                  <p className="social-muted">
                    No se pudieron cargar las invitaciones.
                  </p>
                ) : grupos.invitaciones.length ? (
                  grupos.invitaciones.map((g) => (
                    <div className="social-row" key={g.id}>
                      <div>
                        <strong>{g.nombre}</strong>
                        <p className="social-muted">{g.moneda}</p>
                      </div>
                      <div className="social-actions">
                        <button
                          className="btn btn-primary"
                          disabled={ocupado}
                          onClick={() =>
                            void ejecutar(async () => {
                              await llamar("responder_grupo", {
                                id: g.id,
                                aceptar: true,
                              });
                              await cargar();
                            }, "Invitación aceptada.")
                          }
                        >
                          Aceptar
                        </button>
                        <button
                          className="btn btn-ghost"
                          disabled={ocupado}
                          onClick={() =>
                            void ejecutar(async () => {
                              await llamar("responder_grupo", {
                                id: g.id,
                                aceptar: false,
                              });
                              await cargar();
                            }, "Invitación rechazada.")
                          }
                        >
                          Rechazar
                        </button>
                      </div>
                    </div>
                  ))
                ) : (
                  <p className="social-muted">
                    No tienes invitaciones pendientes.
                  </p>
                )}
              </section>
            </div>
          )}
        </>
      )}
    </main>
  );
}
