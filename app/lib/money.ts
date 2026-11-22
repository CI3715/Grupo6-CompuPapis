// Presentación del decimal canónico enviado por Rust; conserva la precisión recibida.
// Si se indica `decimalesMoneda`, deja exactamente esa escala (mismo redondeo que
// rust_decimal::round_dp: mitad al par) y rellena con ceros; si no, no toca los decimales.
export function formatearMontoLocal(
  cantidad: string,
  simbolo: string,
  mostrarSignoPositivo = false,
  decimalesMoneda?: number,
): string {
  const partes = /^(-?)(\d+)(?:\.(\d{1,28}))?$/.exec(cantidad);
  if (!partes) throw new Error("Formato decimal recibido no válido");
  const [, signo, enteroOriginal, fraccionOriginal = ""] = partes;
  let entero = enteroOriginal;
  let fraccion = fraccionOriginal;

  if (decimalesMoneda !== undefined) {
    const d = decimalesMoneda;
    const conservado = entero + fraccion.slice(0, d).padEnd(d, "0");
    const descartado = fraccion.slice(d);
    let valor = BigInt(conservado);
    if (descartado) {
      const mitad = descartado[0] === "5" && !/[1-9]/.test(descartado.slice(1));
      const sube =
        descartado[0] > "5" ||
        (descartado[0] === "5" &&
          (!mitad || Number(conservado[conservado.length - 1]) % 2 === 1));
      if (sube) valor += BigInt(1);
    }
    const texto = valor.toString().padStart(d + 1, "0");
    entero = texto.slice(0, texto.length - d);
    fraccion = d > 0 ? texto.slice(texto.length - d) : "";
  } else {
    fraccion = fraccion.length < 2 ? fraccion.padEnd(2, "0") : fraccion;
  }

  const esCero = !/[1-9]/.test(entero + fraccion);
  const prefijo = esCero ? "" : signo ? "- " : mostrarSignoPositivo ? "+ " : "";
  const agrupado = entero.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `${prefijo}${simbolo} ${agrupado}${fraccion ? "," + fraccion : ""}`;
}
