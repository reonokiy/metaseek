//! Web server module
//!
//! Provides the HTTP API and web interface for Metaseek.

mod handlers;
mod routes;
pub mod state;
mod templates;

pub use routes::create_router;
pub use state::AppState;
pub use templates::Templates;
