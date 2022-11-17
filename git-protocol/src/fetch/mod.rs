mod arguments;
pub use arguments::Arguments;

///
pub mod delegate;
#[cfg(any(feature = "async-client", feature = "blocking-client"))]
pub use delegate::Delegate;
pub use delegate::{Action, DelegateBlocking, LsRefsAction};

mod error;
pub use error::Error;
///
pub mod refs;
pub use refs::function::refs;
///
pub mod response;
pub use response::Response;

mod handshake;
pub use handshake::upload_pack as handshake;

#[cfg(test)]
mod tests;
