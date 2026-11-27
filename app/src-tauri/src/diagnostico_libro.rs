use crate::domain::{EventoLibro, ordenar_eventos_determinista, validar_proyeccion_eventos};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticoLibro {
    pub compatible: bool,
    pub total_eventos: usize,
    pub primer_evento_incompatible: Option<ReferenciaEvento>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenciaEvento {
    pub id: String,
    pub tipo: String,
    pub momento_registro: String,
    pub dispositivo_origen: String,
}

pub fn diagnosticar(eventos: &[EventoLibro]) -> DiagnosticoLibro {
    let ordenados: Vec<EventoLibro> = ordenar_eventos_determinista(eventos)
        .into_iter()
        .cloned()
        .collect();
    let primero = if validar_proyeccion_eventos(&ordenados).is_ok() {
        None
    } else {
        let (mut bajo, mut alto) = (0, ordenados.len());
        while bajo < alto {
            let medio = bajo + (alto - bajo) / 2;
            if validar_proyeccion_eventos(&ordenados[..=medio]).is_err() {
                alto = medio;
            } else {
                bajo = medio + 1;
            }
        }
        ordenados.get(bajo).map(|evento| ReferenciaEvento {
            id: evento.id.clone(),
            tipo: evento.tipo.clone(),
            momento_registro: evento.momento_registro.to_rfc3339(),
            dispositivo_origen: evento.dispositivo_origen.clone(),
        })
    };
    DiagnosticoLibro {
        compatible: primero.is_none(),
        total_eventos: eventos.len(),
        primer_evento_incompatible: primero,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RepositorioEnMemoria, RepositorioEventos, ejecutar_crear_cuenta};

    #[test]
    fn evento_local_con_reloj_atrasado_o_empatado_avanza_en_sqlite() {
        let ruta = std::env::temp_dir().join(format!("cc-reloj-{}.sqlite", uuid::Uuid::new_v4()));
        let mut repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta.clone()).unwrap();
        let instante = chrono::DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let crear = |id: &str, momento| EventoLibro {
            id: id.into(),
            tipo: "MonedaBaseCambiada".into(),
            momento_registro: momento,
            dispositivo_origen: "d".into(),
            datos_json: "{\"codigo\":\"USD\"}".into(),
        };
        repo.guardar_evento(&crear("primero", instante)).unwrap();
        repo.guardar_evento_local(&crear("segundo", instante))
            .unwrap();
        repo.guardar_evento_local(&crear("tercero", instante - chrono::Duration::days(1)))
            .unwrap();
        let eventos = repo.obtener_eventos().unwrap();
        assert_eq!(eventos[0].momento_registro, instante);
        assert!(eventos[0].momento_registro < eventos[1].momento_registro);
        assert!(eventos[1].momento_registro < eventos[2].momento_registro);
        assert!(diagnosticar(&eventos).compatible);
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn diagnostica_sin_mutar_y_exporta_libro_incompatible() {
        let mut repo = RepositorioEnMemoria::default();
        ejecutar_crear_cuenta(&mut repo, "Cuenta", "efectivo", "USD", "10", "d").unwrap();
        let mut eventos = repo.obtener_eventos().unwrap();
        assert!(diagnosticar(&eventos).compatible);
        let mut malo = eventos[0].clone();
        malo.id = "evento-invalido".into();
        malo.tipo = "TipoDesconocido".into();
        malo.momento_registro += chrono::Duration::seconds(1);
        repo.guardar_evento(&malo).unwrap();
        eventos.push(malo);
        let informe = diagnosticar(&eventos);
        assert!(!informe.compatible);
        assert_eq!(
            informe.primer_evento_incompatible.unwrap().id,
            "evento-invalido"
        );
        let (bytes, cantidad, _) = crate::respaldo::exportar_bytes(
            &repo,
            false,
            None,
            None,
            crate::respaldo::ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(cantidad, 2);
        assert!(
            crate::respaldo::parsear_eventos_ndjson(std::str::from_utf8(&bytes).unwrap()).is_err()
        );
        assert_eq!(repo.obtener_eventos().unwrap(), eventos);
    }
}
