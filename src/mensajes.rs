//! Este modulo define la estructura de los mensajes con los que se comunicaran
//! los locales entre si, y los ecommerce con los locales

use colored::*;
use std::collections::HashSet;
use std::fmt;
use std::io::{self, Read};

use super::aliases::{IdLocal, IdPedido, Puerto};
use crate::pedido::Pedido;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Clone, Copy, Debug, FromPrimitive)]
/// Enumerado de los tipos de mensajes que existen para ser enviados por un socket
pub enum TipoMensaje {
    MensajeServidor = 0,
    MensajeEcommerce,
    MensajeDelegado,
    AckDelegado,
    AckEcommerce,
    Matar,
    Revivir,
}

impl TipoMensaje {
    /// Convierte un byte leido en un tipo de mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut tipo_msg: [u8; 1] = [0; 1];
        buf.read_exact(&mut tipo_msg)?;

        if let Some(tipo_msg) = TipoMensaje::from_u8(tipo_msg[0]) {
            return Ok(tipo_msg);
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("No existe el mensaje de tipo {}", tipo_msg[0]),
        ))
    }
}

/// Mensajes que envia el local al ecommerce para avisarle
/// cual fue el output de su pedido
#[derive(Debug)]
pub enum MensajesServidor {
    PedidoExitoso(IdPedido),
    PedidoCancelado(IdPedido),
    NoHayStock(IdPedido),
}

impl MensajesServidor {
    /// Convierte un mensaje en un array de bytes para poder ser enviado,
    /// agregandole a su vez el tipo
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.push(TipoMensaje::MensajeServidor as u8);

        match self {
            Self::PedidoExitoso(id_pedido) => {
                buf.push(0_u8);
                buf.extend(id_pedido.to_be_bytes())
            }
            Self::PedidoCancelado(id_pedido) => {
                buf.push(1_u8);
                buf.extend(id_pedido.to_be_bytes())
            }
            Self::NoHayStock(id_pedido) => {
                buf.push(2_u8);
                buf.extend(id_pedido.to_be_bytes())
            }
        };
        buf
    }

    /// Convierte bytes leidos en un mensaje del tipo MensajeServidor
    /// # Errors:
    /// * si el buffer de lectura pasado tiene menos bytes que los
    /// necesarios para completar el mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut msg_type: [u8; 1] = [0; 1];
        buf.read_exact(&mut msg_type)?;
        let mut id: [u8; 2] = [0; 2];
        buf.read_exact(&mut id)?;
        let id_pedido = <u16>::from_be_bytes(id);
        match <u8>::from_be_bytes(msg_type) {
            0 => Ok(Self::PedidoExitoso(id_pedido)),
            1 => Ok(Self::PedidoCancelado(id_pedido)),
            2 => Ok(Self::NoHayStock(id_pedido)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("Pedido Invalido"),
            )),
        }
    }
}

/// Mensaje inicial que envía el ecommerce a algun local para
/// realizar un pedido. el id del pedido es el id interno que le asigna
/// el ecommerce al pedido para reconocerlo, mientras que el id que se
/// encuentra dentro del pedido es el identificador del producto
#[derive(Debug, Clone, PartialEq)]
pub struct MensajeEcommerce {
    pub id_pedido: IdPedido,
    pub pedido: Pedido,
}

impl fmt::Display for MensajeEcommerce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} con id {}",
            self.pedido,
            self.id_pedido.to_string().blue()
        )
    }
}

impl MensajeEcommerce {
    /// Crea un nuevo pedido a partir de un identificador de pedido
    /// y un pedido
    pub fn new(id_pedido: IdPedido, pedido: Pedido) -> Self {
        Self { id_pedido, pedido }
    }

    pub fn get_id(&self) -> IdPedido {
        self.id_pedido
    }

    /// Convierte bytes leidos en un mensaje del tipo MensajeEcommerce
    /// # Errors:
    /// * si el buffer de lectura pasado tiene menos bytes que los
    /// necesarios para completar el mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut id_buf: [u8; 2] = [0; 2];
        buf.read_exact(&mut id_buf)?;
        let id_pedido = <u16>::from_be_bytes(id_buf);
        let pedido = Pedido::from_bytes(buf)?;
        Ok(Self::new(id_pedido, pedido))
    }

    /// Convierte un MensajeEcommerce en un array de bytes, incluyendo su
    /// tipo, para poder enviarlo por un stream.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf_message = Vec::new();
        buf_message.push(TipoMensaje::MensajeEcommerce as u8);

        buf_message.extend_from_slice(&(self.id_pedido).to_be_bytes());
        buf_message.extend(self.pedido.as_bytes());
        buf_message
    }
}

/// Mensaje que se envia a un ecommerce para avisarle que su pedido
/// fue recibido correctamente. Dado que se usa UDP como protocolo
/// de transporte, es recomendable hacer uso de este.
#[derive(Debug, PartialEq)]
pub struct AckEcommerce {
    pub id_pedido: IdPedido,
}

impl AckEcommerce {
    /// Crea un ack dado un id de pedido, para avisar que ese pedido fue recibido
    pub fn new(id_pedido: IdPedido) -> Self {
        Self { id_pedido }
    }

    /// Convierte bytes leidos en un mensaje del tipo AckEcommerce
    /// # Errors:
    /// * si el buffer de lectura pasado tiene menos bytes que los
    /// necesarios para completar el mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut id_buf: [u8; 2] = [0; 2];
        buf.read_exact(&mut id_buf)?;
        let id_pedido = <u16>::from_be_bytes(id_buf);
        Ok(Self::new(id_pedido))
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf_message = Vec::new();
        buf_message.push(TipoMensaje::AckEcommerce as u8);
        buf_message.extend_from_slice(&(self.id_pedido).to_be_bytes());
        buf_message
    }
}

/// Mensaje que envia un local a su siguiente cuando no puede
/// resolver un pedido por falta de stock. Incluye el mensaje
/// enviado por el ecommerce con toda la informacion del pedido,
/// el puerto del ecommerce que lo pidio y la lista de locales
/// que no pudieron resolver el pedido.
#[derive(Debug)]
pub struct MensajeDelegado {
    pub mensaje_ecommerce: MensajeEcommerce,
    pub puerto_ecommerce: Puerto,
    pub locales_ack: HashSet<IdLocal>,
}

impl fmt::Display for MensajeDelegado {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Me delegaron {} realizado por ecommerce en puerto {}",
            self.mensaje_ecommerce,
            self.puerto_ecommerce.to_string().green()
        )
    }
}

impl MensajeDelegado {
    /// Crea un mensaje de delegacion
    pub fn new(
        mensaje_ecommerce: MensajeEcommerce,
        puerto: Puerto,
        locales_ack: HashSet<IdLocal>,
    ) -> Self {
        Self {
            mensaje_ecommerce,
            puerto_ecommerce: puerto,
            locales_ack,
        }
    }

    /// Devuelve el pedido al que hace referencia el mensaje
    pub fn get_pedido(&self) -> Pedido {
        self.mensaje_ecommerce.pedido.clone()
    }

    /// Devuelve el id del pedido al que hace referencia el mensaje
    pub fn get_id(&self) -> IdPedido {
        self.mensaje_ecommerce.id_pedido
    }

    /// Convierte bytes leidos en un mensaje del tipo MensajeDelegado
    /// # Errors:
    /// * si el buffer de lectura pasado tiene menos bytes que los
    /// necesarios para completar el mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mensaje_ecommerce = MensajeEcommerce::from_bytes(buf)?;
        let mut puerto_buf: [u8; 2] = [0; 2];
        buf.read_exact(&mut puerto_buf)?;
        let puerto_ecommerce = <u16>::from_be_bytes(puerto_buf);

        let mut buf_len: [u8; 2] = [0; 2];
        buf.read_exact(&mut buf_len)?;

        let len = <u16>::from_be_bytes(buf_len);
        let mut locales_ack: HashSet<IdLocal> = HashSet::new();

        for _ in 0..len {
            let mut buf_id: [u8; 2] = [0; 2];
            buf.read_exact(&mut buf_id)?;
            let id: IdLocal = <IdLocal>::from_be_bytes(buf_id);
            locales_ack.insert(id);
        }

        Ok(Self::new(mensaje_ecommerce, puerto_ecommerce, locales_ack))
    }

    /// Convierte un MensajeDelegado en un array de bytes para poder enviarlo
    /// por un socket
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(TipoMensaje::MensajeDelegado as u8);
        buf.extend(&self.mensaje_ecommerce.as_bytes()[1..]);
        buf.extend(self.puerto_ecommerce.to_be_bytes());
        buf.extend(
            u16::try_from(self.locales_ack.len())
                .unwrap_or(0)
                .to_be_bytes(),
        );
        self.locales_ack
            .iter()
            .for_each(|id| buf.extend(id.to_be_bytes()));
        buf
    }
}

/// Mensaje que envia un local a otro local para avisarle que
/// recibio correctamente la delegacion de un pedido
#[derive(Debug)]
pub struct AckDelegado {
    pub id_pedido: IdPedido,
    pub puerto: Puerto,
}

impl AckDelegado {
    /// Crea un ack dados el puerto del ecommerce que hizo el pedido
    /// y el id del pedido
    pub fn new(id_pedido: IdPedido, puerto: Puerto) -> Self {
        Self { id_pedido, puerto }
    }

    /// Convierte bytes leidos en un mensaje del tipo AckDelegado
    /// # Errors:
    /// * si el buffer de lectura pasado tiene menos bytes que los
    /// necesarios para completar el mensaje
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut id_buf: [u8; 2] = [0; 2];
        buf.read_exact(&mut id_buf)?;
        let id_pedido = <u16>::from_be_bytes(id_buf);
        let mut puerto_buf: [u8; 2] = [0; 2];
        buf.read_exact(&mut puerto_buf)?;
        let puerto = <u16>::from_be_bytes(puerto_buf);
        Ok(Self::new(id_pedido, puerto))
    }

    /// Convierte un AckDelegado en un array de bytes para poder enviarlo
    /// por un socket
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf_mensaje = Vec::new();
        buf_mensaje.push(TipoMensaje::AckDelegado as u8);
        buf_mensaje.extend_from_slice(&(self.id_pedido).to_be_bytes());
        buf_mensaje.extend_from_slice(&(self.puerto).to_be_bytes());
        buf_mensaje
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io;

    #[test]
    fn test_constructor_mensaje_ecommerce() {
        let pedido = Pedido::new(2, 3);
        let msg = MensajeEcommerce::new(1, pedido).as_bytes();

        let mut cursor = io::Cursor::new(msg);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::MensajeEcommerce => {
                let msg = MensajeEcommerce::from_bytes(&mut cursor);
                assert!(msg.is_ok());
                let msg_ecommerce = msg.unwrap();
                assert_eq!(msg_ecommerce.id_pedido, 1);
                assert_eq!(msg_ecommerce.pedido.get_id(), 2);
                assert_eq!(msg_ecommerce.pedido.get_amount(), 3);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_mensaje_delegado() {
        let pedido = Pedido::new(2, 3);

        let msg_ecom = MensajeEcommerce::new(1, pedido);
        let mut set_delegados = HashSet::new();
        set_delegados.insert(1);
        set_delegados.insert(2);
        let msg = MensajeDelegado::new(msg_ecom.clone(), 3402, set_delegados).as_bytes();

        let mut cursor = io::Cursor::new(msg);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::MensajeDelegado => {
                let msg_recv = MensajeDelegado::from_bytes(&mut cursor);
                assert!(msg_recv.is_ok());
                let msg_delegado = msg_recv.unwrap();
                assert_eq!(msg_delegado.mensaje_ecommerce, msg_ecom);
                assert_eq!(msg_delegado.puerto_ecommerce, 3402);
                assert!(msg_delegado.locales_ack.contains(&1));
                assert!(msg_delegado.locales_ack.contains(&2));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_ack_delegado() {
        let ack_del = AckDelegado::new(12, 12000).as_bytes();

        let mut cursor = io::Cursor::new(ack_del);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::AckDelegado => {
                let msg_recv = AckDelegado::from_bytes(&mut cursor).unwrap();
                assert_eq!(msg_recv.id_pedido, 12);
                assert_eq!(msg_recv.puerto, 12000);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_ack_ecommerce() {
        let ack_ecom = AckEcommerce::new(12).as_bytes();

        let mut cursor = io::Cursor::new(ack_ecom);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::AckEcommerce => {
                let msg_recv = AckEcommerce::from_bytes(&mut cursor).unwrap();
                assert_eq!(msg_recv.id_pedido, 12);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_mensaje_servidor_exitoso() {
        let ack_ecom = MensajesServidor::PedidoExitoso(120).as_bytes();

        let mut cursor = io::Cursor::new(ack_ecom);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::MensajeServidor => {
                let msg_recv = MensajesServidor::from_bytes(&mut cursor).unwrap();
                match msg_recv {
                    MensajesServidor::PedidoExitoso(id) => assert_eq!(id, 120),
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_mensaje_servidor_cancelado() {
        let ack_ecom = MensajesServidor::PedidoCancelado(23).as_bytes();

        let mut cursor = io::Cursor::new(ack_ecom);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::MensajeServidor => {
                let msg_recv = MensajesServidor::from_bytes(&mut cursor).unwrap();
                match msg_recv {
                    MensajesServidor::PedidoCancelado(id) => assert_eq!(id, 23),
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_constructor_mensaje_servidor_sin_stock() {
        let ack_ecom = MensajesServidor::NoHayStock(652).as_bytes();

        let mut cursor = io::Cursor::new(ack_ecom);
        let tipo = TipoMensaje::from_bytes(&mut cursor);
        assert!(tipo.is_ok());
        match tipo.unwrap() {
            TipoMensaje::MensajeServidor => {
                let msg_recv = MensajesServidor::from_bytes(&mut cursor).unwrap();
                match msg_recv {
                    MensajesServidor::NoHayStock(id) => assert_eq!(id, 652),
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }
}
