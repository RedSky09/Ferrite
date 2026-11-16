//! Asset loading and 2D sprite tooling.
//!
//! This crate knows nothing about the GPU. It decodes images from disk,
//! slices them into spritesheets, drives keyframe animation, and builds a
//! simple bitmap font. The renderer uploads the RGBA pixels it produces.

pub mod animation;
pub mod font;
pub mod image;
pub mod spritesheet;

pub use animation::{Animation, Frame};
pub use font::Font;
pub use image::Image;
pub use spritesheet::{Rect, Spritesheet};
