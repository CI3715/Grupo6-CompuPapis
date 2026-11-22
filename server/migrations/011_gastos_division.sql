-- Los registros históricos conservan NULL: no es posible reconstruir su regla ni residuo.
ALTER TABLE gastos ADD COLUMN IF NOT EXISTS regla_division TEXT;
ALTER TABLE gasto_participantes ADD COLUMN IF NOT EXISTS recibio_residuo BOOLEAN;

ALTER TABLE gastos ADD CONSTRAINT gastos_regla_division_valida
    CHECK (regla_division IS NULL OR regla_division IN ('iguales', 'porcentajes', 'montos_fijos')) NOT VALID;
ALTER TABLE gastos ADD CONSTRAINT gastos_monto_positivo
    CHECK (monto > 0) NOT VALID;
ALTER TABLE gastos ADD CONSTRAINT gastos_concepto_no_vacio
    CHECK (btrim(concepto) <> '') NOT VALID;
ALTER TABLE gasto_participantes ADD CONSTRAINT gasto_parte_no_negativa
    CHECK (monto_parte >= 0) NOT VALID;
