//! Este modulo define la estructura de guardian, encargada
//! del handle de los pedidos, ya sea interno del local como
//! de los ecommerce, derivados por el local

use actix::prelude::*;
use actix::{Actor, Context};

use std::cmp::Ordering;
use std::collections::HashMap;

use super::mensajes_actores::{Descontar, Respuestas};
use crate::aliases::TablaStock;
use crate::aliases::{CantidadPedido, CantidadProducto, IdPedido, IdProducto, Puerto};
use crate::errores::ErrorGuardian;
use crate::pedido::Pedido;

/// Estructura de guardian. Cuenta con el stock del local y
/// con un mapa en el que guarda los pedidos que fueron bloqueados
/// pero no aun confirmados, identificandolos por la tupla id
/// de pedido y puerto de Ecommerce que lo realizo
pub struct Guardian {
    stock: TablaStock,
    pedidos_bloqueados: HashMap<(IdPedido, Puerto), Pedido>,
}

impl Guardian {
    /// Crea un nuevo guardian dada un stock a resguardar
    pub fn new(stock: TablaStock) -> Self {
        Self {
            stock,
            pedidos_bloqueados: HashMap::new(),
        }
    }

    /// Descuenta una cantidad determinada de un producto, devolviendo
    /// el resultado de la operacion
    /// # Errors
    /// * `ErrorGuardian::NoHaySuficienteStock` si existe producto, pero el stock no es suficiente
    /// * `ErrorGuardian::NoHayStock` si no hay ninguna unidad del producto
    fn descontar_stock(
        &mut self,
        id: IdProducto,
        cantidad: CantidadPedido,
    ) -> Result<(), ErrorGuardian> {
        if let Some(cantidad_disp) = self.stock.get_mut(&id) {
            match cantidad_disp.cmp(&&mut (cantidad as u16)) {
                Ordering::Greater => {
                    *cantidad_disp -= cantidad as u16;
                    return Ok(());
                }
                Ordering::Equal => {
                    self.stock.remove(&id);
                    return Ok(());
                }
                Ordering::Less => return Err(ErrorGuardian::NoHaySuficienteStock),
            }
        }

        Err(ErrorGuardian::NoHayStock)
    }
}

impl Actor for Guardian {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("Frenando la ejecucion del guardian");
        Running::Stop
    }
}

impl Handler<Descontar> for Guardian {
    type Result = ();

    fn handle(&mut self, msg: Descontar, _ctx: &mut Context<Self>) -> Self::Result {
        match self.descontar_stock(msg.pedido.get_id(), msg.pedido.get_amount()) {
            Ok(_) => {
                msg.sender.do_send(Respuestas::PedidoConcretado(msg.id));
            }
            Err(ErrorGuardian::NoHaySuficienteStock) => {
                msg.sender.do_send(Respuestas::StockInsuficiente(msg.id));
            }
            Err(_) => {
                msg.sender.do_send(Respuestas::ProductoNoDisponible(msg.id));
            }
        }
    }
}

/// Mensaje para chequeo interno, que permite saber cuanto stock
/// hay dado un id de producto
#[derive(Message)]
#[rtype(result = "CantidadProducto")]
pub struct ObtenerStock {
    pub id: IdProducto,
}

impl Handler<ObtenerStock> for Guardian {
    type Result = CantidadProducto;

    fn handle(&mut self, msg: ObtenerStock, _ctx: &mut Context<Self>) -> Self::Result {
        self.stock.get(&msg.id).cloned().unwrap_or(0)
    }
}

/// Mensaje que permite bloquear cierta cantidad de stock.
/// # Errors
/// * si hay stock de un producto, pero no tanto como se pidio devuelve ErrorGuardian::NoHaySuficienteStock
/// * si no hay stock del producto devuelve ErrorGuardian::NoHayStock
#[derive(Message)]
#[rtype(result = "Result<(), ErrorGuardian>")]
pub struct Bloquear {
    pedido: Pedido,
    id: (IdPedido, Puerto),
}

impl Bloquear {
    /// Crea un nuevo mensaje de bloqueo de pedido
    pub fn new(pedido: Pedido, id_pedido: IdPedido, puerto: Puerto) -> Self {
        Self {
            pedido,
            id: (id_pedido, puerto),
        }
    }
}

impl Handler<Bloquear> for Guardian {
    type Result = Result<(), ErrorGuardian>;

    fn handle(&mut self, msg: Bloquear, _ctx: &mut Context<Self>) -> Self::Result {
        self.descontar_stock(msg.pedido.get_id(), msg.pedido.get_amount())?;
        self.pedidos_bloqueados.insert(msg.id, msg.pedido);

        Ok(())
    }
}

/// Mensaje que permite confirmar un pedido que se encontraba bloqueado, mediante su identificador.
/// # Errors
/// * si no habia un pedido bloqueado con ese identificador devuelve ErrorGuardian::PedidoInexistente
#[derive(Message)]
#[rtype(result = "Result<(), ErrorGuardian>")]
pub struct Confirmar {
    id: (IdPedido, Puerto),
}

impl Confirmar {
    /// Crea un nuevo mensaje de confirmacion de un pedido bloqueado
    pub fn new(id_pedido: IdPedido, puerto: Puerto) -> Self {
        Self {
            id: (id_pedido, puerto),
        }
    }
}

impl Handler<Confirmar> for Guardian {
    type Result = Result<(), ErrorGuardian>;

    fn handle(&mut self, msg: Confirmar, _ctx: &mut Context<Self>) -> Self::Result {
        match self.pedidos_bloqueados.remove(&msg.id) {
            None => Err(ErrorGuardian::PedidoInexistente),
            Some(_) => Ok(()),
        }
    }
}

/// Mensaje que permite cancelar un pedido que se encontraba bloqueado,
/// mediante su identificador, volviendo a dejar disponible el stock que
/// estaba bloqueado.
/// # Errors
/// * si no habia un pedido bloqueado con ese identificador devuelve ErrorGuardian::PedidoInexistente
#[derive(Message)]
#[rtype(result = "Result<(), ErrorGuardian>")]
pub struct Cancelar {
    id: (IdPedido, Puerto),
}

impl Cancelar {
    /// Crea un nuevo mensaje de cancelacion de un pedido bloqueado
    pub fn new(id_pedido: IdPedido, puerto: Puerto) -> Self {
        Self {
            id: (id_pedido, puerto),
        }
    }
}

impl Handler<Cancelar> for Guardian {
    type Result = Result<(), ErrorGuardian>;

    fn handle(&mut self, msg: Cancelar, _ctx: &mut Context<Self>) -> Self::Result {
        match self.pedidos_bloqueados.remove(&msg.id) {
            Some(p) => {
                *self.stock.entry(p.get_id()).or_insert(0) += p.get_amount() as CantidadProducto;
                Ok(())
            }
            None => Err(ErrorGuardian::PedidoInexistente),
        }
    }
}

#[cfg(test)]
mod tests {

    use actix::actors::mocker::Mocker;

    use super::*;

    fn crear_guardian() -> Addr<Guardian> {
        let mut stock = HashMap::new();
        stock.insert(1, 5);
        stock.insert(2, 5);
        stock.insert(3, 5);

        let guardian = Guardian::new(stock);
        guardian.start()
    }

    #[actix_rt::test]
    async fn test_escenario_descontar_descuenta_cuando_hay_stock_justo() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        //setup
        let empleado: Recipient<Respuestas> =
            Mocker::<Respuestas>::mock(Box::new(move |_msg, _ctx| {
                tx.try_send(_msg).unwrap();
                Box::new(Some(()))
            }))
            .start()
            .recipient();

        let guardian = crear_guardian();
        //when local le pide descontar
        guardian
            .send(Descontar::new(Pedido::new(1, 5), 1, empleado))
            .await
            .unwrap();

        // Then guardian le envia que pudo hacerlo bien
        match rx
            .recv()
            .await
            .unwrap()
            .downcast_ref::<Respuestas>()
            .unwrap()
        {
            Respuestas::PedidoConcretado(id) => assert_eq!(*id, 1),
            _ => panic!(),
        };
    }

    #[actix_rt::test]
    async fn test_escenario_descontar_descuenta_cuando_hay_stock() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        //setup
        let empleado: Recipient<Respuestas> =
            Mocker::<Respuestas>::mock(Box::new(move |_msg, _ctx| {
                tx.try_send(_msg).unwrap();
                Box::new(Some(()))
            }))
            .start()
            .recipient();

        let guardian = crear_guardian();
        guardian
            .send(Descontar::new(Pedido::new(1, 3), 1, empleado))
            .await
            .unwrap();

        // Then guardian le envia resultado pedido
        match rx
            .recv()
            .await
            .unwrap()
            .downcast_ref::<Respuestas>()
            .unwrap()
        {
            Respuestas::PedidoConcretado(id) => assert_eq!(*id, 1),
            _ => panic!(),
        };
    }

    #[actix_rt::test]
    async fn test_escenario_descontar_manda_error_cuando_no_hay_stock() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        //setup
        let empleado: Recipient<Respuestas> =
            Mocker::<Respuestas>::mock(Box::new(move |_msg, _ctx| {
                tx.try_send(_msg).unwrap();
                Box::new(Some(()))
            }))
            .start()
            .recipient();

        let guardian = crear_guardian();
        guardian
            .send(Descontar::new(Pedido::new(7, 3), 1, empleado))
            .await
            .unwrap();

        // Then guardian le envia resultado pedido
        match rx
            .recv()
            .await
            .unwrap()
            .downcast_ref::<Respuestas>()
            .unwrap()
        {
            Respuestas::ProductoNoDisponible(id) => assert_eq!(*id, 1),
            _ => panic!(),
        };
    }

    #[actix_rt::test]
    async fn test_escenario_descontar_manda_error_cuando_no_hay_stock_suficiente() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        //setup
        let empleado: Recipient<Respuestas> =
            Mocker::<Respuestas>::mock(Box::new(move |_msg, _ctx| {
                tx.try_send(_msg).unwrap();
                Box::new(Some(()))
            }))
            .start()
            .recipient();

        let guardian = crear_guardian();
        guardian
            .send(Descontar::new(Pedido::new(3, 7), 1, empleado))
            .await
            .unwrap();

        // Then guardian le envia resultado pedido
        match rx
            .recv()
            .await
            .unwrap()
            .downcast_ref::<Respuestas>()
            .unwrap()
        {
            Respuestas::StockInsuficiente(id) => assert_eq!(*id, 1),
            _ => panic!(),
        };
    }
    #[actix_rt::test]
    async fn bloquear_bloquea_un_stock_cuando_lo_hay() {
        let addr = crear_guardian();

        let res = addr
            .send(Bloquear::new(Pedido::new(1, 1), 1, 1))
            .await
            .unwrap();
        assert!(res.is_ok());
        assert_eq!(addr.send(ObtenerStock { id: 1 }).await.unwrap(), 4);
    }

    #[actix_rt::test]
    async fn bloquear_devuelve_error_cuando_no_hay_producto_con_ese_id() {
        let addr = crear_guardian();

        let res = addr
            .send(Bloquear::new(Pedido::new(7, 10), 1, 1))
            .await
            .unwrap();
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn bloquear_devuelve_error_cuando_no_hay_suficiente_stock_y_stock_se_mantiene() {
        let addr = crear_guardian();

        let res = addr
            .send(Bloquear::new(Pedido::new(3, 10), 1, 1))
            .await
            .unwrap();
        assert!(res.is_err());
        assert_eq!(addr.send(ObtenerStock { id: 3 }).await.unwrap(), 5);
    }

    #[actix_rt::test]
    async fn confirmar_devuelve_error_si_no_habia_pedido_bloqueado() {
        let addr = crear_guardian();

        let res = addr.send(Confirmar::new(1, 1)).await.unwrap();
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn cancelar_devuelve_error_si_no_habia_pedido_bloqueado() {
        let addr = crear_guardian();

        let res = addr.send(Cancelar::new(1, 1)).await.unwrap();
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn confirmar_devuelve_ok_si_habia_pedido_bloqueado() {
        let addr = crear_guardian();

        let res = addr
            .send(Bloquear::new(Pedido::new(1, 1), 1, 1))
            .await
            .unwrap();
        assert!(res.is_ok());

        let res = addr.send(Confirmar::new(1, 1)).await.unwrap();
        assert!(res.is_ok());
        assert_eq!(addr.send(ObtenerStock { id: 1 }).await.unwrap(), 4);
    }

    #[actix_rt::test]
    async fn cancelar_restaura_stock_si_habia_pedido_bloqueado() {
        let addr = crear_guardian();

        let res = addr
            .send(Bloquear::new(Pedido::new(1, 1), 1, 1))
            .await
            .unwrap();
        assert!(res.is_ok());

        let res = addr.send(Cancelar::new(1, 1)).await.unwrap();
        assert!(res.is_ok());

        assert_eq!(addr.send(ObtenerStock { id: 1 }).await.unwrap(), 5);
    }
}
