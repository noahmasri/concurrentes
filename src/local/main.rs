//! Ejecuta un local, con el id enviado como argumento del programa
//! Para inicializarse, lee los archivos "configs/stock{ID}" y "configs/pedidos{ID}"

use actix::prelude::*;
use actix_rt::net::UdpSocket;
use mensajero::Mensajero;
use pidgeonhole::aliases::{IdLocal, TablaStock};
use pidgeonhole::id_a_dir_local;
use pidgeonhole::local::empleado::{self, TomarPedido};
use pidgeonhole::local::{guardian::Guardian, stock};
use pidgeonhole::pedido::{self, Pedido};
use std::env::{self, Args};
use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;

use pidgeonhole::errores::{Error, ErrorDuranteParseo, ErrorServidor};
use pidgeonhole::local::{mensajero, servidor};

/// Obtiene una tabla de stock del archivo preparado para el local dado
fn obtener_stock(id: u16) -> Result<TablaStock, ErrorDuranteParseo> {
    let archivo_stocks = format!("configs/stock{}.json", id);
    let mut stocks_json = File::open(archivo_stocks)?;
    let stocks = stock::from_reader(&mut stocks_json)?;
    Ok(stocks)
}

/// Obtiene una tabla de pedidos del archivo preparado para el local dado
fn obtener_pedidos(id: u16) -> Result<Vec<Pedido>, ErrorDuranteParseo> {
    let archivo_pedidos = format!("configs/pedidos{}.json", id);
    let mut pedidos_json = File::open(archivo_pedidos)?;
    let pedidos = pedido::from_reader(&mut pedidos_json)?;
    Ok(pedidos)
}

/// Obtiene el id del local, a partir de los argumentos del programa
fn obtener_id_local(args: &mut Args) -> Result<IdLocal, ErrorDuranteParseo> {
    let id_str: String = match args.nth(1) {
        Some(id_str) => id_str,
        None => {
            eprintln!("No recibio el id como primer argumento");
            return Err(ErrorDuranteParseo::NoSePudoObtenerId);
        }
    };

    let id: IdLocal = match id_str.parse() {
        Ok(i) => i,
        Err(_) => {
            eprintln!("No puedo parsear el id como primer argumento");
            return Err(ErrorDuranteParseo::NoSePudoObtenerId);
        }
    };
    Ok(id)
}

async fn inicializar_socket(id: IdLocal) -> Result<Arc<UdpSocket>, ErrorServidor> {
    match UdpSocket::bind(id_a_dir_local(id)).await {
        Ok(s) => Ok(Arc::new(s)),
        Err(_) => Err(ErrorServidor::ImposibleInicializar),
    }
}

async fn handle_exit() -> Result<(), Error> {
    signal::ctrl_c().await.map_err(|_e| Error::ErrorEnCtrlC)?;
    Ok(())
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let id = obtener_id_local(&mut env::args())?;
    let stocks = obtener_stock(id)?;
    let pedidos = obtener_pedidos(id)?;
    let guardian_addr: Addr<Guardian> = Guardian::new(stocks).start();
    let recipient = guardian_addr.clone().recipient();

    let handle_clientes = actix_rt::spawn(async move {
        let empleado_addr = empleado::Empleado::new(recipient).start();
        for (id, pedido) in pedidos.into_iter().enumerate() {
            empleado_addr.do_send(TomarPedido::new(pedido.clone(), id));
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    let socket = inicializar_socket(id).await?;
    let mensajero: Addr<Mensajero> = Mensajero::new(socket.clone()).start();
    let mut server_ecommerce = servidor::ServidorEcommerce::new(guardian_addr.clone(), id, socket);

    let handle_server =
        actix_rt::spawn(async move { server_ecommerce.procesar_pedidos(mensajero).await });
    let ctrlc = actix::spawn(async move { handle_exit().await });

    // Retorna cuando alguna de las ramas concurrentes termina su ejecucion
    tokio::select! {
        res = ctrlc => {
            let resultado = res?;
            if resultado.is_err(){
                println!("Hubo un error catcheando la interrupcion");
            }else{
                println!("Saliendo del programa debido a un keyboard interrupt (Ctrl+C)");
            }
        }
        server = handle_server => {
            if server.is_err()  {
                return Err(Error::ErrorEnJoin);
            }
        }
    }

    if handle_clientes.await.is_err() {
        return Err(Error::ErrorEnJoin);
    }
    println!("Finalizando el sistema de actores");
    actix_rt::System::current().stop();
    Ok(())
}
