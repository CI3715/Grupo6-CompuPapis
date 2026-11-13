-- Tabla de Grupos
CREATE TABLE IF NOT EXISTS grupos (
    id UUID PRIMARY KEY,
    nombre TEXT NOT NULL,
    descripcion TEXT,
    moneda VARCHAR(3) NOT NULL, -- Código ISO de la moneda (ej: USD, VES)
    creador_id UUID NOT NULL,
    fecha_creacion TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (creador_id) REFERENCES usuarios(id) ON DELETE CASCADE
);

-- Tabla de Miembros del Grupo (Relación N:M entre usuarios y grupos)
CREATE TABLE IF NOT EXISTS grupo_miembros (
    grupo_id UUID NOT NULL,
    usuario_id UUID NOT NULL,
    es_admin BOOLEAN NOT NULL DEFAULT false,
    estado_invitacion TEXT NOT NULL DEFAULT 'pendiente', -- 'pendiente', 'aceptada', 'rechazada'
    fecha_union TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (grupo_id, usuario_id),
    FOREIGN KEY (grupo_id) REFERENCES grupos(id) ON DELETE CASCADE,
    FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON DELETE CASCADE
);

-- Índice para listar rápidamente los grupos a los que pertenece un usuario
CREATE INDEX IF NOT EXISTS idx_grupo_miembros_usuario ON grupo_miembros(usuario_id);