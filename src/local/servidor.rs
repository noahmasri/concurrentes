//! Este modulo contiene lo necesario para poder manejar los pedidos realizados por ecommrces
//! Requiere de la existencia del guardian, ya que hara a este los pedidos.

use crate::aliases::{IdLocal, IdPedido, MonitorAsync, Puerto};
use crate::local::guardian::{self, Guardian};
use crate::mensajes::{
    AckDelegado, AckEcommerce, MensajeDelegado, MensajeEcommerce, MensajesServidor, TipoMensaje,
};
use crate::{
    errores::ErrorServidor, id_a_dir_local, id_a_dir_medico, puerto_a_id, puerto_a_ip,
    siguiente_id_local, MAX_MENSAJE,
};
use actix::Addr;
use actix_rt::net::UdpSocket;
use actix_rt::time;
use async_recursion::async_recursion;
use colored::Colorize;
use rand::Rng;
use std::collections::HashSet;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout;

use crate::errores::ErrorMensajero;
use crate::local::mensajero::{Desconectar, Enviar, Matar, Mensajero, Reconectar};

/// Estructura que procesa los pedidos obtenidos recibidos por diversos ecommerces
/// mediante un socket, delegando el manejo del stock a un guardian, y los pedidos
/// que no puede cumplir a un local cercano.
pub struct ServidorEcommerce {
    guardian_addr: Addr<Guardian>,
    socket: Arc<UdpSocket>,
    id_local: IdLocal,
    acks_delegados: Arc<MonitorAsync>,
}

impl ServidorEcommerce {
    /// Crea un servidor a partir de un guardian y un id asignado. Inicializa el socket
    /// por el que va a escuchar los pedidos
    pub fn new(guardian_addr: Addr<Guardian>, id: IdLocal, socket: Arc<UdpSocket>) -> Self {
        Self {
            guardian_addr,
            socket,
            id_local: id,
            acks_delegados: Arc::new((Mutex::new(HashSet::new()), Notify::new())),
        }
    }

    /// Pone al servidor a escuchar por el socket por cualquier mensaje que podria llegar.
    /// Los mensajes validos son:
    /// * ack de una delegacion
    /// * mensaje de delegacion
    /// * mensaje de ecommerce
    /// * matar
    pub async fn procesar_pedidos(&mut self, mensajero: Addr<Mensajero>) {
        loop {
            let mut buf: [u8; MAX_MENSAJE as usize] = [0; MAX_MENSAJE as usize];
            let sender = match self.socket.recv_from(&mut buf).await {
                Ok((_, sender)) => sender,
                Err(_err) => {
                    eprintln!("No pudo leer del socket");
                    continue;
                }
            };

            let mut cursor = io::Cursor::new(buf);
            let tipo_msg = match TipoMensaje::from_bytes(&mut cursor) {
                Ok(tipo) => tipo,
                Err(error) => {
                    eprintln!("No pudo parsear el header: {:?}", error);
                    continue;
                }
            };
            let mensajero_addr = mensajero.clone();

            match tipo_msg {
                TipoMensaje::MensajeEcommerce => {
                    self.procesar_pedido_ecommerce(&mut cursor, sender, mensajero_addr)
                        .await;
                }
                TipoMensaje::MensajeDelegado => {
                    self.procesar_pedido_delegado(&mut cursor, sender, mensajero_addr)
                        .await;
                }
                TipoMensaje::AckDelegado => {
                    self.procesar_ack_delegado(&mut cursor).await;
                }
                TipoMensaje::Matar => match self.esperar_a_revivir(&mensajero_addr).await {
                    Ok(sock) => {
                        mensajero_addr.do_send(Reconectar::new(sock));
                        println!("Revivi, ahora a revivir al mensajero");
                    }
                    Err(_) => {
                        mensajero_addr.do_send(Matar);
                        println!("[Lector] El sistema no se pudo recuperar del error, me voy");
                        break;
                    }
                },
                _ => eprintln!("Recibi un mensaje desconocido"),
            }
        }
    }

    /// Cierra el socket, bindea a un nuevo puerto como medico, y espera a ser revivido de forma externa.
    /// Una vez es revivido, vuelve a abrir el socket para continuar la ejecucion
    pub async fn esperar_a_revivir(
        &mut self,
        mensajero: &Addr<Mensajero>,
    ) -> Result<Arc<UdpSocket>, ErrorServidor> {
        println!("Me mataron, tengo que esperar al medico");
        mensajero.do_send(Desconectar);
        self.socket = Arc::new(UdpSocket::bind(id_a_dir_medico(self.id_local)).await?);
        loop {
            let mut buf: [u8; MAX_MENSAJE as usize] = [0; MAX_MENSAJE as usize];
            if self.socket.recv_from(&mut buf).await.is_err() {
                eprintln!("Medico no pudo leer mensaje")
            };

            let mut cursor = std::io::Cursor::new(buf);
            let tipo_msg = match TipoMensaje::from_bytes(&mut cursor) {
                Ok(tipo) => tipo,
                Err(error) => {
                    eprintln!("No pudo parsear el header: {:?}", error);
                    continue;
                }
            };

            if let TipoMensaje::Revivir = tipo_msg {
                break;
            }
        }
        let sock = Arc::new(UdpSocket::bind(id_a_dir_local(self.id_local)).await?);
        self.socket = sock.clone();
        Ok(sock)
    }

    /// Realiza la logica de procesamiento del pedido de un ecommerce
    async fn procesar_pedido_ecommerce(
        &self,
        cursor: &mut dyn Read,
        sender: std::net::SocketAddr,
        mensajero: Addr<Mensajero>,
    ) {
        let mensaje_ecommerce = match MensajeEcommerce::from_bytes(cursor) {
            Ok(msg) => {
                println!(
                    "[SENDER PORT: {}] Recibi un pedido de Ecommerce: [{}]",
                    sender.port(),
                    &msg
                );
                msg
            }
            Err(e) => {
                eprintln!("Hubo un error leyendo un mensaje de ecommerce: {}", e);
                return;
            }
        };

        let msg = AckEcommerce::new(mensaje_ecommerce.id_pedido).as_bytes();
        let msg_error = format!(
            "No le pude enviar un ack al ecommerce en el puerto {} por el pedido {}",
            sender.port().to_string().green(),
            mensaje_ecommerce.get_id().to_string().blue()
        );
        let mensaje_delegado =
            MensajeDelegado::new(mensaje_ecommerce, sender.port(), HashSet::new());
        self.mandar_ack_y_procesar_pedido(msg, mensajero, sender, msg_error, mensaje_delegado)
            .await;
    }

    /// Realiza la logica de procesamiento del pedido de un ecommerce que me fue delegado
    /// por otro local
    async fn procesar_pedido_delegado(
        &self,
        cursor: &mut dyn Read,
        sender: std::net::SocketAddr,
        mensajero: Addr<Mensajero>,
    ) {
        let mensaje_delegado = match MensajeDelegado::from_bytes(cursor) {
            Ok(msg) => {
                println!("[SENDER ID: {}] {}", puerto_a_id(sender.port()), &msg);
                msg
            }
            Err(e) => {
                eprintln!("Hubo un error leyendo un mensaje de ecommerce: {}", e);
                return;
            }
        };

        let msg = AckDelegado::new(mensaje_delegado.get_id(), mensaje_delegado.puerto_ecommerce)
            .as_bytes();
        let sender_id = puerto_a_id(sender.port());
        let msg_error = format!(
            "No le pude enviar un ack al local {} por el pedido {}",
            sender_id,
            mensaje_delegado.get_id().to_string().blue()
        );

        self.mandar_ack_y_procesar_pedido(msg, mensajero, sender, msg_error, mensaje_delegado)
            .await;
    }

    /// Envia un mensaje de ack al remitente, y procesa el pedido dado
    /// Debido a que el mensaje se pasa como un vector de bytes, puede ser utilizado
    /// tanto para mensajes de commerce como para mensajes delegados
    async fn mandar_ack_y_procesar_pedido(
        &self,
        msg: Vec<u8>,
        mensajero: Addr<Mensajero>,
        sender: std::net::SocketAddr,
        msg_error: String,
        mensaje_delegado: MensajeDelegado,
    ) {
        if mensajero.send(Enviar::new(msg, sender)).await.is_err() {
            eprintln!("{}", msg_error);
            return;
        }

        let guardian_addr_clone = self.guardian_addr.clone();
        let ack_delegados_clone = self.acks_delegados.clone();
        let id_local_clone = self.id_local;
        actix_rt::spawn(async move {
            procesar_pedido(
                guardian_addr_clone,
                &mensajero,
                id_local_clone,
                mensaje_delegado,
                ack_delegados_clone,
            )
            .await
        });
    }

    /// Procesa la llegada de un ack de otro local, y notifica las tareas espectantes.
    /// para que puedan continuar su ejecucion
    async fn procesar_ack_delegado(&self, mut cursor: &mut dyn Read) {
        let ack = match AckDelegado::from_bytes(&mut cursor) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!(
                    "Hubo un error leyendo un ack de mensaje de delegacion: {}",
                    e
                );
                return;
            }
        };
        let mut set = self.acks_delegados.0.lock().await;
        set.insert((ack.puerto, ack.id_pedido));
        self.acks_delegados.1.notify_waiters();
    }
}

/// Define si un pedido sera entregado o no, se modela de forma aleatoria
async fn sera_entregado() -> bool {
    let mut rng = rand::thread_rng();
    let y: u64 = rng.gen_range(500..1500);
    let duracion = Duration::from_millis(y);
    time::sleep(duracion).await;
    y < 1000
}

/// Cancela el pedido dado, notificandole al ecommerce del resultado.
async fn cancelar_pedido(
    guardian_addr: Addr<Guardian>,
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
) -> Result<(), ErrorServidor> {
    guardian_addr
        .send(guardian::Cancelar::new(
            mensaje.get_id(),
            mensaje.puerto_ecommerce,
        ))
        .await
        .map_err(|_e| ErrorServidor::GuardianNoDisponible)??;

    let msg = MensajesServidor::PedidoCancelado(mensaje.get_id());
    let ecommerce: SocketAddr = puerto_a_ip(mensaje.puerto_ecommerce).parse()?;

    if let Err(e) = mensajero
        .send(Enviar::new(msg.as_bytes(), ecommerce))
        .await?
    {
        print!(
            "No le pude avisar al ecommerce {} sobre la cancelacion de su pedido {}",
            mensaje.puerto_ecommerce.to_string().green(),
            mensaje.get_id().to_string().blue()
        );
        match e {
            ErrorMensajero::DestinoInaccesible => {
                println!(" porque el ecommerce es inaccesible");
            }
            ErrorMensajero::InternetCaido => {
                println!(" porque se me cayo la conexion");
            }
        }
    }

    Ok(())
}

/// Confirma el pedido dado, notificando al ecommerce el resultado. Si no
/// se puede enviar un mensaje al ecommerce por falta de conectividad, entonces cancela el pedido.
async fn confirmar_pedido(
    guardian_addr: Addr<Guardian>,
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
) -> Result<(), ErrorServidor> {
    let msg = MensajesServidor::PedidoExitoso(mensaje.get_id());
    let ecommerce: SocketAddr = puerto_a_ip(mensaje.puerto_ecommerce).parse()?;
    match mensajero
        .send(Enviar::new(msg.as_bytes(), ecommerce))
        .await?
    {
        Ok(_) => {
            guardian_addr
                .send(guardian::Confirmar::new(
                    mensaje.get_id(),
                    mensaje.puerto_ecommerce,
                ))
                .await
                .map_err(|_e| ErrorServidor::GuardianNoDisponible)??;
        }
        Err(e) => {
            print!(
                "No le pude avisar al ecommerce {} sobre la confirmacion de su pedido {} ",
                mensaje.puerto_ecommerce.to_string().green(),
                mensaje.get_id().to_string().blue()
            );
            match e {
                ErrorMensajero::DestinoInaccesible => {
                    println!("porque el ecommerce es inaccesible");
                }
                ErrorMensajero::InternetCaido => {
                    println!("porque se me cayo la conexion");
                }
            }

            guardian_addr
                .send(guardian::Cancelar::new(
                    mensaje.get_id(),
                    mensaje.puerto_ecommerce,
                ))
                .await
                .map_err(|_e| ErrorServidor::GuardianNoDisponible)??;
        }
    }

    Ok(())
}

/// Define el resultado del pedido, enviando el resultado al ecommerce.
/// El pedido puede ser entregado, o cancelado
async fn resolver_pedido(
    guardian_addr: Addr<Guardian>,
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
) -> Result<(), ErrorServidor> {
    if sera_entregado().await {
        println!(
            "El pedido con id {} de ecommerce en puerto {} fue exitoso",
            mensaje.get_id().to_string().blue(),
            mensaje.puerto_ecommerce.to_string().green()
        );
        confirmar_pedido(guardian_addr, mensajero, mensaje).await
    } else {
        println!(
            "El pedido con id {} de ecommerce en puerto {} fue cancelado",
            mensaje.get_id().to_string().blue(),
            mensaje.puerto_ecommerce.to_string().green()
        );
        cancelar_pedido(guardian_addr, mensajero, mensaje).await
    }
}

/// Procesa un mensaje delegado, esta funcion tambien se utiliza para procesar mensajes de ecommerce,
/// encapsulandolos previamente en un mensaje delegado.
async fn procesar_pedido(
    guardian_addr: Addr<Guardian>,
    mensajero: &Addr<Mensajero>,
    id: IdLocal,
    mensaje: MensajeDelegado,
    acks: Arc<MonitorAsync>,
) -> Result<(), ErrorServidor> {
    let ecommerce: SocketAddr = puerto_a_ip(mensaje.puerto_ecommerce).parse()?;
    if mensaje.locales_ack.contains(&id) {
        println!(
            "Mensaje delegado repetido, con id {}",
            mensaje.get_id().to_string().blue()
        );
        let msg = MensajesServidor::NoHayStock(mensaje.get_id());

        if mensajero
            .send(Enviar::new(msg.as_bytes(), ecommerce))
            .await
            .is_err()
        {
            eprintln!(
                "No le pude avisar al ecommerce {} que nadie tiene stock para su pedido {}",
                mensaje.puerto_ecommerce.to_string().green(),
                mensaje.get_id()
            );
        }
        return Ok(());
    }

    let result = guardian_addr
        .send(guardian::Bloquear::new(
            mensaje.get_pedido(),
            mensaje.get_id(),
            mensaje.puerto_ecommerce,
        ))
        .await
        .map_err(|_e| ErrorServidor::GuardianNoDisponible)?;

    match result {
        Ok(_) => resolver_pedido(guardian_addr, mensajero, mensaje).await,
        Err(_e) => {
            delegar_pedido(acks, mensajero, mensaje, id, ecommerce).await;
            Ok(())
        }
    }
}

/// Delega el pedido al siguiente local disponible, o le envia al ecommerce
/// en caso de que no haya ninguno disponible.
async fn delegar_pedido(
    acks: Arc<MonitorAsync>,
    mensajero: &Addr<Mensajero>,
    mut mensaje: MensajeDelegado,
    id_local: IdLocal,
    ecommerce: SocketAddr,
) {
    println!(
        "No hay stock para el pedido con id {} en puerto {}",
        mensaje.get_id().to_string().blue(),
        mensaje.puerto_ecommerce.to_string().green()
    );

    mensaje.locales_ack.insert(id_local);
    enviar_a_siguiente_local(acks, mensajero, mensaje, id_local, id_local, ecommerce).await;
}

/// Espera a que reciba el ack para determinado mensaje. Esta funcion no devuelve
/// hasta que haya llegado el ack.
async fn esperar_mi_ack(acks: Arc<MonitorAsync>, id: (Puerto, IdPedido)) {
    loop {
        let mut lock = acks.0.lock().await;
        if lock.contains(&id) {
            lock.remove(&id);
            println!(
                "Recibi el ack por el pedido que delegue (puerto {}, id {})",
                id.1.to_string().green(),
                id.0.to_string().blue()
            );
            return;
        }
        drop(lock);
        acks.1.notified().await;
    }
}

/// Envia una notificacion de falta de stock al ecommerce.
async fn notificacion_falta_stock(
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
    ecommerce: SocketAddr,
) {
    let msg = MensajesServidor::NoHayStock(mensaje.get_id());

    if mensajero
        .send(Enviar::new(msg.as_bytes(), ecommerce))
        .await
        .is_err()
    {
        println!(
            "No le pude avisar al ecommerce {} que nadie tiene stock para su pedido {}",
            mensaje.puerto_ecommerce.to_string().green(),
            mensaje.get_id().to_string().blue()
        );
        return;
    }
    println!(
        "Avisando al ecommerce {} que nadie tiene stock para su pedido {}",
        mensaje.puerto_ecommerce.to_string().green(),
        mensaje.get_id().to_string().blue()
    );
}

/// Maneja el error del mensajero, imrpimiendo por pantalla
/// el error y reenviando el mensaje en caso de un error en el destino
async fn manejar_error_mensajero(
    e: ErrorMensajero,
    acks: Arc<MonitorAsync>,
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
    id_propia: IdLocal,
    ecommerce: SocketAddr,
    siguiente_local: IdLocal,
) {
    match e {
        ErrorMensajero::DestinoInaccesible => {
            println!(
                "La delegacion al local {} fallo. Enviare al siguiente local.",
                siguiente_local.to_string().blue()
            );
            enviar_a_siguiente_local(
                acks,
                mensajero,
                mensaje,
                siguiente_local,
                id_propia,
                ecommerce,
            )
            .await;
        }
        ErrorMensajero::InternetCaido => {
            println!("Se me cayo el internet, no pude delegar el pedido")
        }
    }
}

/// Delega el mensaje al siguiente local, y espera a recibir el ack.
/// Si no recibe el ack, envia al siguiente local de la lista
/// Si ningun local tiene stock, envia un mensaje al cliente.
#[async_recursion]
async fn enviar_a_siguiente_local(
    acks: Arc<MonitorAsync>,
    mensajero: &Addr<Mensajero>,
    mensaje: MensajeDelegado,
    id_local: IdLocal,
    id_propia: IdLocal,
    ecommerce: SocketAddr,
) {
    let siguiente_local = siguiente_id_local(id_local);

    if siguiente_local == id_propia {
        notificacion_falta_stock(mensajero, mensaje, ecommerce).await;
        return;
    }

    println!(
        "Delegando el pedido del ecommerce (puerto {}, id {}) al local {}",
        mensaje.puerto_ecommerce.to_string().green(),
        mensaje.get_id().to_string().blue(),
        siguiente_local
    );

    let dir_prox_local: SocketAddr = match id_a_dir_local(siguiente_local).parse() {
        Ok(d) => d,
        Err(_) => {
            println!("No se pudo procesar la direccion del siguiente local");
            return;
        }
    };

    let res = match mensajero
        .send(Enviar::new(mensaje.as_bytes(), dir_prox_local))
        .await
    {
        Ok(r) => r,
        Err(_) => {
            println!("No se pudo comunicar al mensajero, algo raro paso");
            return;
        }
    };

    if let Err(e) = res {
        manejar_error_mensajero(
            e,
            acks,
            mensajero,
            mensaje,
            id_propia,
            ecommerce,
            siguiente_local,
        )
        .await;
        return;
    }

    let timeout_dur = Duration::from_millis(500);
    let res = timeout(
        timeout_dur,
        esperar_mi_ack(acks.clone(), (mensaje.puerto_ecommerce, mensaje.get_id())),
    )
    .await;

    if res.is_err() {
        println!(
            "El local {} no esta disponible, enviando al siguiente",
            siguiente_local
        );
        enviar_a_siguiente_local(
            acks,
            mensajero,
            mensaje,
            siguiente_local,
            id_propia,
            ecommerce,
        )
        .await;
    };
}
