//! Este modulo define aliases a los distintos tipos de datos
//! para hacer sencillo el pase de uno al otro y dar mayor
//! claridad al codigo

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    thread::JoinHandle,
};
use tokio::sync::{Mutex, Notify};

use crate::{ecommerce::handler::Handler, errores::ErrorEcommerce};

pub type IdPedido = u16;
pub type IdProducto = u16;
pub type CantidadProducto = u16;
pub type CantidadPedido = u8;
pub type IdLocal = u16;
pub type Puerto = u16;
pub type IdEcommerce = u16;
pub type MonitorAsync = (Mutex<HashSet<(Puerto, IdPedido)>>, Notify);
pub type TablaStock = HashMap<u16, u16>;
pub type Ecommerce = (Arc<Handler>, JoinHandle<Result<(), ErrorEcommerce>>);
