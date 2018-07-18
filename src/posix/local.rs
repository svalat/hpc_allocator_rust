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
use posix::local;
use common::list::{Listable,ListNode};
use registry::registry::RegionRegistryPtr;
use common::traits::{Allocator,ChunkManager,ChunkManagerPtr,MemorySourcePtr};
use chunk::huge::HugeChunkManager;
use chunk::medium::manager::MediumChunkManager;
use chunk::small::manager::SmallChunkManager;
use common::mpscf_queue::MPSCFQueue;
use common::types::{Addr,Size};

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
}

enum ManagerClass {
	MANAGER_HUGE,
	MANAGER_MEDIUM,
	MANAGER_SMALL,
}

impl LocalAllocator {
	pub fn new(registry: Option<RegionRegistryPtr>, mmsource: Option<MemorySourcePtr>) -> Self {
		unimplemented!();
	}

	fn post_init(&mut self) {
		unimplemented!()
	}

	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		unimplemented!();
	}

	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		unimplemented!();
	}

	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		unimplemented!();
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
		unimplemented!();
	}

	fn internal_malloc(&mut self,size:Size, align: Size, zero: bool) -> Addr {
		unimplemented!();
	}

	fn get_chunk_manager(&mut self, ptr: Addr) -> Option<ChunkManagerPtr> {
		unimplemented!();
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
		unimplemented!();
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		unimplemented!();
	}

    fn get_inner_size(&mut self,ptr: Addr) -> Size {
		unimplemented!();
	}

    fn get_total_size(&mut self,ptr: Addr) -> Size {
		unimplemented!();
	}

    fn get_requested_size(&mut self,ptr: Addr) -> Size {
		unimplemented!();
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
    fn malloc(&mut self,size: Size,align: Size,zero_filled: bool) -> (Addr,bool) {
		unimplemented!();
	}

	fn is_local_chunk_manager(&self, manager: ChunkManagerPtr) -> bool {
		unimplemented!();
	}

    fn flush_remote(&mut self) {
		unimplemented!();
	}
}