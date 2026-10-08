"use client";

import { useState } from "react";

type PingResponse = {
  estado: string;
  mensaje: string;
  version: string;
  timestamp: string;
};

export default function Home() {
  const [datos, setDatos] = useState<PingResponse | null>(null);
  const [latencia, setLatencia] = useState<number | null>(null);
  const [cargando, setCargando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function probarPing() {
    setCargando(true);
    setError(null);
    setDatos(null);
    setLatencia(null);

    const inicio = performance.now();

    try {
      const res = await fetch("http://127.0.0.1:8000/ping");

      if (!res.ok) {
        throw new Error(`El servidor respondió con código HTTP ${res.status}`);
      }

      const data: PingResponse = await res.json();
      const fin = performance.now();

      setDatos(data);
      setLatencia(Math.round(fin - inicio));
    } catch (err) {
      setError(
        "No se pudo conectar con el servidor Rust. Verifica que esté encendido en http://127.0.0.1:8000."
      );
    } finally {
      setCargando(false);
    }
  }

  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif", maxWidth: "480px" }}>
      <h1>Prueba de Conexión (Ping)</h1>

      <button
        onClick={probarPing}
        disabled={cargando}
        style={{
          padding: "0.6rem 1.2rem",
          fontSize: "1rem",
          cursor: cargando ? "not-allowed" : "pointer",
          backgroundColor: cargando ? "#ccc" : "#0070f3",
          color: "white",
          border: "none",
          borderRadius: "6px",
        }}
      >
        {cargando ? "Enviando ping..." : "Probar ping"}
      </button>

      {/* Estado de Error (Servidor Apagado / Fallo de Red) */}
      {error && (
        <div
          style={{
            marginTop: "1.5rem",
            padding: "1rem",
            backgroundColor: "#ffebe9",
            border: "1px solid #ffc1c0",
            borderRadius: "6px",
            color: "#cf222e",
          }}
        >
          <strong>⚠️ Error de Conexión</strong>
          <p style={{ margin: "0.5rem 0 0 0" }}>{error}</p>
        </div>
      )}

      {/* Datos en Pantalla */}
      {datos && (
        <div
          style={{
            marginTop: "1.5rem",
            padding: "1rem",
            border: "1px solid #e1e4e8",
            borderRadius: "6px",
            backgroundColor: "#f6f8fa",
            color: "#24292e",
          }}
        >
          <h2 style={{ marginTop: 0, fontSize: "1.1rem" }}>Respuesta del Servidor</h2>

          <p>
            <strong>Estado:</strong>{" "}
            <span style={{ color: datos.estado === "ok" ? "green" : "orange" }}>
              {datos.estado}
            </span>
          </p>
          <p>
            <strong>Mensaje:</strong> {datos.mensaje}
          </p>
          <p>
            <strong>Versión:</strong> {datos.version}
          </p>
          <p>
            <strong>Latencia:</strong> {latencia} ms
          </p>
          <p style={{ fontSize: "0.85rem", color: "#57606a" }}>
            <strong>Timestamp:</strong> {new Date(datos.timestamp).toLocaleString()}
          </p>
        </div>
      )}
    </main>
  );
}