-- Claves para no repetir gastos
CREATE TABLE IF NOT EXISTS claves_gastos (
    usuario_id UUID NOT NULL,
    clave TEXT NOT NULL,
    grupo_id UUID NOT NULL,
    datos TEXT NOT NULL,
    gasto_id UUID,

    PRIMARY KEY (usuario_id, clave),
    FOREIGN KEY (usuario_id) REFERENCES usuarios(id),
    FOREIGN KEY (grupo_id) REFERENCES grupos(id),
    FOREIGN KEY (gasto_id) REFERENCES gastos(id)
);