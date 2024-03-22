//! Este modulo define la estructura de pedidos, los cuales son enviados entre los distintos procesos del sistema

use std::{
    fmt,
    io::{self, Read},
};

use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Un pedido esta definido por un id de producto, y una cantidad de producto a pedir
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Pedido {
    id_producto: u16,
    cantidad: u8,
}

impl Pedido {
    /// Crea un nuevo pedido con los parametros dados
    pub fn new(id_producto: u16, cantidad: u8) -> Self {
        Self {
            id_producto,
            cantidad,
        }
    }

    /// Obtiene el ID del producto
    pub fn get_id(&self) -> u16 {
        self.id_producto
    }

    /// Obtiene la cantidad de producto a pedir
    pub fn get_amount(&self) -> u8 {
        self.cantidad
    }

    /// Convierte bytes en un pedido valido, para que pueda ser enviado
    /// a traves de la red de forma correcta
    pub fn from_bytes(buf: &mut dyn Read) -> io::Result<Self> {
        let mut id_pr: [u8; 2] = [0; 2];
        let mut cant: [u8; 1] = [0; 1];
        buf.read_exact(&mut id_pr)?;
        buf.read_exact(&mut cant)?;
        let id_producto = <u16>::from_be_bytes(id_pr);
        let cantidad = <u8>::from_be_bytes(cant);

        Ok(Self {
            id_producto,
            cantidad,
        })
    }

    /// Convierte un pedido en un vector de bytes, para que pueda ser enviado
    /// a traves de la red de forma correcta
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf_message = Vec::new();
        buf_message.extend_from_slice(&(self.id_producto).to_be_bytes());
        buf_message.extend_from_slice(&(self.cantidad).to_be_bytes());
        buf_message
    }
}

impl fmt::Display for Pedido {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "pedido compuesto por {} unidades ",
            self.get_amount().to_string().blue()
        )?;
        write!(f, "del producto {}", self.get_id().to_string().blue())
    }
}

/// Parsea un lector de bytes (en formato json) en un vector de pedidos. Simplemente
/// encapsula las funcionalidades de la libreria de json.
pub fn from_reader(reader: &mut dyn Read) -> serde_json::Result<Vec<Pedido>> {
    serde_json::from_reader(reader)
}
