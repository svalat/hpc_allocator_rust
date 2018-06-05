/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module implement the medium free pools to track free segment in
/// segregated free list. It uses the MediumChunk to handle chunks and merge/split
/// them and use a link list to store them in free lists.
/// This is used to implement the MediumChunkManager.

//import
use chunk::medium::chunk::{MediumChunk,MediumChunkPtr};
use common::types::{Size,Addr};
use common::list::List;
use common::consts::*;

/// How to insert chunks
enum ChunkInsertMode {
	/// Insert such a way we take it out first
	FIFO,
	/// Insert such a way we take it out last
	LIFO,
}

/// Define a chunk free list.
type ChunkFreeList = List<MediumChunk>;

/// Define a medium chunk pool with multiple free list
/// segregated by size class.
pub struct MediumFreePool {
	/// Current number of list in use.
	nb_list: usize,
	/// List of size class to attach the lists.
	sizes: [Size; NB_FREE_LIST],
	/// status of the list (free of not)
	status: [bool; NB_FREE_LIST],
	/// all lists.
	list: [ChunkFreeList; NB_FREE_LIST],
}

impl MediumFreePool {
	pub fn new() -> Self {
		panic!("TODO");
	}

	pub fn insert_addr(&mut self,ptr: Addr, size: Size, mode: ChunkInsertMode) {
		panic!("TODO");
	}

	pub fn insert_chunk(&mut self, chunk: MediumChunkPtr, mode: ChunkInsertMode) {
		panic!("TODO");
	}

	pub fn remove(&mut self, chunk: MediumChunkPtr) {
		panic!("TODO");
	}

	pub fn find_chunk(&mut self, inner_size: Size) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}

	pub fn merge(&mut self, chunk: MediumChunkPtr) -> MediumChunkPtr {
		panic!("TODO");
	}

	pub fn try_merge_for_size(&mut self, chunk: MediumChunkPtr, find_inner_size: Size) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}

	pub fn hard_checking(&self) {
		panic!("TODO");
	}

	fn get_free_list(&mut self, inner_size: Size) -> &mut ChunkFreeList {
		panic!("TODO");
	}

	fn get_free_list_by_dichotomy(&mut self, inner_size: Size) -> &mut ChunkFreeList {
		panic!("TODO");
	}

	fn get_free_list_by_analytic(&mut self, inner_size: Size) -> &mut ChunkFreeList {
		panic!("TODO");
	}

	fn get_list_class(&self, list:&ChunkFreeList) -> Size {
		panic!("TODO");
	}

	fn set_empty_status(&mut self, list:&ChunkFreeList, filled: bool) {
		panic!("TODO");
	}

	fn find_adapted_chunk(&mut self, list:&ChunkFreeList, total_size: Size) -> Option<MediumChunkPtr> {
		panic!("TODO");
	}

	fn get_first_next_non_empty_list(&mut self, list:&ChunkFreeList) -> Option<&mut ChunkFreeList> {
		panic!("TODO");
	}
}