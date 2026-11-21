//! main.rs: Servidor HTTP de Cuentas Claras, rutas principales y modelos de eventos
//!
//! Dirigido al equipo backend para gestionar el arranque y contratos de sincronización

pub mod amistades_db;
mod auth;
mod db;
pub mod division;
pub mod gastos_db;
mod grupos;
mod social;
pub mod sincronizacion_db;
pub mod usuarios_db;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::sincronizacion_db::EventoUsuario;

use std::collections::HashSet;

use axum::http::StatusCode;

use crate::gastos_db::ErrorGasto;

use sqlx::{Postgres, Transaction};

#[derive(Serialize)]
struct PingResponse {
    estado: &'static str,
    mensaje: &'static str,
    version: &'static str,
    timestamp: String,
}

// Identificador textual fijo para eventos de gastos compartidos de grupo
#[allow(dead_code)]
pub const TIPO_EVENTO_GASTO_GRUPO: &str = "gasto_grupo";

// Carga útil inmutable con la parte de un gasto compartido por participante
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatosParteGasto {
    pub grupo_id: Uuid,
    pub gasto_id: Uuid,
    pub concepto: String,
    pub fecha: NaiveDate,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

// Evento inmutable del registro de cambios por usuario, con id y cursor creciente
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoCambio {
    id: Uuid,
    /// Posición creciente por usuario usada como cursor de sincronización
    posicion: i64,
    /// Categoría textual del evento para soportar tipos futuros
    tipo: String,
    /// Carga útil con los datos del gasto compartido
    datos: DatosParteGasto,
}

#[allow(dead_code)]
impl EventoCambio {
    // Crea un evento inmutable de gasto grupal y falla si la posición no es positiva
    pub fn nuevo_gasto_grupo(
        id: Uuid,
        posicion: i64,
        datos: DatosParteGasto,
    ) -> Result<Self, &'static str> {
        // Replica la restricción CHECK (posicion > 0) de la tabla eventos_usuario.
        if posicion <= 0 {
            return Err("La posición del evento debe ser mayor que cero");
        }

        Ok(Self {
            id,
            posicion,
            tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
            datos,
        })
    }

    // Convierte un registro de base de datos en un evento inmutable validado
    pub fn desde_evento_db(evento: EventoUsuario) -> Result<Self, &'static str> {
        if evento.posicion <= 0 {
            return Err("La posición del evento debe ser mayor que cero");
        }

        // Agrupa los campos planos de SQLx en el payload del gasto
        let datos = DatosParteGasto {
            grupo_id: evento.grupo_id,
            gasto_id: evento.gasto_id,
            concepto: evento.concepto,
            fecha: evento.fecha,
            moneda: evento.moneda,
            monto_parte: evento.monto_parte,
            recibio_residuo: evento.recibio_residuo,
        };

        Ok(Self {
            id: evento.id,
            posicion: evento.posicion,
            tipo: evento.tipo,
            datos,
        })
    }

    // Devuelve el identificador del evento
    pub fn id(&self) -> Uuid {
        self.id
    }

    // Devuelve la posición usada como cursor
    pub fn posicion(&self) -> i64 {
        self.posicion
    }

    // Devuelve el tipo textual del evento
    pub fn tipo(&self) -> &str {
        &self.tipo
    }

    // Devuelve una referencia de solo lectura a los datos del gasto
    pub fn datos(&self) -> &DatosParteGasto {
        &self.datos
    }
}

// Errores de validación de membresía y destinatarios al emitir eventos de cambio
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorDestinatarios {
    // El grupo no existe o el solicitante no es miembro activo
    GrupoNoAccesible,
    // El gasto no contiene participantes para emitir eventos
    SinParticipantes,
    // Un participante aparece repetido en la división del gasto
    ParticipanteDuplicado(Uuid),
    // Un participante del gasto no pertenece activamente al grupo
    ParticipanteNoMiembro(Uuid),
}

#[allow(dead_code)]
impl ErrorDestinatarios {
    // Devuelve el código HTTP correspondiente, ocultando inexistencia o falta de membresía con 404
    #[allow(dead_code)]
    pub fn codigo_http(&self) -> StatusCode {
        match self {
            Self::GrupoNoAccesible => StatusCode::NOT_FOUND,
            Self::SinParticipantes
            | Self::ParticipanteDuplicado(_)
            | Self::ParticipanteNoMiembro(_) => StatusCode::BAD_REQUEST,
        }
    }

    // Convierte el error de validación al tipo compartido por la capa de gastos
    pub fn a_error_gasto(self) -> ErrorGasto {
        match self {
            Self::GrupoNoAccesible => ErrorGasto::SinAcceso,
            Self::SinParticipantes | Self::ParticipanteDuplicado(_) => {
                ErrorGasto::Invalido("Selecciona participantes únicos.".into())
            }
            Self::ParticipanteNoMiembro(_) => {
                ErrorGasto::Invalido("Cada participante debe ser miembro del grupo.".into())
            }
        }
    }
}

// Valida membresía del solicitante y devuelve solo los participantes activos para emitir eventos
pub fn filtrar_destinatarios_evento(
    solicitante_id: Uuid,
    miembros_activos: &HashSet<Uuid>,
    participantes_gasto: &[Uuid],
) -> Result<Vec<Uuid>, ErrorDestinatarios> {
    // Oculta si el grupo no existe o si el solicitante no pertenece
    if miembros_activos.is_empty() || !miembros_activos.contains(&solicitante_id) {
        return Err(ErrorDestinatarios::GrupoNoAccesible);
    }

    if participantes_gasto.is_empty() {
        return Err(ErrorDestinatarios::SinParticipantes);
    }

    // Evita emitir duplicados y rechaza usuarios ajenos al grupo
    let mut vistos = HashSet::with_capacity(participantes_gasto.len());
    let mut destinatarios = Vec::with_capacity(participantes_gasto.len());

    for &participante_id in participantes_gasto {
        if !miembros_activos.contains(&participante_id) {
            return Err(ErrorDestinatarios::ParticipanteNoMiembro(participante_id));
        }
        if !vistos.insert(participante_id) {
            return Err(ErrorDestinatarios::ParticipanteDuplicado(participante_id));
        }
        destinatarios.push(participante_id);
    }

    // Ordena los ids igual que sincronizacion_db para evitar bloqueos cruzados en PostgreSQL
    destinatarios.sort();
    Ok(destinatarios)
}

// Persiste eventos inmutables para participantes válidos en una transacción y devuelve sus destinatarios
pub async fn emitir_eventos_gasto(
    tx: &mut Transaction<'_, Postgres>,
    solicitante_id: Uuid,
    miembros_activos: &HashSet<Uuid>,
    participantes_gasto: &[Uuid],
    gasto_id: Uuid,
) -> Result<Vec<Uuid>, ErrorGasto> {
    // Verifica membresía y excluye no participantes antes de escribir en base de datos
    let destinatarios = filtrar_destinatarios_evento(
        solicitante_id,
        miembros_activos,
        participantes_gasto,
    )
    .map_err(ErrorDestinatarios::a_error_gasto)?;
    // Delega el INSERT con UUID y cursor creciente, propagando errores SQLx
    sincronizacion_db::guardar_eventos_gasto(tx, gasto_id).await?;

    Ok(destinatarios)
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        estado: "ok",
        mensaje: "Servidor Cuentas Claras activo",
        version: "0.1.0",
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
    let secret = std::env::var("JWT_SECRET").map_err(|_| "Falta JWT_SECRET")?;
    if secret.len() < 32 {
        return Err("JWT_SECRET debe tener al menos 32 bytes".into());
    }

    let pool = db::conectar_y_migrar(&database_url).await?;

    let modo_demostracion = std::env::var("TOKEN_DEMO_1_MINUTO").as_deref() == Ok("1");
    let estado = auth::State::new(pool, secret).con_modo_demostracion(modo_demostracion);
    let app = Router::new()
        .route("/ping", get(ping))
        .merge(grupos::routes(estado.clone()))
        .merge(social::routes(estado.clone()))
        .merge(auth::routes(estado));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("Servidor activo en http://127.0.0.1:8000");
    axum::serve(listener, app).await?;
    Ok(())
}

// Pruebas unitarias de emisión de eventos e invariante INV-17
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Crea un UUID determinista para identificar entidades en pruebas
    fn id_fijo(valor: u128) -> Uuid {
        Uuid::from_u128(valor)
    }

    // Cuota individual de un participante dentro de un gasto grupal.
    struct PartePrueba {
        usuario_id: Uuid,
        monto_parte: &'static str,
        recibio_residuo: bool,
    }

    // Metadatos compartidos de un gasto grupal para casos de prueba
    struct GastoPrueba<'a> {
        grupo_id: Uuid,
        gasto_id: Uuid,
        concepto: &'a str,
        fecha: NaiveDate,
        moneda: &'a str,
    }

    // Registro en memoria por usuario para validar generación de eventos.
    struct RegistroCambiosPrueba {
        siguiente_id_evento: u128,
        eventos_por_usuario: HashMap<Uuid, Vec<EventoCambio>>,
    }

    impl RegistroCambiosPrueba {
        // Valida destinatarios con INV-17 y agrega eventos inmutables por participantes
        fn registrar_gasto(
            &mut self,
            solicitante_id: Uuid,
            miembros_activos: &HashSet<Uuid>,
            gasto: &GastoPrueba<'_>,
            partes: &[PartePrueba],
        ) -> Result<(), ErrorDestinatarios> {
            let participantes: Vec<Uuid> = partes.iter().map(|p| p.usuario_id).collect();
            // Bloquea la emisión completa si falla la membresía o algún participante.
            let destinatarios =
                filtrar_destinatarios_evento(solicitante_id, miembros_activos, &participantes)?;

            let mapa_partes: HashMap<Uuid, &PartePrueba> =
                partes.iter().map(|p| (p.usuario_id, p)).collect();

            for usuario_id in destinatarios {
                let parte = mapa_partes[&usuario_id];
                let lista = self.eventos_por_usuario.entry(usuario_id).or_default();
                // Incrementa la posición local del usuario para conservar cursor creciente.
                let posicion =
                    i64::try_from(lista.len() + 1).expect("La posición debe caber en i64");
                let evento_id = id_fijo(self.siguiente_id_evento);
                self.siguiente_id_evento += 1;

                let datos = DatosParteGasto {
                    grupo_id: gasto.grupo_id,
                    gasto_id: gasto.gasto_id,
                    concepto: String::from(gasto.concepto),
                    fecha: gasto.fecha,
                    moneda: String::from(gasto.moneda),
                    monto_parte: String::from(parte.monto_parte),
                    recibio_residuo: parte.recibio_residuo,
                };

                let evento = EventoCambio::nuevo_gasto_grupo(evento_id, posicion, datos)
                    .expect("La posición secuencial debe ser positiva");
                lista.push(evento);
            }

            Ok(())
        }

        // Devuelve los eventos registrados para un usuario en orden de llegada
        fn eventos_de(&self, usuario_id: Uuid) -> &[EventoCambio] {
            self.eventos_por_usuario
                .get(&usuario_id)
                .map_or(&[], Vec::as_slice)
        }

        // Cuenta el total de eventos almacenados entre todos los usuarios.
        fn total_eventos(&self) -> usize {
            self.eventos_por_usuario.values().map(Vec::len).sum()
        }
    }

    // Comprueba que cada participante reciba exactamente un evento con datos correctos
    #[test]
    fn cada_participante_recibe_un_evento_con_datos_y_redondeo_correctos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(100),
            concepto: "Hotel",
            fecha,
            moneda: "USD",
        };
        // Simula el reparto de 100.00 USD del hotel con céntimo residual.
        let partes = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "33.34",
                recibio_residuo: true,
            },
            PartePrueba {
                usuario_id: beto,
                monto_parte: "33.33",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: carla,
                monto_parte: "33.33",
                recibio_residuo: false,
            },
        ];

        let mut registro = RegistroCambiosPrueba::nuevo();
        let resultado = registro.registrar_gasto(ana, &miembros, &gasto, &partes);
        assert!(resultado.is_ok());
        assert_eq!(registro.total_eventos(), 3);

        for (usuario_id, monto_esperado, residuo_esperado) in [
            (ana, "33.34", true),
            (beto, "33.33", false),
            (carla, "33.33", false),
        ] {
            let eventos = registro.eventos_de(usuario_id);
            assert_eq!(eventos.len(), 1);
            let evento = &eventos[0];
            assert_eq!(evento.tipo(), TIPO_EVENTO_GASTO_GRUPO);
            assert_eq!(evento.posicion(), 1);
            assert_eq!(evento.datos().grupo_id, gasto.grupo_id);
            assert_eq!(evento.datos().gasto_id, gasto.gasto_id);
            assert_eq!(evento.datos().concepto, "Hotel");
            assert_eq!(evento.datos().fecha, fecha);
            assert_eq!(evento.datos().moneda, "USD");
            assert_eq!(evento.datos().monto_parte, monto_esperado);
            assert_eq!(evento.datos().recibio_residuo, residuo_esperado);
        }
    }

    // Verifica que un miembro excluido del gasto no reciba ningún evento
    #[test]
    fn miembro_excluido_de_la_division_no_recibe_eventos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(101),
            concepto: "Taxi",
            fecha,
            moneda: "USD",
        };
        let partes = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "10.00",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: beto,
                monto_parte: "10.00",
                recibio_residuo: false,
            },
        ];

        let mut registro = RegistroCambiosPrueba::nuevo();
        let resultado = registro.registrar_gasto(ana, &miembros, &gasto, &partes);
        assert!(resultado.is_ok());
        assert_eq!(registro.eventos_de(ana).len(), 1);
        assert_eq!(registro.eventos_de(beto).len(), 1);
        assert!(registro.eventos_de(carla).is_empty());
        assert_eq!(registro.total_eventos(), 2);
    }

    // Comprueba INV-17 rechazando ajenos y devolviendo 404 sin generar eventos
    #[test]
    fn inv_17_rechaza_no_miembros_y_mantiene_404_sin_generar_eventos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let externo = id_fijo(99);
        let miembros = HashSet::from([ana, beto]);
        let grupo_vacio = HashSet::new();
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(102),
            concepto: "Cena",
            fecha,
            moneda: "USD",
        };
        let mut registro = RegistroCambiosPrueba::nuevo();

        // Caso participante externo falla y evita emisiones parciales
        let partes_con_externo = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "15.00",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: externo,
                monto_parte: "15.00",
                recibio_residuo: false,
            },
        ];
        let error_externo = registro
            .registrar_gasto(ana, &miembros, &gasto, &partes_con_externo)
            .expect_err("Un participante ajeno debe rechazarse");
        assert_eq!(
            error_externo,
            ErrorDestinatarios::ParticipanteNoMiembro(externo)
        );
        assert_eq!(error_externo.codigo_http(), StatusCode::BAD_REQUEST);
        assert_eq!(registro.total_eventos(), 0);

        // Caso solicitante sin membresía recibe 404 ocultando acceso
        let partes_validas = [PartePrueba {
            usuario_id: ana,
            monto_parte: "30.00",
            recibio_residuo: false,
        }];
        let error_sin_membresia = registro
            .registrar_gasto(externo, &miembros, &gasto, &partes_validas)
            .expect_err("Un solicitante externo debe recibir 404");
        assert_eq!(error_sin_membresia, ErrorDestinatarios::GrupoNoAccesible);
        assert_eq!(error_sin_membresia.codigo_http(), StatusCode::NOT_FOUND);
        assert!(matches!(
            error_sin_membresia.a_error_gasto(),
            ErrorGasto::SinAcceso
        ));
        assert_eq!(registro.total_eventos(), 0);

        // Caso grupo inexistente responde con el mismo 404
        let error_inexistente = registro
            .registrar_gasto(ana, &grupo_vacio, &gasto, &partes_validas)
            .expect_err("Un grupo inexistente debe recibir 404");
        assert_eq!(error_inexistente, ErrorDestinatarios::GrupoNoAccesible);
        assert_eq!(error_inexistente.codigo_http(), StatusCode::NOT_FOUND);
        assert!(matches!(
            error_inexistente.a_error_gasto(),
            ErrorGasto::SinAcceso
        ));
        assert_eq!(registro.total_eventos(), 0);
    }

    // Valida identificadores únicos, orden creciente por usuario y rechazo de posición inválida
    #[test]
    fn eventos_conservan_id_unico_y_orden_creciente_por_usuario() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let grupo_id = id_fijo(10);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 21).expect("Fecha válida");
        let mut registro = RegistroCambiosPrueba::nuevo();

        let hotel = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(201),
            concepto: "Hotel",
            fecha,
            moneda: "USD",
        };
        let taxi = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(202),
            concepto: "Taxi",
            fecha,
            moneda: "USD",
        };
        let cena = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(203),
            concepto: "Cena",
            fecha,
            moneda: "USD",
        };

        registro
            .registrar_gasto(
                ana,
                &miembros,
                &hotel,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "33.34",
                        recibio_residuo: true,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "33.33",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: carla,
                        monto_parte: "33.33",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El primer gasto debe registrarse");

        registro
            .registrar_gasto(
                beto,
                &miembros,
                &taxi,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "10.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "10.00",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El segundo gasto debe registrarse");

        registro
            .registrar_gasto(
                carla,
                &miembros,
                &cena,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "30.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "15.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: carla,
                        monto_parte: "15.00",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El tercer gasto debe registrarse");

        // Comprueba crecimiento estricto del cursor y unicidad global de identificadores.
        let mut ids_unicos = HashSet::new();
        for (usuario_id, posiciones_esperadas) in [
            (ana, &[1, 2, 3][..]),
            (beto, &[1, 2, 3][..]),
            (carla, &[1, 2][..]),
        ] {
            let eventos = registro.eventos_de(usuario_id);
            let posiciones: Vec<i64> = eventos.iter().map(EventoCambio::posicion).collect();
            assert_eq!(posiciones, posiciones_esperadas);

            for par in eventos.windows(2) {
                assert!(par[0].posicion() < par[1].posicion());
            }
            for evento in eventos {
                assert!(ids_unicos.insert(evento.id()));
            }
        }
        assert_eq!(ids_unicos.len(), 8);

        // Verifica la invariante de posición estrictamente positiva en el constructor.
        let datos_invalidos = DatosParteGasto {
            grupo_id,
            gasto_id: id_fijo(204),
            concepto: String::from("Prueba"),
            fecha,
            moneda: String::from("USD"),
            monto_parte: String::from("1.00"),
            recibio_residuo: false,
        };
        assert!(EventoCambio::nuevo_gasto_grupo(id_fijo(9000), 0, datos_invalidos).is_err());
    }
}
