// src/security/mod.rs — Security module
// sandbox (Landlock/Seatbelt/namespace)
// secrets (API key management)
// hmac (IPC authentication)

pub mod agent_spawn;
pub mod envelope;
pub mod hmac;
pub mod sandbox;
pub mod secrets;
