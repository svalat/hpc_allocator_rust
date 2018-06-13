/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// Implement the medium chunk headers and management help functions.

//import
use common::consts::*;
use common::shared::SharedPtrBox;
use common::types::{Addr,Size};
use common::list::{ListNode,Listable};
use common::ops;
use core::mem;

/// To be used to annotate chunk as free
pub const CHUNK_FREE:u32 = 0;

/// To be used to annotate chunk as allocated
pub const CHUNK_ALLOCATED:u32 = 1;

/// To be used to store chunk status (FREE or ALLOCATED)
type ChunkStatus = u32;
pub type MediumChunkPtr = SharedPtrBox<MediumChunk>;

/// Define a medium chunk by its header. Medium chunk are chained in memory
/// such as next and prev chunk are contiguous to current one to be efficiently
/// merged if possible.
pub struct MediumChunk {
	/// Pointer to next contiguous chunk of NULL
	prev: MediumChunkPtr,
	/// Pointer to previous chunk contiguous chunk of NULL
	next: MediumChunkPtr,
	/// Status of chunk (FREE or ALLOCATED)
	status: ChunkStatus,
	/// Mafick number for checking/asserting.
	magick: u32,
}

//implement
impl MediumChunk {
	/// Internal function to setup a MediumChunk in place from a given addres.
	/// It set it up with no next/prev (so NULL).
	/// By default the chunk is marked as ALLOCATED.
	/// 
	/// As it setup the content in place it return a SharedPtrBox pointer to the
	/// ptr memory.
	fn setup(ptr: Addr) -> MediumChunkPtr {
		//checks
		debug_assert!(ptr != 0);
		debug_assert!(ptr % mem::size_of::<Addr>() == 0);

		//create
		let mut res = MediumChunkPtr::new_addr(ptr);
		
		//we set after so can ignore but for safety in debug
		#[cfg(build = "debug")]
		{
			res.prev.set_null();
			res.next.set_null();
		}
		
		//infos
		res.status = CHUNK_ALLOCATED;
		res.magick = MAGICK_VALUE;
		
		//ret
		res
	}

	/// Setup in place a new chunk from a given pointer and a given total size
	/// It will build the header and return a pointer to it.
	pub fn setup_size(ptr: Addr, total_size: Size) -> MediumChunkPtr {
		//errors
		debug_assert!(total_size >= Self::header_size() + BASIC_ALIGN);
		debug_assert!(total_size % BASIC_ALIGN == 0);
		debug_assert!(total_size >= Self::header_size() + mem::size_of::<ListNode>());
		
		//setup first bloc
		let res = Self::setup_prev(ptr,total_size - Self::header_size(),None);
		
		//setup close block
		Self::setup_prev(res.next.get_addr(),0,Some(res.clone()));
		
		res
	}

	/// Setup in place a new chunk header with a given total size and setup
	/// the prev pointer to the given optional prev parameter.
	pub fn setup_prev(ptr: Addr, total_size: Size,prev: Option<MediumChunkPtr>) -> MediumChunkPtr {
		//check
		debug_assert!(total_size == 0 || total_size >= Self::header_size() + BASIC_ALIGN);
		
		//locl
		let mut res = Self::setup(ptr);

		//prev
		match prev {
			Some(prev) => {
				debug_assert!(prev.get_addr() < ptr);
				res.prev = prev;
			},
			None => {
				res.prev.set_null();
			},
		}

		//next
		if total_size == 0 {
			res.next.set_null();
		} else {
			res.next = Self::setup(ptr + total_size);
		}
		
		return res;
	}

	/// Setup in place a new chunk at the given address with the given prev and next chunks.
	pub fn setup_prev_next(ptr: Addr,prev: Option<MediumChunkPtr>,next: MediumChunkPtr) -> MediumChunkPtr {
		//errors
		debug_assert!(next.get_addr() > ptr || next.is_null());

		//generic setup
		let mut res = Self::setup(ptr);
		
		//prev
		match prev {
			Some(prev) => {
				debug_assert!(prev.get_addr() < ptr);
				res.prev = prev;
			},
			None => res.prev.set_null(),
		}
		
		//next
		res.next = next;
		
		return res;
	}

	/// Return the header size to avoid putting mem::size_of() everywhere.
	#[inline]
	pub fn header_size() -> Size {
		mem::size_of::<MediumChunk>()
	}

	/// Return the base address (header address) of the current chunk.
	#[inline]
	pub fn get_root_addr(&self) -> Addr {
		self as * const Self as Addr
	}

	/// Return an optional pointer to a chunk from a content pointer. It will 
	/// decrement the address by the header size and return.
	/// 
	/// If ptr is null, then the option is setup to None.
	#[inline]
	pub fn get_chunk(ptr: Addr) -> Option<MediumChunkPtr> {
		if ptr == 0 {
			None
		} else {
			Some(MediumChunkPtr::new_addr(ptr - Self::header_size()))
		}
	}

	/// Return an optional pointer to a chunk from a content pointer. It will 
	/// decrement the address by the header size and return.
	/// 
	/// If ptr is null, then the option is setup to None.
	/// 
	/// It will also safe check the extracted chunk in debug mode.
	#[inline]
	pub fn get_chunk_safe(ptr: Addr) -> Option<MediumChunkPtr> {
		if ptr == 0 {
			None
		} else {
			let tmp = MediumChunkPtr::new_addr(ptr - Self::header_size());
			tmp.get().check();
			Some(tmp)
		}
	}

	/// Get the total size of the chunk (hread + content)
	#[inline]
	pub fn get_total_size(&self) -> Size {
		if self.next.is_null() {
			Self::header_size()
		} else {
			self.next.get_addr() - self.get_root_addr()
		}
	}

	/// Return the inner size of the chunk (content size).
	#[inline]
	pub fn get_inner_size(&self) -> Size {
		if self.next.is_null() {
			0
		} else {
			self.next.get_addr() - self.get_root_addr() - Self::header_size()
		}
	}

	/// Check all properties of the chunk with expected constains to check if
	/// they all match. 
	/// This use debug_assert!() so it will do nothing in release mode.
	/// And be inline so in theory has zero cost in release.
	#[inline]
	pub fn check(&self) {
		debug_assert!(self.get_inner_size() >= mem::size_of::<ListNode>());
		debug_assert!(self.get_root_addr() != 0);
		debug_assert!(self.status == CHUNK_FREE || self.status == CHUNK_ALLOCATED);
		debug_assert!(self.magick == MAGICK_VALUE);
		if !self.prev.is_null() {
			debug_assert!(self.prev.next.get_addr() == self.get_root_addr());
		}
		if !self.next.is_null() {
			debug_assert!(self.next.prev.get_addr() == self.get_root_addr());
		}
	}

	/// Optionally return the pointer to next chunk.
	#[inline]
	pub fn get_next(&self) -> Option<MediumChunkPtr> {
		if self.next.is_null() {
			None
		} else {
			Some(self.next.clone())
		}
	}

	/// Optionally return the pointer to previous chunk.
	#[inline]
	pub fn get_prev(&self) -> Option<MediumChunkPtr> {
		if self.prev.is_null() {
			None
		} else {
			Some(self.prev.clone())
		}
	}

	/// Return allocation status of the chunk.
	#[inline]
	pub fn get_status(&self) -> ChunkStatus {
		debug_assert!(self.status == CHUNK_FREE || self.status == CHUNK_ALLOCATED);
		self.status
	}

	/// Change allocation status of the chunk.
	#[inline]
	pub fn set_status(&mut self,status: ChunkStatus) {
		debug_assert!(status == CHUNK_FREE || status == CHUNK_ALLOCATED);
		self.status = status;
	}

	/// Split the chunk at the given inner (contant) size.
	/// If size if too big then return None.
	pub fn split(&mut self, inner_size: Size) -> Option<MediumChunkPtr> {
		//round size to multiple of 8
		let total_size = ops::up_to_power_of_2(inner_size,BASIC_ALIGN) + Self::header_size();
		
		//check size
		if total_size > self.get_inner_size() {
			return None;
		}
		
		//split
		let chunk = Self::setup_prev_next(self.get_root_addr()+total_size,Some(MediumChunkPtr::new_ref(self)),self.next.clone());
		
		//update
		if !self.next.is_null() {
			self.next.prev = chunk.clone();
		}
		self.next = chunk.clone();
		
		//debug
		self.check();
		chunk.check();
		
		//ret
		Some(chunk)
	}

	/// Check if the chunk is not linked to any other chunks.
	/// prev/next is NULL in other words.
	pub fn is_single(&self) -> bool {
		debug_assert!(self.get_root_addr() != 0);
		if self.prev.is_null() {
			if self.next.is_null() {
				true
			} else {
				self.next.get_inner_size() == 0
			}
		} else {
			false
		}
	}

	/// Get base address of data content in the chunk (base address + header).
	pub fn get_content_addr(&self) -> Addr {
		self.get_root_addr() + Self::header_size()
	}

	/// Merge all next chunk until the given one.
	/// It will merge on the current chunk by updating its
	/// next entry.
	pub fn merge(&mut self, last: MediumChunkPtr) {
		//errors
		self.check();
		last.check();
		debug_assert!(last.get_addr() >= self.get_root_addr());

		//extract
		let mut first = MediumChunkPtr::new_ref(self);
		if first.get_addr() == last.get_addr() {
			return;
		}
		
		//merge
		let mut last = last.clone();
		if !last.next.is_null() {
			last.next.prev = first.clone();
		}
		
		first.next = last.next.clone();
	}

	/// Check if the chunk contain (in the inner content part) the given
	/// address.
	pub fn contain(&self, ptr: Addr) -> bool {
		let base = self.get_content_addr();
		let end = base + self.get_inner_size();
		ptr >= base && ptr < end
	}

	/// Init the content as ListNode to put the chunk into a free list.
	pub fn setup_as_listable(&mut self) {
		let node = unsafe{&mut *(self.get_content_addr() as * mut ListNode)};
		*node = ListNode::new();
	}
}

impl Listable<MediumChunk> for MediumChunk {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
		let addr = self.get_content_addr();
		//debug_assert!(addr % mem::size_of::<ListNode>() == 0);
		unsafe{&*(addr as * const ListNode)}
	}

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
		let addr = self.get_content_addr();
		//debug_assert!(addr % mem::size_of::<ListNode>() == 0);
		unsafe{&mut *(addr as * mut ListNode)}
	}

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const Self {
		((elmt as Addr) - Self::header_size()) as * const Self
	}

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut Self {
		((elmt as Addr) - Self::header_size()) as * mut Self
	}
}

#[cfg(test)]
mod tests
{
	use chunk::medium::chunk::*;
	use portability::osmem;
	use core::mem;

	#[test]
	fn struct_size() {
		//size
		assert_eq!(MediumChunk::header_size(), mem::size_of::<MediumChunk>());
		assert_eq!(MediumChunk::header_size(), 3*mem::size_of::<Size>());
	}

	#[test]
	fn setup_size() {
		let ptr = osmem::mmap(0,4096);

		let chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get();

		assert_eq!(chunk.get_total_size(), 4096 - MediumChunk::header_size());
		assert_eq!(chunk.get_inner_size(), 4096 - 2* MediumChunk::header_size());

		assert_eq!(chunk.get_next().unwrap().get().get_inner_size(), 0);
		assert_eq!(chunk.get_prev().is_none(),true);

		assert_eq!(chunk.is_single(), true);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn get_root_addr() {
		let ptr = osmem::mmap(0,4096);
		let chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get();
		assert_eq!(chunk.get_root_addr(), ptr);
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn get_chunk() {
		let ptr = osmem::mmap(0,4096);
		let chunk = MediumChunk::setup_size(ptr, 4096);
		
		let chunk2 = MediumChunk::get_chunk(chunk.get().get_content_addr());
		assert_eq!(chunk2.unwrap().get_addr(), ptr);

		let chunk3 = MediumChunk::get_chunk(0);
		assert_eq!(chunk3.is_none(), true);
		
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn get_chunk_safe() {
		let ptr = osmem::mmap(0,4096);
		let chunk = MediumChunk::setup_size(ptr, 4096);
		
		let chunk2 = MediumChunk::get_chunk_safe(chunk.get().get_content_addr());
		assert_eq!(chunk2.unwrap().get_addr(), ptr);

		let chunk3 = MediumChunk::get_chunk_safe(0);
		assert_eq!(chunk3.is_none(), true);
		
		osmem::munmap(ptr, 4096);
	}

	#[cfg(debug_assertions)]
	#[test]
	#[should_panic]
	fn get_chunk_safe_panic() {
		let ptr = osmem::mmap(0,4096);
	
		let chunk2 = MediumChunk::get_chunk_safe(ptr+MediumChunk::header_size());
		assert_eq!(chunk2.unwrap().get_addr(), ptr);

		osmem::munmap(ptr, 4096);
	}

	#[cfg(not(debug_assertions))]
	#[test]
	fn get_chunk_safe_panic() {
		let ptr = osmem::mmap(0,4096);
	
		let chunk2 = MediumChunk::get_chunk_safe(ptr+MediumChunk::header_size());
		assert_eq!(chunk2.unwrap().get_addr(), ptr);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn status() {
		let ptr = osmem::mmap(0,4096);

		let mut chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get_mut();

		assert_eq!(chunk.get_status(), CHUNK_ALLOCATED);

		chunk.set_status(CHUNK_FREE);

		assert_eq!(chunk.get_status(), CHUNK_FREE);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn split_no() {
		let ptr = osmem::mmap(0,4096);

		let mut chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get_mut();

		let residut = chunk.split(4096 - MediumChunk::header_size());

		assert_eq!(residut.is_none(), true);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn split_ok() {
		let ptr = osmem::mmap(0,4096);

		let mut chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get_mut();

		let residut_tmp = chunk.split(1024);
		let residut = residut_tmp.as_ref().unwrap().get();

		residut.check();

		assert_eq!(chunk.get_total_size(), 1024 + MediumChunk::header_size());
		assert_eq!(chunk.get_inner_size(), 1024);

		assert_eq!(residut.get_total_size(), 4096 - 1024 - 2*MediumChunk::header_size());
		assert_eq!(residut.get_inner_size(), 4096 - 1024 - 3*MediumChunk::header_size());

		assert_eq!(chunk.get_next().unwrap().get().get_root_addr(), residut.get_root_addr());
		assert_eq!(chunk.get_prev().is_none(),true);

		assert_eq!(residut.get_next().unwrap().get().get_inner_size(),0);
		assert_eq!(residut.get_prev().unwrap().get_addr(), chunk.get_root_addr());

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn merge() {
		let ptr = osmem::mmap(0,4096);

		let mut chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get_mut();

		let mut residut_tmp1 = chunk.split(512);
		let residut1 = residut_tmp1.as_mut().unwrap().get_mut();

		let mut residut_tmp2 = residut1.split(512);
		let residut2 = residut_tmp2.as_mut().unwrap().get_mut();

		let residut_tmp3 = residut2.split(512);
		let residut3 = residut_tmp3.as_ref().unwrap();

		chunk.merge(residut3.clone());

		assert_eq!(chunk.get_total_size(), 4096 - MediumChunk::header_size());
		assert_eq!(chunk.get_inner_size(), 4096 - 2*MediumChunk::header_size());

		assert_eq!(chunk.get_next().unwrap().get().get_inner_size(), 0);
		assert_eq!(chunk.get_prev().is_none(),true);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn contain() {
		let ptr = osmem::mmap(0,4096);

		let chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get();

		assert_eq!(chunk.contain(ptr), false);
		assert_eq!(chunk.contain(ptr+MediumChunk::header_size()), true);
		assert_eq!(chunk.contain(ptr+4096-MediumChunk::header_size()-1), true);
		assert_eq!(chunk.contain(ptr+4096-MediumChunk::header_size()), false);

		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn list_node() {
		let ptr = osmem::mmap(0,4096);

		let mut chunk = MediumChunk::setup_size(ptr, 4096);
		let chunk = chunk.get_mut();

		chunk.setup_as_listable();

		let node = MediumChunk::get_list_node(chunk);

		assert_eq!(node.is_none(),true);

		osmem::munmap(ptr, 4096);
	}
}