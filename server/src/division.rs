use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct Participante {
    pub usuario_id: Uuid,
    pub nombre_usuario: String,
    pub valor: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regla {
    Iguales,
    Porcentajes,
    MontosFijos,
}
#[derive(Debug, Serialize)]
pub struct Parte {
    pub usuario_id: Uuid,
    pub monto: String,
    pub recibio_residuo: bool,
}

// Representa dinero y porcentajes en centésimas enteras, sin coma flotante.
pub fn centesimas(texto: &str) -> Result<i64, String> {
    let (entero, fraccion) = texto.split_once('.').unwrap_or((texto, ""));
    if entero.is_empty()
        || !entero.bytes().all(|b| b.is_ascii_digit())
        || fraccion.len() > 2
        || !fraccion.bytes().all(|b| b.is_ascii_digit())
        || (texto.contains('.') && fraccion.is_empty())
    {
        return Err("Debe ser un decimal no negativo con hasta dos decimales.".into());
    }
    let e = entero.parse::<i64>().map_err(|_| "Monto fuera de rango.")?;
    let f = match fraccion.len() {
        0 => 0,
        1 => fraccion.parse::<i64>().unwrap() * 10,
        _ => fraccion.parse::<i64>().unwrap(),
    };
    e.checked_mul(100)
        .and_then(|v| v.checked_add(f))
        .ok_or_else(|| "Monto fuera de rango.".into())
}
pub fn formato(n: i64) -> String {
    format!("{}.{:02}", n / 100, n % 100)
}
pub fn dividir(
    total: &str,
    regla: &Regla,
    participantes: &[Participante],
) -> Result<Vec<Parte>, String> {
    let total = centesimas(total)?;
    if total <= 0 || total > 999_999_999_999 {
        return Err("El monto debe ser positivo y caber en la base de datos.".into());
    }
    if participantes.is_empty() {
        return Err("Debe existir al menos un participante.".into());
    }
    let mut ids = HashSet::new();
    if participantes
        .iter()
        .any(|p| !ids.insert(p.usuario_id) || p.nombre_usuario.is_empty())
    {
        return Err("Los participantes deben ser únicos.".into());
    }
    let mut orden: Vec<_> = participantes.iter().collect();
    orden.sort_by(|a, b| {
        a.nombre_usuario
            .cmp(&b.nombre_usuario)
            .then(a.usuario_id.cmp(&b.usuario_id))
    });
    let valores: Vec<i64> = match regla {
        Regla::Iguales => vec![1; orden.len()],
        _ => orden
            .iter()
            .map(|p| {
                centesimas(
                    p.valor
                        .as_deref()
                        .ok_or_else(|| "Falta un valor de división.".to_string())?,
                )
            })
            .collect::<Result<_, _>>()?,
    };
    let suma = valores
        .iter()
        .try_fold(0_i64, |a, b| a.checked_add(*b))
        .ok_or("Suma fuera de rango.")?;
    match regla {
        Regla::Porcentajes if suma != 10000 => {
            return Err(format!(
                "Los porcentajes suman {}; {} {}.",
                formato(suma),
                if suma < 10000 { "faltan" } else { "sobran" },
                formato((10000 - suma).abs())
            ));
        }
        Regla::MontosFijos if suma != total => {
            return Err(format!(
                "Los montos suman {}; {} {}.",
                formato(suma),
                if suma < total { "faltan" } else { "sobran" },
                formato((total - suma).abs())
            ));
        }
        _ => (),
    }
    let denominador = match regla {
        Regla::Iguales => orden.len() as i128,
        Regla::Porcentajes => 10000,
        Regla::MontosFijos => 1,
    };
    let mut partes: Vec<i64> = valores
        .iter()
        .map(|v| match regla {
            Regla::MontosFijos => *v,
            _ => ((total as i128 * *v as i128) / denominador) as i64,
        })
        .collect();
    let residuo = total - partes.iter().sum::<i64>();
    let mut result = Vec::with_capacity(orden.len());
    for (i, p) in orden.iter().enumerate() {
        let extra = (i as i64) < residuo;
        if extra {
            partes[i] += 1;
        }
        result.push(Parte {
            usuario_id: p.usuario_id,
            monto: formato(partes[i]),
            recibio_residuo: extra,
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(n: &str, v: Option<&str>) -> Participante {
        Participante {
            usuario_id: Uuid::new_v4(),
            nombre_usuario: n.into(),
            valor: v.map(str::to_owned),
        }
    }
    #[test]
    fn ejemplos_y_residuos() {
        let x = dividir(
            "100.00",
            &Regla::Iguales,
            &[p("carla", None), p("ana", None), p("beto", None)],
        )
        .unwrap();
        assert_eq!(
            x.iter().map(|p| p.monto.as_str()).collect::<Vec<_>>(),
            vec!["33.34", "33.33", "33.33"]
        );
        assert!(x[0].recibio_residuo);
        let x = dividir(
            "10.00",
            &Regla::Porcentajes,
            &[
                p("ana", Some("33.33")),
                p("beto", Some("33.33")),
                p("carla", Some("33.34")),
            ],
        )
        .unwrap();
        assert_eq!(
            x.iter().map(|p| p.monto.as_str()).collect::<Vec<_>>(),
            vec!["3.34", "3.33", "3.33"]
        );
        assert!(
            dividir("10", &Regla::Porcentajes, &[p("ana", Some("95"))])
                .unwrap_err()
                .contains("faltan 5.00")
        );
    }
    #[test]
    fn conserva_total_en_multiples_escenarios() {
        for n in 1..20 {
            for total in 1..500 {
                let ps: Vec<_> = (0..n).map(|i| p(&format!("u{i:02}"), None)).collect();
                let x = dividir(&formato(total), &Regla::Iguales, &ps).unwrap();
                assert_eq!(
                    x.iter().map(|p| centesimas(&p.monto).unwrap()).sum::<i64>(),
                    total
                );
            }
        }
    }
    #[test]
    fn porcentajes_y_balances_conservan_total_en_multiples_casos() {
        for monto in [1_i64, 10, 99, 100, 10001, 999_999_999_999] {
            for a in (0_i64..=10000).step_by(137) {
                let b = (10000 - a) / 3;
                let c = 10000 - a - b;
                let datos = [
                    p("ana", Some(&formato(a))),
                    p("beto", Some(&formato(b))),
                    p("carla", Some(&formato(c))),
                ];
                let partes = dividir(&formato(monto), &Regla::Porcentajes, &datos).unwrap();
                let suma = partes
                    .iter()
                    .map(|parte| centesimas(&parte.monto).unwrap())
                    .sum::<i64>();
                assert_eq!(suma, monto);
                let balance_total = monto - suma;
                assert_eq!(balance_total, 0);
            }
        }
    }

    #[test]
    fn montos_grandes_y_fijos() {
        let x = dividir(
            "9999999999.99",
            &Regla::MontosFijos,
            &[p("ana", Some("9999999999.99"))],
        )
        .unwrap();
        assert_eq!(x[0].monto, "9999999999.99");
        assert!(
            dividir("10", &Regla::MontosFijos, &[p("ana", Some("11"))])
                .unwrap_err()
                .contains("sobran 1.00")
        );
    }
}
