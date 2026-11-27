"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { EstadoSincronizacion } from "@/app/sincronizacion";
import { formatearFechaHora } from "@/lib/utilidades";

export interface UsuarioSesion {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  correo: string;
  fecha_alta?: string;
  expira_en?: string;
  vencimiento?: string;
}

interface ErrorDominio {
  campo: string;
  mensaje: string;
}

interface AuthBarProps {
  usuario: UsuarioSesion | null;
  sesionVencida: boolean;
  vistaAuth: "ninguna" | "login" | "registro" | "perfil";
  onCambiarVistaAuth: (
    vista: "ninguna" | "login" | "registro" | "perfil",
  ) => void;
  onCerrarSesion: () => void;
  sincronizacion?: EstadoSincronizacion;
}

// --- BARRA SUPERIOR DE SESIÓN ---
export function TopAuthBar({
  usuario,
  sesionVencida,
  vistaAuth,
  onCambiarVistaAuth,
  onCerrarSesion,
  sincronizacion,
}: AuthBarProps) {
  return (
    <>
      <nav className="top-auth-bar" aria-label="Barra superior de sesión">
        <div className="top-auth-brand">
          <span className="brand-mark">
            <span className="brand-dot" aria-hidden="true" />
            <span>Cuentas Claras</span>
          </span>
          <span className="top-auth-status top-auth-tagline">
            Modo Local-First • Datos financieros 100% en este dispositivo
          </span>
        </div>

        <div className="top-auth-actions">
          {usuario ? (
            <>
              {sincronizacion && (
                <>
                  <span className="top-auth-status" role="status">
                    {sincronizacion.sincronizando
                      ? "Sincronizando…"
                      : sincronizacion.ultimaSync
                        ? `Última sincronización: ${formatearFechaHora(sincronizacion.ultimaSync)}`
                        : "Aún sin sincronizar"}
                    {sincronizacion.errorSync &&
                      ` · ${sincronizacion.errorSync}`}
                  </span>
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
                    disabled={sincronizacion.sincronizando}
                    onClick={() => void sincronizacion.sincronizar()}
                  >
                    Sincronizar
                  </button>
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
                    title="Reiniciar cursor a cero"
                    disabled={sincronizacion.sincronizando}
                    onClick={() => void sincronizacion.reiniciarCursor()}
                  >
                    Reiniciar cursor
                  </button>
                </>
              )}
              <button
                type="button"
                className="user-pill"
                onClick={() =>
                  onCambiarVistaAuth(
                    vistaAuth === "perfil" ? "ninguna" : "perfil",
                  )
                }
                title="Ver perfil de usuario"
                aria-expanded={vistaAuth === "perfil"}
              >
                <span className="user-pill-dot" aria-hidden="true" />
                <span>{usuario.nombre_visible}</span>
              </button>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                onClick={onCerrarSesion}
              >
                Cerrar sesión
              </button>
            </>
          ) : (
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() =>
                onCambiarVistaAuth(
                  vistaAuth === "ninguna" ? "login" : "ninguna",
                )
              }
            >
              {vistaAuth === "ninguna" ? "Iniciar sesión" : "Cuentas"}
            </button>
          )}
        </div>
      </nav>

      {/* Aviso de sesión expirada (Sección 2.4: no bloquea funciones personales) */}
      {sesionVencida && !usuario && (
        <div className="session-banner-expired" role="status">
          <span>
            <strong>Tu sesión venció, inicia sesión de nuevo.</strong> Tus
            cuentas, movimientos y patrimonio siguen funcionando normalmente.
          </span>
          <button
            type="button"
            className="btn btn-secondary btn-xs"
            onClick={() => onCambiarVistaAuth("login")}
          >
            Iniciar sesión
          </button>
        </div>
      )}
    </>
  );
}

// --- PANTALLAS DE REGISTRO, INICIO DE SESIÓN Y PERFIL ---
interface AuthScreensProps {
  vista: "login" | "registro" | "perfil";
  usuario: UsuarioSesion | null;
  onCambiarVista: (vista: "ninguna" | "login" | "registro" | "perfil") => void;
  onAutenticado: (usuario: UsuarioSesion) => void;
  onCerrarSesion: () => void;
  onSesionVencida: () => void;
}

export function AuthScreens({
  vista,
  usuario,
  onCambiarVista,
  onAutenticado,
  onCerrarSesion,
  onSesionVencida,
}: AuthScreensProps) {
  // Estados Login
  const [correoLogin, setCorreoLogin] = useState("");
  const [passwordLogin, setPasswordLogin] = useState("");

  // Estados Registro
  const [nombreUsuario, setNombreUsuario] = useState("");
  const [nombreVisible, setNombreVisible] = useState("");
  const [correoRegistro, setCorreoRegistro] = useState("");
  const [passwordRegistro, setPasswordRegistro] = useState("");

  // Estados Feedback y Perfil Detallado
  const [cargando, setCargando] = useState(false);
  const [error, setError] = useState<ErrorDominio | null>(null);
  const [mensajeExito, setMensajeExito] = useState<string | null>(null);
  const [perfilConsultado, setPerfilConsultado] =
    useState<UsuarioSesion | null>(null);
  const perfilDetallado =
    usuario && perfilConsultado?.id === usuario.id
      ? { ...usuario, ...perfilConsultado }
      : usuario;

  // La consulta se descarta si se abandona el perfil antes de recibir respuesta.
  useEffect(() => {
    if (vista !== "perfil") return;
    let activo = true;
    invoke<UsuarioSesion>("obtener_perfil")
      .then((datos) => {
        if (activo) setPerfilConsultado(datos);
      })
      .catch((error: unknown) => {
        if (
          activo &&
          typeof error === "object" &&
          error !== null &&
          "campo" in error &&
          error.campo === "sesion"
        )
          onSesionVencida();
      });
    return () => {
      activo = false;
    };
  }, [vista, onSesionVencida]);

  const cambiarVista = (
    siguiente: "ninguna" | "login" | "registro" | "perfil",
  ) => {
    setError(null);
    setMensajeExito(null);
    onCambiarVista(siguiente);
  };

  // Formatea errores sin exponer trazas técnicas (RNF-17 y RNF-18) 14]
  const normalizarError = (err: unknown): ErrorDominio => {
    if (
      typeof err === "object" &&
      err !== null &&
      "campo" in err &&
      "mensaje" in err
    ) {
      return err as ErrorDominio;
    }
    const texto = String(err ?? "").toLowerCase();
    if (
      texto.includes("connect") ||
      texto.includes("network") ||
      texto.includes("refused") ||
      texto.includes("timeout") ||
      texto.includes("dns")
    ) {
      return {
        campo: "conexion",
        mensaje:
          "No hay conexión con el servidor en este momento. Puedes seguir usando tus cuentas y movimientos locales sin problemas.",
      };
    }
    return {
      campo: "general",
      mensaje:
        typeof err === "string" && err.trim()
          ? err
          : "No se pudo completar la operación.",
    };
  };

  // Flujo 1: Registro -> invoke('registrar_usuario')
  const manejarRegistro = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setMensajeExito(null);
    setCargando(true);

    try {
      await invoke("registrar_usuario", {
        nombreUsuario,
        nombreVisible,
        correo: correoRegistro,
        password: passwordRegistro,
      });

      // Limpiamos contraseña de memoria inmediatamente (RNF-05) 14]
      setPasswordRegistro("");
      setMensajeExito(
        "Cuenta creada correctamente. Ahora puedes iniciar sesión con tu correo.",
      );
      setCorreoLogin(correoRegistro);
      onCambiarVista("login");
    } catch (err) {
      setError(normalizarError(err));
    } finally {
      setCargando(false);
    }
  };

  // Flujo 2 y 3: Inicio de sesión -> invoke('iniciar_sesion') y perfil 16]
  const manejarLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setMensajeExito(null);
    setCargando(true);

    try {
      const respuesta = await invoke<{
        usuario: UsuarioSesion;
        vencimiento: string;
      }>("iniciar_sesion", {
        correo: correoLogin,
        password: passwordLogin,
      });

      const sesionNormalizada: UsuarioSesion = {
        ...respuesta.usuario,
        vencimiento: respuesta.vencimiento,
      };

      // Limpiamos contraseña de memoria inmediatamente (RNF-05) 14]
      setPasswordLogin("");
      onAutenticado(sesionNormalizada);
      cambiarVista("ninguna");
    } catch (err) {
      setPasswordLogin("");
      setError(normalizarError(err));
    } finally {
      setCargando(false);
    }
  };

  // Vista de Perfil Propio (GET /usuarios/yo) 16]
  if (vista === "perfil" && perfilDetallado) {
    const vigencia = perfilDetallado.vencimiento ?? perfilDetallado.expira_en;

    return (
      <section className="auth-screen-container" aria-label="Perfil de usuario">
        <div className="form-card">
          <div className="auth-card-header">
            <h2 className="form-title">Perfil de usuario</h2>
            <button
              type="button"
              className="btn btn-ghost btn-icon"
              aria-label="Cerrar perfil"
              onClick={() => cambiarVista("ninguna")}
            >
              ✕
            </button>
          </div>
          <p className="form-subtitle">
            Identidad verificada en el servidor. Ningún dato financiero sale de
            tu dispositivo.
          </p>

          <div className="profile-card-grid">
            <div>
              <span className="profile-item-label">Nombre visible</span>
              <span className="profile-item-value">
                {perfilDetallado.nombre_visible}
              </span>
            </div>
            <div>
              <span className="profile-item-label">Nombre de usuario</span>
              <span className="profile-item-value">
                @{perfilDetallado.nombre_usuario}
              </span>
            </div>
            <div>
              <span className="profile-item-label">Correo electrónico</span>
              <span className="profile-item-value">
                {perfilDetallado.correo}
              </span>
            </div>
            {vigencia && (
              <div>
                <span className="profile-item-label">Vigencia de sesión</span>
                <span className="profile-item-value">
                  {formatearFechaHora(vigencia)}
                </span>
              </div>
            )}
          </div>

          <div className="form-actions form-actions-spaced">
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => cambiarVista("ninguna")}
            >
              Cuentas
            </button>
            <button
              type="button"
              className="btn btn-danger"
              onClick={onCerrarSesion}
            >
              Cerrar sesión
            </button>
          </div>
        </div>
      </section>
    );
  }

  // Vistas de Inicio de Sesión y Registro
  return (
    <section
      className="auth-screen-container"
      aria-label="Autenticación de cuenta"
    >
      <div className="form-card">
        <div className="auth-card-header">
          <span className="header-tag">Cuenta en el servidor (Opcional)</span>
          <button
            type="button"
            className="btn btn-ghost btn-icon"
            aria-label="Cerrar pantalla de sesión"
            onClick={() => cambiarVista("ninguna")}
          >
            ✕
          </button>
        </div>

        {/* Selector entre Iniciar sesión y Registrarse */}
        <div className="auth-tabs" role="tablist" aria-label="Tipo de acceso">
          <button
            type="button"
            role="tab"
            aria-selected={vista === "login"}
            className={`auth-tab ${vista === "login" ? "active" : ""}`}
            onClick={() => cambiarVista("login")}
          >
            Iniciar sesión
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={vista === "registro"}
            className={`auth-tab ${vista === "registro" ? "active" : ""}`}
            onClick={() => cambiarVista("registro")}
          >
            Crear cuenta
          </button>
        </div>

        <div className="offline-notice">
          Registrarse es opcional. Tus cuentas, movimientos, tasas y patrimonio
          funcionan sin cuenta y sin conexión; ningún dato financiero se envía
          al servidor.
        </div>

        {mensajeExito && (
          <div className="success-alert" role="status">
            {mensajeExito}
          </div>
        )}

        {error && (
          <div className="error-alert" role="alert">
            <div className="error-alert-body">
              <strong aria-hidden="true">!</strong>
              <div>
                {error.campo !== "general" && error.campo !== "conexion" && (
                  <strong>Campo «{error.campo}»: </strong>
                )}
                <span>{error.mensaje}</span>
              </div>
            </div>
          </div>
        )}

        {vista === "login" ? (
          <form onSubmit={manejarLogin} noValidate>
            <div className="form-group">
              <label className="form-label" htmlFor="login-correo">
                Correo electrónico
              </label>
              <input
                id="login-correo"
                type="email"
                value={correoLogin}
                onChange={(e) => setCorreoLogin(e.target.value)}
                placeholder="ejemplo@correo.com"
                autoComplete="email"
                className={`form-control ${error?.campo === "correo" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="login-password">
                Contraseña
              </label>
              <input
                id="login-password"
                type="password"
                value={passwordLogin}
                onChange={(e) => setPasswordLogin(e.target.value)}
                placeholder="••••••••••"
                autoComplete="current-password"
                className={`form-control ${error?.campo === "password" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-actions">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => cambiarVista("ninguna")}
              >
                Continuar sin cuenta
              </button>
              <button
                type="submit"
                className="btn btn-primary"
                disabled={cargando}
              >
                {cargando ? "Ingresando..." : "Iniciar sesión"}
              </button>
            </div>
          </form>
        ) : (
          <form onSubmit={manejarRegistro} noValidate>
            <div className="form-group">
              <label className="form-label" htmlFor="registro-usuario">
                Nombre de usuario
              </label>
              <input
                id="registro-usuario"
                type="text"
                value={nombreUsuario}
                onChange={(e) => setNombreUsuario(e.target.value)}
                placeholder="ej: test._1"
                autoComplete="username"
                className={`form-control ${error?.campo === "nombre_usuario" ? "input-error" : ""}`}
              />
              <span className="field-hint">
                De 3 a 30 caracteres: solo minúsculas, dígitos, punto (.) y
                guion bajo (_).
              </span>
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="registro-nombre">
                Nombre visible
              </label>
              <input
                id="registro-nombre"
                type="text"
                value={nombreVisible}
                onChange={(e) => setNombreVisible(e.target.value)}
                placeholder="El nombre que verán tus amigos"
                autoComplete="name"
                className={`form-control ${error?.campo === "nombre_visible" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="registro-correo">
                Correo electrónico
              </label>
              <input
                id="registro-correo"
                type="email"
                value={correoRegistro}
                onChange={(e) => setCorreoRegistro(e.target.value)}
                placeholder="ejemplo@correo.com"
                autoComplete="email"
                className={`form-control ${error?.campo === "correo" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="registro-password">
                Contraseña
              </label>
              <input
                id="registro-password"
                type="password"
                value={passwordRegistro}
                onChange={(e) => setPasswordRegistro(e.target.value)}
                placeholder="Mínimo 10 caracteres"
                autoComplete="new-password"
                className={`form-control ${error?.campo === "password" ? "input-error" : ""}`}
              />
              <span className="field-hint">
                Debe tener al menos 10 caracteres.
              </span>
            </div>

            <div className="form-actions">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => cambiarVista("login")}
              >
                Ya tengo cuenta
              </button>
              <button
                type="submit"
                className="btn btn-primary"
                disabled={cargando}
              >
                {cargando ? "Creando cuenta..." : "Registrarse"}
              </button>
            </div>
          </form>
        )}
      </div>
    </section>
  );
}
