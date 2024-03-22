//! Este modulo permite generar archivos de stock aleatorios, donde
//! una gran parte de las cosas son parametrizables. Los archivos generados
//! son en formato json, generando un diccionario de id productos
//! con su cantidad respectiva.
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
};

use rand::Rng;

use crate::aliases::TablaStock;

/// Genera una tabla de stock de forma aleatorio, con los parametros dados
fn generar_tabla_aleatoria(
    rango_cant: (u16, u16),
    rango_ids: (u16, u16),
    max_tam: u16,
) -> TablaStock {
    let mut tabla = HashMap::new();
    let mut rng_cant = rand::thread_rng();
    let mut rng_ids = rand::thread_rng();

    (0..max_tam).for_each(|_| {
        let cant = rng_cant.gen_range(rango_cant.0..=rango_cant.1);
        let id = rng_ids.gen_range(rango_ids.0..=rango_ids.1);
        tabla.insert(id, cant);
    });

    tabla
}

/// A partir de un nombre de archivo, un rango de la cantidad de productos
/// por id, un rango de los ids y una cantidad de maxima de productos disponibles
/// genera un archivo de stock. Se especifica el maximo ya que se genera esa
/// cantidad de combinaciones, pero podria pasar que se inserten ids duplicados,
/// y eso genere que hayan menos.
pub fn generar_arch_stock_aleatorio(
    nombre_arch: &str,
    rango_cant: (u16, u16),
    rango_ids: (u16, u16),
    max_tam: u16,
) -> io::Result<()> {
    let mut file = File::create(nombre_arch)?;
    let tabla_stock = generar_tabla_aleatoria(rango_cant, rango_ids, max_tam);
    let json_data = serde_json::to_string_pretty(&tabla_stock)?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::local::stock;

    use super::*;
    #[test]
    fn generar_archivos_randon() {
        let res = generar_arch_stock_aleatorio("configs/stock1.json", (0, 500), (0, 1000), 100);
        let mut stocks_json = File::open("configs/stock1.json").unwrap();
        let stocks = stock::from_reader(&mut stocks_json);

        assert!(res.is_ok());
        assert!(stocks.is_ok());
        assert!(stocks.unwrap().len() <= 100);
    }
}
