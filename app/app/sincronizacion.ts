"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

import type { ResultadoSync } from "@/lib/tipos";
import { normalizarError } from "@/lib/utilidades";

const INTERVALO_MS = 5 * 60 * 1000;

export function useSincronizacion({
  sesion,
  usuarioId,
  onSesionVencida,
  onSincronizado,
}: {
  sesion: boolean;
  usuarioId: string | null;
  onSesionVencida: () => void;
  onSincronizado: () => Promise<void>;
}) {
  const [sincronizando, setSincronizando] = useState(false);
  const [ultimaSync, setUltimaSync] = useState<string | null>(null);
  const [errorSync, setErrorSync] = useState("");
  const enCurso = useRef(new Set<string>());
  const usuarioActual = useRef<string | null>(usuarioId);
  const callbacks = useRef({ onSesionVencida, onSincronizado });

  useEffect(() => {
    callbacks.current = { onSesionVencida, onSincronizado };
  });

  useEffect(() => {
    usuarioActual.current = usuarioId;
    return () => {
      usuarioActual.current = null;
    };
  }, [usuarioId]);

  const sincronizar = useCallback(async () => {
    if (!sesion || !usuarioId || enCurso.current.has(usuarioId)) return;
    enCurso.current.add(usuarioId);
    setSincronizando(true);
    setErrorSync("");
    try {
      const r = await invoke<ResultadoSync>("sincronizar");
      if (usuarioActual.current !== usuarioId) return;
      setUltimaSync(r.ultimaSincronizacion);
      await callbacks.current.onSincronizado();
    } catch (err: unknown) {
      if (usuarioActual.current !== usuarioId) return;
      const e = normalizarError(err);
      if (e.campo === "sesion") callbacks.current.onSesionVencida();
      else setErrorSync(e.mensaje);
    } finally {
      enCurso.current.delete(usuarioId);
      if (usuarioActual.current === usuarioId) setSincronizando(false);
    }
  }, [sesion, usuarioId]);

  const reiniciarCursor = useCallback(async () => {
    try {
      await invoke("reiniciar_cursor");
    } catch (err: unknown) {
      const e = normalizarError(err);
      setErrorSync(e.mensaje);
    }
  }, []);

  // Al abrir con sesión vigente, periódica (5 min) y al volver la conexión
  useEffect(() => {
    if (!sesion) {
      let activo = true;
      queueMicrotask(() => {
        if (activo) {
          setUltimaSync(null);
          setErrorSync("");
          setSincronizando(false);
        }
      });
      return () => {
        activo = false;
      };
    }
    let activo = true;
    void invoke<string | null>("obtener_ultima_sincronizacion")
      .then((fecha) => {
        if (activo) setUltimaSync((actual) => actual ?? fecha);
      })
      .catch(() => undefined);
    queueMicrotask(() => {
      if (activo) void sincronizar();
    });
    const temporizador = window.setInterval(
      () => void sincronizar(),
      INTERVALO_MS,
    );
    const alVolverRed = () => void sincronizar();
    window.addEventListener("online", alVolverRed);
    return () => {
      activo = false;
      window.clearInterval(temporizador);
      window.removeEventListener("online", alVolverRed);
    };
  }, [sesion, sincronizar]);

  return { sincronizar, sincronizando, ultimaSync, errorSync, reiniciarCursor };
}

export type EstadoSincronizacion = ReturnType<typeof useSincronizacion>;
