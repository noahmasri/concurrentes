//! Este modulo define tipos de errores que pueden darse en
//! la ejecucion

use actix::MailboxError;
use rayon::ThreadPoolBuildError;
use std::io;
use std::net::AddrParseError;
use std::sync::{MutexGuard, PoisonError, WaitTimeoutResult};
use tokio::task::JoinError;

/// Enumerativo que engloba a todos los tipos de errores posibles desde
/// todos los modulos
#[derive(Debug)]
pub enum Error {
    ErrorDeGuardian(ErrorGuardian),
    ErrorDeEcommerce(ErrorEcommerce),
    ErrorDeParseo(ErrorDuranteParseo),
    ErrorDeServidor(ErrorServidor),
    ErrorEnJoin,
    ErrorEnCtrlC,
}

impl From<ErrorDuranteParseo> for Error {
    fn from(err: ErrorDuranteParseo) -> Self {
        Error::ErrorDeParseo(err)
    }
}
impl From<JoinError> for Error {
    fn from(_err: JoinError) -> Self {
        Error::ErrorEnJoin
    }
}
impl From<ErrorGuardian> for Error {
    fn from(err: ErrorGuardian) -> Self {
        Error::ErrorDeGuardian(err)
    }
}

impl From<ErrorEcommerce> for Error {
    fn from(err: ErrorEcommerce) -> Self {
        Error::ErrorDeEcommerce(err)
    }
}

impl From<ErrorServidor> for Error {
    fn from(err: ErrorServidor) -> Self {
        Error::ErrorDeServidor(err)
    }
}

/// Enumerativo que define todos los errores que pueden darse
/// desde el guardian
#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorGuardian {
    NoHayStock,
    NoHaySuficienteStock,
    PedidoInexistente,
}

/// Enumerativo que define todos los errores que pueden darse
/// desde el ecommerce
#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorEcommerce {
    ErrorMonitor,
    ErrorCreandoTareas,
    PedidoTimeout,
    AckTimeout,
    CantidadCero,
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for ErrorEcommerce {
    fn from(_error: PoisonError<MutexGuard<'_, T>>) -> Self {
        ErrorEcommerce::ErrorMonitor
    }
}

impl From<ThreadPoolBuildError> for ErrorEcommerce {
    fn from(_value: ThreadPoolBuildError) -> Self {
        ErrorEcommerce::ErrorCreandoTareas
    }
}
impl<T> From<PoisonError<(MutexGuard<'_, T>, WaitTimeoutResult)>> for ErrorEcommerce {
    fn from(_err: PoisonError<(MutexGuard<'_, T>, WaitTimeoutResult)>) -> Self {
        ErrorEcommerce::ErrorCreandoTareas
    }
}

impl From<io::Error> for ErrorEcommerce {
    fn from(_err: io::Error) -> Self {
        ErrorEcommerce::ErrorCreandoTareas
    }
}

/// Enumerativo que define todos los errores que pueden darse
/// en el parseo de los archivos de stock y de pedidos
#[derive(Debug)]
pub enum ErrorDuranteParseo {
    NoSePudoAbrirArchivo,
    FormatoArchivoInvalido,
    NoSePudoObtenerId,
    NoSeHalloArchivoPedidos,
}

impl From<io::Error> for ErrorDuranteParseo {
    fn from(_err: io::Error) -> Self {
        ErrorDuranteParseo::NoSePudoAbrirArchivo
    }
}
impl From<serde_json::Error> for ErrorDuranteParseo {
    fn from(_err: serde_json::Error) -> Self {
        ErrorDuranteParseo::FormatoArchivoInvalido
    }
}

/// Enumerativo que define todos los errores que pueden darse
/// en el servidor y su contacto con los actores intervinientes
#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorServidor {
    ImposibleRevivir,
    ImposibleInicializar,
    GuardianNoDisponible,
    MensajeroNoDisponible,
    DireccionInvalidaEcommerce,
    PedidoInexistente,
    ErrorDesconocido,
}

impl From<io::Error> for ErrorServidor {
    fn from(_err: io::Error) -> Self {
        ErrorServidor::ImposibleRevivir
    }
}

impl From<AddrParseError> for ErrorServidor {
    fn from(_e: AddrParseError) -> Self {
        ErrorServidor::DireccionInvalidaEcommerce
    }
}

impl From<MailboxError> for ErrorServidor {
    fn from(_e: MailboxError) -> Self {
        ErrorServidor::MensajeroNoDisponible
    }
}

impl From<ErrorGuardian> for ErrorServidor {
    fn from(e: ErrorGuardian) -> Self {
        match e {
            ErrorGuardian::PedidoInexistente => ErrorServidor::PedidoInexistente,
            _ => ErrorServidor::ErrorDesconocido,
        }
    }
}

/// Enumerativo que define todos los errores que pueden darse
/// desde el mensajero
#[derive(Debug)]
pub enum ErrorMensajero {
    InternetCaido,
    DestinoInaccesible,
}

impl From<io::Error> for ErrorMensajero {
    fn from(_err: io::Error) -> Self {
        ErrorMensajero::DestinoInaccesible
    }
}
