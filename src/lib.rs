/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

//we want to avoid to take special language things inside the allocator
#![feature(lang_items,core,libc)]
#![no_std]
#![allow(dead_code)]

//load modules
mod common;
mod registry;
mod portability;
mod chunk;
mod mmsource;

#[cfg(not(test))]
pub mod export;
