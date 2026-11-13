CREATE TABLE IF NOT EXISTS solicitudes_amistad (
    id UUID PRIMARY KEY,
    emisor_id UUID NOT NULL REFERENCES usuarios(id),
    destinatario_id UUID NOT NULL REFERENCES usuarios(id),
    estado TEXT DEFAULT 'pendiente' NOT NULL CHECK (estado IN ('pendiente', 'aceptada', 'rechazada')),
    creada_en TIMESTAMPTZ NOT NULL,
    respondida_en TIMESTAMPTZ,
    CHECK (emisor_id <> destinatario_id),
    CHECK (
        (estado = 'pendiente' AND respondida_en IS NULL) OR
        (estado <> 'pendiente' AND respondida_en IS NOT NULL)
    )
);

-- Evitamos solicitudes repetidas
CREATE UNIQUE INDEX IF NOT EXISTS solicitud_pendiente_unica
ON solicitudes_amistad (
    LEAST(emisor_id, destinatario_id),
    GREATEST(emisor_id, destinatario_id)
) WHERE estado = 'pendiente';

-- Guardamos las amistades aceptadas
CREATE TABLE IF NOT EXISTS amistades (
    id UUID PRIMARY KEY,
    usuario1_id UUID NOT NULL REFERENCES usuarios(id),
    usuario2_id UUID NOT NULL REFERENCES usuarios(id),
    creada_en TIMESTAMPTZ NOT NULL,
    UNIQUE (usuario1_id, usuario2_id),
    CHECK (usuario1_id < usuario2_id)
);