//! Crea 10 archivos de stock y 10 archivos de pedidos aleatorios, en la carpeta `configs`.

use pidgeonhole::generators::{
    pedidos_gen::generar_arch_pedidos_aleatorio, stock_gen::generar_arch_stock_aleatorio,
};
fn main() {
    for i in 0..10 {
        let nombre = format!("configs/stock{}.json", i);
        generar_arch_stock_aleatorio(nombre.as_str(), (0, 500), (0, 200), 100)
            .unwrap_or_else(|_| println!("Error en creacion"));
    }

    for i in 0..20 {
        let nombre = format!("configs/pedidos{}.json", i);
        generar_arch_pedidos_aleatorio(nombre.as_str(), (1, 20), (0, 200), 100)
            .unwrap_or_else(|_| println!("Error en creacion"));
    }
}
