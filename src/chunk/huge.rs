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
use common::types::{Addr,Size,SSize};
use common::consts::*;
use registry::segment::RegionSegment;
use chunk::padding::PaddedChunk;

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

	fn get_mm_source(&mut self) -> &mut MemorySource {
		unsafe{&mut *self.mmsource}
	}

	fn malloc(&mut self,size: Size,align: Size,zero_filled: bool) -> (Addr,bool) {
		let mut zero = zero_filled;
		let mut checked_size = size;

		//errors
		//allocAssert(this != NULL);
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
		
		//request memory to mm source
		let p = self as * mut ChunkManager;
		let (segment,z) = self.get_mm_source().map(checked_size,zero,Some(p));
		//allocCondWarning(segment != NULL,"Caution, get OOM in huge allocation method.");
		
		//setup zero
		zero = z;

		//ok this is good get ptr
		let mut res = segment.get_content_addr();
		if res == 0 {
			return (0,zero);
		}
		
		//check for padding
		if res % align != 0 {
			res = PaddedChunk::new_from_segment(segment,PaddedChunk::calc_padding_for_segment(segment,align,size)).get_addr();
		}
		
		//final check
		debug_assert!(res % align == 0);
		debug_assert!(res != 0 && segment.contain(res) && segment.contain(res + size-1));
		
		return (res,zero);
	}
}

//impl trait
impl ChunkManager for HugeChunkManager {
	fn free(&mut self,addr: Addr) {
        //trivial
		if addr == 0 {
			return;
		}
		
		//remove padding
		let addr = PaddedChunk::unpad(addr);

		//TODO make a safe version of this function with checking (if possible)
		let segment = RegionSegment::get_segment(addr);

		//return it to mm source
		self.get_mm_source().unmap(segment);
    }

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
        let old_ptr = ptr;

		//trivial
		if ptr == 0 && size == 0 {
			return 0;
		} else if ptr == 0 {
			return self.malloc(size,1,false).0;
		} else if size == 0 {
			self.free(ptr);
			return 0;
		}
		
		//check if padded
		let ptr = PaddedChunk::unpad(ptr);
		
		//get old size
		let segment = RegionSegment::get_segment(ptr);
		//allocAssert(segment != NULL);
		//TODO assume
		let old_size = segment.get_inner_size();
		let delta = old_size as SSize - size as SSize;
		
		//if can resuse old one without resize
		if old_size >= size && delta <= REALLOC_THREASHOLD {
			return old_ptr;
		}
		
		//remap
		let p = self as * mut ChunkManager;
		let new_segment = self.get_mm_source().remap(segment,size,Some(p));
		//allocCondWarning(newSegment != NULL,"Get OOM in realloc of huge segment.");
		debug_assert!(new_segment.get_inner_size() >= size);
		
		return new_segment.get_content_addr();
    }

	fn get_inner_size(&mut self,ptr: Addr) -> Size {
        //trivial
		if ptr == 0 {
			return 0;
		}
		
		//unpadd
		let real_ptr = PaddedChunk::unpad(ptr);
		debug_assert!(real_ptr <= ptr);
		let delta = ptr as i64 - real_ptr as i64;
		debug_assert!(delta >= 0);
		
		let segment = RegionSegment::get_segment(ptr);
		segment.get_inner_size() - delta as Size
    }

	fn get_total_size(&mut self,ptr: Addr) -> Size {
        //trivial
		if ptr == 0 {
			return 0;
		}
		
		//unpadd
		let real_ptr = PaddedChunk::unpad(ptr);
		debug_assert!(real_ptr <= ptr);
		let delta = ptr as i64 - real_ptr as i64;
		debug_assert!(delta >= 0);
		
		let segment = RegionSegment::get_segment(ptr);
		segment.get_total_size() - delta as Size
    }

	fn get_requested_size(&mut self,_ptr: Addr) -> Size {
        UNSUPPORTED
    }
	
    fn hard_checking(&mut self) {
        //TODO
    }

	fn is_thread_safe(&self) -> bool {
        true
    }

	fn remote_free(&mut self,ptr: Addr) {
		self.free(ptr);
        panic!("Should not be used as HugeChunkManager is thread safe !");
    }

    fn set_parent_chunk_manager(&mut self,parent: Option<* mut ChunkManager>) {
        self.parent = parent;
    }

    fn get_parent_chunk_manager(&mut self) -> Option<* mut ChunkManager> {
        self.parent
    }
}
