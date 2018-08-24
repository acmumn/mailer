//! Some utilities.

use failure::Error;

/// Combines two Results.
pub fn and<E, T, U>(l: Result<T, E>, r: Result<U, E>) -> Result<(T, U), E> {
    match (l, r) {
        (Ok(l), Ok(r)) => Ok((l, r)),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}

/// Logs an error, including its causes and backtrace (if possible).
pub fn log_err(err: Error) {
    let mut first = true;
    let num_errs = err.iter_chain().count();
    if num_errs <= 1 {
        error!("{}", err);
    } else {
        for cause in err.iter_chain() {
            if first {
                first = false;
                error!("           {}", cause);
            } else {
                error!("caused by: {}", cause);
            }
        }
    }
    let bt = err.backtrace().to_string();
    if bt != "" {
        error!("{}", bt);
    }
}
