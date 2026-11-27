import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Diagnostico = {
  compatible: boolean;
  totalEventos: number;
  primerEventoIncompatible: null | {
    id: string;
    tipo: string;
    momentoRegistro: string;
    dispositivoOrigen: string;
  };
};

export default function DiagnosticoLibro({
  onRespaldar,
}: {
  onRespaldar: () => void;
}) {
  const [resultado, setResultado] = useState<Diagnostico | null>(null);
  const [error, setError] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  async function diagnosticar() {
    setOcupado(true);
    setError(false);
    try {
      setResultado(await invoke<Diagnostico>("diagnosticar_libro"));
    } catch {
      setError(true);
    } finally {
      setOcupado(false);
    }
  }
  return (
    <section className="form-card" aria-label="Diagnóstico del libro">
      <h2 className="section-title">Revisar el libro</h2>
      <p>
        El diagnóstico identifica el primer evento que no puede proyectarse. El
        libro se conserva sin cambios.
      </p>
      <div className="diagnostico-acciones">
        <button
          className="btn btn-primary"
          type="button"
          disabled={ocupado}
          onClick={diagnosticar}
        >
          {ocupado ? "Revisando…" : "Revisar eventos"}
        </button>
        <button className="btn btn-ghost" type="button" onClick={onRespaldar}>
          Exportar copia
        </button>
      </div>
      {error && (
        <p role="alert">No se pudo leer el libro para completar la revisión.</p>
      )}
      {resultado && (
        <div role="status">
          <p>
            {resultado.compatible
              ? "El libro se puede proyectar."
              : "Se encontró un evento que no se puede proyectar. Conserva la copia para revisarla."}{" "}
            ({resultado.totalEventos} eventos)
          </p>
          {resultado.primerEventoIncompatible && (
            <p className="diagnostico-identificador">
              Evento: {resultado.primerEventoIncompatible.id}
              <br />
              Tipo: {resultado.primerEventoIncompatible.tipo}
              <br />
              Registro: {resultado.primerEventoIncompatible.momentoRegistro}
              <br />
              Dispositivo:{" "}
              {resultado.primerEventoIncompatible.dispositivoOrigen}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
