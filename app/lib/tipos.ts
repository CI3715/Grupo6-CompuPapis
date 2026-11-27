// Tipos compartidos entre la pantalla principal y las pantallas de Patrimonio y Tasas.

export interface Moneda {
  codigo: string;
  nombre: string;
  simbolo: string;
  decimales: number;
}

export interface Cuenta {
  id: string;
  nombre: string;
  tipo: string;
  moneda: string;
  saldo: {
    cantidad: string;
    moneda: string;
  };
}

export interface SubtotalMoneda {
  codigo: string;
  simbolo: string;
  subtotal: string;
}

export interface ResumenCuentas {
  cuentas: Cuenta[];
  subtotales: SubtotalMoneda[];
}

export interface ErrorDominio {
  campo: string;
  mensaje: string;
}

export interface Categoria {
  nombre: string;
  tipo: "ingreso" | "gasto";
}

export interface Movimiento {
  id: string;
  fecha: string;
  tipo: "ingreso" | "gasto" | "ajuste";
  categoria: string;
  nota?: string;
  monto: string;
  montoVigente: string;
  corregido: boolean;
  esAjuste: boolean;
  referenciaId?: string;
  esGastoDeGrupo?: boolean;
}

export interface DetalleCuenta {
  saldoActual: string;
  composicion: {
    inicial: string;
    ingresos: string;
    gastos: string;
    ajustes: string;
  };
  movimientos: Movimiento[];
}

export type Pantalla =
  "cuentas" | "patrimonio" | "tasas" | "social" | "respaldo";

export interface Tasa {
  id: string;
  origen: string;
  destino: string;
  valor: string;
  fecha: string;
  fuente: string;
}

export type TasaUsada = Omit<Tasa, "id">;

// Subtotal por moneda con su conversión a la moneda base
// (los campos extra llegan solo desde consultar_patrimonio)
export interface SubtotalPatrimonio extends SubtotalMoneda {
  tasa?: TasaUsada | null;
  inversa?: boolean;
  enBase?: string | null;
}

export interface FilaPatrimonio {
  cuentaId: string;
  nombre: string;
  moneda: string;
  saldo: string;
  tasa: TasaUsada | null;
  inversa: boolean;
  enBase: string | null;
}

export interface Patrimonio {
  monedaBase: string;
  fecha: string;
  filas: FilaPatrimonio[];
  subtotales: SubtotalPatrimonio[];
  total: string | null;
  faltantes: { origen: string; destino: string }[];
}

export interface PropsPantalla {
  onVolver: () => void;
  onIr: () => void;
}

// --- Entrega 7 ---
export interface ResultadoSync {
  ultimaSincronizacion: string | null;
  aplicados: number;
}

export interface PendienteAplicar {
  id: string; // id del evento de origen (eventos_usuario.id)
  gasto_id: string;
  grupo_id: string;
  concepto: string;
  fecha: string;
  moneda: string;
  monto_parte: string;
  recibio_residuo: boolean;
}

export interface GastoEnCola {
  clave: string; // Idempotency-Key (claves_gastos.clave)
  grupo_id: string;
  concepto: string;
  monto: string;
  estado: "pendiente" | "rechazado";
  motivo: string | null;
}

// --- Entrega 8: Respaldo y Sincronización Personal ---
export interface ResumenExportacion {
  ruta: string;
  cantidadEventos: number;
  cifrado: boolean;
  fechaExportacion: string;
}

export interface EventoRechazado {
  eventoId: string;
  motivo: string;
}

export interface ResultadoFusion {
  nuevos: number;
  yaExistian: number;
  rechazados: EventoRechazado[];
}

export type TipoDestinoSync = "carpeta" | "git";

export interface EstadoConfigSyncPersonal {
  configurado: boolean;
  dispositivoId: string;
  tipo: TipoDestinoSync | null;
  destino: string | null;
  rama: string | null;
  usuarioGit: string | null;
  tieneFrase: boolean;
  tieneCredencialGit: boolean;
  ultimaSincronizacion: string | null;
}

export interface InformeSyncPersonal {
  enviados: number;
  recibidos: number;
  yaExistian: number;
  rechazados: EventoRechazado[];
  ultimaSincronizacion: string;
}
