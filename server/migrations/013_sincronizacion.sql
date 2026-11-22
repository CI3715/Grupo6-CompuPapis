-- Gastos compartidos de cada usuario
CREATE TABLE eventos_usuario (
    id UUID PRIMARY KEY,
    usuario_id UUID NOT NULL,
    posicion BIGINT NOT NULL,
    tipo TEXT NOT NULL,
    gasto_id UUID NOT NULL,
    grupo_id UUID NOT NULL,
    concepto TEXT NOT NULL,
    fecha DATE NOT NULL,
    moneda VARCHAR(3) NOT NULL,
    monto_parte NUMERIC(12, 2) NOT NULL,
    recibio_residuo BOOLEAN NOT NULL,

    FOREIGN KEY (gasto_id, usuario_id) REFERENCES gasto_participantes(gasto_id, usuario_id),
    FOREIGN KEY (grupo_id) REFERENCES grupos(id),
    UNIQUE (usuario_id, posicion),
    UNIQUE (usuario_id, gasto_id),
    CHECK (posicion > 0),
    CHECK (monto_parte >= 0)
);