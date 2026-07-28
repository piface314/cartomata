//! # Cartomata
//!
//! A library to create templates for TCG card images and render them automatically.

mod abox;
#[cfg(feature = "cli")]
pub mod cli;
pub mod data;
pub mod decode;
#[cfg(feature = "diff")]
pub mod diff;
pub mod error;
pub mod image;
pub mod layer;
pub mod logs;
pub mod pipeline;
pub mod template;
pub mod text;

pub use error::{Error, Result};
