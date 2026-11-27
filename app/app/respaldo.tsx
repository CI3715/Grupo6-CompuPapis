"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

import type {
  ErrorDominio,
  EstadoConfigSyncPersonal,
  InformeSyncPersonal,
  ResultadoFusion,
  ResumenExportacion,
  TipoDestinoSync,
} from "@/lib/tipos";
import { formatearFechaHora, normalizarError } from "@/lib/utilidades";

type Pestana = "copia" | "sync";

export default function PantallaRespaldo({
  alCambiarLibro,
  informeInicial,
}: {
  alCambiarLibro: () => Promise<void>;
  informeInicial?: InformeSyncPersonal | null;
}) {
  const [pestana, setPestana] = useState<Pestana>("copia");

  // --- Exportar ---
  const [exportarCifrado, setExportarCifrado] = useState(true);
  const [rutaExportar, setRutaExportar] = useState("");
  const [fraseExportar, setFraseExportar] = useState("");
  const [confirmarFraseExportar, setConfirmarFraseExportar] = useState("");
  const [exportando, setExportando] = useState(false);
  const [errorExportar, setErrorExportar] = useState<ErrorDominio | null>(null);
  const [resumenExportar, setResumenExportar] =
    useState<ResumenExportacion | null>(null);

  // --- Importar ---
  const [rutaImportar, setRutaImportar] = useState("");
  const [fraseImportar, setFraseImportar] = useState("");
  const [archivoEsCifrado, setArchivoEsCifrado] = useState<boolean | null>(
    null,
  );
  const [importando, setImportando] = useState(false);
  const [errorImportar, setErrorImportar] = useState<ErrorDominio | null>(null);
  const [resultadoImportar, setResultadoImportar] =
    useState<ResultadoFusion | null>(null);

  // --- Sincronización personal (Git / Carpeta compartida) ---
  const [estadoSync, setEstadoSync] = useState<EstadoConfigSyncPersonal | null>(
    null,
  );
  const [tipoDestino, setTipoDestino] = useState<TipoDestinoSync>("carpeta");
  const [destino, setDestino] = useState("");
  const [ramaGit, setRamaGit] = useState("main");
  const [usuarioGit, setUsuarioGit] = useState("");
  const [credencialGit, setCredencialGit] = useState("");
  const [fraseSync, setFraseSync] = useState("");
  const [guardandoConfig, setGuardandoConfig] = useState(false);
  const [sincronizando, setSincronizando] = useState(false);
  const [errorSync, setErrorSync] = useState<ErrorDominio | null>(null);
  const [avisoConfig, setAvisoConfig] = useState("");
  const [informeSync, setInformeSync] = useState<InformeSyncPersonal | null>(
    informeInicial ?? null,
  );

  const sugerirRuta = useCallback(async (cifrado: boolean) => {
    try {
      const ruta = await invoke<string>("ruta_respaldo_sugerida", { cifrado });
      setRutaExportar(ruta);
    } catch {
      // Fuera de Tauri
    }
  }, []);

  const cargarConfig = useCallback(async () => {
    try {
      const est = await invoke<EstadoConfigSyncPersonal>(
        "obtener_config_sync_personal",
      );
      setEstadoSync(est);
      if (est.tipo) setTipoDestino(est.tipo);
      if (est.destino) setDestino(est.destino);
      if (est.rama) setRamaGit(est.rama);
      if (est.usuarioGit) setUsuarioGit(est.usuarioGit);
    } catch (err: unknown) {
      setErrorSync(normalizarError(err));
    }
  }, []);

  useEffect(() => {
    let activo = true;
    invoke<string>("ruta_respaldo_sugerida", { cifrado: true })
      .then((ruta) => {
        if (activo) setRutaExportar(ruta);
      })
      .catch(() => {
        // Fuera de Tauri
      });
    invoke<EstadoConfigSyncPersonal>("obtener_config_sync_personal")
      .then((est) => {
        if (!activo) return;
        setEstadoSync(est);
        if (est.tipo) setTipoDestino(est.tipo);
        if (est.destino) setDestino(est.destino);
        if (est.rama) setRamaGit(est.rama);
        if (est.usuarioGit) setUsuarioGit(est.usuarioGit);
      })
      .catch((err: unknown) => {
        if (activo) setErrorSync(normalizarError(err));
      });
    return () => {
      activo = false;
    };
  }, []);

  const cambiarModoExportacion = (cifrado: boolean) => {
    setExportarCifrado(cifrado);
    setErrorExportar(null);
    setResumenExportar(null);
    void sugerirRuta(cifrado);
  };

  const manejarExportar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorExportar(null);
    setResumenExportar(null);
    setExportando(true);
    try {
      const res = await invoke<ResumenExportacion>("exportar_copia_seguridad", {
        ruta: rutaExportar,
        cifrado: exportarCifrado,
        frase: exportarCifrado ? fraseExportar : null,
        confirmacion: exportarCifrado ? confirmarFraseExportar : null,
      });
      // Limpia frase de paso de memoria en UI inmediatamente (RF-INT-05)
      setFraseExportar("");
      setConfirmarFraseExportar("");
      setResumenExportar(res);
      setRutaImportar(res.ruta);
    } catch (err: unknown) {
      setErrorExportar(normalizarError(err));
    } finally {
      setExportando(false);
    }
  };

  const comprobarArchivoImportar = async () => {
    setErrorImportar(null);
    setArchivoEsCifrado(null);
    if (!rutaImportar.trim()) return;
    try {
      const esCif = await invoke<boolean>("verificar_archivo_cifrado", {
        ruta: rutaImportar,
      });
      setArchivoEsCifrado(esCif);
    } catch (err: unknown) {
      setErrorImportar(normalizarError(err));
    }
  };

  const manejarImportar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorImportar(null);
    setResultadoImportar(null);
    setImportando(true);
    try {
      const res = await invoke<ResultadoFusion>("importar_copia_seguridad", {
        ruta: rutaImportar,
        frase: fraseImportar || null,
      });
      setFraseImportar("");
      setResultadoImportar(res);
      await alCambiarLibro();
    } catch (err: unknown) {
      setFraseImportar("");
      setErrorImportar(normalizarError(err));
    } finally {
      setImportando(false);
    }
  };

  const manejarGuardarConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorSync(null);
    setAvisoConfig("");
    setGuardandoConfig(true);
    try {
      const est = await invoke<EstadoConfigSyncPersonal>(
        "guardar_config_sync_personal",
        {
          tipo: tipoDestino,
          destino,
          rama: tipoDestino === "git" ? ramaGit : null,
          usuarioGit:
            tipoDestino === "git" && usuarioGit.trim()
              ? usuarioGit.trim()
              : null,
          frase: fraseSync || null,
          credencialGit:
            tipoDestino === "git" && credencialGit ? credencialGit : null,
        },
      );
      setFraseSync("");
      setCredencialGit("");
      setEstadoSync(est);
      setAvisoConfig(
        "Configuración guardada. Los secretos quedaron protegidos en el almacén de secretos del sistema operativo.",
      );
    } catch (err: unknown) {
      setErrorSync(normalizarError(err));
    } finally {
      setGuardandoConfig(false);
    }
  };

  const manejarSincronizarAhora = async () => {
    setErrorSync(null);
    setAvisoConfig("");
    setSincronizando(true);
    try {
      const inf = await invoke<InformeSyncPersonal>(
        "sincronizar_personal_ahora",
      );
      setInformeSync(inf);
      await cargarConfig();
      await alCambiarLibro();
    } catch (err: unknown) {
      setErrorSync(normalizarError(err));
    } finally {
      setSincronizando(false);
    }
  };

  const manejarQuitarDestino = async () => {
    setErrorSync(null);
    setAvisoConfig("");
    try {
      await invoke("limpiar_config_sync_personal");
      setDestino("");
      setUsuarioGit("");
      setCredencialGit("");
      setFraseSync("");
      setInformeSync(null);
      await cargarConfig();
      setAvisoConfig(
        "Destino de sincronización desvinculado y secretos eliminados del sistema.",
      );
    } catch (err: unknown) {
      setErrorSync(normalizarError(err));
    }
  };

  return (
    <main className="page-view">
      {/* Tarjeta Hero estilo Cashea */}
      <section className="hero-card" aria-label="Respaldo y sincronización">
        <div className="hero-top">
          <div>
            <span className="hero-eyebrow">
              Cuentas Claras • Privacidad Local-First
            </span>
            <h1 className="hero-title">Respaldo y sincronización</h1>
            <p className="hero-subtitle">
              Exporta copias cifradas con frase de paso o sincroniza tus
              dispositivos propios con Git o una carpeta compartida.
            </p>
            <div className="hero-meta-row">
              {estadoSync?.dispositivoId && (
                <span className="hero-badge">
                  Dispositivo: {estadoSync.dispositivoId.slice(0, 8)}
                </span>
              )}
              {estadoSync?.ultimaSincronizacion && (
                <span className="hero-meta-text">
                  Última sincronización:{" "}
                  {formatearFechaHora(estadoSync.ultimaSincronizacion)}
                </span>
              )}
            </div>
          </div>

          {estadoSync?.configurado && (
            <button
              type="button"
              className="btn btn-hero"
              disabled={sincronizando}
              onClick={() => void manejarSincronizarAhora()}
            >
              {sincronizando ? "Sincronizando…" : "Sincronizar ahora"}
            </button>
          )}
        </div>
      </section>

      <nav
        className="social-tabs"
        role="tablist"
        aria-label="Opciones de respaldo y sincronización"
      >
        <button
          type="button"
          role="tab"
          aria-selected={pestana === "copia"}
          className={`btn btn-sm ${pestana === "copia" ? "btn-primary" : "btn-ghost"}`}
          onClick={() => setPestana("copia")}
        >
          Copia de seguridad (Archivo)
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={pestana === "sync"}
          className={`btn btn-sm ${pestana === "sync" ? "btn-primary" : "btn-ghost"}`}
          onClick={() => setPestana("sync")}
        >
          Sincronizar mis dispositivos
        </button>
      </nav>

      {pestana === "copia" ? (
        <div className="social-grid">
          {/* EXPORTAR COPIA DE SEGURIDAD */}
          <section
            className="form-card"
            aria-label="Exportar copia de seguridad"
          >
            <h2 className="form-title">Exportar copia de seguridad</h2>
            <p className="form-subtitle">
              Genera un único archivo con todos los eventos de tu libro. No
              modifica tu libro ni incluye credenciales ni tokens.
            </p>

            <div className="auth-tabs" role="radiogroup" aria-label="Formato">
              <button
                type="button"
                className={`auth-tab ${exportarCifrado ? "active" : ""}`}
                onClick={() => cambiarModoExportacion(true)}
              >
                Cifrado (recomendado)
              </button>
              <button
                type="button"
                className={`auth-tab ${!exportarCifrado ? "active" : ""}`}
                onClick={() => cambiarModoExportacion(false)}
              >
                Sin cifrar (texto legible)
              </button>
            </div>

            {exportarCifrado ? (
              <div className="warning-alert" role="note">
                <strong>Importante:</strong> cifra tu copia con Argon2id y
                AES-256-GCM.{" "}
                <strong>
                  Si olvidas la frase de paso, el respaldo no se puede
                  recuperar.
                </strong>
              </div>
            ) : (
              <div className="warning-alert" role="note">
                <strong>Advertencia de privacidad:</strong> el archivo sin
                cifrar contiene tus cuentas y movimientos en texto plano legible
                para revisarlo o reconstruir saldos con otra herramienta.
                Guárdalo en un lugar seguro.
              </div>
            )}

            {errorExportar && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <span>{errorExportar.mensaje}</span>
                </div>
              </div>
            )}

            {resumenExportar && (
              <div className="success-alert" role="status">
                <strong>Copia exportada correctamente.</strong>{" "}
                {resumenExportar.cantidadEventos} eventos guardados en{" "}
                <code>{resumenExportar.ruta}</code> (
                {resumenExportar.cifrado ? "cifrado" : "sin cifrar"}).
              </div>
            )}

            <form onSubmit={manejarExportar} noValidate>
              <div className="form-group">
                <label className="form-label" htmlFor="ruta-exportar">
                  Ruta del archivo de destino
                </label>
                <input
                  id="ruta-exportar"
                  type="text"
                  className={`form-control ${errorExportar?.campo === "ruta" ? "input-error" : ""}`}
                  value={rutaExportar}
                  onChange={(e) => setRutaExportar(e.target.value)}
                  placeholder="/ruta/cuentas-claras.ccbackup"
                />
              </div>

              {exportarCifrado && (
                <>
                  <div className="form-group">
                    <label className="form-label" htmlFor="frase-exportar">
                      Frase de paso (mínimo 12 caracteres)
                    </label>
                    <input
                      id="frase-exportar"
                      type="password"
                      autoComplete="new-password"
                      className={`form-control ${errorExportar?.campo === "frase" ? "input-error" : ""}`}
                      value={fraseExportar}
                      onChange={(e) => setFraseExportar(e.target.value)}
                      placeholder="Escribe una frase larga y memorable"
                    />
                  </div>

                  <div className="form-group">
                    <label
                      className="form-label"
                      htmlFor="confirmar-frase-exportar"
                    >
                      Repite la frase de paso
                    </label>
                    <input
                      id="confirmar-frase-exportar"
                      type="password"
                      autoComplete="new-password"
                      className={`form-control ${errorExportar?.campo === "confirmacion" ? "input-error" : ""}`}
                      value={confirmarFraseExportar}
                      onChange={(e) =>
                        setConfirmarFraseExportar(e.target.value)
                      }
                      placeholder="Confirma tu frase de paso"
                    />
                  </div>
                </>
              )}

              <div className="form-actions">
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={exportando}
                >
                  {exportando ? "Exportando…" : "Exportar copia de seguridad"}
                </button>
              </div>
            </form>
          </section>

          {/* IMPORTAR COPIA DE SEGURIDAD */}
          <section
            className="form-card"
            aria-label="Importar copia de seguridad"
          >
            <h2 className="form-title">Importar copia de seguridad</h2>
            <p className="form-subtitle">
              Fusiona eventos en una sola transacción: nunca borra eventos
              locales y puedes importar el mismo archivo varias veces sin
              duplicar nada.
            </p>

            {errorImportar && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <span>{errorImportar.mensaje}</span>
                </div>
              </div>
            )}

            {resultadoImportar && (
              <div className="success-alert" role="status">
                <strong>Importación completada:</strong>{" "}
                {resultadoImportar.nuevos} nuevos,{" "}
                {resultadoImportar.yaExistian} ya existían
                {resultadoImportar.rechazados.length > 0
                  ? `, ${resultadoImportar.rechazados.length} rechazados`
                  : ", 0 rechazados"}
                .
                {resultadoImportar.rechazados.length > 0 && (
                  <div className="mt-sm">
                    {resultadoImportar.rechazados.map((r) => (
                      <p key={r.eventoId} className="account-meta text-normal">
                        Evento {r.eventoId.slice(0, 8)}: {r.motivo}
                      </p>
                    ))}
                  </div>
                )}
              </div>
            )}

            <form onSubmit={manejarImportar} noValidate>
              <div className="form-group">
                <label className="form-label" htmlFor="ruta-importar">
                  Ruta del archivo a importar
                </label>
                <div className="social-inline">
                  <input
                    id="ruta-importar"
                    type="text"
                    className={`form-control ${errorImportar?.campo === "ruta" ? "input-error" : ""}`}
                    value={rutaImportar}
                    onChange={(e) => {
                      setRutaImportar(e.target.value);
                      setArchivoEsCifrado(null);
                    }}
                    placeholder="/ruta/cuentas-claras.ccbackup"
                  />
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    onClick={() => void comprobarArchivoImportar()}
                  >
                    Verificar
                  </button>
                </div>
                {archivoEsCifrado !== null && (
                  <span className="field-hint">
                    {archivoEsCifrado
                      ? "Archivo cifrado detectado: ingresa tu frase de paso."
                      : "Archivo en texto legible (sin cifrar) detectado."}
                  </span>
                )}
              </div>

              {archivoEsCifrado !== false && (
                <div className="form-group">
                  <label className="form-label" htmlFor="frase-importar">
                    Frase de paso (solo si el archivo está cifrado)
                  </label>
                  <input
                    id="frase-importar"
                    type="password"
                    autoComplete="current-password"
                    className={`form-control ${errorImportar?.campo === "frase" ? "input-error" : ""}`}
                    value={fraseImportar}
                    onChange={(e) => setFraseImportar(e.target.value)}
                    placeholder="Ingresa la frase usada al exportar"
                  />
                </div>
              )}

              <div className="form-actions">
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={importando}
                >
                  {importando ? "Importando…" : "Importar copia"}
                </button>
              </div>
            </form>
          </section>
        </div>
      ) : (
        <div className="social-grid">
          {/* CONFIGURAR DESTINO DE SINCRONIZACIÓN PERSONAL */}
          <section
            className="form-card"
            aria-label="Configurar destino de sincronización"
          >
            <h2 className="form-title">Destino de sincronización</h2>
            <p className="form-subtitle">
              Cada dispositivo escribe lotes cifrados únicamente en su propia
              carpeta. La frase y credenciales de Git se guardan en el almacén
              de secretos del sistema operativo.
            </p>

            <div
              className="auth-tabs"
              role="radiogroup"
              aria-label="Tipo de destino"
            >
              <button
                type="button"
                className={`auth-tab ${tipoDestino === "carpeta" ? "active" : ""}`}
                onClick={() => setTipoDestino("carpeta")}
              >
                Carpeta compartida
              </button>
              <button
                type="button"
                className={`auth-tab ${tipoDestino === "git" ? "active" : ""}`}
                onClick={() => setTipoDestino("git")}
              >
                Repositorio Git
              </button>
            </div>

            {errorSync && (
              <div className="error-alert" role="alert">
                <div className="error-alert-body">
                  <strong aria-hidden="true">!</strong>
                  <span>{errorSync.mensaje}</span>
                </div>
              </div>
            )}

            {avisoConfig && (
              <div className="success-alert" role="status">
                {avisoConfig}
              </div>
            )}

            <form onSubmit={manejarGuardarConfig} noValidate>
              <div className="form-group">
                <label className="form-label" htmlFor="sync-destino">
                  {tipoDestino === "carpeta"
                    ? "Ruta de la carpeta compartida (Syncthing, Dropbox, Drive)"
                    : "URL del repositorio Git remoto (privado)"}
                </label>
                <input
                  id="sync-destino"
                  type="text"
                  className={`form-control ${errorSync?.campo === "destino" ? "input-error" : ""}`}
                  value={destino}
                  onChange={(e) => setDestino(e.target.value)}
                  placeholder={
                    tipoDestino === "carpeta"
                      ? "/home/usuario/Sync/cuentas-claras"
                      : "https://github.com/usuario/mi-libro-privado.git"
                  }
                />
              </div>

              {tipoDestino === "git" && (
                <div className="form-row">
                  <div>
                    <label className="form-label" htmlFor="sync-rama">
                      Rama
                    </label>
                    <input
                      id="sync-rama"
                      type="text"
                      className="form-control"
                      value={ramaGit}
                      onChange={(e) => setRamaGit(e.target.value)}
                      placeholder="main"
                    />
                  </div>
                  <div>
                    <label className="form-label" htmlFor="sync-git-user">
                      Usuario Git (opcional)
                    </label>
                    <input
                      id="sync-git-user"
                      type="text"
                      className="form-control"
                      value={usuarioGit}
                      onChange={(e) => setUsuarioGit(e.target.value)}
                      placeholder="tu-usuario"
                    />
                  </div>
                  <div>
                    <label className="form-label" htmlFor="sync-git-token">
                      Token / Contraseña Git
                    </label>
                    <input
                      id="sync-git-token"
                      type="password"
                      autoComplete="new-password"
                      className={`form-control ${errorSync?.campo === "credenciales" ? "input-error" : ""}`}
                      value={credencialGit}
                      onChange={(e) => setCredencialGit(e.target.value)}
                      placeholder={
                        estadoSync?.tieneCredencialGit
                          ? "Guardada en el sistema (dejar vacío para conservar)"
                          : "Token de acceso personal"
                      }
                    />
                  </div>
                </div>
              )}

              <div className="form-group">
                <label className="form-label" htmlFor="sync-frase">
                  Frase de paso de cifrado (mínimo 12 caracteres)
                </label>
                <input
                  id="sync-frase"
                  type="password"
                  autoComplete="new-password"
                  className={`form-control ${errorSync?.campo === "frase" ? "input-error" : ""}`}
                  value={fraseSync}
                  onChange={(e) => setFraseSync(e.target.value)}
                  placeholder={
                    estadoSync?.tieneFrase
                      ? "Guardada en el almacén del SO (dejar vacío para conservar)"
                      : "Usa la misma frase en todos tus dispositivos"
                  }
                />
                <span className="field-hint">
                  Se guarda en el almacén de secretos del sistema operativo,
                  nunca en el libro ni en el destino.
                </span>
              </div>

              <div className="form-actions">
                {estadoSync?.configurado && (
                  <button
                    type="button"
                    className="btn btn-danger btn-sm"
                    onClick={() => void manejarQuitarDestino()}
                  >
                    Quitar destino
                  </button>
                )}
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={guardandoConfig}
                >
                  {guardandoConfig ? "Guardando…" : "Guardar configuración"}
                </button>
              </div>
            </form>
          </section>

          {/* ESTADO E INFORME DE SINCRONIZACIÓN */}
          <section className="form-card" aria-label="Estado de sincronización">
            <h2 className="form-title">Estado de sincronización</h2>
            <p className="form-subtitle">
              Se sincroniza automáticamente al abrir la aplicación si hay un
              destino configurado, y en cualquier momento con el botón.
            </p>

            <div className="profile-card-grid">
              <div>
                <span className="profile-item-label">Estado</span>
                <span className="profile-item-value">
                  {estadoSync?.configurado
                    ? `Configurado (${estadoSync.tipo === "git" ? "Git" : "Carpeta"})`
                    : "Sin configurar"}
                </span>
              </div>
              <div>
                <span className="profile-item-label">
                  Última sincronización
                </span>
                <span className="profile-item-value">
                  {estadoSync?.ultimaSincronizacion
                    ? formatearFechaHora(estadoSync.ultimaSincronizacion)
                    : "Aún sin sincronizar"}
                </span>
              </div>
              <div>
                <span className="profile-item-label">Frase en llavero SO</span>
                <span className="profile-item-value">
                  {estadoSync?.tieneFrase ? "Protegida" : "Pendiente"}
                </span>
              </div>
              <div>
                <span className="profile-item-label">ID de dispositivo</span>
                <span className="profile-item-value">
                  {estadoSync?.dispositivoId ?? "—"}
                </span>
              </div>
            </div>

            {informeSync && (
              <div className="success-alert mt-sm" role="status">
                <strong>Último resultado:</strong> {informeSync.enviados}{" "}
                enviados, {informeSync.recibidos} recibidos
                {informeSync.yaExistian > 0
                  ? `, ${informeSync.yaExistian} ya existían`
                  : ""}
                , {informeSync.rechazados.length} rechazados.
                {informeSync.rechazados.length > 0 && (
                  <div className="mt-sm">
                    {informeSync.rechazados.map((r) => (
                      <p key={r.eventoId} className="account-meta text-normal">
                        Rechazado {r.eventoId.slice(0, 8)}: {r.motivo}
                      </p>
                    ))}
                  </div>
                )}
              </div>
            )}

            <div className="form-actions form-actions-spaced">
              <button
                type="button"
                className="btn btn-primary"
                disabled={!estadoSync?.configurado || sincronizando}
                onClick={() => void manejarSincronizarAhora()}
              >
                {sincronizando ? "Sincronizando…" : "Sincronizar ahora"}
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
