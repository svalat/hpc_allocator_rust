/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// Provide wrappers on libc to avoid putting unsafe everywhere in the code and ease portability if function is not available.

//import
extern crate libc;

//import
use common::types::{Addr,Size};

/// wrapper to memcpy
pub fn memcpy(to: Addr, from: Addr, size: Size) {
	unsafe{libc::memcpy(to as *mut libc::c_void,from as *const libc::c_void,size)};
}

/// wrapper to memset
pub fn memset(ptr: Addr, value:  i32, size: Size) {
    unsafe{libc::memset(ptr as * mut libc::c_void,value,size);}
}
