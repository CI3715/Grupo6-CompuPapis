import type { PropsPantalla } from "@/lib/tipos";

export default function EncabezadoPantalla({
  titulo,
  etiquetaIr,
  onIr,
}: PropsPantalla & { titulo: string; etiquetaIr: string }) {
  return (
    <header className="header">
      <div>
        <span className="header-tag">Cuentas Claras • Local-First</span>
        <h1 className="header-title">{titulo}</h1>
      </div>
      <div className="header-actions">
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={onIr}
        >
          Ir a {etiquetaIr} →
        </button>
      </div>
    </header>
  );
}
