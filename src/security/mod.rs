// src/security/mod.rs — Security module
// Story 6.1: sandbox (Landlock/Seatbelt/namespace)
// Story 6.2: secrets (API key management)
// Story 6.x: hmac (IPC authentication)

pub mod hmac;
pub mod sandbox;
pub mod secrets;
