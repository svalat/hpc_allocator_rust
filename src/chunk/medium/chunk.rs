/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// Implement the medium chunk headers and management help functions.

//import
use common::shared::SharedPtrBox;
use common::types::{Addr,Size};
use common::list::{ListNode,Listable};

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
	pub fn setup_size(ptr: Addr, total_size: Size, prev: Option<MediumChunkPtr>) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn setup_prev_next(ptr: Addr,prev: Option<MediumChunkPtr>,next: MediumChunkPtr) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn get_chunk(ptr: Addr) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn get_chunk_sage(ptr: Addr) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn get_total_size(&self) -> Size {
		panic!("TODO");
	}

	pub fn get_innter_size(&self) -> Size {
		panic!("TODO");
	}

	pub fn check(&self) {
		panic!("TODO");
	}

	pub fn get_next(&self) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}

	pub fn get_prev(&self) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}

	pub fn get_status(&self) -> ChunkStatus {
		panic!("TODO");
	}

	pub fn set_status(&mut self,status: ChunkStatus) {
		panic!("TODO");
	}

	pub fn split(&mut self, inner_size: Size) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn is_single(&self) -> bool {
		panic!("TODO");
	}

	pub fn get_ptr(&self) -> Addr {
		panic!("TODO");
	}

	pub fn merge(&mut self, last: MediumChunkPtr) {
		panic!("TODO");
	}

	pub fn contain(&self, ptr: Addr) -> bool {
		panic!("TODO");
	}
}

impl Listable<MediumChunk> for MediumChunk {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
		panic!("TODO");
	}

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
		panic!("TODO");
	}

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const Self {
		panic!("TODO");
	}

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut Self {
		panic!("TODO");
	}
}

#[cfg(test)]
mod tests
{
	use chunk::medium::chunk::*;
	use core::mem;
	use common::types::*;

	#[test]
	fn struct_size() {
		assert_eq!(mem::size_of::<MediumChunk>(), 3*mem::size_of::<Size>());
	}
}