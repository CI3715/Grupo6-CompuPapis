import type { ErrorDominio } from "@/lib/tipos";

export function normalizarError(error: unknown): ErrorDominio {
  if (
    typeof error === "object" &&
    error !== null &&
    "campo" in error &&
    "mensaje" in error &&
    typeof error.campo === "string" &&
    typeof error.mensaje === "string"
  ) {
    return { campo: error.campo, mensaje: error.mensaje };
  }
  return {
    campo: "general",
    mensaje:
      "No se pudo completar la operación. Abre la aplicación con Tauri e inténtalo de nuevo.",
  };
}

export function obtenerClaseBadgeMoneda(codigo: string): string {
  switch (codigo.toUpperCase()) {
    case "USD":
      return "badge badge-usd";
    case "VES":
      return "badge badge-ves";
    case "EUR":
      return "badge badge-eur";
    case "USDT":
      return "badge badge-usdt";
    default:
      return "badge";
  }
}

export function formatearFecha(iso: string): string {
  const [anio, mes, dia] = iso.split("-");
  return anio && mes && dia ? `${dia}/${mes}/${anio}` : iso;
}