//! # Cartomata
//!
//! A library to create templates for TCG card images and render them automatically.

mod abox;
#[cfg(feature = "cli")]
pub mod cli;
pub mod data;
pub mod decode;
pub mod error;
pub mod image;
pub mod layer;
pub mod pipeline;
pub mod text;

pub use error::{Error, Result};
