//! Arnés compartido de los experimentos de `examples/`.
//!
//! Vive fuera de `src/` a propósito: es código de experimento, no del motor, y así no
//! entra en el crate que se empaqueta en el APK ni aparece en la referencia de módulos.
//!
//! Cada experimento lo incluye con `#[path = "comun/mod.rs"] mod comun;`, y el test de
//! `tests/arnes_experimentos_tests.rs` lo incluye con
//! `#[path = "../examples/comun/mod.rs"] mod comun;` para poder probarlo de verdad.
//!
//! Nada de aquí cambia lo que mide un experimento: es el mismo código que estaba
//! copiado en los 47, movido tal cual.

// Cada experimento usa sólo una parte del arnés; sin esto, los 47 compilan con avisos
// por cada ayudante que ese experimento concreto no llama.
#![allow(dead_code)]

pub mod color;
pub mod diagnostico;
pub mod entrada;
pub mod gpu;
pub mod registro;
