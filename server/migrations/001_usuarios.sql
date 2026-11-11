CREATE TABLE IF NOT EXISTS usuarios (
    id UUID PRIMARY KEY,
    nombre_usuario TEXT NOT NULL UNIQUE,
    nombre_visible TEXT NOT NULL,
    correo TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    fecha_alta TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS usuarios_correo_normalizado ON usuarios (lower(correo));
