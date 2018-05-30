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

/*impl MemorySource for DummyMMSource {
	fn map(&mut self,inner_size: Size, zero_filled: bool, manager: & mut ChunkManager) -> (RegionSegment, bool)
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
		let pmanager = manager as *mut ChunkManager;
		let registry = unsafe{*self.registry};
		if !self.registry.is_null() && !pmanager.is_null() {
			res = registry.set_entry(ptr,total_size,pmanager);
		} else {
			res = RegionSegment::new(ptr,total_size,pmanager);
		}

		//ret		
		(res,true)
	}

	fn remap(&mut self,old_segment: *mut RegionSegment,new_inner_size: Size, manager: & mut ChunkManager) -> RegionSegment
	{
		//TODO replace
		self.map(new_inner_size,true,manager).0
	}

	fn unmap(&mut self,segment: & mut RegionSegment)
	{

	}
}*/