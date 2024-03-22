use clap::Parser;
use pidgeonhole::desconexion::dios;

fn main() {
    let dios = dios::Dios::parse();
    dios.ejecutar();
}
