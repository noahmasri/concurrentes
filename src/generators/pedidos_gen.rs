//! Este modulo permite generar archivos de pedidos aleatorios, donde
//! una gran parte de las cosas son parametrizables. Los archivos generados
//! son en formato json, generando una lista de diccionarios de id productos
//! con su cantidad respectiva.

use std::{
    fs::File,
    io::{self, Write},
};

use rand::Rng;

use crate::pedido::Pedido;

/// Genera un vector aleatorio de pedidos, con los parametros dados
fn generar_pedidos_aleatorios(
    rango_cant: (u8, u8),
    rango_ids: (u16, u16),
    max_tam: u16,
) -> Vec<Pedido> {
    let mut pedidos = Vec::new();
    let mut rng_cant = rand::thread_rng();
    let mut rng_ids = rand::thread_rng();

    (0..max_tam).for_each(|_| {
        let cant = rng_cant.gen_range(rango_cant.0..=rango_cant.1);
        let id = rng_ids.gen_range(rango_ids.0..=rango_ids.1);
        pedidos.push(Pedido::new(id, cant));
    });
    pedidos
}

/// A partir de un nombre de archivo, un rango de la cantidad de productos
/// por pedido, un rango de los ids y una cantidad de pedidos genera un
/// archivo de pedidos
pub fn generar_arch_pedidos_aleatorio(
    nombre_arch: &str,
    rango_cant: (u8, u8),
    rango_ids: (u16, u16),
    max_tam: u16,
) -> io::Result<()> {
    let mut file = File::create(nombre_arch)?;
    let vec_pedidos = generar_pedidos_aleatorios(rango_cant, rango_ids, max_tam);
    let json_data = serde_json::to_string_pretty(&vec_pedidos)?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn generar_archivos_randon() {
        let res = generar_arch_pedidos_aleatorio("configs/pedidos1.json", (0, 250), (0, 1000), 100);
        let mut stocks_json = File::open("configs/pedidos1.json").unwrap();
        let pedidos = crate::pedido::from_reader(&mut stocks_json);

        assert!(res.is_ok());
        assert!(pedidos.is_ok());
        assert_eq!(pedidos.unwrap().len(), 100);
    }
}
