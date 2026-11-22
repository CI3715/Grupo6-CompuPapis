"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

import type { ResultadoSync } from "@/lib/tipos";
import { normalizarError } from "@/lib/utilidades";

const INTERVALO_MS = 5 * 60 * 1000;

export function useSincronizacion({
  sesion,
  onSesionVencida,
  onSincronizado,
}: {
  sesion: boolean;
  onSesionVencida: () => void;
  onSincronizado: () => Promise<void>;
}) {
  const [sincronizando, setSincronizando] = useState(false);
  const [ultimaSync, setUltimaSync] = useState<string | null>(null);
  const [errorSync, setErrorSync] = useState("");
  const enCurso = useRef(false);
  const callbacks = useRef({ onSesionVencida, onSincronizado });

  useEffect(() => {
    callbacks.current = { onSesionVencida, onSincronizado };
  });

  const sincronizar = useCallback(async () => {
    if (!sesion || enCurso.current) return;
    enCurso.current = true;
    setSincronizando(true);
    setErrorSync("");
    try {
      const r = await invoke<ResultadoSync>("sincronizar");
      setUltimaSync(r.ultimaSincronizacion);
      await callbacks.current.onSincronizado();
    } catch (err: unknown) {
      const e = normalizarError(err);
      if (e.campo === "sesion") callbacks.current.onSesionVencida();
      else setErrorSync(e.mensaje);
    } finally {
      enCurso.current = false;
      setSincronizando(false);
    }
  }, [sesion]);

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
    if (!sesion) return;
    void sincronizar();
    const temporizador = window.setInterval(() => void sincronizar(), INTERVALO_MS);
    const alVolverRed = () => void sincronizar();
    window.addEventListener("online", alVolverRed);
    return () => {
      window.clearInterval(temporizador);
      window.removeEventListener("online", alVolverRed);
    };
  }, [sesion, sincronizar]);

  return { sincronizar, sincronizando, ultimaSync, errorSync, reiniciarCursor };
}

export type EstadoSincronizacion = ReturnType<typeof useSincronizacion>;