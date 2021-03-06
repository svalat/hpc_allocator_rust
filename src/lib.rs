/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

//we want to avoid to take special language things inside the allocator
#![feature(lang_items,libc)]
//#![feature(panic_implementation)]
#![feature(core_intrinsics)]
#![no_std]
#![allow(dead_code)]
#![feature(llvm_asm)]

//load modules
mod common;
mod registry;
mod portability;
mod chunk;
mod mmsource;
mod posix;

#[cfg(not(test))]
pub mod export;
