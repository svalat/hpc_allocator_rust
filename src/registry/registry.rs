/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This implement the region registry to keep track of all the allocated segments
///and map their related chunk manager.

//import
use common::consts::*;
use common::types::*;
use common::traits::*;
use common::ops::*;
use registry::segment::*;
use portability::spinlock::*;
use core::ptr;

pub type RegistryEntry = * const RegionSegment;

///Define the global registry
pub struct RegionRegistry {
	regions: SpinLock<[RegistryEntry; MAX_REGIONS]>,
}

impl RegionRegistry {
	///constructor
	pub fn new() -> Self {
		RegionRegistry {
			regions: SpinLock::new([ptr::null(); MAX_REGIONS]),
		}
	}

	pub fn set_entry( &mut self, ptr: Addr ,total_size: Size,manager: *mut ChunkManager) -> RegionSegment {
		//errors
		debug_assert!(ptr != 0);
		debug_assert!(total_size >= REGION_SPLITTING);
		debug_assert!(!manager.is_null());

		let res = RegionSegment::new(ptr,total_size,manager);
		self.set_segment_entry(res);
		
		res
	}
	
	pub fn set_segment_entry( &mut self, segment: RegionSegment ) {
		let ptr = segment.get_root_addr();

		//errors
		segment.full_sanitify_check();
		//TODO
		//allocAssert(! (chain->flags & SCTK_ALLOC_CHAIN_DISABLE_REGION_REGISTER));

		//warn if too small
		let size = segment.get_total_size();
		debug_assert!(size < REGION_SPLITTING);
		//TODO wanring
		//warning("Caution, using macro blocs smaller than SCTK_MACRO_BLOC_SIZE is dangerous, check usage of flag SCTK_ALLOC_CHAIN_DISABLE_REGION_REGISTER.");

		//TODO can be optimized by playing with REGIN size multiples with ++
		let end_ptr = ceil_to_power_of_2(ptr + size,REGION_SPLITTING);
		let mut offset = 0;
		while ptr + offset < end_ptr
		{
			//get
			let local_entry = self.get_region_entry(ptr + offset,true);

			//cehck and write
			match local_entry {
				Some(entry) => {
					debug_assert!(entry.is_null());
					*entry = segment.get_ptr();
				},
				None => panic!("Fail to find entry in registry !"),
			}

			//move
			offset += REGION_SPLITTING;
		}
	}

	fn get_region_entry<'a>( &'a mut self, ptr: Addr , create_if_not_exist: bool) -> Option<&'a mut RegistryEntry> {
		//TODO
		None
	}
}
