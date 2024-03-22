//! Este modulo contiene lo necesario para poder llevar a cabo pedidos, dado un local
//! inicializado. Requiere de la existencia del guardian, ya que hara a este los pedidos

use super::mensajes_actores::{
    Descontar, PedidoConcretado, ProductoNoDisponible, Respuestas, StockInsuficiente,
};
use crate::pedido::Pedido;
use actix::prelude::*;
use colored::Colorize;

/// Estructura de empleado. Cuenta con una direccion del gua
pub struct Empleado {
    guardian: Recipient<Descontar>,
}

impl Empleado {
    /// Crea un nuevo empleado dado un guardian
    pub fn new(guardian: Recipient<Descontar>) -> Self {
        Self { guardian }
    }
}

impl Actor for Empleado {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("Frenando la ejecucion del empleado");
        Running::Stop
    }
}

/// Mensaje para que procese los pedidos de forma asincrónica
/// enviando su dirección al guardian para recibir el resultado
#[derive(Message)]
#[rtype(result = "()")]
pub struct TomarPedido {
    pedido: Pedido,
    id_pedido: usize,
}

impl TomarPedido {
    pub fn new(pedido: Pedido, id_pedido: usize) -> Self {
        Self { pedido, id_pedido }
    }
}

impl Handler<TomarPedido> for Empleado {
    type Result = ();

    fn handle(&mut self, msg: TomarPedido, ctx: &mut Context<Self>) -> Self::Result {
        println!(
            "Se realizo localmente {} con id {}",
            msg.pedido, msg.id_pedido
        );
        self.guardian.do_send(Descontar::new(
            msg.pedido,
            msg.id_pedido,
            ctx.address().recipient(),
        ));
    }
}

/// Handler para un mensaje respuesta enviado por un guardian
impl Handler<Respuestas> for Empleado {
    type Result = ();

    fn handle(&mut self, msg: Respuestas, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            Respuestas::PedidoConcretado(id) => {
                ctx.address().do_send(PedidoConcretado::new(id));
            }
            Respuestas::ProductoNoDisponible(id) => {
                ctx.address().do_send(ProductoNoDisponible::new(id));
            }
            Respuestas::StockInsuficiente(id) => {
                ctx.address().do_send(StockInsuficiente::new(id));
            }
        }
    }
}

impl Handler<PedidoConcretado> for Empleado {
    type Result = ();

    fn handle(&mut self, msg: PedidoConcretado, _ctx: &mut Context<Self>) -> Self::Result {
        println!(
            "El pedido local con id {} fue exitoso!",
            msg.id.to_string().green()
        );
    }
}

impl Handler<StockInsuficiente> for Empleado {
    type Result = ();

    fn handle(&mut self, msg: StockInsuficiente, _ctx: &mut Context<Self>) -> Self::Result {
        println!(
            "No hay suficiente stock para manejar el pedido con id {}",
            msg.id.to_string().green()
        );
    }
}

impl Handler<ProductoNoDisponible> for Empleado {
    type Result = ();

    fn handle(&mut self, msg: ProductoNoDisponible, _ctx: &mut Context<Self>) -> Self::Result {
        println!(
            "El producto de {} no esta disponible",
            msg.id.to_string().green()
        );
    }
}
#[cfg(test)]
mod tests {
    use actix::{actors::mocker::Mocker, Actor, Recipient};

    use crate::{local::mensajes_actores::Descontar, pedido::Pedido};

    use super::{Empleado, TomarPedido};

    #[actix_rt::test]
    async fn test_escenario_se_realizan_multiples_pedidos_y_se_procesan_ordenados() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        //setup
        let guardian: Recipient<Descontar> =
            Mocker::<Descontar>::mock(Box::new(move |_msg, _ctx| {
                tx.try_send(_msg).unwrap();
                Box::new(Some(()))
            }))
            .start()
            .recipient();

        let empleado_addr = Empleado::new(guardian).start();

        let pedidos = vec![Pedido::new(1, 1), Pedido::new(2, 2)];
        //when empleado genera pedidos
        pedidos
            .iter()
            .enumerate()
            .for_each(|(id, pedido)| empleado_addr.do_send(TomarPedido::new(pedido.clone(), id)));

        //Then le llegan en orden y correctamente
        let rcved = rx.recv().await.unwrap();
        let msg1 = rcved.downcast_ref::<Descontar>().unwrap();

        assert_eq!((msg1.pedido).get_id(), pedidos[0].get_id());
        assert_eq!((msg1.pedido).get_amount(), pedidos[0].get_amount());
        assert_eq!(msg1.id, 0);
        assert_eq!(msg1.sender, empleado_addr.clone().recipient());

        let rcved = rx.recv().await.unwrap();

        let msg2 = rcved.downcast_ref::<Descontar>().unwrap();

        assert_eq!((msg2.pedido).get_id(), pedidos[1].get_id());
        assert_eq!((msg2.pedido).get_amount(), pedidos[1].get_amount());
        assert_eq!(msg2.id, 1);
        assert_eq!(msg2.sender, empleado_addr.recipient());
    }
}
