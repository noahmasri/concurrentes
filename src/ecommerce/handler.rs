//! Este modulo contiene funciones que permiten el manejo apropiado
//! de los pedidos de un ecommerce

use crate::errores::ErrorEcommerce;
use colored::*;
use rayon::ThreadPoolBuilder;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self};
use std::time::Duration;

use crate::aliases::{Ecommerce, IdLocal, IdPedido};
use crate::mensajes::{AckEcommerce, MensajeEcommerce, MensajesServidor, TipoMensaje};
use crate::pedido::Pedido;

use crate::{id_a_dir_local, CANTIDAD_LOCALES, MAX_MENSAJE};
use rand::Rng;

/// Estructura que maneja el envio de pedidos a los locales, junto con la lectura de
/// acusos de recibo y de finalizacion.
pub struct Handler {
    socket: UdpSocket,
    pedidos_pendientes: (Mutex<HashMap<IdPedido, Pedido>>, Condvar),
    acks: (Mutex<HashSet<IdPedido>>, Condvar),
}

impl Handler {
    /// Inicializa un handler, al que se le pasa la cantidad de pedidos de los que se debera
    /// hacer cargo. Inicializa un hilo que escucha por un socket por las respuestas de los
    /// locales.
    pub fn new(cant_pedidos: usize) -> std::io::Result<Ecommerce> {
        let addr = "127.0.0.1:0";
        let socket = UdpSocket::bind(addr)?;
        let handler = Arc::new(Self {
            socket,
            pedidos_pendientes: (Mutex::new(HashMap::new()), Condvar::new()),
            acks: (Mutex::new(HashSet::new()), Condvar::new()),
        });

        let handler_clone = handler.clone();

        let handle = thread::spawn(move || handler_clone.read_loop(cant_pedidos));

        Ok((handler, handle))
    }

    /// Lee del socket asociado y espera a los acuses de recibo y las confirmaciones de
    /// los pedidos
    /// Esta funcion es bloqueante, y finalice una vez se lea la confirmacion de todos los pedidos
    fn read_loop(&self, mut cant_pedidos: usize) -> Result<(), ErrorEcommerce> {
        loop {
            let mut buf: [u8; MAX_MENSAJE as usize] = [0; MAX_MENSAJE as usize];
            if self.socket.recv_from(&mut buf).is_err() {
                eprintln!("No pudo leer mensaje del socket");
                continue;
            }

            let mut cursor = io::Cursor::new(buf);

            let tipo_msg = match TipoMensaje::from_bytes(&mut cursor) {
                Ok(tipo) => tipo,
                Err(error) => {
                    eprintln!("No pudo parsear el header: {:?}", error);
                    continue;
                }
            };

            match tipo_msg {
                TipoMensaje::AckEcommerce => {
                    self.procesar_ack_ecommerce(&mut cursor)?;
                }
                TipoMensaje::MensajeServidor => {
                    let mensaje = match MensajesServidor::from_bytes(&mut cursor) {
                        Ok(mensaje) => mensaje,
                        Err(error) => {
                            eprintln!("No pudo procesar mensaje servidor: {:?}", error);
                            continue;
                        }
                    };

                    let id = self.procesar_mensaje_servidor(mensaje);

                    if self.pedidos_pendientes.0.lock()?.remove(&id).is_some() {
                        self.pedidos_pendientes.1.notify_all();
                        cant_pedidos -= 1;
                        if cant_pedidos == 0 {
                            return Ok(());
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    /// Procesa un mensaje proveniente del servidor, y devuelve el id del pedido asociado.
    /// Imprime por pantalla el resultado del pedido
    fn procesar_mensaje_servidor(&self, mensaje: MensajesServidor) -> IdPedido {
        match mensaje {
            MensajesServidor::PedidoExitoso(id) => {
                println!("El pedido con id {} fue exitoso", id.to_string().blue());
                id
            }
            MensajesServidor::PedidoCancelado(id) => {
                println!("El pedido con id {} fue cancelado", id.to_string().blue());
                id
            }
            MensajesServidor::NoHayStock(id) => {
                println!(
                    "No hay stock en ninguna tienda para el pedido con id {}",
                    id.to_string().blue()
                );
                id
            }
        }
    }

    /// Procesa un ack proveniente del local, e imprime por pantalla el resultado.
    /// Notifica a los hilos esperando el ack para que puedan continuar
    fn procesar_ack_ecommerce(&self, cursor: &mut dyn Read) -> Result<(), ErrorEcommerce> {
        let ack = match AckEcommerce::from_bytes(cursor) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Hubo un error leyendo un ack de uno de mis pedidos: {}", e);
                return Ok(());
            }
        };

        let mut acks_guard = self.acks.0.lock()?;
        acks_guard.insert(ack.id_pedido);
        self.acks.1.notify_all();
        Ok(())
    }

    /// Devuelve el id de la tienda mas cercana al ecommerce (modelado con un random)
    pub fn encontrar_tienda_cercana() -> IdLocal {
        rand::thread_rng().gen_range(0..CANTIDAD_LOCALES) as IdLocal
    }

    /// Procesa todos los pedidos pasados por parametro, de forma concurrente. Crea una
    /// tarea en un threadpool por cada pedido
    pub fn procesar_pedidos(
        handler: Arc<Self>,
        pedidos: Vec<Pedido>,
    ) -> Result<(), ErrorEcommerce> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()?;

        for (id_pedido, pedido) in pedidos.into_iter().enumerate() {
            let arc_clone = handler.clone();
            pool.spawn(move || {
                if let Err(e) = arc_clone.procesar_pedido(id_pedido, pedido) {
                    eprintln!("Error procesando el pedido {}: {:?}", id_pedido, e);
                }
            });
        }

        Ok(())
    }

    /// Envia el pedido a la tienda mas cercana
    fn procesar_pedido(&self, id_pedido: usize, pedido: Pedido) -> Result<(), ErrorEcommerce> {
        if pedido.get_amount() == 0 {
            return Err(ErrorEcommerce::CantidadCero);
        }
        let id_pedido = id_pedido as IdPedido;
        let msg = MensajeEcommerce::new(id_pedido, pedido);

        let id_local = Self::encontrar_tienda_cercana();

        self.enviar_pedido(msg, id_local)
    }

    /// Espera el ack del pedido, y devuelve el resultado de la espera.
    /// Devuelve error si se cumple un tiempo limite sin recibir el ack.
    fn esperar_ack(
        &self,
        mensaje: MensajeEcommerce,
        id_local: IdLocal,
    ) -> Result<(), ErrorEcommerce> {
        let id_pedido = mensaje.id_pedido;
        let (_, wait_result) = self
            .acks
            .1
            .wait_timeout_while(self.acks.0.lock()?, Duration::from_millis(500), |acks| {
                !acks.remove(&id_pedido)
            })
            .map_err(Into::<ErrorEcommerce>::into)?;

        if wait_result.timed_out() {
            println!(
                "No recibi ack de pedido {}, reenviando a {}",
                id_pedido.to_string().blue(),
                (id_local + 1) % CANTIDAD_LOCALES as IdLocal
            );
            return Err(ErrorEcommerce::AckTimeout);
        } else {
            println!("Recibi ack de pedido {}", id_pedido.to_string().blue());
        }
        Ok(())
    }

    /// Espera a la finalicacion del pedido, y devuelve el resultado de la espera.
    /// Devuelve error si se cumple un tiempo limite sin recibir el ack
    fn esperar_finalizacion(
        &self,
        mensaje: MensajeEcommerce,
        id_local: IdLocal,
    ) -> Result<(), ErrorEcommerce> {
        self.pedidos_pendientes
            .0
            .lock()?
            .insert(mensaje.id_pedido, mensaje.pedido.clone());

        let (_, wait_result) = self
            .pedidos_pendientes
            .1
            .wait_timeout_while(
                self.pedidos_pendientes.0.lock()?,
                Duration::from_secs(3),
                |pedidos| pedidos.contains_key(&mensaje.id_pedido),
            )
            .map_err(Into::<ErrorEcommerce>::into)?;

        if wait_result.timed_out() {
            let siguiente_local = (id_local + 1) % CANTIDAD_LOCALES as IdLocal;
            self.enviar_pedido(mensaje, siguiente_local)?;
        }
        Ok(())
    }

    // Envia un pedido a una tienda y espera en el monitor a esperar que le devuelvan el ack
    // correspondiente. Si recibe el ack, espera por el veredicto final sobre el destino del
    // pedido, si no lo recibe envia a la tienda siguiente. Si el veredicto final no llega,
    // entonces vuelve a enviar el pedido a otra tienda.
    fn enviar_pedido(
        &self,
        mensaje: MensajeEcommerce,
        id_local: IdLocal,
    ) -> Result<(), ErrorEcommerce> {
        let dir_tienda_cercana = id_a_dir_local(id_local);

        println!(
            "Enviando {} a tienda {}",
            &mensaje,
            id_local.to_string().red()
        );

        let msg_bytes = mensaje.as_bytes();

        if self.socket.send_to(&msg_bytes, dir_tienda_cercana).is_err() {
            eprintln!("No pudo enviar mensaje a traves del socket");
        }

        if self.esperar_ack(mensaje.clone(), id_local).is_err() {
            self.enviar_pedido(mensaje, (id_local + 1) % CANTIDAD_LOCALES as IdLocal)?;
            return Ok(());
        }
        self.esperar_finalizacion(mensaje, id_local)
    }
}
