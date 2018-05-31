/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module impelment the basic wrapper to memory management functions

//import
extern crate libc;

//import
use common::types::{Addr,Size};
use common::consts::*;

///wrapper to mmap function
pub fn mmap(addr:Addr,size:Size) -> Addr
{
	//check
	debug_assert!(addr % SMALL_PAGE_SIZE == 0);
	debug_assert!(size % SMALL_PAGE_SIZE == 0);
	debug_assert!(size != 0);

	//vers
	let res;

	//case
	if addr == 0 {
		res = unsafe{libc::mmap(0 as *mut libc::c_void, size,libc::PROT_READ | libc::PROT_WRITE, libc::MAP_ANON | libc::MAP_PRIVATE, -1,0)};
	} else {
		res = unsafe{libc::mmap(addr as *mut libc::c_void, size,libc::PROT_READ | libc::PROT_WRITE, libc::MAP_ANON | libc::MAP_PRIVATE | libc::MAP_FIXED, -1,0)};
	}
	
	//check error
	if res == libc::MAP_FAILED {
		//TODO REPLACE BY WARNING
		panic!("Out of memory, failed to request memory to the OS via mmap.");
	}

	res as Addr
}

pub fn munmap(addr:Addr,size:Size) -> bool {
	//check
	debug_assert!(addr % SMALL_PAGE_SIZE == 0);
	debug_assert!(size % SMALL_PAGE_SIZE == 0);
	debug_assert!(size != 0);

	//call
	let ret = unsafe{libc::munmap(addr as *mut libc::c_void,size)};

	//warn
	if ret != 0 {
		panic!("Failed to return memory to the OS via munmap.");
	}

	//ret
	ret != 0
}

pub fn mremap(addr:Addr,old_size:Size,new_size:Size,dest_addr:Addr) -> Addr {
	//check
	debug_assert!(addr % SMALL_PAGE_SIZE == 0);
	debug_assert!(old_size % SMALL_PAGE_SIZE == 0);
	debug_assert!(new_size % SMALL_PAGE_SIZE == 0);

	//call
	let ret;
	if dest_addr == 0 {
		ret = unsafe{libc::mremap(addr as *mut libc::c_void,old_size,new_size,libc::MREMAP_MAYMOVE)};
	} else {
		ret = unsafe{libc::mremap(addr as *mut libc::c_void,old_size,new_size,libc::MREMAP_MAYMOVE | libc::MREMAP_FIXED,dest_addr)};
		if ret != libc::MAP_FAILED {
			assert_eq!(ret as Addr,dest_addr);
		}
	}

	//check
	if ret == libc::MAP_FAILED {
		panic!("Failed to remap memory via mremap.");
	}

	//ret
	ret as Addr
}

#[cfg(test)]
mod tests
{
	use common::consts::*;
	use portability::osmem;

	#[test]
	fn test_mmap_mremap_munap() {
		let ptr = osmem::mmap(0,2*4096);
		assert!(ptr != 0);
		assert!(ptr % SMALL_PAGE_SIZE == 0);

		let ptr = osmem::mremap(ptr,2*4096,4*4096,0);
		assert!(ptr != 0);
		assert!(ptr % SMALL_PAGE_SIZE == 0);

		osmem::munmap(ptr,4*4096);
	}
}