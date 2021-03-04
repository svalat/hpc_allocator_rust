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
use chunk::small::manager::{SmallChunkManager,SMALL_CHUNK_MAX_SIZE};
use common::mpscf_queue::MPSCFQueue;
use common::types::{Addr,Size};
use common::consts::*;
use portability::libc;

/// Define a local allocator to be used to build the UMA/NUMA posix allocator by creating one local
/// allocator for every thread and store it into a TLS.
/// This allocator use the remote free queue to handle remote free without forcing the whole
/// managers to be thread safe.
pub struct LocalAllocator {
	list_handler: ListNode,//CAUTION, This should be first
	registry: Option<RegionRegistryPtr>,
	mmsource: Option<MemorySourcePtr>,
	huge: HugeChunkManager,
	medium: MediumChunkManager,
	small: SmallChunkManager,
	is_init: bool,
	rfq: MPSCFQueue,
	use_lock: bool,
	parent: Option<ChunkManagerPtr>,
}

#[derive(PartialEq)]
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
			parent: None,
		}
	}

	pub fn post_init(&mut self, parent_chunk_manager: ChunkManagerPtr) {
		self.huge.set_parent_chunk_manager(Some(parent_chunk_manager.clone()));
		self.medium.set_parent_chunk_manager(Some(parent_chunk_manager.clone()));
		self.small.set_parent_chunk_manager(Some(parent_chunk_manager.clone()));
	}

	pub fn malloc(&mut self,mut size: Size,align: Size,zero_filled: bool) -> Addr {
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

	pub fn free(&mut self,addr: Addr) {
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

	pub fn calloc(&mut self,nmemb: Size, size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);
		
		//do it
		return self.internal_malloc(size*nmemb,BASIC_ALIGN,true);
	}

	pub fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		//errors
		debug_assert!(self.is_init);
		
		//trivial
		let res;
		if ptr == NULL {
			res = self.internal_malloc(size, BASIC_ALIGN, false);
		} else if size == 0 {
			self.free(ptr);
			res = NULL;
		} else {
			//get manager
			let manager = self.get_chunk_manager(ptr);

			//handle errors
			match manager {
				//ok
				Some(mut manager) => {
					//to compare
					let small_ptr = ChunkManagerPtr::new_ref(& self.small);
					let medium_ptr = ChunkManagerPtr::new_ref(&self.medium);
					let huge_ptr = ChunkManagerPtr::new_ref(&self.huge);

					//check if can strictly realloc in one kind of allocator
					let size_class = LocalAllocator::get_size_class(size);
					let is_realloc_in_small = size_class == ManagerClass::ManagerSmall && manager == small_ptr;
					let is_realloc_in_medium = size_class == ManagerClass::ManagerMedium && manager == medium_ptr;
					let is_realloc_in_huge = size_class == ManagerClass::ManagerHuge && manager == huge_ptr;

					//local and same class realloc otherwise alloc/copy/free
					if is_realloc_in_small {
						res = self.small.realloc(ptr, size);
					} else if is_realloc_in_medium {
						res = self.medium.realloc(ptr, size);
					} else if is_realloc_in_huge {
						res = self.huge.realloc(ptr, size);
					} else {
						let current_size = self.get_inner_size(ptr);
						res = self.internal_malloc(size, BASIC_ALIGN, false);
						if size < current_size {
							libc::memcpy(res, ptr, size);
						} else {
							libc::memcpy(res, ptr, current_size);
						}
						manager.get_mut().free(ptr);
					}
				},

				//manage bad relloc as we can
				None => {
					//TODO print warning
					//panic!("The old segment isn't managed by current memory allocator, try to copy, but create a memory leak and may segfault during unsage copy !");

					res = self.internal_malloc(size, BASIC_ALIGN, false);
					libc::memcpy(res, ptr, size);
				}
			}
		}

		//final
		return res;
	}

	pub fn posix_memalign(&mut self,memptr: * mut *mut Addr,align: Size,size: Size) -> i32 {
		//errors
		debug_assert!(self.is_init);

		//do it
		let tmp = self.internal_malloc(size,align,false);
		unsafe{*memptr = tmp as * mut Addr};

		if tmp == 0 {
			return -1;
		} else {
			return 0;
		}
	}

	pub fn aligned_alloc(&mut self, align:Size, size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);

		return self.internal_malloc(size,align,false);
	}

	pub fn valloc(&mut self, _size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);

		unimplemented!();
	}

	pub fn memalign(&mut self, align: Size, size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);

		//do it
		let tmp = self.internal_malloc(size,align,false);
		
		return tmp;
	}

	pub fn pvalloc(&mut self, _size: Size) -> Addr {
		//errors
		debug_assert!(self.is_init);

		//not supported
		unimplemented!();
	}

	pub fn rebind_mmsource(&mut self,mmsource:Option<MemorySourcePtr>) {
		self.mmsource = mmsource.clone();
		self.small.rebind_mm_source(mmsource.clone());
		self.medium.rebind_mm_source(mmsource.clone());
		self.huge.rebind_mm_source(mmsource.clone().unwrap());
	}

	fn internal_malloc(&mut self,size:Size, align: Size, zero: bool) -> Addr {
		//errors
		debug_assert!(self.is_init);

		//if alignement is greater than sie, size size otherwise we may
		//select the wrong size class
		let fsize;
		if align > size {
			fsize = align;
		} else {
			fsize = size;
		}

		//round size
		let ptr;
		let zeroed;
		if fsize <= SMALL_CHUNK_MAX_SIZE {
			let (a,b) = self.small.malloc(fsize, align, zero);
			ptr = a;
			zeroed = b;
		} else if size > HUGE_ALLOC_THREASHOLD {
			let (a,b) = self.huge.malloc(fsize, align, zero);
			ptr = a;
			zeroed = b;
		} else {
			let (a,b) = self.medium.malloc(fsize, align, zero);
			ptr = a;
			zeroed = b;
		}

		//if need reset
		if ptr != 0 && zero && ! zeroed {
			assert!(false);
			libc::memset(ptr, 0, fsize);
		}

		//final
		return ptr;
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
		let small_ptr = ChunkManagerPtr::new_ref(& self.small);
		let medium_ptr = ChunkManagerPtr::new_ref(&self.medium);
		let huge_ptr = ChunkManagerPtr::new_ref(&self.huge);

		return small_ptr != manager && medium_ptr != manager && huge_ptr != manager;
	}

	fn get_size_class(size: Size) -> ManagerClass {
		if size < SMALL_CHUNK_MAX_SIZE {
			return ManagerClass::ManagerSmall;
		} else if size < HUGE_ALLOC_THREASHOLD {
			return ManagerClass::ManagerMedium;
		} else {
			return ManagerClass::ManagerHuge;
		}
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
}

impl Listable<LocalAllocator> for LocalAllocator {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
		&self.list_handler
	}

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
		&mut self.list_handler
	}

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const Self {
		((elmt as Addr)) as * const Self
	}

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut Self {
		((elmt as Addr)) as * mut Self
	}
}

impl ChunkManager for LocalAllocator {
	fn free(&mut self,addr: Addr) {
		self.free(addr);
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		return self.realloc(ptr, size);
	}

	fn get_inner_size(&self,ptr: Addr) -> Size {
		return self.get_inner_size(ptr);
	}

	fn get_total_size(&self,ptr: Addr) -> Size {
		return self.get_total_size(ptr);
	}

	fn get_requested_size(&self,ptr: Addr) -> Size {
		return self.get_requested_size(ptr);
	}
	
	fn hard_checking(&mut self,) {
		self.small.hard_checking();
		self.medium.hard_checking();
		self.huge.hard_checking();
	}

	fn is_thread_safe(&self) -> bool {
		return false;
	}

	fn remote_free(&mut self,_ptr: Addr) {
		//errors
		debug_assert!(self.is_init);

		panic!("Should not get remote free here !");
	}

	fn set_parent_chunk_manager(&mut self,parent: Option<ChunkManagerPtr>) {
		self.parent = parent;
	}

	fn get_parent_chunk_manager(&mut self) -> Option<ChunkManagerPtr> {
		return self.parent.clone();
	}
}

impl Allocator for LocalAllocator {
	fn malloc(&mut self,size: Size,align: Size,zero_filled: bool) -> Addr {
		return self.malloc(size, align, zero_filled);
	}

	fn is_local_chunk_manager(&self, manager: ChunkManagerPtr) -> bool {
		return ! self.is_distant_manager(manager);
	}

	fn flush_remote(&mut self) {
		let handler = self.rfq.dequeue_all();
		match handler {
			Some(handler) => {
				let mut next = handler;
				while !next.is_null() {
					let tmp = next.next;
					let addr = next.get_addr();
					self.free(addr);
					next = tmp;
				}
			},
			None => {}
		}
	}
}