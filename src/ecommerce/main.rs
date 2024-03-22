use std::fs::{self, File};
use std::path::PathBuf;

use pidgeonhole::ecommerce::handler;
use pidgeonhole::errores::{self, ErrorDuranteParseo, ErrorEcommerce};
use pidgeonhole::pedido;
use rand::seq::IteratorRandom;

fn obtener_archivo_pedidos_rand() -> Result<PathBuf, ErrorDuranteParseo> {
    let mut rng = rand::thread_rng();
    let files = fs::read_dir("configs/ecommerces")?;
    let file = files
        .choose(&mut rng)
        .ok_or(ErrorDuranteParseo::NoSeHalloArchivoPedidos)??;
    Ok(file.path())
}

fn main() -> Result<(), errores::Error> {
    let archivo_json = obtener_archivo_pedidos_rand()?;

    let mut pedidos_json = File::open(archivo_json).map_err(Into::<ErrorDuranteParseo>::into)?;

    let pedidos =
        pedido::from_reader(&mut pedidos_json).map_err(Into::<ErrorDuranteParseo>::into)?;

    let (handler, handle) =
        handler::Handler::new(pedidos.len()).map_err(Into::<ErrorEcommerce>::into)?;

    handler::Handler::procesar_pedidos(handler, pedidos)?;

    if handle.join().is_err() {
        println!("No pudo joinear el hilo del read loop")
    }

    Ok(())
}
