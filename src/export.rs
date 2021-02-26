
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

//use registry::registry::RegionRegistry;
//use mmsource::cached::CachedMMSource;
//use common::traits::{MemorySourcePtr};
use core::panic::PanicInfo;
use core::intrinsics;
use common::types::*;
//use chunk::huge::HugeChunkManager;
//use chunk::medium::manager::MediumChunkManager;
//use chunk::small::manager::SmallChunkManager;
//use common::consts::*;
//use common::shared::SharedPtrBox;
use posix::seq::SeqAllocator;

// Entry point for this program
#[no_mangle]
pub extern fn malloc(size: libc::size_t) -> *mut libc::c_void {
	/*let mut registry = RegionRegistry::new();
	let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
	let pmmsource = MemorySourcePtr::new_ref_mut(&mut mmsource);
	let mut huge_manager = HugeChunkManager::new(pmmsource.clone());
	let mut medium_manager = MediumChunkManager::new(true, Some(pmmsource.clone()));
	let mut small_manager = SmallChunkManager::new(true, Some(pmmsource.clone()));

	if size < 128 {
		small_manager.malloc(size,BASIC_ALIGN,false).0 as *mut libc::c_void
	} else if size < 1024 {
		medium_manager.malloc(size,BASIC_ALIGN,false).0 as *mut libc::c_void
	} else {
		huge_manager.malloc(size,BASIC_ALIGN,false).0 as *mut libc::c_void
	}*/
	let mut allocator = SeqAllocator::new();
	return allocator.malloc(size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn calloc(nmemb: libc::size_t, size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.calloc(nmemb as Size, size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn posix_memalign(memptr: * mut *mut libc::c_void,align: libc::size_t,size: libc::size_t) -> libc::int32_t {
	let mut allocator = SeqAllocator::new();
	return allocator.posix_memalign(memptr as *mut *mut Addr, align as Size, size as Size) as libc::int32_t;
}

#[no_mangle]
pub extern "C" fn aligned_alloc(align: libc::size_t, size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.aligned_alloc(align as Size, size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn valloc(size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.valloc(size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn memalign(align: libc::size_t, size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.memalign(align as Size, size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn pvalloc(size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.pvalloc(size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn free(addr: *mut libc::c_void) {
	let mut allocator = SeqAllocator::new();
	allocator.free(addr as Addr);
}

#[no_mangle]
pub extern "C" fn realloc(ptr: *mut libc::c_void,size: libc::size_t) -> *mut libc::c_void {
	let mut allocator = SeqAllocator::new();
	return allocator.realloc(ptr as Addr, size as Size) as *mut libc::c_void;
}

#[no_mangle]
pub extern "C" fn get_inner_size(ptr: *mut libc::c_void) -> libc::size_t {
	let allocator = SeqAllocator::new();
	return allocator.get_inner_size(ptr as Addr) as libc::size_t;
}

#[no_mangle]
pub extern "C" fn get_total_size(ptr: *mut libc::c_void) -> libc::size_t {
	let allocator = SeqAllocator::new();
	return allocator.get_total_size(ptr as Addr) as libc::size_t;
}

#[no_mangle]
pub extern "C" fn get_requested_size(ptr: *mut libc::c_void) -> libc::size_t {
	let allocator = SeqAllocator::new();
	return allocator.get_requested_size(ptr as Addr) as libc::size_t;
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume()
{

}

// These functions and traits are used by the compiler, but not
// for a bare-bones hello world. These are normally
// provided by libstd.
#[lang = "eh_personality"] 
#[no_mangle]
pub extern fn eh_personality() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	intrinsics::abort()
}

/*#[lang = "eh_unwind_resume"]
#[no_mangle]
pub extern fn rust_eh_unwind_resume() {
}*/
