//! Este modulo define la estructura de un mensajero, encargado
//! del envio de mensajes por medio de un socket con el resto
//! de los locales e ecommerces

use crate::errores::ErrorMensajero;
use actix::ActorContext;
use actix::{fut, Actor, Context, Handler, Message, ResponseFuture, Running};
use actix_rt::net::UdpSocket;
use std::{net::SocketAddr, sync::Arc};

/// Estructura encargada de realizar el envio de mensajes hacia otros procesos
pub struct Mensajero {
    socket: Option<Arc<UdpSocket>>,
}

impl Mensajero {
    /// Crea un mensajero, recibiendo un socket por el cual hara el envio de mensajes
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket: Some(socket),
        }
    }
}

impl Actor for Mensajero {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("Frenando la ejecucion del mensajero");
        Running::Stop
    }
}

/// Mensaje para realizar un envio de datos a traves de un socket a la direccion pasada.
/// # Errors
/// * `ErrorMensajero::InternetCaido` si se desconecto la tienda del resto por problemas de conectividad
/// * `ErrorGuardian::DestinoInaccesible` si no se logra enviar el mensaje al receptor
#[derive(Message)]
#[rtype(result = "Result<(), ErrorMensajero>")]
pub struct Enviar {
    mensaje: Vec<u8>,
    target: SocketAddr,
}

impl Enviar {
    pub fn new(mensaje: Vec<u8>, target: SocketAddr) -> Self {
        Self { mensaje, target }
    }
}

impl Handler<Enviar> for Mensajero {
    type Result = ResponseFuture<Result<(), ErrorMensajero>>;
    fn handle(&mut self, msg: Enviar, _ctx: &mut Context<Self>) -> Self::Result {
        // si la tarea nacio antes de la muerte del socket, deberia ejecutarse igual
        let sock = self.socket.clone();
        match sock {
            Some(s) => Box::pin(async move {
                let res = s.send_to(&msg.mensaje, msg.target).await?;
                Ok(())
            }),
            None => Box::pin(fut::err(ErrorMensajero::InternetCaido)),
        }
    }
}

/// Mensaje para frenar la conexion a internet del mensajero.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Desconectar;

impl Handler<Desconectar> for Mensajero {
    type Result = ();
    fn handle(&mut self, _msg: Desconectar, _ctx: &mut Context<Self>) -> Self::Result {
        self.socket = None;
    }
}

/// Mensaje que permite devolver la conexion, enviandole un nuevo socket
/// por el que mandar los mensajes
#[derive(Message)]
#[rtype(result = "()")]
pub struct Reconectar {
    nuevo_sock: Arc<UdpSocket>,
}

impl Reconectar {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { nuevo_sock: socket }
    }
}

impl Handler<Reconectar> for Mensajero {
    type Result = ();
    fn handle(&mut self, msg: Reconectar, _ctx: &mut Context<Self>) -> Self::Result {
        self.socket = Some(msg.nuevo_sock);
    }
}

/// Mensaje que frena al actor tras no poder recuperarse de una falla
/// en la conectividad
#[derive(Message)]
#[rtype(result = "()")]
pub struct Matar;
impl Handler<Matar> for Mensajero {
    type Result = ();
    fn handle(&mut self, _msg: Matar, ctx: &mut Context<Self>) -> Self::Result {
        println!("[Mensajero] El sistema no se pudo recuperar del error, me voy");
        ctx.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use actix::Actor;
    use async_std::future;
    use tokio::net::UdpSocket;

    use super::*;

    #[actix_rt::test]
    async fn test_mensajero_envia_correctamente_cuando_esta_vivo() {
        //setup
        let socket_mensajero = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_recipiente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let msg: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mensajero = Mensajero::new(Arc::new(socket_mensajero)).start();
        //when envio un mensaje y el mensajero esta vivo
        let res = mensajero
            .send(Enviar::new(
                msg.clone(),
                socket_recipiente.local_addr().unwrap(),
            ))
            .await;
        //then puedo comunicarme con el mensajero y este puede enviar mi mensaje
        assert!(res.is_ok());
        assert!(res.unwrap().is_ok());

        // y el receptor los recibe bien
        let mut buf: [u8; 5] = [0; 5];
        socket_recipiente.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf.to_vec(), msg);
    }

    #[actix_rt::test]
    async fn test_mensajero_no_envia_cuando_esta_desconectado() {
        //setup
        let socket_mensajero = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_recipiente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let msg: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mensajero = Mensajero::new(Arc::new(socket_mensajero)).start();
        //when desconecto
        let res_desconexion = mensajero.send(Desconectar).await;
        assert!(res_desconexion.is_ok());
        //y envio algo
        let res = mensajero
            .send(Enviar::new(
                msg.clone(),
                socket_recipiente.local_addr().unwrap(),
            ))
            .await;

        //then puedo hablar con el mensajero pero me dice que no pudo enviar
        assert!(res.is_ok());
        assert!(res.unwrap().is_err());

        // y el receptor no recibe nada
        let mut buf: [u8; 5] = [0; 5];
        let future = socket_recipiente.recv_from(&mut buf);
        let dur = Duration::from_secs(2);
        assert!(future::timeout(dur, future).await.is_err());
    }

    #[actix_rt::test]
    async fn test_mensajero_no_recibe_cuando_esta_muerto() {
        //setup
        let socket_mensajero = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_recipiente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let msg: Vec<u8> = vec![1, 2, 3, 4, 5];

        let mensajero = Mensajero::new(Arc::new(socket_mensajero));
        let mensajero_addr = mensajero.start();
        //when mato
        assert!(mensajero_addr.send(Matar).await.is_ok());
        //y envio algo
        println!("voy a enviar");
        let res = mensajero_addr
            .send(Enviar::new(
                msg.clone(),
                socket_recipiente.local_addr().unwrap(),
            ))
            .await;
        assert!(res.is_err());

        let mut buf: [u8; 5] = [0; 5];
        let future = socket_recipiente.recv_from(&mut buf);
        let dur = Duration::from_secs(2);
        assert!(future::timeout(dur, future).await.is_err());
    }

    #[actix_rt::test]
    async fn test_mensajero_envia_luego_de_revivir() {
        //setup
        let socket_mensajero = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let socket_recipiente = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let msg: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mensajero = Mensajero::new(Arc::new(socket_mensajero)).start();
        //when desconecto
        let res_desconexion = mensajero.send(Desconectar).await;
        assert!(res_desconexion.is_ok());
        //y envio algo
        let res = mensajero
            .send(Enviar::new(
                msg.clone(),
                socket_recipiente.local_addr().unwrap(),
            ))
            .await;

        //then puedo hablar con el mensajero pero me dice que no pudo enviar
        assert!(res.is_ok());
        assert!(res.unwrap().is_err());

        // y el receptor no recibe nada
        let mut buf: [u8; 5] = [0; 5];
        let future = socket_recipiente.recv_from(&mut buf);
        let dur = Duration::from_secs(2);
        assert!(future::timeout(dur, future).await.is_err());
        let nuevo_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        //when reconecto
        let res_conexion = mensajero
            .send(Reconectar::new(Arc::new(nuevo_socket)))
            .await;
        assert!(res_conexion.is_ok());

        //y envio algo
        let res = mensajero
            .send(Enviar::new(
                msg.clone(),
                socket_recipiente.local_addr().unwrap(),
            ))
            .await;

        //then puedo comunicarme con el mensajero y este puede enviar mi mensaje
        assert!(res.is_ok());
        assert!(res.unwrap().is_ok());

        // y el receptor los recibe bien
        let mut buf: [u8; 5] = [0; 5];
        socket_recipiente.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf.to_vec(), msg);
    }
}
