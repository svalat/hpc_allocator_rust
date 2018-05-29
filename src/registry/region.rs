/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///The address space is split into regions which are huge (1TB) we then put pointers
///to map the segments into this region.

//import
extern crate libc;

//import
use common::consts::*;
use registry::segment::RegionSegment;
use common::types::{Addr,Size};
use core::ptr;
use portability::osmem;

///Define a region entry which is now just a pointer to a segment
pub type RegionEntry = * const RegionSegment;

///Define a region which is mainly an array of entries and some basic operation.
pub struct Region
{
	//void clear(void);
	//bool isEmpty(void) const;
	//void unmapRegisteredMemory(void);
	entries: [RegionEntry; REGION_ENTRIES],
}

//implement
impl Region {
	///create a new region from a pointer and return a pointer to region
	///
	///**ptr**: Base address where to put the region
	///**clear**: Say if we need to explicitly clear the memory content. 
	///If allocate with mmap directly, not needed and it save physical memory.
	pub fn new(ptr: Addr,clear:bool) -> * mut Self {
		//check
		assert!(ptr != 0);

		//cast
		let regptr = ptr as * mut Region;

		//clear
		if clear {
			unsafe{(*regptr).clear()};
		}

		regptr
	}

	///explicitly clear the pointers. Not needed if originaly allocated by mmap.
	pub fn clear(self: &mut Self) {
		for i in 0..REGION_ENTRIES {
			self.entries[i] = ptr::null();
		}
	}

	///Check if the region contain segments or not.
	pub fn is_empty(self: &Self) -> bool {
		let mut ret = true;

		for i in 0..REGION_ENTRIES {
			if !self.entries[i].is_null() {
				ret = false;
			}
		}

		ret
	}

	///unmap all the registred regions (more for unit tests)
	pub fn unmap_registered_memory(self: &mut Self) {
		let mut last: RegionEntry = ptr::null();

		//loop on all segments to free them
		for i in 0..REGION_ENTRIES {
			if self.entries[i] != ptr::null() && self.entries[i] != last {
				let entry = unsafe{&*self.entries[i]};
				osmem::munmap(entry.get_root_addr(),entry.get_total_size());
			}
			last = self.entries[i];
			self.entries[i] = ptr::null();
		}
	}

	///register a segment into the region.
	///
	///**id** id of the entry to setup.
	///**entry** value to setup.
	#[inline]
	pub fn set(self: &mut Self,id:Size,entry:RegionEntry) {
		debug_assert!(id < REGION_ENTRIES);
		self.entries[id] = entry;
	}

	///unregister a segment into the region.
	///
	///**id** id of the entry to unset.
	#[inline]
	pub fn unset(self: &mut Self,id:Size) {
		debug_assert!(id < REGION_ENTRIES);
		self.entries[id] = ptr::null();
	}

	///return the requested entry in the region.
	#[inline]
	pub fn get(self: &Self,id:Size) -> RegionEntry {
		debug_assert!(id < REGION_ENTRIES);
		self.entries[id]
	}
}

#[cfg(test)]
mod tests
{
	use common::consts::*;
	use registry::region::*;
	use registry::segment::*;
	use common::traits::*;
	use core::mem;
	use portability::osmem;
	use chunk::dummy::DummyChunkManager;

	#[test]
	fn region_entry_size() {
		assert_eq!(mem::size_of::<RegionEntry>(), 8);
	}

	#[test]
	fn region_entries() {
		assert_eq!(REGION_ENTRIES,524288);
	}

	#[test]
	fn clear() {
		let ptr = osmem::mmap(0,1024*4096);
		let region = Region::new(ptr,false);
		unsafe{(*region).clear()};
		osmem::munmap(ptr,1024*4096);
	}

	#[test]
	fn is_empty() {
		let ptr = osmem::mmap(0,1024*4096);
		let region = Region::new(ptr,false);
		let region = unsafe{&mut *region};

		let mut manager = DummyChunkManager::new();
		let pmanager = &mut manager as *mut ChunkManager;

		let ptr2 = osmem::mmap(0,1024*4096);
		let seg = RegionSegment::new(ptr2,1024*4096,pmanager);
		
		assert_eq!(region.is_empty(),true);
		region.set(10,&seg as RegionEntry);
		assert_eq!(region.is_empty(),false);
		
		osmem::munmap(ptr,1024*4096);
		osmem::munmap(ptr2,1024*4096);
	}

	#[test]
	fn unmap_registered_memory() {
		let ptr = osmem::mmap(0,1024*4096);
		let region = Region::new(ptr,false);
		let region = unsafe{&mut *region};

		let mut manager = DummyChunkManager::new();
		let pmanager = &mut manager as *mut ChunkManager;

		let ptr2 = osmem::mmap(0,1024*4096);
		let seg = RegionSegment::new(ptr2,1024*4096,pmanager);
		region.set(10,&seg as RegionEntry);
		region.set(11,&seg as RegionEntry);

		region.unmap_registered_memory();

		osmem::munmap(ptr,1024*4096);
	}
}