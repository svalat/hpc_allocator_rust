/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This implement a huge allocator. In practice it just forward directly the allocations
/// to its memory source which handle the caching.

//import
use common::traits::{ChunkManager,MemorySource};
use common::types::{Addr,Size};

//decl
pub struct HugeChunkManager {
	/// Keep track of the parent chunk manager
	parent: Option<* mut ChunkManager>,
	mmsource: * mut MemorySource,
}

//impl
impl HugeChunkManager {
	pub fn new(mmsource: * mut MemorySource) -> HugeChunkManager {
		HugeChunkManager {
			parent:None,
			mmsource: mmsource,
		}
	}

	pub fn rebind_mm_source(&mut self,mmsource: * mut MemorySource) {
		self.mmsource = mmsource;
	}
}

//impl trait
impl ChunkManager for HugeChunkManager {
	fn free(&mut self,_addr: Addr) {
       /* //trivial
		if (addr == 0)
			return;
		
		//remove padding
		ptr = PaddedChunk::unpad(ptr);

		//TODO make a safe version of this function with checking (if possible)
		RegionSegmentHeader * segment = RegionSegmentHeader::getSegment(ptr);

		//return it to mm source
		memorySource->unmap(segment);*/
    }

	fn realloc(&mut self,_ptr: Addr,_size:Size) -> Addr {
        panic!("This is fake implementation, should not be called !");
    }

	fn get_inner_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }
	fn get_total_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }

	fn get_requested_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }
	
    fn hard_checking(&mut self) {
        panic!("This is fake implementation, should not be called !");
    }

	fn is_thread_safe(&mut self) -> bool {
        panic!("This is fake implementation, should not be called !");
    }

	fn remote_free(&mut self,_ptr: Addr) {
        panic!("This is fake implementation, should not be called !");
    }

    fn set_parent_chunk_manager(&mut self,_parent: * mut ChunkManager) {
        panic!("This is fake implementation, should not be called !");
    }

    fn get_parent_chunk_manager(&mut self) -> * mut ChunkManager {
        panic!("This is fake implementation, should not be called !");
    }
}
