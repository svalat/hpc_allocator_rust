/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///Implement a local allocator which will be used to build all UMA and NUMA allocators
///by building one local allocator for every thread.

//import
use common::list::{Listable,ListNode};
use registry::registry::RegionRegistryPtr;
use common::traits::{Allocator,ChunkManager,ChunkManagerPtr,MemorySourcePtr};
use chunk::huge::HugeChunkManager;
use chunk::medium::manager::MediumChunkManager;
use chunk::small::manager::SmallChunkManager;
use common::mpscf_queue::MPSCFQueue;
use common::types::{Addr,Size};
use common::consts::*;

/// Define a local allocator to be used to build the UMA/NUMA posix allocator by creating one local
/// allocator for every thread and store it into a TLS.
/// This allocator use the remote free queue to handle remote free without forcing the whole
/// managers to be thread safe.
struct LocalAllocator {
	list_handler: ListNode,
	registry: Option<RegionRegistryPtr>,
	mmsource: Option<MemorySourcePtr>,
	huge: HugeChunkManager,
	medium: MediumChunkManager,
	small: SmallChunkManager,
	is_init: bool,
	rfq: MPSCFQueue,
	use_lock: bool,
}

enum ManagerClass {
	ManagerHuge,
	ManagerMedium,
	ManagerSmall,
}

impl LocalAllocator {
	pub fn new(use_lock: bool, registry: Option<RegionRegistryPtr>, mmsource: Option<MemorySourcePtr>) -> Self {
		Self {
			list_handler: ListNode::new(),
			registry: registry,
			mmsource: mmsource.clone(),
			huge: HugeChunkManager::new(mmsource.clone().unwrap()),
			medium: MediumChunkManager::new(use_lock, mmsource.clone()),
			small: SmallChunkManager::new(use_lock, mmsource.clone()),
			is_init: true,
			rfq: MPSCFQueue::new(),
			use_lock: use_lock,
		}
	}

	fn post_init(&mut self) {
		unimplemented!()
	}

	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);
		
		//do it
		return self.internal_malloc(size*nmemb,BASIC_ALIGN,true);
	}

	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		//errors
		debug_assert!(self.is_init);

		//do it
		unsafe{*memptr = self.internal_malloc(size,align,false) as * mut Addr};

		return 0;
	}

	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);

		return self.internal_malloc(size,align,false);
	}

	pub fn valloc(&mut self, size: Size) -> Addr {
		unimplemented!();
	}

	pub fn memalign(&mut self, align: Size, size: Size) -> Addr {
		unimplemented!();
	}

	pub fn flush_remote(&mut self) {
		unimplemented!();
	}

	pub fn is_local_chunk_manager(& self,manager: ChunkManagerPtr) -> bool {
		unimplemented!();
	}

	pub fn rebind_mmsource(&mut self,mmsource:Option<MemorySourcePtr>) {
		self.mmsource = mmsource.clone();
		self.small.rebind_mm_source(mmsource.clone());
		self.medium.rebind_mm_source(mmsource.clone());
		self.huge.rebind_mm_source(mmsource.clone().unwrap());
	}

	fn internal_malloc(&mut self,size:Size, align: Size, zero: bool) -> Addr {
		unimplemented!();
	}

	fn get_chunk_manager(&self, ptr: Addr) -> Option<ChunkManagerPtr> {
		//errors
		debug_assert!(self.is_init);

		//search region segment
		let segment = self.registry.as_ref().unwrap().get_segment(ptr);
		debug_assert!(segment.is_some());
		
		match segment {
			Some(segment) => {
				debug_assert!(segment.contain(ptr));
				return segment.get_manager();
			},
			None => {
				return None
			},
		}
	}

	fn is_distant_manager(&self, manager: ChunkManagerPtr) -> bool {
		unimplemented!();
	}

	fn get_size_class(size: Size) -> ManagerClass {
		unimplemented!();
	}
}

impl Listable<LocalAllocator> for LocalAllocator {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
		unimplemented!();
	}

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
		unimplemented!();
	}

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const Self {
		unimplemented!();
	}

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut Self {
		unimplemented!();
	}
}

impl ChunkManager for LocalAllocator {
	fn free(&mut self,addr: Addr) {
		//errors
		debug_assert!(self.is_init);
		
		//trivial
		if addr == NULL {
			return;
		}
		
		//get manager
		let chunk_manager = self.get_chunk_manager(addr);
		debug_assert!(chunk_manager.is_some());
		
		//free it
		match chunk_manager {
			Some(mut chunk_manager) => chunk_manager.free(addr),
			None => {},
		}
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		unimplemented!();
	}

    fn get_inner_size(&self,ptr: Addr) -> Size {
		//errors
		debug_assert!(self.is_init);

		//trivial
		if ptr == NULL {
			return 0;
		}

		//get manager
		let chunk_manager = self.get_chunk_manager(ptr);
		debug_assert!(chunk_manager.is_some());

		//get size
		match chunk_manager {
			Some(manager) => return manager.get_inner_size(ptr),
			None => return 0,
		}
	}

    fn get_total_size(&self,ptr: Addr) -> Size {
		//errors
		debug_assert!(self.is_init);

		//trivial
		if ptr == NULL {
			return 0;
		}

		//get manager
		let chunk_manager = self.get_chunk_manager(ptr);
		debug_assert!(chunk_manager.is_some());

		//get size
		match chunk_manager {
			Some(manager) => return manager.get_total_size(ptr),
			None => return 0,
		}
	}

    fn get_requested_size(&self,ptr: Addr) -> Size {
		//errors
		debug_assert!(self.is_init);

		//trivial
		if ptr == NULL {
			return 0;
		}

		//get manager
		let chunk_manager = self.get_chunk_manager(ptr);
		debug_assert!(chunk_manager.is_some());

		//get size
		match chunk_manager {
			Some(manager) => return manager.get_requested_size(ptr),
			None => return 0,
		}
	}
	
    fn hard_checking(&mut self,) {
		unimplemented!();
	}

	fn is_thread_safe(&self) -> bool {
		unimplemented!();
	}

    fn remote_free(&mut self,ptr: Addr) {
		unimplemented!();
	}

    fn set_parent_chunk_manager(&mut self,parent: Option<ChunkManagerPtr>) {
		unimplemented!();
	}

    fn get_parent_chunk_manager(&mut self) -> Option<ChunkManagerPtr> {
		unimplemented!();
	}
}

impl Allocator for LocalAllocator {
    fn malloc(&mut self,mut size: Size,align: Size,zero_filled: bool) -> Addr {
		//errors
		debug_assert!(self.is_init);

		//to be compatible with glibc policy which didn't return NULL in this case.
		//otherwise we got crash in sed/grep/nano ...
		//@todo Optimize by returning a specific fixed address instead of alloc size=1
		if size == 0 {
			size = 1;
		}
		
		//call internal malloc
		return self.internal_malloc(size,align,zero_filled);
	}

	fn is_local_chunk_manager(&self, manager: ChunkManagerPtr) -> bool {
		unimplemented!();
	}

    fn flush_remote(&mut self) {
		unimplemented!();
	}
}