//! Ease of use functions to make cleaner code.

// -------------------------------------------------------------------------------------------------

pub trait ErrOnSome {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>;
}

/// Return an error if `Some(_)`, otherwise do nothing.
impl<T> ErrOnSome for Option<T> {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>
    {
        match self {
            None => Ok(()),
            Some(_) => Err(err),
        }
    }
}

// -------------------------------------------------------------------------------------------------

pub trait ErrUnlessTrue {
    fn err_unless_true<E>(&self, err: E) -> Result<(), E>;
}

/// Return an error if `None` or `Some(false)`, otherwise do nothing.
impl ErrUnlessTrue for Option<bool> {
    fn err_unless_true<E>(&self, err: E) -> Result<(), E>
    {
        match self {
            None => Err(err),
            Some(false) => Err(err),
            Some(true) => Ok(()),
        }
    }
}

