-- La actividad es opcional para conservar los grupos existentes.
ALTER TABLE grupos ADD COLUMN IF NOT EXISTS actividad TEXT;
