/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This implement the medium chunk allocator by using internally the MediumFreePool
/// and MidiumChunk.

//import
use chunk::medium::pools::{ChunkInsertMode,MediumFreePool};
use chunk::medium::chunk::*;
use portability::spinlock::SpinLock;
use common::traits::{ChunkManager,ChunkManagerPtr,MemorySourcePtr};
use registry::registry::RegionRegistry;
use common::types::{Addr,Size,SSize};
use common::consts::*;
use common::ops;
use chunk::padding::PaddedChunk;
use common::shared::SharedPtrBox;
use core::mem;
use registry::segment::RegionSegment;
use portability::libc;

/// Group content to protect by spinlock
struct MediumChunkManagerLocked {
	pools: MediumFreePool,
	mmsource: Option<MemorySourcePtr>,
}

/// Implement the medium chunk allocator based on MediumFreePool
struct MediumChunkManager {
	locked: SpinLock<MediumChunkManagerLocked>,
	registry: Option<SharedPtrBox<RegionRegistry>>,
	use_lock: bool,
	parent: Option<ChunkManagerPtr>,
}

//implement
impl MediumChunkManager {
	/// Construct a new chunk manager.
	/// 
	/// @param use_lock Define if we use spinlocks to protect the shared state or not.
	/// This make the code more efficient if used inside thread local alloctor.
	/// @param mmsource Define the memory source to use to fetch macro blocs.
	pub fn new(use_lock: bool, mmsource: Option<MemorySourcePtr>) -> Self {
		Self {
			locked: SpinLock::new(MediumChunkManagerLocked {
				pools: MediumFreePool::new(),
				mmsource: mmsource, 
			}),
			registry: None,
			use_lock: use_lock,
			parent: None,
		}
	}

	/// Allocate a new segment.
	pub fn malloc(&mut self, size: Size, align:Size, zero_filled: bool) -> (Addr,bool) {
		let mut zero = zero_filled;
		let mut checked_size = size;

		//errors
		debug_assert!(align >= BASIC_ALIGN);
		
		//trivial
		if checked_size == 0 {
			return (0,zero);
		} else if checked_size < MEDIUM_MIN_INNER_SIZE {
			checked_size = MEDIUM_MIN_INNER_SIZE;
		}
		
		//add place for padding
		if align > BASIC_ALIGN {
			checked_size += align;
		}
		
		//align size
		checked_size = ops::up_to_power_of_2(checked_size,BASIC_ALIGN);
		
		//take lock for the current function
		let mut chunk;
		{
			let mut guard = self.locked.optional_lock(self.use_lock);
		
			//try to get memory
			chunk = guard.pools.find_chunk( checked_size );
			match chunk {
				Some(_) => zero = false,
				None => {
					let (tchunk, tzero) = Self::refill(&mut *guard,checked_size,zero,SharedPtrBox::new_ptr_mut(self as * const ChunkManager as * mut ChunkManager));
					chunk = tchunk;
					zero = tzero;
				},
			}
			
			//error out of memory (unlocking is managed by TakeLock destructor)
			match chunk.as_ref() {
				Some(chunk) => {
					//try to split
					let residut = Self::split(chunk.clone(),checked_size);
					debug_assert!(chunk.get().get_inner_size() >= checked_size);
					match residut {
						Some(x) => guard.pools.insert_chunk(x,ChunkInsertMode::LIFO),
						None => {},
					}
				},
				None => return (0,zero),
			}
		}
		
		//ok this is good get ptr
		let chunk = chunk.unwrap();
		let mut res = chunk.get_content_addr();
		
		//check for padding
		if res % align != 0 {
			res = PaddedChunk::pad(res,PaddedChunk::calc_padding(res,align,size,chunk.get_inner_size()),size);
		}
		
		//final check
		debug_assert!(res % align == 0);
		debug_assert!(res != 0);
		debug_assert!(chunk.contain(res));
		debug_assert!(chunk.contain(res + size - 1));
		
		//return
		return (res,zero);
	}

	pub fn rebind_mm_source(&mut self,mmsource: Option<MemorySourcePtr>) {
		self.locked.lock().mmsource = mmsource;
	}

	fn refill(locked: &mut MediumChunkManagerLocked, size: Size, zero_filled: bool, manager: ChunkManagerPtr) -> (Option<MediumChunkPtr>, bool) {
		//errors
		debug_assert!(size > 0);
		
		//trivial
		let mmsource;
		match locked.mmsource.as_mut() {
			Some(x) => mmsource = x,
			None => return (None, zero_filled),
		}
		
		//request mem
		let (segment, zero) = mmsource.map(size,zero_filled,Some(manager));
		debug_assert!(segment.get_inner_size() >= size);
		
		//get inner segment
		let addr = segment.get_content_addr();
		
		//build chunk
		let inner_size = segment.get_inner_size();
		let chunk = MediumChunk::setup_size(addr,inner_size);
		
		//ok return it
		return (Some(chunk),zero);
	}

	fn split(mut chunk: MediumChunkPtr, inner_size: Size) -> Option<MediumChunkPtr> {
		//trivial
		if chunk.is_null() {
			return None;
		}
		
		//align request size
		let inner_size = ops::up_to_power_of_2(inner_size,MEDIUM_MIN_INNER_SIZE);
		
		//get avail size
		let avail_size = chunk.get_inner_size();
		
		//check minimal size
		if avail_size - inner_size <= MEDIUM_MIN_INNER_SIZE + mem::size_of::<MediumChunk>() {
			return None;
		}
		
		//resize
		return chunk.split(inner_size);
	}

	pub fn fill(&mut self,addr: Addr, size: Size, registry: Option<SharedPtrBox<RegionRegistry>>) {
		//errors
		debug_assert!(addr != NULL);
		debug_assert!(size > 0);
		
		//trivial
		if addr == NULL || size == 0 {
			return;
		}
		
		//if need register, create macro bloc
		let chunk;
		match registry {
			Some(mut registry) => {
				let segment = registry.set_entry(addr,size,ChunkManagerPtr::new_ref_mut(self));
				chunk = MediumChunk::setup_size(segment.get_content_addr(),segment.get_inner_size());
			},
			None => {
				chunk = MediumChunk::setup_size(addr,size);
			},
		}
		
		//put in free list
		self.locked.optional_lock(self.use_lock).pools.insert_chunk(chunk,ChunkInsertMode::FIFO);
	}
}

impl ChunkManager for MediumChunkManager {
	fn free(&mut self,addr: Addr) {
		//trivial
		if addr == 0 {
			return;
		}
		
		//check if padded
		let ptr = PaddedChunk::unpad(addr);
		
		//get chunk
		let chunk = MediumChunk::get_chunk_safe(ptr);
		let mut schunk;
		if chunk.is_none() {
			return;
		} else{
			schunk = chunk.unwrap();
		}
		
		//check status
		if schunk.get_status() == CHUNK_FREE {
			//allocCondWarning(ALLOC_DO_WARNING,"Double free, ignoring the request.");
			panic!("Double free corruption !");
		}
		
		//take lock for the current function
		let mmsource;
		{
			let mut guard = self.locked.optional_lock(self.use_lock);
			//try merge
			schunk = guard.pools.merge(schunk);
			mmsource = guard.mmsource.clone();
			
			//if whe have a source, we may try to check if we can clear the bloc
			//we didn't do it here to avoid to take time in critical section
			//as this actions didn't require the local lock
			if guard.mmsource.is_none() || schunk.is_single() == false {
				guard.pools.insert_chunk(schunk,ChunkInsertMode::FIFO);
				return;
			}
		}
		
		//if need final free to mm source
		debug_assert!(schunk.is_single());
		mmsource.unwrap().unmap(RegionSegment::get_from_content_ptr(schunk.get_root_addr()));
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		let old_ptr = ptr;
	
		//trivial
		if ptr == NULL && size == NULL {
			return NULL;
		} else if ptr == NULL {
			return self.malloc(size, BASIC_ALIGN, false).0;
		} else if size == NULL {
			self.free(ptr);
			return NULL;
		}
		
		//check if padded
		let ptr = PaddedChunk::unpad(ptr);
		
		//get old size
		let chunk = MediumChunk::get_chunk_safe(ptr);
		let schunk;
		match chunk {
			Some(x) => schunk = x,
			None => {
				panic!("Try to reallocate an invalid segment, cannot proceed, return NULL");
				//return NULL;
			}
		}

		//TODO assume
		let old_size = schunk.get_inner_size();
		let delta = old_size as SSize - size as SSize;
		
		//if can resuse old one without resize
		if old_size >= size && delta <= REALLOC_THREASHOLD {
			return old_ptr;
		}
		
		//check if can realloc the next one
		//TODO maybe find a way to avoid to retake the lock for next malloc call
		{
			let mut guard = self.locked.optional_lock(self.use_lock);

			//try merge
			let merged;
			if size > old_size {
				merged = guard.pools.try_merge_for_size(schunk.clone(),size);
			} else {
				merged = Some(schunk.clone());
			}

			//is not merged
			match merged {
				Some(merged) => {
					//check
					debug_assert!(merged == schunk);
					debug_assert!(merged.get_inner_size() >= size);
			
					//check for split
					let residut = Self::split(merged.clone(),size);
					debug_assert!(merged.get_inner_size() >= size);
					match residut {
						Some(x) => guard.pools.insert_chunk(x,ChunkInsertMode::LIFO),
						None => {},
					}
									
					//ok return, the lock is auto removed by TakeLock destructor
					return merged.get_content_addr();
				},
				None => {},
			}
		}
		
		//ok do alloc/copy/free
		let new_ptr = self.malloc(size,BASIC_ALIGN,false).0;
		if new_ptr != NULL {
			libc::memcpy(new_ptr,ptr,size.max(old_size));
		}

		//free olf
		self.free(ptr);
		
		//Return
		return new_ptr;
	}

	fn get_inner_size(&mut self,ptr: Addr) -> Size {
		//trivial
		if ptr == NULL {
			return 0;
		}
		
		//unpadd
		let real_ptr = PaddedChunk::unpad(ptr);
		debug_assert!(real_ptr <= ptr);
		let delta = ptr - real_ptr;
		
		let chunk = MediumChunk::get_chunk_safe(real_ptr);
		match chunk {
			Some(chunk) => return chunk.get_inner_size() - delta,
			None => return 0,
		}
	}

    fn get_total_size(&mut self,ptr: Addr) -> Size {
		//trivial
		if ptr == NULL {
			return 0;
		}
		
		//unpadd
		let real_ptr = PaddedChunk::unpad(ptr);
		debug_assert!(real_ptr <= ptr);
		
		let chunk = MediumChunk::get_chunk_safe(real_ptr);
		match chunk {
			Some(chunk) => return chunk.get_total_size(),
			None => return 0,
		}
	}

    fn get_requested_size(&mut self,_ptr: Addr) -> Size {
		UNSUPPORTED
	}
	
    fn hard_checking(&mut self,) {
		self.locked.lock().pools.hard_checking();
	}

	fn is_thread_safe(&self) -> bool {
		self.use_lock
	}

    fn remote_free(&mut self,ptr: Addr) {
		if self.use_lock {
			self.free(ptr);
		} else {
			panic!("Unsuppported remoteFree() function for medium allocators without locks.");
		}
	}

    fn set_parent_chunk_manager(&mut self,parent: Option<ChunkManagerPtr>) {
		self.parent = parent;
	}

    fn get_parent_chunk_manager(&mut self) -> Option<ChunkManagerPtr> {
		self.parent.clone()
	}
}

#[cfg(test)]
mod tests
{
	use chunk::medium::manager::*;
	use mmsource::dummy::DummyMMSource;
	use registry::registry::RegionRegistry;
	use portability::osmem;

	#[test]
	fn build() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = DummyMMSource::new(Some(&mut registry));
		let mut _manager = MediumChunkManager::new(false, Some(MemorySourcePtr::new_ptr_mut(&mut mmsource)));
	}

	#[test]
	fn fill() {
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn fill_register() {
		let mut registry = RegionRegistry::new();
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,Some(SharedPtrBox::new_ref_mut(&mut registry)));

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>() + mem::size_of::<RegionSegment>());
		assert_eq!(zero, false);

		assert_eq!(ptr,registry.get_segment(res).unwrap().get().get_root_addr());
		//assert!(pmanager == registry.get_segment(res).unwrap().get_manager().unwrap());
		registry.get_segment(res).unwrap().get_manager().unwrap().free(res);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn malloc() {
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>() * 2 + 64);
		assert_eq!(zero, false);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn free() {
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		manager.free(res);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_basic_1() {
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(128,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		let res = manager.realloc(res,64);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(manager.get_inner_size(res), 128);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_basic_2() {
		let mut manager = MediumChunkManager::new(false, None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, NULL);
		assert_eq!(zero, false);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		let res = manager.realloc(res,128);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(manager.get_inner_size(res), 128);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_basic_move() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		manager.malloc(64,BASIC_ALIGN,false);

		let (res2,_) = manager.malloc(128,BASIC_ALIGN,false);
		manager.free(res2);
		
		let res = manager.realloc(res,128);
		assert_eq!(res, res2);
		assert_eq!(manager.get_inner_size(res), 128);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_free() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		let res2 = manager.realloc(res,0);
		assert_eq!(res2, NULL);

		let (res,zero) = manager.malloc(64,BASIC_ALIGN,false);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());
		assert_eq!(zero, false);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_malloc() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let res = manager.realloc(NULL,64);
		assert_eq!(res, ptr + mem::size_of::<MediumChunk>());

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_not_enougth() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,_) = manager.malloc(64,BASIC_ALIGN,false);
		let res = manager.realloc(res,4*1024*1024);
		assert_eq!(res, NULL);

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn realloc_copy() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,_) = manager.malloc(64,BASIC_ALIGN,false);
		manager.malloc(64,BASIC_ALIGN,false);

		let p = res as * mut u8;
		for i in 0..64 {
			unsafe{p.offset(i).write(i as u8)};
		}

		let res = manager.realloc(res,128);
		
		let p = res as * const u8;
		for i in 0..64 {
			assert_eq!(unsafe{p.offset(i).read()},i as u8);
		}

		osmem::munmap(ptr,2*1024*1024);
	}

	#[test]
	fn get_size() {
		let mut manager = MediumChunkManager::new(false, None);

		let ptr = osmem::mmap(0,2*1024*1024);
		manager.fill(ptr, 2*1024*1024,None);

		let (res,_) = manager.malloc(64,BASIC_ALIGN,false);
		
		assert_eq!(manager.get_inner_size(res),64);
		assert_eq!(manager.get_total_size(res),64+mem::size_of::<MediumChunk>());
		assert_eq!(manager.get_requested_size(res),UNSUPPORTED);
	}

	#[test]
	fn mmsource_refill() {
		//TODO need mocking which does not work with no_std !
	}

	#[test]
	fn mmsource_free() {
		//TODO need mocking which does not work with no_std !
	}

	#[test]
	fn full_workflow() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = DummyMMSource::new(Some(&mut registry));
		let mut manager = MediumChunkManager::new(false, Some(MemorySourcePtr::new_ptr_mut(&mut mmsource)));

		//alloc
		let (ptr,_zero) = manager.malloc(64, BASIC_ALIGN, false);

		//realloc
		let mut pmanager = registry.get_segment(ptr).unwrap().get_manager().unwrap();
		let ptr = pmanager.realloc(ptr,128);

		//free
		let mut pmanager = registry.get_segment(ptr).unwrap().get_manager().unwrap();
		pmanager.free(ptr);

		//check free
		assert_eq!(registry.get_segment(ptr).is_none(),true);
	}
}