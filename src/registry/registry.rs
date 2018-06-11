/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This implement the region registry to keep track of all the allocated segments
/// and map their related chunk manager.
///
/// **Trick**: The region registry need to use RegionSegment (macro blocs) which are larger
/// than a given size. But we do not require a size exactly multiple of it because we can handle
/// overlapping on one entries.

//import
use common::consts::*;
use common::types::*;
use common::traits::ChunkManagerPtr;
use common::ops;
use registry::segment::{RegionSegment,RegionSegmentPtr};
use registry::region::Region;
use portability::spinlock::SpinLock;
use portability::osmem;
use core::ptr;
use core::mem;
use common::shared::SharedPtrBox;

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

	/// Setup a new entry and building the related RegionSegment from given address. It
	/// also register it inside the registry before returning the region segment.
	/// 
	/// @param ptr Address to use and where to store the segment header.
	/// @param total_size Define the total size of the segment (accounting the RegionSegment header).
	/// @param manager Optional pointer to a manager to register to the RegionRegistry.
	pub fn set_entry( &mut self, ptr: Addr ,total_size: Size,manager: ChunkManagerPtr) -> RegionSegmentPtr {
		//errors
		debug_assert!(ptr != 0);
		debug_assert!(total_size >= REGION_SPLITTING);
		debug_assert!(!manager.is_null());

		let res = RegionSegment::new(ptr,total_size,Some(manager));
		self.set_segment_entry(res.clone());
		
		res
	}
	
	/// Register an existing segment.
	pub fn set_segment_entry( &mut self, segment: RegionSegmentPtr ) {
		//errors
		segment.full_sanitify_check();

		let ptr = segment.get_root_addr();
		let size = segment.get_total_size();

		//TODO
		//allocAssert(! (chain->flags & SCTK_ALLOC_CHAIN_DISABLE_REGION_REGISTER));

		//warn if too small
		debug_assert!(size >= REGION_SPLITTING);
		//TODO wanring
		//warning("Caution, using macro blocs smaller than SCTK_MACRO_BLOC_SIZE is dangerous, check usage of flag SCTK_ALLOC_CHAIN_DISABLE_REGION_REGISTER.");

		//compute end
		let end_ptr = ops::ceil_to_power_of_2(ptr + size,REGION_SPLITTING);
		let mut offset = 0;
		while ptr + offset < end_ptr
		{
			//set
			self.set_one_segment_entry(ptr+offset,segment.clone());

			//move
			offset += REGION_SPLITTING;
		}
	}

	/// Internal function used to register one entry into the region registry (a segment might cover many).
	fn set_one_segment_entry(&mut self, ptr:Addr, segment: RegionSegmentPtr) {
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
		{
			let region = self.get_region_or_create(ptr);
			region.set(id,segment);
			//TODO remove
			assert!(!region.get(id).is_null());
		}
		//TODO remove
		assert!(!self.get_region(ptr).is_none());
		assert!(!self.get_region(ptr).unwrap().get(id).is_null());
	}

	/// Unset an entry into the registry. As for set_one_segment_entry it might be required
	/// to unse many entries for one segment.
	/// 
	/// TODO see if we can avoid duplication with set_one_segment_entry by playing with some
	/// mut ref
	fn unset_one_segment_entry(&mut self, ptr:Addr, entry: RegionSegmentPtr) {
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
		if region.get(id) == entry {
			region.unset(id);
		}
	}

	/// Check if as a registered entry for the given address.
	pub fn has_entry(&self, ptr: Addr) -> bool {
		let seg = self.get_segment(ptr);
		! seg.is_none()
	}

	/// Return the segment related to a given address and panic if not found.
	#[inline]
	pub fn get_segment_safe(&self, ptr:Addr) -> RegionSegmentPtr {
		let seg = self.get_segment(ptr);
		match seg {
			Some(x) => x,
			None => panic!("Extact segement in registry but don't have for address {}",ptr),
		}
	}

	/// Optionally return the region of a given address.
	fn get_region_entry(&self,ptr:Addr) -> Option<RegionSegmentPtr> {
		if ptr > PHYS_MAX_ADDR {
			//allocWarning("Invalid address range in request for region registry : %p !",ptr);
			panic!("Invalid address range in request for region registry : {} !",ptr);
			//return None;
		}

		//get the local region
		let region = self.get_region(ptr);
		if region.is_none() {
			return None;
		}

		//compute ID in regino
		let id = (ptr % REGION_SIZE) / REGION_SPLITTING;

		//check
		debug_assert!(id < REGION_ENTRIES);

		//return id
		let entry = region.unwrap().get(id);
		if entry.is_null() {
			return None;
		} else {
			let ret = entry;
			return Some(ret);
		}
	}

	/// Optionally return the segment of a given address.
	pub fn get_segment(& self,ptr: Addr) -> Option<RegionSegmentPtr> {
		let mut entry = self.get_region_entry(ptr);
		
		//try previous
		if entry.is_none() || entry.as_ref().unwrap().get_root_addr() > ptr {
			entry = self.get_region_entry(ptr-REGION_SPLITTING);
		}
		
		//check next
		if entry.is_none() {
			return None;
		} else if entry.as_ref().unwrap().get_root_addr() > ptr {
			return None;
		}

		//check
		let entry = entry.unwrap();
		if entry.contain(ptr) {
			return Some(entry);
		} else {
			return None;
		}
	}

	/// Remove the registration at a given address.
	pub fn remove_from_ptr(&mut self,ptr: Addr) {
		let seg = self.get_segment(ptr);
		match seg {
			Some(seg) => self.remove_from_segment(seg),
			None => {} //TODO WARNING
		}
	}

	/// unregister the given segment.
	pub fn remove_from_segment(&mut self, segment: RegionSegmentPtr) {
		//check
		segment.sanity_check();

		//extract
		let ptr = segment.get_root_addr();
		let size = segment.get_total_size();

		//loop
		let mut offset = 0;
		while offset < size {
			self.unset_one_segment_entry(ptr + offset, segment.clone());
			offset += REGION_SPLITTING;
		}
	}

	/// Internage function to get the region corresponding to a given address.
	fn get_region(&self,ptr:Addr) -> Option<&Region> {
		let id = self.get_region_id(ptr);
		if self.regions.nolock_safe_read()[id].is_null() {
			None
		} else {
			Some(unsafe{& *(self.regions.nolock_safe_read()[id])})
		}
	}

	/// Simiar to get_region but create the regoin if not present.
	fn get_region_or_create(&mut self,ptr: Addr) -> &mut Region {
		let id = self.get_region_id(ptr);
		if self.regions.nolock_safe_read()[id].is_null() {
			self.setup_new_region(ptr)
		} else {
			unsafe{&mut *(self.regions.nolock_safe_read()[id] as * mut Region)}
		}
	}

	/// Create a new region for the given address.
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
			let regions = &mut *(self.regions.lock());
			//check if already mapped, otherwise, to nothing
			if regions[id].is_null() {
				// @todo this may be better to hardly control this address choice, maybe use the allocator when a first chain is available.
				let addr = osmem::mmap(0,mem::size_of::<Region>());
				let region = Region::new(addr,false);
				// @todo PARALLEL check for atomic operation instead of lock 
				regions[id] = region;
			}

			//TODO remove
			debug_assert!(!regions[id].is_null());

			//setup ret
			ret = regions[id];
		}

		//TODO remove
		debug_assert!(!self.get_region(ptr).is_none());

		//return pointer to the region
		unsafe {&mut *(ret as * mut Region)}
	}

	/// Compute the region ID for a given address.
	fn get_region_id(&self, addr: Addr) -> Size {
		//errors
		debug_assert!(addr != 0);
		debug_assert!(addr < PHYS_MAX_ADDR);
		
		//TODO can be optimize if we consider power of 2
		let id = addr / REGION_SIZE;
		debug_assert!(id < MAX_REGIONS);
		
		id
	}

	/// Unmap all the registred memory. This might be usefull for some unit tests.
	fn unmap_all_memory(&mut self) {
		let mut regions = self.regions.lock();
		
		for i in 0..MAX_REGIONS {
			if !regions[i].is_null() {
				let mut region = unsafe{&mut *(regions[i]  as * mut Region)};
				region.unmap_registered_memory();
				osmem::munmap(regions[i] as Addr,mem::size_of::<Region>());
				regions[i] = ptr::null();
			}
		}
	}
}

#[cfg(test)]
mod tests
{
	use registry::registry::*;
	use portability::osmem;
	use chunk::dummy::DummyChunkManager;
	use common::traits::ChunkManagerPtr;

	#[test]
	fn full_workflow_one_segment() {
		//manager
		let mut manager = DummyChunkManager::new();
		let pmanager: ChunkManagerPtr = ChunkManagerPtr::new_ref_mut(&mut manager);

		//setup segment
		let size = 3*1024*1024;
		let ptr = osmem::mmap(0,size);
		let seg = RegionSegment::new(ptr,size,Some(pmanager));

		//regitry
		let mut registry = RegionRegistry::new();
		registry.set_segment_entry(seg.clone());

		//check request before
		let ret = registry.get_segment(ptr-1);
		assert!(ret.is_none());

		//check request after
		let ret = registry.get_segment(ptr+size);
		assert!(ret.is_none());

		//first
		let ret = registry.get_segment(ptr);
		assert!(!ret.is_none());
		assert_eq!(ret.unwrap().get_root_addr(), ptr );

		//last
		let ret = registry.get_segment(ptr + size - 1);
		assert!(!ret.is_none());
		assert_eq!(ret.unwrap().get_root_addr(), ptr);

		//unregister
		registry.remove_from_segment(seg);
		osmem::munmap(ptr,size);

		//clear mem
		registry.unmap_all_memory();
	}

	#[test]
	fn full_workflow_overlap_left_before() {
		//manager
		let mut manager = DummyChunkManager::new();
		let pmanager: ChunkManagerPtr = ChunkManagerPtr::new_ref_mut(&mut manager);

		//setup segment 1
		let size = 5*1024*1024;
		let ptr = osmem::mmap(0,size);
		let seg1 = RegionSegment::new(ptr,size/2,Some(pmanager.clone()));
		let seg2 = RegionSegment::new(ptr+size/2,size/2,Some(pmanager));

		//registry
		let mut registry = RegionRegistry::new();
		registry.set_segment_entry(seg1.clone());
		registry.set_segment_entry(seg2.clone());

		//check request before
		let ret = registry.get_segment(ptr-1);
		assert!(ret.is_none());

		//check request after
		let ret = registry.get_segment(ptr+size);
		assert!(ret.is_none());

		//first
		for i in 0..size/2 {
			let ret = registry.get_segment(ptr+i);
			assert!(!ret.is_none());
			assert_eq!(ret.unwrap().get_root_addr(), ptr );
		}

		//second
		for i in 0..size/2 {
			let ret = registry.get_segment(ptr+size/2+i);
			assert!(!ret.is_none());
			assert_eq!(ret.unwrap().get_root_addr(), ptr+size/2 );
		}

		//unregister
		registry.remove_from_segment(seg1);
		registry.remove_from_segment(seg2);
		osmem::munmap(ptr,size);

		//clear mem
		registry.unmap_all_memory();
	}

	#[test]
	fn full_workflow_overlap_left_after() {
		//manager
		let mut manager = DummyChunkManager::new();
		let pmanager: ChunkManagerPtr = SharedPtrBox::new_ref_mut(&mut manager);

		//setup segment 1
		let size = 5*1024*1024;
		let ptr = osmem::mmap(0,size);
		let seg1 = RegionSegment::new(ptr,size/2,Some(pmanager.clone()));
		let seg2 = RegionSegment::new(ptr+size/2,size/2,Some(pmanager));

		//registry
		let mut registry = RegionRegistry::new();
		registry.set_segment_entry(seg2.clone());
		registry.set_segment_entry(seg1.clone());

		//check request before
		let ret = registry.get_segment(ptr-1);
		assert!(ret.is_none());

		//check request after
		let ret = registry.get_segment(ptr+size);
		assert!(ret.is_none());

		//first
		let step = 1;
		for i in 0..size/2/step {
			let ret = registry.get_segment(ptr+i*step);
			assert!(!ret.is_none());
			assert_eq!(ret.unwrap().get_root_addr(), ptr );
		}

		//second
		for i in 0..size/2/step {
			let ret = registry.get_segment(ptr+size/2+i*step);
			assert!(!ret.is_none());
			assert_eq!(ret.unwrap().get_root_addr(), ptr+size/2 );
		}

		//unregister
		registry.remove_from_segment(seg1);
		registry.remove_from_segment(seg2);
		osmem::munmap(ptr,size);

		//clear mem
		registry.unmap_all_memory();
	}
}