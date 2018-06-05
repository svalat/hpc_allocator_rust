/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module implement the dummy memory source which directly forward
///the requests to the OS without doing any caching.

use common::types::Size;
use common::traits::{MemorySource,ChunkManager};
use common::consts::*;
use common::ops;
use portability::osmem;
use registry::registry::RegionRegistry;
use registry::segment::RegionSegment;
use core::mem;

pub struct DummyMMSource {
	registry: Option<* mut RegionRegistry>,
}

impl DummyMMSource {
	/// Create a new memory source. It optionally take a region registry to be used for registration.
	/// 
	/// TODO we should use a SharedPtrBox instead of bypassing locally the ownership and casting to pointers.
	pub fn new(registry: Option<& mut RegionRegistry>) -> DummyMMSource {
		let ptr;
		
		match registry {
			Some(x) => ptr = Some(x as * mut RegionRegistry),
			None => ptr = None,
		}

		DummyMMSource {
			registry: ptr
		}
	}

	/// Retur reference to the registry for internal use.
	#[inline]
	fn get_registry(&mut self) -> &mut RegionRegistry {
		ops::ref_from_option_ptr(self.registry)
	}
}

/// Implement the memory source trait to be use inside chunk managers and allocators.
impl MemorySource for DummyMMSource {
	fn map(&mut self,inner_size: Size, _zero_filled: bool, manager: Option<* mut ChunkManager>) -> (RegionSegment, bool)
	{
		//errors
		debug_assert!(inner_size > 0);
		
		//compute total size
		let mut total_size = inner_size + mem::size_of::<RegionSegment>();
		
		//TODO warning if really too small
		
		//if to small
		if total_size < REGION_SPLITTING {
			total_size = REGION_SPLITTING;
		}
		
		//roudn to multiple of page size
		let total_size = ops::up_to_power_of_2(total_size,SMALL_PAGE_SIZE);
		
		//allocate
		let ptr = osmem::mmap(0,total_size);

		//register
		let res;
		let has_manager = !manager.is_none();
		if !self.registry.is_none() && has_manager {
			let registry = unsafe{&mut *self.registry.unwrap()};
			let pmanager = manager.unwrap();
			res = registry.set_entry(ptr,total_size,pmanager);
		} else {
			let pmanager = manager.unwrap();
			res = RegionSegment::new(ptr,total_size,Some(pmanager));
		}

		//ret		
		(res,true)
	}

	fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: Option<* mut ChunkManager>) -> RegionSegment
	{
		//errors
		old_segment.sanity_check();
		
		//checkup size
		let mut total_size = new_inner_size + mem::size_of::<RegionSegment>();
		if total_size < REGION_SPLITTING {
			total_size = REGION_SPLITTING;
		}
		total_size = ops::up_to_power_of_2(total_size,SMALL_PAGE_SIZE);

		//unregister
		if self.registry.is_some() && old_segment.has_manager() {
			let registry = unsafe{&mut *self.registry.unwrap()};
			registry.remove_from_segment(old_segment);
		}

		//remap
		let ptr = osmem::mremap(old_segment.get_root_addr(),old_segment.get_total_size(),total_size,0);

		//register
		let res;
		let has_manager = manager.is_some();
		let pmanager = manager.unwrap();
		if self.registry.is_some() && has_manager {
			res = self.get_registry().set_entry(ptr,total_size,pmanager);
		} else {
			res = RegionSegment::new(ptr,total_size,Some(pmanager));
		}

		res
	}

	fn unmap(&mut self,segment: RegionSegment)
	{
		//errors
		segment.sanity_check();
			
		//unregister
		if self.registry.is_some() && segment.has_manager() {
			self.get_registry().remove_from_segment(segment);
		}
		
		//unmap it
		osmem::munmap(segment.get_root_addr(),segment.get_total_size());
	}
}

#[cfg(test)]
mod tests
{
	use mmsource::dummy::*;
	use chunk::dummy::*;

	#[test]
	fn test_full_workflow() {
		let mut registry = RegionRegistry::new();
		let mut manager = DummyChunkManager::new();
		let mut mmsource = DummyMMSource::new(Some(&mut registry));

		//map
		let (seg1,zeroed) = mmsource.map(2*1024*1024,true,Some(&mut manager));
		assert!(seg1.get_inner_size() >= 2*1024*1024);
		assert_eq!(zeroed,true);

		//a secod to force mremap to move
		let (seg2,zeroed2) = mmsource.map(2*1024*1024,true,Some(&mut manager));
		assert!(seg2.get_inner_size() >= 2*1024*1024);
		assert_eq!(zeroed2,true);

		//check registry
		let seg1_check = registry.get_segment(seg1.get_root_addr());
		assert_eq!(seg1_check.unwrap().get_root_addr(),seg1.get_root_addr());

		//remap
		let seg1_remap = mmsource.remap(seg1,4*1024*1024,Some(&mut manager));

		//check registry
		if seg1.get_root_addr() != seg1_remap.get_root_addr() {
			let seg1_check = registry.get_segment(seg1.get_root_addr());
			assert!(seg1_check.is_none());
		}
		let seg1_check = registry.get_segment(seg1_remap.get_root_addr());
		assert_eq!(seg1_check.unwrap().get_root_addr(),seg1_remap.get_root_addr());

		//unmap
		mmsource.unmap(seg1_remap);
		let seg1_check = registry.get_segment(seg1_remap.get_root_addr());
		assert!(seg1_check.is_none());
	}
}
