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
use chunk::medium::chunk::{MediumChunk,MediumChunkPtr};
use portability::spinlock::SpinLock;
use common::traits::{ChunkManager,MemorySource};
use registry::registry::RegionRegistry;
use common::types::{Addr,Size};
use common::consts::*;
use common::ops;
use chunk::padding::PaddedChunk;

struct MediumAllocatorLocked {
	pools: MediumFreePool,
	mmsource: Option<* mut MemorySource>,
}

/// Implement the medium chunk allocator based on MediumFreePool
struct MediumAllocator {
	locked: SpinLock<MediumAllocatorLocked>,
	registry: Option<* mut RegionRegistry>,
	use_lock: bool,
}

//implement
impl MediumAllocator {
	pub fn new(use_lock: bool, mmsource: Option<* mut MemorySource>) -> Self {
		Self {
			locked: SpinLock::new(MediumAllocatorLocked {
				pools: MediumFreePool::new(),
				mmsource: mmsource, 
			}),
			registry: None,
			use_lock: use_lock,
		}
	}

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
					let (tchunk, tzero) = Self::refill(&mut *guard,checked_size,zero,self as * const ChunkManager as * mut ChunkManager);
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
		let mut res = chunk.get_addr();
		
		//check for padding
		if res % align != 0 {
			res = PaddedChunk::pad(res,PaddedChunk::calc_padding(res,align,size,chunk.get_inner_size()),size);
		}
		
		//final check
		debug_assert!(res % align == 0);
		debug_assert!(res != 0 && chunk.contain(res) && chunk.contain(res + size - 1));
		
		//return
		return (res,zero);
	}

	fn refill(locked: &mut MediumAllocatorLocked, size: Size, zero_filled: bool, manager: * mut ChunkManager) -> (Option<MediumChunkPtr>, bool) {
		//errors
		/*debug_assert!(size > 0);
		
		//trivial
		let mmsource;
		match locked.mmsource.as_mut() {
			Some(x) => mmsource = x,
			None => return (None, zero_filled),
		}
		
		//request mem
		let (segment, zero) =  unsafe{*mmsource}.map(size,zero_filled,Some(manager));
		debug_assert!(segment.get_inner_size() >= size);
		
		//get inner segment
		let addr = segment.get_content_addr();
		
		//build chunk
		let inner_size = segment.get_inner_size();
		let chunk = MediumChunk::setup_size(addr,inner_size);
		
		//ok return it
		return (Some(chunk),zero);*/
		panic!("tmp");
	}

	fn split(chunk: MediumChunkPtr, inner_size: Size) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}
}

impl ChunkManager for MediumAllocator {
	fn free(&mut self,addr: Addr) {
		panic!("TODO");
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
		panic!("TODO");
	}

	fn get_inner_size(&mut self,ptr: Addr) -> Size {
		panic!("TODO");
	}

    fn get_total_size(&mut self,ptr: Addr) -> Size {
		panic!("TODO");
	}

    fn get_requested_size(&mut self,ptr: Addr) -> Size {
		panic!("TODO");
	}
	
    fn hard_checking(&mut self,) {
		panic!("TODO");
	}

	fn is_thread_safe(&self) -> bool {
		panic!("TODO");
	}

    fn remote_free(&mut self,ptr: Addr) {
		panic!("TODO");
	}

    fn set_parent_chunk_manager(&mut self,parent: Option<* mut ChunkManager>) {
		panic!("TODO");
	}

    fn get_parent_chunk_manager(&mut self) -> Option<* mut ChunkManager> {
		panic!("TODO");
	}
}
