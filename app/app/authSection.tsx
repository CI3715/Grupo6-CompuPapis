"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  onCambiarVistaAuth: (vista: "ninguna" | "login" | "registro" | "perfil") => void;
  onCerrarSesion: () => void;
}

// --- BARRA SUPERIOR DE SESIÓN ---
export function TopAuthBar({
  usuario,
  sesionVencida,
  vistaAuth,
  onCambiarVistaAuth,
  onCerrarSesion,
}: AuthBarProps) {
  return (
    <>
      <nav className="top-auth-bar">
        <div className="top-auth-brand">
          <span className="header-tag" style={{ margin: 0 }}>
            Cuentas Claras
          </span>
          <span className="top-auth-status">
            Modo Local-First • Datos financieros 100% en este dispositivo
          </span>
        </div>

        <div className="top-auth-actions">
          {usuario ? (
            <>
              <button
                type="button"
                className="user-pill"
                onClick={() =>
                  onCambiarVistaAuth(vistaAuth === "perfil" ? "ninguna" : "perfil")
                }
                title="Ver perfil de usuario"
              >
                <span>●</span>
                <span>{usuario.nombre_visible}</span>
              </button>
              <button
                type="button"
                className="btn btn-ghost"
                style={{ padding: "6px 12px", fontSize: "13px" }}
                onClick={onCerrarSesion}
              >
                Cerrar sesión
              </button>
            </>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              style={{ padding: "7px 14px", fontSize: "13px" }}
              onClick={() =>
                onCambiarVistaAuth(vistaAuth === "ninguna" ? "login" : "ninguna")
              }
            >
              {vistaAuth === "ninguna" ? "Iniciar sesión" : "Volver a mis finanzas"}
            </button>
          )}
        </div>
      </nav>

      {/* Aviso de sesión expirada (Sección 2.4: no bloquea funciones personales) */}
      {sesionVencida && !usuario && (
        <div className="session-banner-expired">
          <span>
            <strong>Tu sesión venció, inicia sesión de nuevo.</strong> Tus cuentas, movimientos y patrimonio siguen funcionando normalmente 14].
          </span>
          <button
            type="button"
            className="btn btn-secondary"
            style={{ padding: "5px 12px", fontSize: "12px" }}
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
}

export function AuthScreens({
  vista,
  usuario,
  onCambiarVista,
  onAutenticado,
  onCerrarSesion,
}: AuthScreensProps) {
  // Estados Login
  const [correoLogin, setCorreoLogin] = useState("");
  const [contrasenaLogin, setContrasenaLogin] = useState("");

  // Estados Registro
  const [nombreUsuario, setNombreUsuario] = useState("");
  const [nombreVisible, setNombreVisible] = useState("");
  const [correoRegistro, setCorreoRegistro] = useState("");
  const [contrasenaRegistro, setContrasenaRegistro] = useState("");

  // Estados Feedback y Perfil Detallado
  const [cargando, setCargando] = useState(false);
  const [error, setError] = useState<ErrorDominio | null>(null);
  const [mensajeExito, setMensajeExito] = useState<string | null>(null);
  const [perfilDetallado, setPerfilDetallado] = useState<UsuarioSesion | null>(usuario);

  useEffect(() => {
    setError(null);
    setMensajeExito(null);
  }, [vista]);

  useEffect(() => {
    setPerfilDetallado(usuario);
  }, [usuario]);

  // Consulta GET /usuarios/yo (obtener_perfil) al abrir la vista de perfil 16]
  useEffect(() => {
    if (vista === "perfil") {
      invoke<UsuarioSesion>("obtener_perfil")
        .then((datos) => {
          setPerfilDetallado((prev) => ({
            ...prev,
            ...datos,
          }));
        })
        .catch(() => {
          // Si ocurre algún problema de red, mantiene los datos existentes en memoria
        });
    }
  }, [vista]);

  // Formatea errores sin exponer trazas técnicas (RNF-17 y RNF-18) 14]
  const normalizarError = (err: unknown): ErrorDominio => {
    if (typeof err === "object" && err !== null && "campo" in err && "mensaje" in err) {
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
      mensaje: typeof err === "string" && err.trim() ? err : "No se pudo completar la operación.",
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
        contrasena: contrasenaRegistro,
      });

      // Limpiamos contraseña de memoria inmediatamente (RNF-05) 14]
      setContrasenaRegistro("");
      setMensajeExito("Cuenta creada correctamente. Ahora puedes iniciar sesión con tu correo.");
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
      const respuesta = await invoke<any>("iniciar_sesion", {
        correo: correoLogin,
        contrasena: contrasenaLogin,
      });

      // Desempaqueta la respuesta del servidor (SesionAuth / SesionServidor)
      const sesionNormalizada: UsuarioSesion = respuesta.usuario
        ? {
            ...respuesta.usuario,
            vencimiento: respuesta.vencimiento ?? respuesta.expira_en,
            expira_en: respuesta.expira_en ?? respuesta.vencimiento,
          }
        : respuesta;

      // Limpiamos contraseña de memoria inmediatamente (RNF-05) 14]
      setContrasenaLogin("");
      setPerfilDetallado(sesionNormalizada);
      onAutenticado(sesionNormalizada);
      onCambiarVista("ninguna");
    } catch (err) {
      setContrasenaLogin("");
      setError(normalizarError(err));
    } finally {
      setCargando(false);
    }
  };

  // Vista de Perfil Propio (GET /usuarios/yo) 16]
  if (vista === "perfil" && perfilDetallado) {
    const vigencia = perfilDetallado.vencimiento ?? perfilDetallado.expira_en;

    return (
      <section className="auth-screen-container">
        <div className="form-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <h2 className="form-title">Perfil de usuario</h2>
            <button
              type="button"
              className="btn btn-ghost"
              style={{ padding: "4px 10px" }}
              onClick={() => onCambiarVista("ninguna")}
            >
              ✕
            </button>
          </div>
          <p className="form-subtitle">
            Identidad verificada en el servidor. Ningún dato financiero sale de tu dispositivo.
          </p>

          <div className="profile-card-grid">
            <div>
              <span className="profile-item-label">Nombre visible</span>
              <span className="profile-item-value">{perfilDetallado.nombre_visible}</span>
            </div>
            <div>
              <span className="profile-item-label">Nombre de usuario</span>
              <span className="profile-item-value">@{perfilDetallado.nombre_usuario}</span>
            </div>
            <div>
              <span className="profile-item-label">Correo electrónico</span>
              <span className="profile-item-value">{perfilDetallado.correo}</span>
            </div>
            {vigencia && (
              <div>
                <span className="profile-item-label">Vigencia de sesión</span>
                <span className="profile-item-value">{vigencia}</span>
              </div>
            )}
          </div>

          <div className="form-actions" style={{ marginTop: "24px" }}>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => onCambiarVista("ninguna")}
            >
              Volver a mis cuentas
            </button>
            <button
              type="button"
              className="btn btn-danger"
              style={{ border: "1px solid var(--error-border)" }}
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
    <section className="auth-screen-container">
      <div className="form-card">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <span className="header-tag">Cuenta en el servidor (Opcional)</span>
          <button
            type="button"
            className="btn btn-ghost"
            style={{ padding: "4px 10px" }}
            onClick={() => onCambiarVista("ninguna")}
          >
            ✕
          </button>
        </div>

        {/* Selector entre Iniciar sesión y Registrarse */}
        <div className="auth-tabs">
          <button
            type="button"
            className={`auth-tab ${vista === "login" ? "active" : ""}`}
            onClick={() => onCambiarVista("login")}
          >
            Iniciar sesión
          </button>
          <button
            type="button"
            className={`auth-tab ${vista === "registro" ? "active" : ""}`}
            onClick={() => onCambiarVista("registro")}
          >
            Crear cuenta
          </button>
        </div>

        <div className="offline-notice">
          Registrarse es opcional. Tus cuentas, movimientos, tasas y patrimonio funcionan sin cuenta y sin conexión; ningún dato financiero se envía al servidor.
        </div>

        {mensajeExito && (
          <div
            style={{
              backgroundColor: "var(--brand-light)",
              border: "1px solid var(--brand-primary)",
              color: "#065F46",
              padding: "12px 14px",
              borderRadius: "8px",
              fontSize: "13px",
              marginBottom: "16px",
            }}
          >
            {mensajeExito}
          </div>
        )}

        {error && (
          <div className="error-alert">
            <strong>!</strong>
            <div>
              {error.campo !== "general" && error.campo !== "conexion" && (
                <strong>Campo «{error.campo}»: </strong>
              )}
              <span>{error.mensaje}</span>
            </div>
          </div>
        )}

        {vista === "login" ? (
          <form onSubmit={manejarLogin} noValidate>
            <div className="form-group">
              <label className="form-label">Correo electrónico</label>
              <input
                type="email"
                value={correoLogin}
                onChange={(e) => setCorreoLogin(e.target.value)}
                placeholder="ejemplo@correo.com"
                autoComplete="email"
                className={`form-control ${error?.campo === "correo" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label">Contraseña</label>
              <input
                type="password"
                value={contrasenaLogin}
                onChange={(e) => setContrasenaLogin(e.target.value)}
                placeholder="••••••••••"
                autoComplete="current-password"
                className={`form-control ${error?.campo === "contrasena" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-actions">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => onCambiarVista("ninguna")}
              >
                Continuar sin cuenta
              </button>
              <button type="submit" className="btn btn-primary" disabled={cargando}>
                {cargando ? "Ingresando..." : "Iniciar sesión"}
              </button>
            </div>
          </form>
        ) : (
          <form onSubmit={manejarRegistro} noValidate>
            <div className="form-group">
              <label className="form-label">Nombre de usuario</label>
              <input
                type="text"
                value={nombreUsuario}
                onChange={(e) => setNombreUsuario(e.target.value)}
                placeholder="ej: elias.jaovich_1"
                className={`form-control ${error?.campo === "nombre_usuario" ? "input-error" : ""}`}
              />
              <span className="field-hint">
                De 3 a 30 caracteres: solo minúsculas, dígitos, punto (.) y guion bajo (_).
              </span>
            </div>

            <div className="form-group">
              <label className="form-label">Nombre visible</label>
              <input
                type="text"
                value={nombreVisible}
                onChange={(e) => setNombreVisible(e.target.value)}
                placeholder="El nombre que verán tus amigos"
                className={`form-control ${error?.campo === "nombre_visible" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label">Correo electrónico</label>
              <input
                type="email"
                value={correoRegistro}
                onChange={(e) => setCorreoRegistro(e.target.value)}
                placeholder="ejemplo@correo.com"
                className={`form-control ${error?.campo === "correo" ? "input-error" : ""}`}
              />
            </div>

            <div className="form-group">
              <label className="form-label">Contraseña</label>
              <input
                type="password"
                value={contrasenaRegistro}
                onChange={(e) => setContrasenaRegistro(e.target.value)}
                placeholder="Mínimo 10 caracteres"
                className={`form-control ${error?.campo === "contrasena" ? "input-error" : ""}`}
              />
              <span className="field-hint">Debe tener al menos 10 caracteres.</span>
            </div>

            <div className="form-actions">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => onCambiarVista("login")}
              >
                Ya tengo cuenta
              </button>
              <button type="submit" className="btn btn-primary" disabled={cargando}>
                {cargando ? "Creando cuenta..." : "Registrarse"}
              </button>
            </div>
          </form>
        )}
      </div>
    </section>
  );
}