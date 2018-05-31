/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module implement the dummy memory source which directly forward
///the requests to the OS without doing any caching.

use common::types::*;
use common::traits::*;
use common::consts::*;
use common::ops::*;
use portability::osmem;
use registry::registry::*;
use registry::segment::*;
use core::mem;

struct DummyMMSource {
	registry: * mut RegionRegistry,
}

impl DummyMMSource {
	pub fn new(registry: * mut RegionRegistry) -> DummyMMSource {
		DummyMMSource {
			registry: registry
		}
	}
}

impl MemorySource for DummyMMSource {
	fn map(&mut self,inner_size: Size, _zero_filled: bool, manager: & mut ChunkManager) -> (RegionSegment, bool)
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
		let total_size = up_to_power_of_2(total_size,SMALL_PAGE_SIZE);
		
		//allocate
		let ptr = osmem::mmap(0,total_size);

		//register
		let res;
		let pmanager = manager as *const ChunkManager as *mut ChunkManager;
		if !self.registry.is_null() && !pmanager.is_null() {
			let registry = unsafe{&mut *self.registry};
			res = registry.set_entry(ptr,total_size,pmanager);
		} else {
			res = RegionSegment::new(ptr,total_size,pmanager);
		}

		//ret		
		(res,true)
	}

	fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: & mut ChunkManager) -> RegionSegment
	{
		//errors
		old_segment.sanity_check();
		
		//checkup size
		let mut total_size = new_inner_size + mem::size_of::<RegionSegment>();
		if total_size < REGION_SPLITTING {
			total_size = REGION_SPLITTING;
		}
		total_size = up_to_power_of_2(total_size,SMALL_PAGE_SIZE);

		//unregister
		let registry = unsafe{&mut *self.registry};
		if !self.registry.is_null() && !old_segment.get_manager().is_none() {
			registry.remove_from_segment(old_segment);
		}

		//remap
		let ptr = osmem::mremap(old_segment.get_root_addr(),old_segment.get_total_size(),total_size,0);

		//register
		let res;
		let pmanager = manager as *const ChunkManager as *mut ChunkManager;
		if !self.registry.is_null() && !pmanager.is_null() {
			res = registry.set_entry(ptr,total_size,pmanager);
		} else {
			res = RegionSegment::new(ptr,total_size,pmanager);
		}

		res
	}

	fn unmap(&mut self,segment: RegionSegment)
	{
		//errors
		segment.sanity_check();
			
		//unregister
		if !self.registry.is_null() && !segment.get_manager().is_none() {
			let registry = unsafe{&mut *self.registry};
			registry.remove_from_segment(segment);
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
		let mut mmsource = DummyMMSource::new(&mut registry as * mut RegionRegistry);

		//map
		let (seg1,zeroed) = mmsource.map(2*1024*1024,true,&mut manager);
		assert!(seg1.get_inner_size() >= 2*1024*1024);
		assert_eq!(zeroed,true);

		//check registry
		let seg1_check = registry.get_segment(seg1.get_root_addr());
		assert_eq!(seg1_check.unwrap().get_root_addr(),seg1.get_root_addr());

		//remap
		let seg1_remap = mmsource.remap(seg1,4*1024*1024,&mut manager);

		//check registry
		let seg1_check = registry.get_segment(seg1.get_root_addr());
		assert!(seg1_check.is_none());
		let seg1_check = registry.get_segment(seg1_remap.get_root_addr());
		assert_eq!(seg1_check.unwrap().get_root_addr(),seg1_remap.get_root_addr());

		//unmap
		mmsource.unmap(seg1_remap);
		let seg1_check = registry.get_segment(seg1_remap.get_root_addr());
		assert!(seg1_check.is_none());
	}
}
