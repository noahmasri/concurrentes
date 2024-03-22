//! Este modulo define los metodos necesarios para el parseo de una tabla de stock
//! a partir de un archivo fuente
use crate::aliases::TablaStock;
use std::io::Read;

/// A partir de un reader en formato json, crea una tabla de stock. Simplemente
/// encapsula las funcionalidades de la libreria de json.
pub fn from_reader(reader: &mut dyn Read) -> Result<TablaStock, serde_json::Error> {
    serde_json::from_reader(reader)
}
