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
use registry::region::*;
use portability::spinlock::*;
use portability::osmem;
use core::ptr;
use core::mem;

pub type RegistryPtr = * const Region;

///Define the global registry
pub struct RegionRegistry {
	regions: SpinLock<[RegistryPtr; MAX_REGIONS]>,
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
			//set
			self.set_one_segment_entry(ptr+offset,segment);

			//move
			offset += REGION_SPLITTING;
		}
	}

	fn set_one_segment_entry(&mut self, ptr:Addr, segment: RegionSegment) {
		if ptr > PHYS_MAX_ADDR {
			//TODO use warning
			//allocWarning("Invalid address range in request for region registry : %p !",ptr);
			panic!("Invalid address range in request for region registry : {} !",ptr);
			//None
		}

		//compute ID in region
		let id = ((ptr) % REGION_SIZE) / REGION_SPLITTING;

		//check
		debug_assert!(id < REGION_ENTRIES);

		//get the local region
		let region = self.get_region_or_create(ptr);
		region.set(id,segment.get_ptr())
	}

	pub fn get_segment(& self,ptr: Addr) -> Option<RegionSegment> {
		if ptr > PHYS_MAX_ADDR {
			//TODO use fatal
			//allocWarning("Invalid address range in request for region registry : %p !",ptr);
			panic!("Invalid address range in request for region registry : {} !",ptr);
			//None
		}

		//compute ID in region
		let id = ((ptr) % REGION_SIZE) / REGION_SPLITTING;

		//check
		debug_assert!(id < REGION_ENTRIES);

		//get the local region
		let id = self.get_region_id(ptr);
		if self.regions.nolock_safe_read()[id].is_null() {
			None
		} else {
			let region = unsafe{& *(self.regions.nolock_safe_read()[id])};
			Some(unsafe{*region.get(id)})
		}
	}

	pub fn remote_from_ptr(&mut self,ptr: Addr) {
		let seg = self.get_segment(ptr);
		match seg {
			Some(seg) => self.remove_from_segment(seg),
			None => {} //TODO WARNING
		}
	}

	pub fn remove_from_segment(&mut self, segment: RegionSegment) {
		
	}

	//TODO
	fn get_region_or_create(&mut self,ptr: Addr) -> &mut Region {
		let id = self.get_region_id(ptr);
		if self.regions.nolock_safe_read()[id].is_null() {
			self.setup_new_region(ptr)
		} else {
			unsafe{&mut *(self.regions.nolock_safe_read()[id] as * mut Region)}
		}
	}

	fn setup_new_region(&mut self, ptr:Addr) -> &mut Region {
		let ret;
	
		//errors
		if ptr >= PHYS_MAX_ADDR {
			//TODO replace by assume
			panic!("Address is too big to be registered into the global region registry !");
		}
		
		//get region ID
		let id = self.get_region_id(ptr);

		//ensure init and take the lock
		{
			//take lock
			let mut regions = *(self.regions.lock());
			//check if already mapped, otherwise, to nothing
			if regions[id].is_null() {
				// @todo this may be better to hardly control this address choice, maybe use the allocator when a first chain is available.
				let addr = osmem::mmap(0,mem::size_of::<Region>());
				let region = Region::new(addr,false);
				// @todo PARALLEL check for atomic operation instead of lock 
				regions[id] = region;
			}

			//setup ret
			ret = regions[id];
		}

		//return pointer to the region
		unsafe {&mut *(ret as * mut Region)}
	}

	fn get_region_id(&self, addr: Addr) -> Size {
		//errors
		debug_assert!(addr != 0);
		debug_assert!(addr < PHYS_MAX_ADDR);
		
		//TODO can be optimize if we consider power of 2
		let id = addr / REGION_SIZE;
		debug_assert!(id < MAX_REGIONS);
		
		id
	}
}
