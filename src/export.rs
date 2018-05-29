
/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///Export the C API of the allocator

// Pull in the system libc library for what crt0.o likely requires
extern crate libc;

// Entry point for this program
#[no_mangle]
pub extern fn malloc(size: libc::size_t) -> *mut libc::c_void {
	let ret;
	unsafe {
		ret = libc::mmap(0 as *mut libc::c_void, size*4096,libc::PROT_READ | libc::PROT_WRITE, libc::MAP_ANON | libc::MAP_PRIVATE, 0,0);
		panic!("Hello");
	}
	ret
}

// These functions and traits are used by the compiler, but not
// for a bare-bones hello world. These are normally
// provided by libstd.
#[lang = "eh_personality"] 
extern fn eh_personality() {}

#[lang = "panic_fmt"] 
fn panic_fmt() -> ! { loop {} }

#[lang = "eh_unwind_resume"]
#[no_mangle]
pub extern fn rust_eh_unwind_resume() {
}

#[no_mangle]
pub extern fn rust_begin_unwind() {

}