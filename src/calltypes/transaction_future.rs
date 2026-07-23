use std::{future::Future, pin::Pin};

use crate::errors::UniswapV3Error;

pub type TransactionFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, UniswapV3Error>> + Send + 'static>>;
