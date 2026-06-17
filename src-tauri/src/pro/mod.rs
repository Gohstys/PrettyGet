// Capa Pro de PrettyGet. Todo lo de pago vive aquí, aislado del core gratuito.
// El gating es en tiempo de ejecución: el mismo binario es Free hasta que se
// activa una licencia válida.

pub mod commands;
pub mod daemon;
pub mod entitlements;
pub mod hwid;
pub mod iac;
pub mod license;
pub mod remote_deploy;
pub mod state_sync;

pub use entitlements::{AppState, Entitlements, EntitlementsView};
pub use license::Feature;
