// Presentación del decimal canónico enviado por Rust; conserva la precisión recibida.
// El dominio valida la escala de cada moneda y redondea únicamente el total.
export function formatearMontoLocal(
  cantidad: string,
  simbolo: string,
  mostrarSignoPositivo = false,
): string {
  const partes = /^(-?)(\d+)(?:\.(\d{1,28}))?$/.exec(cantidad);
  if (!partes) throw new Error("Formato decimal recibido no válido");
  const [, signo, entero, fraccion = ""] = partes;
  const decimales = fraccion.padEnd(2, "0");
  const esCero = !/[1-9]/.test(entero + decimales);
  const prefijo = esCero ? "" : signo ? "- " : mostrarSignoPositivo ? "+ " : "";
  const agrupado = entero.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `${prefijo}${simbolo} ${agrupado},${decimales}`;
}
