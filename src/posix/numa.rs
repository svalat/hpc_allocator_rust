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
extern crate libc;

//import
use posix::local::LocalAllocator;
use registry::registry::RegionRegistry;
use mmsource::cached::CachedMMSource;
use common::shared::SharedPtrBox;
use common::types::{Addr,Size};
use common::consts::*;
use common::traits::{Allocator, ChunkManagerPtr};
use core::mem;
use portability::osmem;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Global variable to store the registry
static mut GBL_NUMA_ALLOCATOR: Addr = 0;
static mut GBL_PTHREAD_KEY: libc::pthread_key_t = 0;
static mut GBL_PROTECT_INIT: AtomicUsize = AtomicUsize::new(0);

/// Object handling the thread local memory allocator.
pub struct ThreadNumaAllocator {
	allocator: SharedPtrBox<LocalAllocator>,
	region_registry: SharedPtrBox<RegionRegistry>,
}

pub struct ThreadNumaAllocatorHandler {
	allocator: SharedPtrBox<ThreadNumaAllocator>,
}

/// Object representing the global state of the allocator with all threads
/// objects tracking.
pub struct NumaAllocator {
	region_registry: SharedPtrBox<RegionRegistry>,
	egg_memory_source: SharedPtrBox<CachedMMSource>,
	egg_allocator: SharedPtrBox<LocalAllocator>,
}

/// Object to handle a NUMA allocator
pub struct NumaAllocatorHandler {
	numa_allocator: SharedPtrBox<NumaAllocator>,
}

#[inline]
pub fn min(a: Size, b: Size) -> Size {
	if a < b {
		return a;
	} else {
		return b;
	}
}

/// Check first initialization to avoid multiple init
pub fn check_already_init() -> bool {
	// Already init
	unsafe {
		if GBL_NUMA_ALLOCATOR != 0 {
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

	// allocate
	let total_size = NumaAllocator::egg_mem_size();
	let ptr = osmem::mmap(0, total_size);

	//calc addresses
	let numa_allocator_size = mem::size_of::<NumaAllocator>();
	let numa_allocator_addr = ptr;
	let other_egg_element_addr = ptr + numa_allocator_size;

	//spawn
	let mut numa_allocator: SharedPtrBox<NumaAllocator> = SharedPtrBox::new_addr(numa_allocator_addr);
	*numa_allocator.get_mut() = NumaAllocator::new(other_egg_element_addr);

	//create key
	//TODO create a function for destructor
	unsafe{libc::pthread_key_create(&mut GBL_PTHREAD_KEY, None)};

	unsafe {
		// commit
		GBL_NUMA_ALLOCATOR = numa_allocator_addr;

		// update atomic protection to release threads in waiting queue
		GBL_PROTECT_INIT.store(2, Ordering::Relaxed);
	}
}

impl NumaAllocator {
	pub fn new(other_egg_element_addr: Addr) -> Self {
		// calc size
		let registry_size = mem::size_of::<RegionRegistry>();
		let egg_mm_source_size = mem::size_of::<CachedMMSource>();

		//calc addresses
		let registry_addr = other_egg_element_addr as Addr;
		let egg_mm_source_addr = registry_addr + registry_size;
		let egg_allocator_addr = egg_mm_source_addr + egg_mm_source_size;

		//build boxes
		let mut region_registry: SharedPtrBox<RegionRegistry> = SharedPtrBox::new_addr(registry_addr);
		let mut egg_mm_source: SharedPtrBox<CachedMMSource> = SharedPtrBox::new_addr(egg_mm_source_addr);
		let mut egg_allocator: SharedPtrBox<LocalAllocator> = SharedPtrBox::new_addr(egg_allocator_addr);

		//spawn
		*region_registry.get_mut() = RegionRegistry::new();
		*egg_mm_source.get_mut() = CachedMMSource::new_default(Some(region_registry.clone()));
		*egg_allocator.get_mut() = LocalAllocator::new(true, Some(region_registry.clone()), Some(SharedPtrBox::new_ref_mut(egg_mm_source.get_mut())));
		egg_allocator.clone().post_init(ChunkManagerPtr::new_ref_mut(&mut *egg_allocator.clone().get_mut()));

		//build & return
		Self {
			region_registry: region_registry,
			egg_memory_source: egg_mm_source,
			egg_allocator: egg_allocator,
		}
	}

	pub fn egg_mem_size() -> Size {
		// calc size
		let registry_size = mem::size_of::<RegionRegistry>();
		let egg_mm_source_size = mem::size_of::<CachedMMSource>();
		let egg_allocator_size = mem::size_of::<LocalAllocator>();
		let numa_allocator_size = mem::size_of::<NumaAllocator>();

		// total size
		let total_size = registry_size + egg_mm_source_size + egg_allocator_size + numa_allocator_size;
		let total_size = total_size + (SMALL_PAGE_SIZE - total_size % SMALL_PAGE_SIZE);

		// ret
		return total_size;
	}

	pub fn get_new_local_allocator(&mut self) -> SharedPtrBox<LocalAllocator> {
		let size = mem::size_of::<LocalAllocator>();
		let ptr = self.egg_allocator.malloc(size, BASIC_ALIGN, false);
		let mut local_allocator: SharedPtrBox<LocalAllocator> = SharedPtrBox::new_addr(ptr);
		*local_allocator.get_mut() = LocalAllocator::new(true, Some(self.region_registry.clone()), Some(SharedPtrBox::new_ref_mut(self.egg_memory_source.get_mut())));
		local_allocator.clone().post_init(ChunkManagerPtr::new_ref_mut(&mut *local_allocator.clone().get_mut()));
		return local_allocator;
	}

	pub fn get_new_thread_allocator(&mut self) -> SharedPtrBox<ThreadNumaAllocator> {
		//get allocator
		let allocator = self.get_new_local_allocator();
		let registry = self.region_registry.clone();

		//allocate
		let size = mem::size_of::<ThreadNumaAllocator>();
		let ptr = self.egg_allocator.malloc(size, BASIC_ALIGN, false);

		//spawn
		let mut thread_alloc: SharedPtrBox<ThreadNumaAllocator> = SharedPtrBox::new_addr(ptr);
		*thread_alloc.get_mut() = ThreadNumaAllocator::new(allocator.clone(), registry.clone());

		//ret
		return thread_alloc.clone();
	}
}

impl ThreadNumaAllocator {
	#[inline]
	pub fn new(alloc: SharedPtrBox<LocalAllocator>, registry: SharedPtrBox<RegionRegistry>) -> Self {
		Self {
			allocator: alloc,
			region_registry: registry,
		}
	}

	#[inline]
	pub fn flush_remote(&mut self) {
		self.allocator.flush_remote();
	}

	#[inline]
	pub fn is_distant_manager(&mut self, chunk_manager: ChunkManagerPtr) -> bool {
		if chunk_manager.is_thread_safe() {
			return false;
		} else {
			return !self.allocator.is_local_chunk_manager(chunk_manager.clone());
		}
	}

	#[inline]
	pub fn get_chunk_manager(&mut self, ptr: Addr) -> ChunkManagerPtr {
		//get insfos
		let region_registry = self.region_registry.get_mut();

		//get manager
		let segment = region_registry.get_segment_safe(ptr);
		let chunk_manager = segment.clone().get_mut().get_manager_mut().unwrap();

		//return
		return chunk_manager.clone();
	}

	#[inline]
	pub fn get_chunk_manager_const(& self, ptr: Addr) -> ChunkManagerPtr {
		//get insfos
		let region_registry = self.region_registry.get();

		//get manager
		let segment = region_registry.get_segment_safe(ptr);
		let chunk_manager = segment.clone().get_mut().get_manager_mut().unwrap();

		//return
		return chunk_manager.clone();
	}

	pub fn malloc(&mut self,size: Size) -> Addr {
		if size < BASIC_ALIGN {
			// TODO
			return self.allocator.malloc(size, BASIC_ALIGN, false);
		} else {
			return self.allocator.malloc(size, BASIC_ALIGN, false);
		}
	}

	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		return self.allocator.calloc(nmemb, size);
	}
	
	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		return self.allocator.posix_memalign(memptr, align, size);
	}

	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		return self.allocator.aligned_alloc(align, size);
	}

	pub fn valloc(&mut self, size: Size) -> Addr {
		return self.allocator.valloc(size);
	}

	pub fn memalign(&mut self, align: Size, size: Size) -> Addr {
		return self.allocator.memalign(align, size);
	}

	pub fn pvalloc(&mut self, size: Size) -> Addr {
		return self.allocator.pvalloc(size);
	}

	pub fn free(&mut self,addr: Addr) {
		//nothing to do
		if addr == NULL {
			return;
		}

		//get chunk manager
		let mut chunk_manager = self.get_chunk_manager(addr);

		//if local
		if self.is_distant_manager(chunk_manager.clone()) {
			chunk_manager.remote_free(addr);
		} else {
			self.allocator.free(addr);
		}
	}

	pub fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		//vars
		let mut res: Addr = NULL;
		
		//simple alloc
		if ptr == NULL {
			res = self.malloc(size);
		} else if size == 0 {
			self.free(ptr);
		} else {
			//get manager and fetch TLS locally for fast use
			let mut chunk_manager = self.get_chunk_manager(ptr);
			
			//manage bad realloc as we can
			if chunk_manager.is_null() {
				//TODO : made this cas optional for resistant mode
				//allocWarning("The old segment isn't managed by current memory allocator, try to copy, but create a memory leak and may segfault during unsafe copy.");
				res = self.malloc(size);
				unsafe{libc::memcpy(res as *mut libc::c_void,ptr as *mut libc::c_void,size)};
			} else if self.is_distant_manager(chunk_manager.clone()) {
				let new_ptr = self.malloc(size);
				let inner_size = chunk_manager.get_inner_size(ptr);
				let copy_size = min(size,inner_size);
				unsafe{libc::memcpy(new_ptr as * mut libc::c_void,ptr as * mut libc::c_void,copy_size)};
				self.free(ptr);
				res = new_ptr;
			} else {
				let parent_chunk_manager = chunk_manager.get_parent_chunk_manager();
				if parent_chunk_manager.is_some() {
					res = parent_chunk_manager.unwrap().realloc(ptr, size);
				} else {
					res = chunk_manager.realloc(ptr, size);
				}
			}
		}

		return res;
	}

	pub fn get_inner_size(&self,ptr: Addr) -> Size {
		if ptr == NULL {
			return 0;
		} else {
			let chunk_manager = self.get_chunk_manager_const(ptr);
			return chunk_manager.get_inner_size(ptr);
		}
	}

	pub fn get_total_size(&self,ptr: Addr) -> Size {
		if ptr == NULL {
			return 0;
		} else {
			let chunk_manager = self.get_chunk_manager_const(ptr);
			return chunk_manager.get_total_size(ptr);
		}
	}

	pub fn get_requested_size(&self,ptr: Addr) -> Size {
		if ptr == NULL {
			return 0;
		} else {
			let chunk_manager = self.get_chunk_manager_const(ptr);
			return chunk_manager.get_requested_size(ptr);
		}
	}
}

impl NumaAllocatorHandler {
	pub fn new() -> Self {
		unsafe {
			// TODO need to implement a full atomic based spinlock to avoid dual init
			if GBL_NUMA_ALLOCATOR == 0 {
				init();
			}
			Self {
				numa_allocator: SharedPtrBox::new_addr(GBL_NUMA_ALLOCATOR)
			}
		}
	}

	pub fn get_numa_allocator(&mut self) -> SharedPtrBox<NumaAllocator> {
		return self.numa_allocator.clone();
	}
}

impl ThreadNumaAllocatorHandler {
	pub fn new() -> Self {
		unsafe {
			// TODO need to implement a full atomic based spinlock to avoid dual init
			if GBL_NUMA_ALLOCATOR == 0 {
				init();
			}
			
			//get thread specific
			let ptr = libc::pthread_getspecific(GBL_PTHREAD_KEY) as Addr;

			//init
			if ptr == NULL {
				let mut numa_allocator_handler = NumaAllocatorHandler::new();
				let allocator = numa_allocator_handler.get_numa_allocator().get_new_thread_allocator();
				libc::pthread_setspecific(GBL_PTHREAD_KEY, allocator.get_addr() as * mut libc::c_void);
				Self {
					allocator: allocator
				}
			} else {
				let mut allocator: SharedPtrBox<ThreadNumaAllocator> = SharedPtrBox::new_addr(ptr as Addr);
				allocator.flush_remote();
				//return
				Self {
					allocator: allocator
				}
			}
		}
	}

	#[inline]
	pub fn malloc(&mut self,size: Size) -> Addr {
		return self.allocator.malloc(size);
	}

	#[inline]
	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		return self.allocator.calloc(nmemb, size);
	}
	
	#[inline]
	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		return self.allocator.posix_memalign(memptr, align, size);
	}

	#[inline]
	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		return self.allocator.aligned_alloc(align, size);
	}

	#[inline]
	pub fn valloc(&mut self, size: Size) -> Addr {
		return self.allocator.valloc(size);
	}

	#[inline]
	pub fn memalign(&mut self, align: Size, size: Size) -> Addr {
		return self.allocator.memalign(align, size);
	}

	#[inline]
	pub fn pvalloc(&mut self, size: Size) -> Addr {
		return self.allocator.pvalloc(size);
	}

	#[inline]
	pub fn free(&mut self,addr: Addr) {
		self.allocator.free(addr);
	}

	#[inline]
	pub fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		return self.allocator.realloc(ptr, size);
	}

	#[inline]
	pub fn get_inner_size(&self,ptr: Addr) -> Size {
		return self.allocator.get_inner_size(ptr);
	}

	#[inline]
	pub fn get_total_size(&self,ptr: Addr) -> Size {
		return self.allocator.get_total_size(ptr);
	}

	#[inline]
	pub fn get_requested_size(&self,ptr: Addr) -> Size {
		return self.allocator.get_requested_size(ptr);
	}
}



/*
/// Basic implementation of an allocator
impl NumaAllocator {
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
	use posix::uma::*;

	// CAUTION HERE WE USE A GLOBAL ALLOCATOR SO TEST MUST BE WRITTEN
	// TO BE REPRODUCIBLE AND NOT INTERFER TOGETHER

	#[test]
	fn basic_1() {
		let mut allocator = NumaAllocator::new();
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
		let mut allocator = NumaAllocator::new();
		let _ptr0 = allocator.malloc(32);
		let ptr1 = allocator.malloc(32);
		assert_ne!(ptr1, 0);
		let mut allocator = NumaAllocator::new();
		allocator.free(ptr1);
		let mut allocator = NumaAllocator::new();
		let ptr2 = allocator.malloc(32);
		assert_ne!(ptr2, 0);
		let mut allocator = NumaAllocator::new();
		allocator.free(ptr2);
		assert_eq!(ptr1, ptr2);
	}

	#[test]
	fn basic_realloc() {
		let mut allocator = NumaAllocator::new();
		let ptr1 = allocator.malloc(64);
		let ptr2 = allocator.malloc(64);
		assert_ne!(ptr1, ptr2);
		let ptr3 = allocator.realloc(ptr1, 64);
		assert_ne!(ptr1, ptr3);
		allocator.free(ptr2);
		allocator.free(ptr3);
	}
}
*/

#[cfg(test)]
mod tests
{
	extern crate std;
	use posix::numa::*;

	// CAUTION HERE WE USE A GLOBAL ALLOCATOR SO TEST MUST BE WRITTEN
	// TO BE REPRODUCIBLE AND NOT INTERFER TOGETHER

	#[test]
	fn basic_1() {
		let mut allocator = ThreadNumaAllocatorHandler::new();
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
		let mut allocator = ThreadNumaAllocatorHandler::new();
		let _ptr0 = allocator.malloc(32);
		let ptr1 = allocator.malloc(32);
		assert_ne!(ptr1, 0);
		let mut allocator = ThreadNumaAllocatorHandler::new();
		allocator.free(ptr1);
		let mut allocator = ThreadNumaAllocatorHandler::new();
		let ptr2 = allocator.malloc(32);
		assert_ne!(ptr2, 0);
		let mut allocator = ThreadNumaAllocatorHandler::new();
		allocator.free(ptr2);
		assert_eq!(ptr1, ptr2);
	}

	#[test]
	fn basic_realloc() {
		let mut allocator = ThreadNumaAllocatorHandler::new();
		let ptr1 = allocator.malloc(64);
		let ptr2 = allocator.malloc(64);
		assert_ne!(ptr1, ptr2);
		let ptr3 = allocator.realloc(ptr1, 64);
		assert_eq!(ptr1, ptr3);
		allocator.free(ptr2);
		allocator.free(ptr3);
	}
}
