-- Tabla de Gastos Compartidos dentro de un grupo
CREATE TABLE IF NOT EXISTS gastos (
    id UUID PRIMARY KEY,
    grupo_id UUID NOT NULL,
    pagado_por UUID NOT NULL,
    concepto TEXT NOT NULL,
    monto NUMERIC(12, 2) NOT NULL, -- Monto total con 2 decimales exactos
    fecha_gasto TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (grupo_id) REFERENCES grupos(id) ON DELETE CASCADE,
    FOREIGN KEY (pagado_por) REFERENCES usuarios(id) ON DELETE CASCADE
);

-- Detalle de cuánto le corresponde pagar a cada miembro por gasto
CREATE TABLE IF NOT EXISTS gasto_participantes (
    gasto_id UUID NOT NULL,
    usuario_id UUID NOT NULL,
    monto_parte NUMERIC(12, 2) NOT NULL, -- La porción exacta que le toca asumir
    PRIMARY KEY (gasto_id, usuario_id),
    FOREIGN KEY (gasto_id) REFERENCES gastos(id) ON DELETE CASCADE,
    FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON DELETE CASCADE
);

-- Índices para optimizar consultas de gastos por grupo y cálculo de balances
CREATE INDEX IF NOT EXISTS idx_gastos_grupo ON gastos(grupo_id);
CREATE INDEX IF NOT EXISTS idx_gasto_participantes_usuario ON gasto_participantes(usuario_id);