mod result;

#[allow(clippy::module_inception)]
mod router;

use result::{amount_in_future, amount_out_future};
pub use router::SwapRouter;
