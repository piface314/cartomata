//! # Cartomata
//!
//! A library to create templates for TCG card images and render them automatically.

pub mod cli;
pub mod color;
pub mod data;
pub mod decode;
pub mod error;
pub mod layer;
pub mod template;
pub mod text;

pub use error::{Error, Result};
