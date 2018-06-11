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
use common::shared::SharedPtrBox;

//decl
pub struct HugeChunkManager {
	/// Keep track of the parent chunk manager
	parent: Option<SharedPtrBox<ChunkManager>>,
	mmsource: SharedPtrBox<MemorySource>,
}

//impl
impl HugeChunkManager {
	pub fn new(mmsource: SharedPtrBox<MemorySource>) -> HugeChunkManager {
		HugeChunkManager {
			parent:None,
			mmsource: mmsource,
		}
	}

	pub fn rebind_mm_source(&mut self,mmsource: SharedPtrBox<MemorySource>) {
		self.mmsource = mmsource;
	}

	fn get_mm_source(&mut self) -> &mut MemorySource {
		self.mmsource.get_mut()
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
		let manager: SharedPtrBox<ChunkManager> = SharedPtrBox::new_ref_mut(self);
		let (segment,z) = self.get_mm_source().map(checked_size,zero,Some(manager));
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
			res = PaddedChunk::new_from_segment(segment.clone(),align,size).get_content_addr();
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
		let segment = RegionSegment::get_from_content_ptr(addr);

		//return it to mm source
		self.get_mm_source().unmap(segment);
    }

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
        let old_ptr = ptr;

		//trivial
		if ptr == 0 && size == 0 {
			return 0;
		} else if ptr == 0 {
			return self.malloc(size,BASIC_ALIGN,false).0;
		} else if size == 0 {
			self.free(ptr);
			return 0;
		}
		
		//check if padded
		let ptr = PaddedChunk::unpad(ptr);
		
		//get old size
		let segment = RegionSegment::get_from_content_ptr(ptr);
		//allocAssert(segment != NULL);
		//TODO assume
		let old_size = segment.get_inner_size();
		let delta = old_size as SSize - size as SSize;
		
		//if can resuse old one without resize
		if old_size >= size && delta <= REALLOC_THREASHOLD {
			return old_ptr;
		}
		
		//remap
		let manager: SharedPtrBox<ChunkManager> = SharedPtrBox::new_ref_mut(self);
		let new_segment = self.get_mm_source().remap(segment,size,Some(manager));
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
		let delta = ptr - real_ptr;
		
		let segment = RegionSegment::get_from_content_ptr(real_ptr);
		segment.get_inner_size() - delta
    }

	fn get_total_size(&mut self,ptr: Addr) -> Size {
        //trivial
		if ptr == 0 {
			return 0;
		}
		
		//unpadd
		let real_ptr = PaddedChunk::unpad(ptr);
		
		let segment = RegionSegment::get_from_content_ptr(real_ptr);
		segment.get_total_size()
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

    fn set_parent_chunk_manager(&mut self,parent: Option<SharedPtrBox<ChunkManager>>) {
        self.parent = parent;
    }

    fn get_parent_chunk_manager(&mut self) -> Option<SharedPtrBox<ChunkManager>> {
        self.parent.clone()
    }
}

#[cfg(test)]
mod tests
{
	use chunk::huge::*;
	use registry::registry::RegionRegistry;
	use mmsource::cached::CachedMMSource;
	use common::shared::SharedPtrBox;
	use chunk::dummy::DummyChunkManager;

	#[test]
	fn basic() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));

		let (ptr,zero) = huge.malloc(4096, BASIC_ALIGN, false);
		assert_eq!(zero, true);
		assert!(ptr != 0);
		assert_eq!(huge.get_inner_size(ptr),2*1024*1024-32);
		assert_eq!(huge.get_total_size(ptr),2*1024*1024);

		let ptr = huge.realloc(ptr, 4*1024*1024);
		assert_eq!(huge.get_inner_size(ptr),4*1024*1024+4096-32);
		assert_eq!(huge.get_total_size(ptr),4*1024*1024+4096);

		huge.free(ptr);

		let (ptr, zero) = huge.malloc(0,BASIC_ALIGN,false);
		assert_eq!(ptr, 0);
		assert_eq!(zero, false);

		huge.free(0);

		mmsource.free_all();
	}

	#[test]
	fn min_size() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));

		let (ptr,zero) = huge.malloc(8, BASIC_ALIGN, false);
		assert_eq!(zero, true);
		assert!(ptr != 0);
		assert_eq!(huge.get_inner_size(ptr),2*1024*1024-32);
		assert_eq!(huge.get_total_size(ptr),2*1024*1024);

		huge.free(ptr);

		mmsource.free_all();
	}

	#[test]
	fn align() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));

		let (ptr,zero) = huge.malloc(8, 128, false);
		assert_eq!(zero, true);
		assert!(ptr != 0);
		assert!(ptr % 128 == 0);

		assert_eq!(huge.get_inner_size(ptr),2*1024*1024-32-96);
		assert_eq!(huge.get_total_size(ptr),2*1024*1024);

		assert_eq!(huge.get_inner_size(0),0);
		assert_eq!(huge.get_total_size(0),0);

		assert_eq!(huge.get_requested_size(ptr), UNSUPPORTED);

		huge.free(ptr);

		mmsource.free_all();
	}

	#[test]
	fn rebind_mm_source() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));

		huge.rebind_mm_source(SharedPtrBox::new_ref_mut(&mut mmsource));

		mmsource.free_all();
	}

	#[test]
	fn realloc() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));

		let ptr = huge.realloc(0,0);
		assert_eq!(ptr, 0);

		let ptr = huge.realloc(0,1*1024*1024);

		huge.realloc(ptr,0);

		mmsource.free_all();
	}

	#[test]
	fn is_thread_safe() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));
		assert_eq!(huge.is_thread_safe(), true);
	}

	#[test]
	#[should_panic]
	fn remote_free() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));
		huge.remote_free(0);
	}

	#[test]
	fn chunk_manager() {
		let mut registry = RegionRegistry::new();
		let mut mmsource = CachedMMSource::new_default(Some(SharedPtrBox::new_ref_mut(&mut registry)));
		let mut huge = HugeChunkManager::new(SharedPtrBox::new_ref_mut(&mut mmsource));
		let mut manager = DummyChunkManager::new();

		huge.set_parent_chunk_manager(Some(SharedPtrBox::new_ref_mut(&mut manager)));

		assert_eq!(huge.get_parent_chunk_manager().unwrap().get_ptr(), (&manager as * const DummyChunkManager));
	}
}
