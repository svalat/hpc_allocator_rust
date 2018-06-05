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
const CHUNK_FREE:u32 = 0;

/// To be used to annotate chunk as allocated
const CHUNK_ALLOCATED:u32 = 1;

/// To be used to store chunk status (FREE or ALLOCATED)
type ChunkStatus = u32;
type MediumChunkPtr = SharedPtrBox<MediumChunk>;

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

	#[inline]
	fn header_size() -> Size {
		mem::size_of::<MediumChunk>()
	}

	#[inline]
	fn get_root_addr(&self) -> Addr {
		self as * const Self as Addr
	}

	#[inline]
	pub fn get_chunk(ptr: Addr) -> Option<MediumChunkPtr> {
		if ptr == 0 {
			None
		} else {
			Some(MediumChunkPtr::new_addr(ptr - Self::header_size()))
		}
	}

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

	#[inline]
	pub fn get_total_size(&self) -> Size {
		if self.next.is_null() {
			Self::header_size()
		} else {
			self.next.get_addr() - self.get_root_addr()
		}
	}

	#[inline]
	pub fn get_inner_size(&self) -> Size {
		if self.next.is_null() {
			0
		} else {
			self.next.get_addr() - self.get_root_addr() - Self::header_size()
		}
	}

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

	#[inline]
	pub fn get_next(&self) -> Option<MediumChunkPtr> {
		if self.next.is_null() {
			None
		} else {
			Some(self.next.clone())
		}
	}

	#[inline]
	pub fn get_prev(&self) -> Option<MediumChunkPtr> {
		if self.prev.is_null() {
			None
		} else {
			Some(self.next.clone())
		}
	}

	#[inline]
	pub fn get_status(&self) -> ChunkStatus {
		debug_assert!(self.status == CHUNK_FREE || self.status == CHUNK_ALLOCATED);
		self.status
	}

	#[inline]
	pub fn set_status(&mut self,status: ChunkStatus) {
		debug_assert!(status == CHUNK_FREE || status == CHUNK_ALLOCATED);
		self.status = status;
	}

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

	pub fn is_single(&self) -> bool {
		debug_assert!(self.get_root_addr() != 0);
		self.prev.is_null() && self.next.is_null()
	}

	pub fn get_content_addr(&self) -> Addr {
		self.get_root_addr() + Self::header_size()
	}

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

	pub fn contain(&self, ptr: Addr) -> bool {
		let base = self.get_content_addr();
		let end = base + self.get_inner_size();
		ptr >= base && ptr < end
	}
}

impl Listable<MediumChunk> for MediumChunk {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
		unsafe{&*(self.get_content_addr() as * const ListNode)}
	}

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
		unsafe{&mut *(self.get_content_addr() as * mut ListNode)}
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
	use core::mem;

	#[test]
	fn struct_size() {
		//size
		assert_eq!(MediumChunk::header_size(), mem::size_of::<MediumChunk>());
		assert_eq!(MediumChunk::header_size(), 3*mem::size_of::<Size>());
	}
}