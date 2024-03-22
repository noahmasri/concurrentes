use aliases::{IdLocal, Puerto};

pub mod aliases;
pub mod desconexion;
pub mod ecommerce;
pub mod errores;
pub mod generators;
pub mod local;
pub mod mensajes;
pub mod pedido;

/// Convierte un identificador de un local, a su direccion IP
pub fn id_a_dir_local(id: IdLocal) -> String {
    format!("127.0.0.1:{}", 9000 + id)
}

/// Convierte un identificador de un local, a la direccion IP de su medico
pub fn id_a_dir_medico(id: IdLocal) -> String {
    format!("127.0.0.1:{}", 10000 + id)
}

/// Convierte un puerto de local a su identificador
pub fn puerto_a_id(puerto: Puerto) -> IdLocal {
    puerto - 9000
}

/// Convierte un puerto de ecommerce a su direccion IP
pub fn puerto_a_ip(puerto: Puerto) -> String {
    format!("127.0.0.1:{puerto}")
}

/// Obtiene el siguiente local en la lista, dado un local
pub fn siguiente_id_local(id: IdLocal) -> IdLocal {
    (id + 1) % u16::from(CANTIDAD_LOCALES)
}

/// Cantidad de locales a ejecutar
pub const CANTIDAD_LOCALES: u8 = 4;

/// Longitud maxima de los mensajes enviados entre los procesos
pub const MAX_MENSAJE: u8 = 100;
