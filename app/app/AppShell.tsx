"use client";

import type { ReactNode } from "react";
import type { EstadoSincronizacion } from "@/app/sincronizacion";
import type { UsuarioSesion } from "@/app/authSection";
import type { Pantalla } from "@/lib/tipos";
import { formatearFechaHora } from "@/lib/utilidades";

interface AppShellProps {
  pantallaActual: Pantalla;
  onCambiarPantalla: (destino: Pantalla) => void;
  usuario: UsuarioSesion | null;
  sesionVencida: boolean;
  vistaAuth: "ninguna" | "login" | "registro" | "perfil";
  onCambiarVistaAuth: (
    vista: "ninguna" | "login" | "registro" | "perfil",
  ) => void;
  onCerrarSesion: () => void;
  sincronizacion?: EstadoSincronizacion;
  estadoServidor: string;
  probandoServidor: boolean;
  onProbarServidor: () => void;
  children: ReactNode;
}

function IconoCuentas() {
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
      <path d="M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1" />
      <path d="M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4" />
    </svg>
  );
}

function IconoPatrimonio() {
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
      <path d="M3 3v18h18" />
      <path d="m19 9-5 5-4-4-3 3" />
    </svg>
  );
}

function IconoSocial() {
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
      <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
      <path d="M16 3.13a4 4 0 0 1 0 7.75" />
    </svg>
  );
}

function IconoTasas() {
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
      <path d="m17 2 4 4-4 4" />
      <path d="M3 11v-1a4 4 0 0 1 4-4h14" />
      <path d="m7 22-4-4 4-4" />
      <path d="M21 13v1a4 4 0 0 1-4 4H3" />
    </svg>
  );
}

function IconoSync({ girando }: { girando?: boolean }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={girando ? "icon-spin" : undefined}
      aria-hidden="true"
    >
      <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
      <path d="M3 3v5h5" />
      <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
      <path d="M16 16h5v5" />
    </svg>
  );
}

const ITEMS_NAV: {
  id: Pantalla;
  etiqueta: string;
  etiquetaCorta: string;
  icono: () => ReactNode;
}[] = [
  {
    id: "cuentas",
    etiqueta: "Mis cuentas",
    etiquetaCorta: "Cuentas",
    icono: IconoCuentas,
  },
  {
    id: "patrimonio",
    etiqueta: "Patrimonio",
    etiquetaCorta: "Patrimonio",
    icono: IconoPatrimonio,
  },
  {
    id: "social",
    etiqueta: "Amigos y grupos",
    etiquetaCorta: "Social",
    icono: IconoSocial,
  },
  {
    id: "tasas",
    etiqueta: "Tasas de cambio",
    etiquetaCorta: "Tasas",
    icono: IconoTasas,
  },
];

const TITULOS_PANTALLA: Record<Pantalla, string> = {
  cuentas: "Mis cuentas",
  patrimonio: "Patrimonio",
  social: "Amigos y grupos",
  tasas: "Tasas de cambio",
};

export default function AppShell({
  pantallaActual,
  onCambiarPantalla,
  usuario,
  sesionVencida,
  vistaAuth,
  onCambiarVistaAuth,
  onCerrarSesion,
  sincronizacion,
  estadoServidor,
  probandoServidor,
  onProbarServidor,
  children,
}: AppShellProps) {
  const iniciales = usuario?.nombre_visible
    ? usuario.nombre_visible
        .trim()
        .split(/\s+/)
        .slice(0, 2)
        .map((p) => p[0]?.toUpperCase() ?? "")
        .join("")
    : "";

  return (
    <div className="fintech-shell">
      {/* SIDEBAR FIJO EN ESCRITORIO */}
      <aside className="app-sidebar" aria-label="Menú principal">
        <div className="sidebar-brand">
          <div className="sidebar-logo" aria-hidden="true">
            CC
          </div>
          <div>
            <span className="sidebar-brand-name">Cuentas Claras</span>
            <span className="sidebar-brand-sub">Local-First</span>
          </div>
        </div>

        <nav className="sidebar-nav" aria-label="Navegación principal">
          <span className="sidebar-nav-label">Principal</span>
          {ITEMS_NAV.map((item) => {
            const activo =
              pantallaActual === item.id && vistaAuth === "ninguna";
            const Icono = item.icono;
            return (
              <button
                key={item.id}
                type="button"
                className={`sidebar-nav-item ${activo ? "is-active" : ""}`}
                aria-current={activo ? "page" : undefined}
                onClick={() => {
                  onCambiarVistaAuth("ninguna");
                  onCambiarPantalla(item.id);
                }}
              >
                <span className="sidebar-nav-icon">
                  <Icono />
                </span>
                <span>{item.etiqueta}</span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-footer">
          <details className="sidebar-server-details">
            <summary>Estado del servidor</summary>
            <div className="server-details-body">
              <button
                type="button"
                className="btn btn-secondary btn-xs"
                disabled={probandoServidor}
                onClick={onProbarServidor}
              >
                {probandoServidor ? "Comprobando…" : "Probar conexión"}
              </button>
              {estadoServidor && (
                <p className="server-details-status" role="status">
                  {estadoServidor}
                </p>
              )}
            </div>
          </details>

          {usuario ? (
            <div className="sidebar-user-card">
              <button
                type="button"
                className="sidebar-user-btn"
                onClick={() =>
                  onCambiarVistaAuth(
                    vistaAuth === "perfil" ? "ninguna" : "perfil",
                  )
                }
                aria-expanded={vistaAuth === "perfil"}
              >
                <span className="sidebar-user-avatar" aria-hidden="true">
                  {iniciales || "U"}
                </span>
                <span className="sidebar-user-meta">
                  <strong className="sidebar-user-name">
                    {usuario.nombre_visible}
                  </strong>
                  <span className="sidebar-user-handle">
                    @{usuario.nombre_usuario}
                  </span>
                </span>
              </button>
              <button
                type="button"
                className="btn btn-ghost btn-xs sidebar-logout-btn"
                onClick={onCerrarSesion}
              >
                Cerrar sesión
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="btn btn-primary sidebar-login-btn"
              onClick={() =>
                onCambiarVistaAuth(
                  vistaAuth === "ninguna" ? "login" : "ninguna",
                )
              }
            >
              {vistaAuth === "ninguna" ? "Iniciar sesión" : "Volver a cuentas"}
            </button>
          )}
        </div>
      </aside>

      {/* COLUMNA PRINCIPAL */}
      <div className="fintech-main">
        {/* BARRA SUPERIOR COMPACTA */}
        <header className="app-topbar">
          <div className="app-topbar-left">
            <div className="topbar-mobile-brand">
              <span className="sidebar-logo sidebar-logo-sm" aria-hidden="true">
                CC
              </span>
              <span className="topbar-mobile-title">Cuentas Claras</span>
            </div>
            <div className="topbar-desktop-context">
              <span className="topbar-section-title">
                {vistaAuth === "perfil"
                  ? "Perfil de usuario"
                  : vistaAuth === "login"
                    ? "Iniciar sesión"
                    : vistaAuth === "registro"
                      ? "Crear cuenta"
                      : TITULOS_PANTALLA[pantallaActual]}
              </span>
              <span className="top-auth-status top-auth-tagline">
                Datos financieros 100% locales en este dispositivo
              </span>
            </div>
          </div>

          <div className="app-topbar-actions">
            {usuario ? (
              <>
                {sincronizacion && (
                  <>
                    <span
                      className="top-auth-status topbar-sync-status"
                      role="status"
                    >
                      {sincronizacion.sincronizando
                        ? "Sincronizando…"
                        : sincronizacion.ultimaSync
                          ? `Sync: ${formatearFechaHora(sincronizacion.ultimaSync)}`
                          : "Sin sincronizar"}
                      {sincronizacion.errorSync &&
                        ` · ${sincronizacion.errorSync}`}
                    </span>
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      disabled={sincronizacion.sincronizando}
                      onClick={() => void sincronizacion.sincronizar()}
                    >
                      <IconoSync girando={sincronizacion.sincronizando} />
                      <span className="hide-xs">Sincronizar</span>
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost btn-sm hide-sm"
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
                  className="user-pill topbar-mobile-user"
                  onClick={() =>
                    onCambiarVistaAuth(
                      vistaAuth === "perfil" ? "ninguna" : "perfil",
                    )
                  }
                  title="Ver perfil de usuario"
                >
                  <span className="user-pill-dot" aria-hidden="true" />
                  <span>{usuario.nombre_visible}</span>
                </button>
              </>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-sm topbar-mobile-login"
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
        </header>

        {/* BANNER DE SESIÓN VENCIDA (no bloquea funciones personales) */}
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

        {/* ÁREA DE CONTENIDO SCROLLEABLE */}
        <div className="app-content-scroll">{children}</div>

        {/* BARRA DE NAVEGACIÓN INFERIOR EN MÓVIL (ESTILO CASHEA) */}
        <nav className="app-bottom-nav" aria-label="Navegación móvil">
          {ITEMS_NAV.map((item) => {
            const activo =
              pantallaActual === item.id && vistaAuth === "ninguna";
            const Icono = item.icono;
            return (
              <button
                key={item.id}
                type="button"
                className={`bottom-nav-item ${activo ? "is-active" : ""}`}
                aria-current={activo ? "page" : undefined}
                onClick={() => {
                  onCambiarVistaAuth("ninguna");
                  onCambiarPantalla(item.id);
                }}
              >
                <span className="bottom-nav-icon">
                  <Icono />
                </span>
                <span className="bottom-nav-label">{item.etiquetaCorta}</span>
              </button>
            );
          })}
        </nav>
      </div>
    </div>
  );
}
