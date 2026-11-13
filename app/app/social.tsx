"use client";

import VistaGasto from "@/app/VistaGasto";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

type Amigo = {
  id: string;
  usuario_id: string;
  nombre_usuario: string;
  nombre_visible: string;
};
type Solicitud = { id: string; emisor_id: string; destinatario_id: string };
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
  es_admin: boolean;
};
type Miembro = {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  es_admin: boolean;
};
type Detalle = Omit<Grupo, "es_admin"> & { miembros: Miembro[] };
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
  volver,
  sesion,
  iniciarSesion,
  sesionVencida,
  onSesionVencida,
}: {
  volver: () => void;
  sesion: boolean;
  iniciarSesion: () => void;
  sesionVencida: boolean;
  onSesionVencida: () => void;
}) {
  const [seccion, setSeccion] = useState<Seccion>("amigos");
  const [amistades, setAmistades] = useState<Amistades | null>(null);
  const [grupos, setGrupos] = useState<Grupos | null>(null);
  const [detalle, setDetalle] = useState<Detalle | null>(null);
  const [busqueda, setBusqueda] = useState("");
  const [encontrado, setEncontrado] = useState<Usuario | null>(null);
  const [nombre, setNombre] = useState("");
  const [descripcion, setDescripcion] = useState("");
  const [moneda, setMoneda] = useState("USD");
  const [seleccion, setSeleccion] = useState("");
  const [error, setError] = useState("");
  const [aviso, setAviso] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [cargandoInicial, setCargandoInicial] = useState(true);

  const cargar = useCallback(async () => {
    const [a, g] = await Promise.all([
      llamar<Amistades>("listar_amistades"),
      llamar<Grupos>("listar_grupos"),
    ]);
    setAmistades(a);
    setGrupos(g);
  }, []);
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
          else setError(errorTexto(e));
        }
      })
      .finally(() => {
        if (activo) setCargandoInicial(false);
      });
    return () => {
      activo = false;
    };
  }, [sesion, onSesionVencida]);
  async function ejecutar(accion: () => Promise<void>, mensaje = "") {
    setOcupado(true);
    setError("");
    setAviso("");
    try {
      await accion();
      setAviso(mensaje);
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
    setDetalle(await llamar<Detalle>("detalle_grupo", { id }));
    setSeleccion("");
  }
  const admin =
    detalle && grupos?.grupos.find((g) => g.id === detalle.id)?.es_admin;
  return (
    <main className="app-container social-page">
      <header className="header">
        <div>
          <span className="header-tag">Cuentas Claras • Compartido</span>
          <h1 className="header-title">Amigos y grupos</h1>
        </div>
        <button className="btn btn-ghost" type="button" onClick={volver}>
          Volver a mis cuentas
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
          <nav className="social-tabs" aria-label="Secciones compartidas">
            <button
              type="button"
              className={`btn ${seccion === "amigos" ? "btn-primary" : "btn-ghost"}`}
              onClick={() => {
                setSeccion("amigos");
                setDetalle(null);
              }}
            >
              Amigos
            </button>
            <button
              type="button"
              className={`btn ${seccion === "grupos" ? "btn-primary" : "btn-ghost"}`}
              onClick={() => setSeccion("grupos")}
            >
              Grupos
            </button>
          </nav>
          {error && (
            <div className="error-alert" role="alert">
              <p>{error}</p>
              {!amistades && !grupos && !cargandoInicial && (
                <button
                  type="button"
                  className="btn btn-ghost"
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
                      <span>Usuario {s.emisor_id.slice(0, 8)}</span>
                      <div className="social-actions">
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
                      <span>Usuario {s.destinatario_id.slice(0, 8)}</span>
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
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => setDetalle(null)}
                >
                  ← Mis grupos
                </button>
                <h2 className="form-title">{detalle.nombre}</h2>
                <p className="social-muted">
                  {detalle.descripcion || "Sin descripción"} · {detalle.moneda}
                </p>
                <h3>Miembros</h3>
                {detalle.miembros.map((m) => (
                  <div className="social-row" key={m.id}>
                    <span>
                      {m.nombre_visible}{" "}
                      <span className="social-muted">@{m.nombre_usuario}</span>
                    </span>
                    {m.es_admin && (
                      <span className="social-muted">Administrador</span>
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
              <VistaGasto miembros={detalle.miembros} moneda={detalle.moneda} />
              <section className="form-card">
                <h2 className="form-title">Gastos y balances</h2>
                <p className="social-muted">
                  El registro de gastos y los balances estarán disponibles
                  cuando se complete su conexión con el servidor.
                </p>
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
                        moneda,
                      });
                      setNombre("");
                      setDescripcion("");
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
                        <strong>{g.nombre}</strong>
                        <p className="social-muted">
                          {g.moneda} ·{" "}
                          {g.es_admin ? "Administrador" : "Miembro"}
                        </p>
                      </div>
                      <button
                        className="btn btn-secondary"
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
