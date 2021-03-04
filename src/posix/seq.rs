/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This file implement the UMA allocator considering a local allocator for 
/// every thread and using TLS (Thread Local Storage) to keep track of them.

//import
use posix::local::LocalAllocator;
use registry::registry::RegionRegistry;
use mmsource::cached::CachedMMSource;
use common::shared::SharedPtrBox;
use common::types::{Addr,Size};
use common::consts::*;
use common::traits::{ChunkManager};
use core::mem;
use portability::osmem;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Global variable to store the registry
static mut GBL_REGION_REGISTRY: Addr = 0;
static mut GBL_MEMORY_SOURCE: Addr = 0;
static mut GBL_MEMORY_ALLOCATOR: Addr = 0;
static mut GBL_PROTECT_INIT: AtomicUsize = AtomicUsize::new(0);

/// Implement a sequential version of the memory allocator.
/// It just setup all the LocalAllocator environnement and redirect
/// calls with spinlock to protect for multi-threading.
pub struct SeqAllocator {
	local_allocator: SharedPtrBox<LocalAllocator>,
}

/// Check first initialization to avoid multiple init
pub fn check_already_init() -> bool {
	// Already init
	unsafe {
		if GBL_MEMORY_ALLOCATOR != 0 {
			return true;
		}
	}

	// make atomic
	unsafe {
		// get status
		let mut status = GBL_PROTECT_INIT.load(Ordering::SeqCst);

		// if 0, not init
		if status == 0 {
			// try to get action by CAS
			match GBL_PROTECT_INIT.compare_exchange(0, 1, Ordering::SeqCst, Ordering::Acquire) {
				Ok(value) => status = value,
				Err(_) => ()
			}

			// we swap and do not get 1 someone is already doing init
			if status != 0 {
				while status != 2 {
					status = GBL_PROTECT_INIT.load(Ordering::Relaxed);
				}
				return true;
			}
		} else if status == 1 {
			while status != 2 {
				status = GBL_PROTECT_INIT.load(Ordering::Relaxed);
			}
			return true;
		}
	}

	//ok need to init
	return false;
}

/// Initialize the memory allocator global variables
pub fn init() {
	//check already init
	if check_already_init() {
		return;
	}

	// calc size
	let registry_size = mem::size_of::<RegionRegistry>();
	let mm_source_size = mem::size_of::<CachedMMSource>();
	let allocator_size = mem::size_of::<LocalAllocator>();

	// total size
	let total_size = registry_size + mm_source_size + allocator_size;
	let total_size = total_size + (SMALL_PAGE_SIZE - total_size % SMALL_PAGE_SIZE);

	// allocate
	let ptr = osmem::mmap(0, total_size);

	// unsaface global variable handling
	unsafe {
		// setup ptrs
		GBL_REGION_REGISTRY = ptr;
		GBL_MEMORY_SOURCE = GBL_REGION_REGISTRY + registry_size;
		let allocatr_addr = GBL_MEMORY_SOURCE + mm_source_size;

		// create box
		let mut registry_ptr: SharedPtrBox<RegionRegistry> = SharedPtrBox::new_addr(GBL_REGION_REGISTRY);
		let mut mm_source_ptr: SharedPtrBox<CachedMMSource> = SharedPtrBox::new_addr(GBL_MEMORY_SOURCE);
		let mut allocator: SharedPtrBox<LocalAllocator> = SharedPtrBox::new_addr(allocatr_addr);

		// spawn
		*registry_ptr.get_mut() = RegionRegistry::new();
		*mm_source_ptr.get_mut() = CachedMMSource::new_default(Some(registry_ptr.clone()));
		let source = mm_source_ptr.get_mut(); 
		*allocator.get_mut() = LocalAllocator::new(true, Some(registry_ptr.clone()), Some(SharedPtrBox::new_ref_mut(source)));

		// commit
		GBL_MEMORY_ALLOCATOR = allocatr_addr;

		// update atomic protection to release threads in waiting queue
		GBL_PROTECT_INIT.store(2, Ordering::Relaxed);
	}
}

/// Basic implementation of an allocator
impl SeqAllocator {
	pub fn new() -> Self {
		unsafe {
			// TODO need to implement a full atomic based spinlock to avoid dual init
			if GBL_MEMORY_ALLOCATOR == 0 {
				init();
			}
			Self {
				local_allocator: SharedPtrBox::new_addr(GBL_MEMORY_ALLOCATOR)
			}
		}
	}

	pub fn malloc(&mut self,size: Size) -> Addr {
		if size < BASIC_ALIGN {
			// TODO
			return self.local_allocator.malloc(size, BASIC_ALIGN, false);
		} else {
			return self.local_allocator.malloc(size, BASIC_ALIGN, false);
		}
	}

	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		return self.local_allocator.calloc(nmemb, size);
	}
	
	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		return self.local_allocator.posix_memalign(memptr, align, size);
	}

	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		return self.local_allocator.aligned_alloc(align, size);
	}

	pub fn valloc(&mut self, size: Size) -> Addr {
		return self.local_allocator.valloc(size);
	}

	pub fn memalign(&mut self, align: Size, size: Size) -> Addr {
		return self.local_allocator.memalign(align, size);
	}

	pub fn pvalloc(&mut self, size: Size) -> Addr {
		return self.local_allocator.pvalloc(size);
	}

	pub fn free(&mut self,addr: Addr) {
		self.local_allocator.free(addr);
	}

	pub fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		return self.local_allocator.realloc(ptr, size);
	}

	pub fn get_inner_size(&self,ptr: Addr) -> Size {
		return self.local_allocator.get_inner_size(ptr);
	}

	pub fn get_total_size(&self,ptr: Addr) -> Size {
		return self.local_allocator.get_total_size(ptr);
	}

	pub fn get_requested_size(&self,ptr: Addr) -> Size {
		return self.local_allocator.get_requested_size(ptr);
	}
}

#[cfg(test)]
mod tests
{
	extern crate std;
	use posix::seq::*;

	// CAUTION HERE WE USE A GLOBAL ALLOCATOR SO TEST MUST BE WRITTEN
	// TO BE REPRODUCIBLE AND NOT INTERFER TOGETHER

	#[test]
	fn basic_1() {
		let mut allocator = SeqAllocator::new();
		let _ptr0 = allocator.malloc(8);
		let ptr1 = allocator.malloc(8);
		assert_ne!(ptr1, 0);
		allocator.free(ptr1);
		let ptr2 = allocator.malloc(8);
		assert_ne!(ptr2, 0);
		allocator.free(ptr2);
		assert_eq!(ptr1, ptr2);
	}

	#[test]
	fn basic_renew() {
		let mut allocator = SeqAllocator::new();
		let _ptr0 = allocator.malloc(32);
		let ptr1 = allocator.malloc(32);
		assert_ne!(ptr1, 0);
		let mut allocator = SeqAllocator::new();
		allocator.free(ptr1);
		let mut allocator = SeqAllocator::new();
		let ptr2 = allocator.malloc(32);
		assert_ne!(ptr2, 0);
		let mut allocator = SeqAllocator::new();
		allocator.free(ptr2);
		assert_eq!(ptr1, ptr2);
	}

	#[test]
	fn basic_realloc() {
		let mut allocator = SeqAllocator::new();
		let ptr1 = allocator.malloc(64);
		let ptr2 = allocator.malloc(64);
		assert_ne!(ptr1, ptr2);
		let ptr3 = allocator.realloc(ptr1, 64);
		assert_eq!(ptr1, ptr3);
		allocator.free(ptr2);
		allocator.free(ptr3);
	}
}