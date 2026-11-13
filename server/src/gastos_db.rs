//! Persistencia de gastos compartidos; el acceso se comprueba en cada operación.
use crate::division::{self, Participante, Regla};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug)]
pub enum ErrorGasto {
    SinAcceso,
    Invalido(String),
    Base(sqlx::Error),
}
impl From<sqlx::Error> for ErrorGasto {
    fn from(error: sqlx::Error) -> Self {
        Self::Base(error)
    }
}
#[derive(Debug)]
pub struct ParticipanteNuevo {
    pub usuario_id: Uuid,
    pub valor: Option<String>,
}
#[derive(Debug)]
pub struct GastoNuevo {
    pub grupo_id: Uuid,
    pub pagado_por: Uuid,
    pub concepto: String,
    pub monto: String,
    pub fecha: NaiveDate,
    pub regla: Regla,
    pub participantes: Vec<ParticipanteNuevo>,
}
#[derive(Debug, Serialize)]
pub struct ParteGuardada {
    pub usuario_id: Uuid,
    pub nombre_usuario: String,
    pub monto: String,
    pub recibio_residuo: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct GastoGuardado {
    pub id: Uuid,
    pub pagado_por: Uuid,
    pub concepto: String,
    pub monto: String,
    pub fecha: String,
    pub regla: Option<String>,
    pub partes: Vec<ParteGuardada>,
}
#[derive(Debug, Serialize)]
pub struct Balance {
    pub usuario_id: Uuid,
    pub nombre_usuario: String,
    pub nombre_visible: String,
    pub pagado: String,
    pub corresponde: String,
    pub balance: String,
}

pub async fn registrar(
    pool: &PgPool,
    solicitante: Uuid,
    datos: GastoNuevo,
) -> Result<GastoGuardado, ErrorGasto> {
    if datos.concepto.trim().is_empty() {
        return Err(ErrorGasto::Invalido("El concepto es obligatorio.".into()));
    }
    let mut ids = HashSet::new();
    if datos.participantes.is_empty()
        || datos
            .participantes
            .iter()
            .any(|p| !ids.insert(p.usuario_id))
    {
        return Err(ErrorGasto::Invalido(
            "Selecciona participantes únicos.".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    // Bloquea las membresías durante la validación y escritura de las partes.
    let miembros = sqlx::query("SELECT m.usuario_id, u.nombre_usuario FROM grupo_miembros m JOIN usuarios u ON u.id=m.usuario_id WHERE m.grupo_id=$1 AND m.estado_invitacion='aceptada' FOR SHARE OF m")
        .bind(datos.grupo_id).fetch_all(&mut *tx).await?;
    let nombres: std::collections::HashMap<Uuid, String> = miembros
        .into_iter()
        .map(|r| (r.get("usuario_id"), r.get("nombre_usuario")))
        .collect();
    if !nombres.contains_key(&solicitante) {
        return Err(ErrorGasto::SinAcceso);
    }
    if !nombres.contains_key(&datos.pagado_por) {
        return Err(ErrorGasto::Invalido(
            "Quien pagó debe ser miembro del grupo.".into(),
        ));
    }
    let participantes: Vec<Participante> = datos
        .participantes
        .iter()
        .map(|p| {
            let nombre = nombres.get(&p.usuario_id).ok_or_else(|| {
                ErrorGasto::Invalido("Cada participante debe ser miembro del grupo.".into())
            })?;
            Ok(Participante {
                usuario_id: p.usuario_id,
                nombre_usuario: nombre.clone(),
                valor: p.valor.clone(),
            })
        })
        .collect::<Result<_, ErrorGasto>>()?;
    let partes = division::dividir(&datos.monto, &datos.regla, &participantes)
        .map_err(ErrorGasto::Invalido)?;
    let regla = match datos.regla {
        Regla::Iguales => "iguales",
        Regla::Porcentajes => "porcentajes",
        Regla::MontosFijos => "montos_fijos",
    };
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO gastos(id,grupo_id,pagado_por,concepto,monto,fecha_gasto,regla_division) VALUES ($1,$2,$3,$4,$5::text::numeric,$6::date::timestamp AT TIME ZONE 'UTC',$7)")
        .bind(id).bind(datos.grupo_id).bind(datos.pagado_por).bind(datos.concepto.trim()).bind(&datos.monto).bind(datos.fecha).bind(regla).execute(&mut *tx).await?;
    let mut guardadas = Vec::with_capacity(partes.len());
    for parte in partes {
        sqlx::query("INSERT INTO gasto_participantes(gasto_id,usuario_id,monto_parte,recibio_residuo) VALUES($1,$2,$3::text::numeric,$4)")
            .bind(id).bind(parte.usuario_id).bind(&parte.monto).bind(parte.recibio_residuo).execute(&mut *tx).await?;
        guardadas.push(ParteGuardada {
            usuario_id: parte.usuario_id,
            nombre_usuario: nombres[&parte.usuario_id].clone(),
            monto: parte.monto,
            recibio_residuo: Some(parte.recibio_residuo),
        });
    }
    tx.commit().await?;
    Ok(GastoGuardado {
        id,
        pagado_por: datos.pagado_por,
        concepto: datos.concepto.trim().into(),
        monto: division::formato(division::centesimas(&datos.monto).map_err(ErrorGasto::Invalido)?),
        fecha: datos.fecha.to_string(),
        regla: Some(regla.into()),
        partes: guardadas,
    })
}

/// Lectura en una transacción de instantánea coherente para gastos y balances.
pub async fn consultar(
    pool: &PgPool,
    solicitante: Uuid,
    grupo: Uuid,
) -> Result<(Vec<GastoGuardado>, Vec<Balance>), ErrorGasto> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let miembro: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM grupo_miembros WHERE grupo_id=$1 AND usuario_id=$2 AND estado_invitacion='aceptada')").bind(grupo).bind(solicitante).fetch_one(&mut *tx).await?;
    if !miembro {
        return Err(ErrorGasto::SinAcceso);
    }
    // Un gasto histórico inconsistente no debe producir un balance aparentemente válido.
    let inconsistente: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM gastos g WHERE g.grupo_id=$1 AND (           NOT EXISTS (SELECT 1 FROM grupo_miembros m WHERE m.grupo_id=g.grupo_id AND m.usuario_id=g.pagado_por AND m.estado_invitacion='aceptada')            OR NOT EXISTS (SELECT 1 FROM gasto_participantes p WHERE p.gasto_id=g.id)            OR (SELECT COALESCE(SUM(p.monto_parte),0) FROM gasto_participantes p WHERE p.gasto_id=g.id) <> g.monto            OR EXISTS (SELECT 1 FROM gasto_participantes p WHERE p.gasto_id=g.id AND NOT EXISTS (SELECT 1 FROM grupo_miembros m WHERE m.grupo_id=g.grupo_id AND m.usuario_id=p.usuario_id AND m.estado_invitacion='aceptada'))))"
    ).bind(grupo).fetch_one(&mut *tx).await?;
    if inconsistente {
        return Err(ErrorGasto::Invalido(
            "Los datos de gastos del grupo son inconsistentes.".into(),
        ));
    }
    let filas = sqlx::query("SELECT id,pagado_por,concepto,monto::text AS monto,(fecha_gasto AT TIME ZONE 'UTC')::date::text AS fecha,regla_division FROM gastos WHERE grupo_id=$1 ORDER BY fecha_gasto,id").bind(grupo).fetch_all(&mut *tx).await?;
    let mut gastos = Vec::with_capacity(filas.len());
    for fila in filas {
        let id: Uuid = fila.get("id");
        let rows=sqlx::query("SELECT p.usuario_id,u.nombre_usuario,p.monto_parte::text AS monto,p.recibio_residuo FROM gasto_participantes p JOIN usuarios u ON u.id=p.usuario_id WHERE p.gasto_id=$1 ORDER BY u.nombre_usuario,p.usuario_id").bind(id).fetch_all(&mut *tx).await?;
        let partes = rows
            .into_iter()
            .map(|r| ParteGuardada {
                usuario_id: r.get("usuario_id"),
                nombre_usuario: r.get("nombre_usuario"),
                monto: r.get("monto"),
                recibio_residuo: r.get("recibio_residuo"),
            })
            .collect();
        gastos.push(GastoGuardado {
            id,
            pagado_por: fila.get("pagado_por"),
            concepto: fila.get("concepto"),
            monto: fila.get("monto"),
            fecha: fila.get("fecha"),
            regla: fila.get("regla_division"),
            partes,
        });
    }
    let filas=sqlx::query("SELECT u.id,u.nombre_usuario,u.nombre_visible, COALESCE(p.pagado,0)::numeric(30,2)::text AS pagado, COALESCE(d.corresponde,0)::numeric(30,2)::text AS corresponde, (COALESCE(p.pagado,0)-COALESCE(d.corresponde,0))::numeric(30,2)::text AS balance FROM grupo_miembros m JOIN usuarios u ON u.id=m.usuario_id LEFT JOIN (SELECT pagado_por,SUM(monto) pagado FROM gastos WHERE grupo_id=$1 GROUP BY pagado_por) p ON p.pagado_por=u.id LEFT JOIN (SELECT gp.usuario_id,SUM(gp.monto_parte) corresponde FROM gasto_participantes gp JOIN gastos g ON g.id=gp.gasto_id WHERE g.grupo_id=$1 GROUP BY gp.usuario_id) d ON d.usuario_id=u.id WHERE m.grupo_id=$1 AND m.estado_invitacion='aceptada' ORDER BY u.nombre_usuario,u.id").bind(grupo).fetch_all(&mut *tx).await?;
    let balances = filas
        .into_iter()
        .map(|r| Balance {
            usuario_id: r.get("id"),
            nombre_usuario: r.get("nombre_usuario"),
            nombre_visible: r.get("nombre_visible"),
            pagado: r.get("pagado"),
            corresponde: r.get("corresponde"),
            balance: r.get("balance"),
        })
        .collect();
    tx.commit().await?;
    Ok((gastos, balances))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requiere PostgreSQL aislado de pruebas"]
    async fn migracion_conserva_gastos_anteriores_sin_inventar_residuos() {
        let url = std::env::var("E5_TEST_DATABASE_URL").expect("Configura E5_TEST_DATABASE_URL");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let esquema = format!("e6_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE SCHEMA {esquema}"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(&format!("SET search_path TO {esquema}"))
            .execute(&pool)
            .await
            .unwrap();
        for sql in [
            include_str!("../migrations/001_usuarios.sql"),
            include_str!("../migrations/002_grupos.sql"),
            include_str!("../migrations/003_gastos.sql"),
            include_str!("../migrations/010_amistades.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&pool).await.unwrap();
        }
        let usuario = Uuid::new_v4();
        let grupo = Uuid::new_v4();
        let gasto = Uuid::new_v4();
        sqlx::query("INSERT INTO usuarios(id,nombre_usuario,nombre_visible,correo,password_hash) VALUES($1,'ana','Ana','ana@example.com','hash')").bind(usuario).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO grupos(id,nombre,moneda,creador_id) VALUES($1,'Anterior','USD',$2)",
        )
        .bind(grupo)
        .bind(usuario)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO gastos(id,grupo_id,pagado_por,concepto,monto) VALUES($1,$2,$3,'Anterior',12.34)").bind(gasto).bind(grupo).bind(usuario).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO gasto_participantes(gasto_id,usuario_id,monto_parte) VALUES($1,$2,12.34)",
        )
        .bind(gasto)
        .bind(usuario)
        .execute(&pool)
        .await
        .unwrap();
        crate::db::ejecutar_migraciones(&pool).await.unwrap();
        let row=sqlx::query("SELECT g.monto::text AS monto,g.regla_division,p.monto_parte::text AS parte,p.recibio_residuo FROM gastos g JOIN gasto_participantes p ON p.gasto_id=g.id WHERE g.id=$1").bind(gasto).fetch_one(&pool).await.unwrap();
        assert_eq!(row.get::<String, _>("monto"), "12.34");
        assert_eq!(row.get::<String, _>("parte"), "12.34");
        assert_eq!(row.get::<Option<String>, _>("regla_division"), None);
        assert_eq!(row.get::<Option<bool>, _>("recibio_residuo"), None);
        crate::db::ejecutar_migraciones(&pool).await.unwrap();
        sqlx::query("SET search_path TO public")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(&format!("DROP SCHEMA {esquema} CASCADE"))
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requiere PostgreSQL aislado de pruebas"]
    async fn gastos_balances_aislamiento_y_rollback() {
        let url = std::env::var("E5_TEST_DATABASE_URL").expect("Configura E5_TEST_DATABASE_URL");
        let pool = crate::db::conectar_y_migrar(&url).await.unwrap();
        let ana = Uuid::new_v4();
        let beto = Uuid::new_v4();
        let carla = Uuid::new_v4();
        let ajeno = Uuid::new_v4();
        let grupo = Uuid::new_v4();
        let sufijo = Uuid::new_v4().simple().to_string();
        for (id, nombre) in [
            (ana, "ana"),
            (beto, "beto"),
            (carla, "carla"),
            (ajeno, "otro"),
        ] {
            let usuario = format!("{nombre}_{}", &sufijo[..8]);
            sqlx::query("INSERT INTO usuarios(id,nombre_usuario,nombre_visible,correo,password_hash) VALUES($1,$2,$3,$4,'hash')").bind(id).bind(&usuario).bind(nombre).bind(format!("{usuario}@example.com")).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO grupos(id,nombre,moneda,creador_id) VALUES($1,'Viaje','USD',$2)")
            .bind(grupo)
            .bind(ana)
            .execute(&pool)
            .await
            .unwrap();
        for id in [ana, beto, carla] {
            sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,estado_invitacion) VALUES($1,$2,'aceptada')").bind(grupo).bind(id).execute(&pool).await.unwrap();
        }
        let fecha = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        let integrantes = |valores: [Option<&str>; 3]| {
            [ana, beto, carla]
                .into_iter()
                .zip(valores)
                .map(|(usuario_id, valor)| ParticipanteNuevo {
                    usuario_id,
                    valor: valor.map(str::to_string),
                })
                .collect()
        };
        let gasto = registrar(
            &pool,
            ana,
            GastoNuevo {
                grupo_id: grupo,
                pagado_por: ana,
                concepto: "Cena".into(),
                monto: "100.00".into(),
                fecha,
                regla: Regla::Iguales,
                participantes: integrantes([None, None, None]),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            gasto
                .partes
                .iter()
                .map(|p| p.monto.as_str())
                .collect::<Vec<_>>(),
            vec!["33.34", "33.33", "33.33"]
        );
        assert_eq!(
            gasto
                .partes
                .iter()
                .map(|p| p.recibio_residuo)
                .collect::<Vec<_>>(),
            vec![Some(true), Some(false), Some(false)]
        );
        registrar(
            &pool,
            beto,
            GastoNuevo {
                grupo_id: grupo,
                pagado_por: beto,
                concepto: "Transporte".into(),
                monto: "60.00".into(),
                fecha,
                regla: Regla::Porcentajes,
                participantes: integrantes([Some("50"), Some("25"), Some("25")]),
            },
        )
        .await
        .unwrap();
        let (gastos, balances) = consultar(&pool, carla, grupo).await.unwrap();
        assert_eq!(gastos.len(), 2);
        assert_eq!(
            balances
                .iter()
                .map(|b| b.balance.as_str())
                .collect::<Vec<_>>(),
            vec!["36.66", "11.67", "-48.33"]
        );
        assert!(matches!(
            consultar(&pool, ajeno, grupo).await,
            Err(ErrorGasto::SinAcceso)
        ));
        assert!(matches!(
            consultar(&pool, ajeno, Uuid::new_v4()).await,
            Err(ErrorGasto::SinAcceso)
        ));
        let err = registrar(
            &pool,
            ana,
            GastoNuevo {
                grupo_id: grupo,
                pagado_por: ana,
                concepto: "Malo".into(),
                monto: "10.00".into(),
                fecha,
                regla: Regla::Porcentajes,
                participantes: integrantes([Some("50"), Some("25"), Some("20")]),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err,ErrorGasto::Invalido(ref m) if m.contains("faltan 5.00")));
        let (guardados, _) = consultar(&pool, ana, grupo).await.unwrap();
        assert_eq!(guardados.len(), 2);
        let error_ajeno = registrar(
            &pool,
            ajeno,
            GastoNuevo {
                grupo_id: grupo,
                pagado_por: ana,
                concepto: "Ajeno".into(),
                monto: "1.00".into(),
                fecha,
                regla: Regla::Iguales,
                participantes: integrantes([None, None, None]),
            },
        )
        .await;
        assert!(matches!(error_ajeno, Err(ErrorGasto::SinAcceso)));
        sqlx::query(
            "UPDATE gasto_participantes SET monto_parte=0 WHERE gasto_id=$1 AND usuario_id=$2",
        )
        .bind(gasto.id)
        .bind(ana)
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            consultar(&pool, ana, grupo).await,
            Err(ErrorGasto::Invalido(_))
        ));
        sqlx::query(
            "UPDATE gasto_participantes SET monto_parte=33.34 WHERE gasto_id=$1 AND usuario_id=$2",
        )
        .bind(gasto.id)
        .bind(ana)
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
        let pool = PgPool::connect(&url).await.unwrap();
        let (guardados, _) = consultar(&pool, ana, grupo).await.unwrap();
        assert_eq!(guardados.len(), 2);
        let guardado = guardados.iter().find(|g| g.id == gasto.id).unwrap();
        assert_eq!(guardado.fecha, "2026-10-23");
        assert_eq!(guardado.partes[0].recibio_residuo, Some(true));
        sqlx::query("DELETE FROM grupos WHERE id=$1")
            .bind(grupo)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM usuarios WHERE id=ANY($1)")
            .bind(&[ana, beto, carla, ajeno][..])
            .execute(&pool)
            .await
            .unwrap();
    }
}
