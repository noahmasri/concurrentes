//! Este modulo define los mensajes intercambiados entre el guardian y el local

use crate::pedido::Pedido;
use actix::prelude::*;

/// Mensajes que envia el guardian al local para notificar el resultado de determinado pedido
#[derive(Message, PartialEq)]
#[rtype(result = "()")]
pub enum Respuestas {
    PedidoConcretado(usize),
    StockInsuficiente(usize),
    ProductoNoDisponible(usize),
}

/// Mensaje que envia el local a si mismo, para responder ante un pedido concetrado
#[derive(Message)]
#[rtype(result = "()")]
pub struct PedidoConcretado {
    pub id: usize,
}

impl PedidoConcretado {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

///  Mensaje que envia el local a si mismo, para responder ante un stock insuficiente
#[derive(Message)]
#[rtype(result = "()")]
pub struct StockInsuficiente {
    pub id: usize,
}

impl StockInsuficiente {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

/// Mensaje que envia el local a si mismo, para responder ante un producto no disponible (sin stock)
#[derive(Message)]
#[rtype(result = "()")]
pub struct ProductoNoDisponible {
    pub id: usize,
}

impl ProductoNoDisponible {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

/// Mensaje que envia el local al guardian para descontar directamente el stock de un pedido, confirmandolo
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Descontar {
    pub pedido: Pedido,
    pub id: usize,
    pub sender: Recipient<Respuestas>,
}

impl Descontar {
    pub fn new(pedido: Pedido, id: usize, sender: Recipient<Respuestas>) -> Self {
        Self { pedido, id, sender }
    }
}
