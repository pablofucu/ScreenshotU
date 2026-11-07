//! Integracion con Nintendo Switch (solo escritorio Windows). La logica pura esta en
//! `visor_core::nintendo`; aqui va lo que toca el sistema (credenciales, ventana de login, red).

#[allow(dead_code)] // hasta que lo use el cliente del historial (SW-16)
pub mod auth;
#[allow(dead_code)]
pub mod secrets;
