/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///Implement the segment description.

//import
use common::types::{Size,Addr};
use common::traits::{ChunkManager};
use common::consts::*;
use core::mem;

///A region is a segment of the memory of a size at least 
///MACRO_BLOC_SIZE, it is used to be handled by the 
///memory source and registred into the region registry.
///It is handled by a chunk manager
#[derive(Copy,Clone)]
pub struct RegionSegment
{
	///Base address, this eat 8 bytes but permit to copy the struct instead of having to handle
	///unsafe pointer everywhere
	base: Addr,
	///Keep track of the size of the segement
	size: Size,
	///pointer to the chunk manager to handle its content
	manager: * mut ChunkManager,
}

//Implementation
impl RegionSegment {
	///Construct a region segment
	///
	/// **ptr**: Base address of the segment, should ideally be aligned on page size.
	/// **total_size**: Total size of the segment. Header will be added on start so inner content size will be lower.
	/// **manager**: Pointer to the chunk manager to manage chunks inside the segment.
	pub fn new(ptr: Addr,total_size: Size, manager: *mut ChunkManager) -> RegionSegment {
		//check
		debug_assert!(ptr % SMALL_PAGE_SIZE == 0);

		//cast address into struct ref
		let regptr = ptr as * mut RegionSegment;

		//fill
		let mut region = unsafe{ *regptr };
		region.base = ptr;
		region.size = total_size;
		region.manager = manager;

		//return
		region
	}

	///Return a segment from address.
	pub fn get_segment(ptr: Addr) -> RegionSegment {
		//check
		debug_assert!(ptr != 0);
		debug_assert!(ptr % SMALL_PAGE_SIZE == 0);

		//convert
		let regptr = ptr as * mut RegionSegment;
		let region = unsafe{ *regptr };

		//ret
		region
	}

	///Make some sanity check of content to help debugging and quickly find issues.
	#[inline]
	fn sanity_check(self: &Self) {
		//check
		debug_assert!(self.base != 0);
		debug_assert!(self.base % SMALL_PAGE_SIZE == 0);
		debug_assert!(self.size != 0);
		debug_assert!(self.size % SMALL_PAGE_SIZE == 0);
	}

	///Update manager
	pub fn set_manager(self:&mut Self,manager: *mut ChunkManager) {
		//check
		self.sanity_check();
		debug_assert!(self.manager.is_null() || manager.is_null() || self.manager == manager);

		//setup
		self.manager = manager;
	}

	//return the base addr
	#[inline]
	pub fn get_root_addr(self:&Self) -> Addr {
		//check
		self.sanity_check();
		
		self.base
	}

	///Return base address to store content (base segment address with header offset).
	pub fn get_content_addr(self:&Self) -> Addr {
		//check
		self.sanity_check();

		//ret
		self.base + mem::size_of::<RegionSegment>()
	}

	///Check if the segemnt contain the given address.
	#[inline]
	pub fn contain(self:&Self,addr:Addr) -> bool {
		//check
		self.sanity_check();

		//test
		addr >= self.base && addr < self.base + self.size
	}

	///Return the total size of the segement.
	#[inline]
	pub fn get_total_size(self: &Self) -> Size {
		//check
		self.sanity_check();

		//ret
		self.size
	}

	///Return the inner size of the segment (total size minus headers).
	#[inline]
	pub fn get_inner_size(self: &Self) -> Size {
		//check
		self.sanity_check();

		//ret
		self.size - mem::size_of::<RegionSegment>()
	}

	///Return manager in safe way
	#[inline]
	pub fn get_manager(self: &Self) -> Option<&ChunkManager> {
		//check
		self.sanity_check();

		//switch
		if self.manager.is_null() {
			None
		} else {
			Some(unsafe{&*self.manager})
		}
	}

	///Retuen mutable manager in safe way
	#[inline]
	pub fn get_manager_mut(self: & Self) -> Option<&mut ChunkManager> {
		//check
		self.sanity_check();

		//switch
		if self.manager.is_null() {
			None
		} else {
			Some(unsafe{&mut *self.manager})
		}
	}
}

#[cfg(test)]
mod tests
{
	use registry::segment::*;
	use core::mem;
	use portability::osmem;
	use chunk::dummy::DummyChunkManager;

	#[test]
	fn struct_size() {
		assert_eq!(mem::size_of::<RegionSegment>(), 32);
	}

	#[test]
	fn new() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let _reg = RegionSegment::new(ptr,4*4096,pmanager);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_segment() {
		let ptr = osmem::mmap(0,4*4096);
		let _reg = RegionSegment::get_segment(ptr);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn set_manager() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let mut reg = RegionSegment::new(ptr,4*4096,pmanager);
		reg.set_manager(pmanager);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_content_addr() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		let addr = reg.get_content_addr();
		assert_eq!(addr,ptr+32);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn contain() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		assert_eq!(reg.contain(ptr),true);
		assert_eq!(reg.contain(ptr+4*4096-1),true);
		assert_eq!(reg.contain(ptr+4*4096),false);
		assert_eq!(reg.contain(ptr-1),false);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_total_size() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		assert_eq!(reg.get_total_size(),4*4096);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_inner_size() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		assert_eq!(reg.get_inner_size(),4*4096-32);
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_manager() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		reg.get_manager().unwrap();
		osmem::munmap(ptr,4*4096);
	}

	#[test]
	fn get_manager_mut() {
		let ptr = osmem::mmap(0,4*4096);
		//TODO replace by MOCK
		let mut manager = DummyChunkManager{};
		let pmanager = &mut manager as *mut ChunkManager;
		let reg = RegionSegment::new(ptr,4*4096,pmanager);
		reg.get_manager_mut().unwrap();
		osmem::munmap(ptr,4*4096);
	}
}