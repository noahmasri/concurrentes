//! Este modulo contiene una estructura util que permite enviar señales a
//! una direccion derivada de un identificador para que corte su señal, o
//! que la retorne

use crate::mensajes::TipoMensaje;
use crate::{id_a_dir_local, id_a_dir_medico};
use clap::Parser;

use std::net::UdpSocket;

/// Estructura que envia mensajes de aviso a un local.
/// Contiene el identificador del local al que se desea avisar, y
/// un flag de si lo debe matar o revivir
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Dios {
    #[arg(short, long)]
    id: u16,

    #[arg(short, long, default_value_t = false)]
    kill: bool,
}

impl Dios {
    /// Ejecuta la accion determinada, imprimiendo por pantalla el resultado de la operación
    pub fn ejecutar(&self) {
        let puerta_al_cielo = match UdpSocket::bind("127.0.0.1:0") {
            Ok(socket) => socket,
            Err(error) => {
                println!("No se pudo abrir la puerta al cielo: {:?}", error);
                return;
            }
        };

        if self.kill {
            let direccion_objetivo = id_a_dir_local(self.id);
            match puerta_al_cielo.send_to(&[TipoMensaje::Matar as u8], direccion_objetivo) {
                Ok(_) => println!("Objetivo cumplido, la presa esta en el cielo"),
                Err(_) => println!("La presa fue dificil de matar, no se cumplio el objetivo"),
            }
        } else {
            let direccion_objetivo = id_a_dir_medico(self.id);
            match puerta_al_cielo.send_to(&[TipoMensaje::Revivir as u8], direccion_objetivo) {
                Ok(_) => println!("Objetivo cumplido, la presa volvio a la vida"),
                Err(_) => {
                    println!("No se pudo revivir, la esta pasando demasiado bien en el cielo")
                }
            }
        }
    }
}
