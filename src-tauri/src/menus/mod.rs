// src-tauri/src/scripts/mod.rs
// Re-export scripts

mod autofill;
pub mod csdlnganh;
pub mod quanlytruonghoc;
pub mod taphuan;
pub mod temis;

pub mod quanlymatkhau;
pub use autofill::get_autofill_script;
