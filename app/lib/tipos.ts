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

// --- Entrega 4 ---
export type Pantalla = "cuentas" | "patrimonio" | "tasas";

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