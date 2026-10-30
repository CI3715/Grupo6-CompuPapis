import type { PropsPantalla } from "@/lib/tipos";

export default function EncabezadoPantalla({
  titulo,
  etiquetaIr,
  onVolver,
  onIr,
}: PropsPantalla & { titulo: string; etiquetaIr: string }) {
  return (
    <header className="header">
      <div>
        <span className="header-tag">Cuentas Claras • Local-First</span>
        <h1 className="header-title">{titulo}</h1>
      </div>
      <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
        <button type="button" className="btn btn-ghost" onClick={onVolver}>
          Cuentas
        </button>
        <button type="button" className="btn btn-secondary" onClick={onIr}>
          {etiquetaIr}
        </button>
      </div>
    </header>
  );
}
