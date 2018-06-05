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
use portability::arch;

/// Provide the default list of size to be used to build segregated lists.
// CAUTION, IF YOU CHANGE THIS YOU NEED TO ADAPT reverse_default_free_sizes() OR
// SET USE FAST_REVERSE to false.
static FREE_LIST_SIZES: [Size;NB_FREE_LIST] = [16, 24,
        32,    64,   96,  128,  160,   192,   224,   256,    288,    320,
        352,  384,  416,  448,  480,   512,   544,   576,    608,    640,
        672,  704,  736,  768,  800,   832,   864,   896,    928,    960,
        992, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
        524288, 1048576, 2*1024*1024, Size::max_value(), Size::max_value(), Size::max_value(), Size::max_value(), Size::max_value()
];
/// If use default sizes we can use the fast reverse function instead of doing dichotomic search.
const FAST_REVERSE: bool = true;

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
	/// If enable fast reverse function of use dichotomic
	fast_reverse: bool,
	/// status of the list (free of not)
	status: [bool; NB_FREE_LIST],
	/// all lists.
	list: [ChunkFreeList; NB_FREE_LIST],
}

impl MediumFreePool {
	fn get_nb_list_from_array() -> usize {
		//check
		debug_assert!(FREE_LIST_SIZES.len() <= NB_FREE_LIST);

		//search end
		for i in 0..FREE_LIST_SIZES.len() {
			if FREE_LIST_SIZES[i] == Size::max_value() {
				return i;
			}
		}

		//error
		panic!("List is empty !");
	}

	pub fn new() -> Self {
		Self {
			nb_list: Self::get_nb_list_from_array(),
			sizes: FREE_LIST_SIZES,
			fast_reverse: FAST_REVERSE,
			status: [true; NB_FREE_LIST],
			list: [ChunkFreeList::new(); NB_FREE_LIST],
		}
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

	fn reverse_default_free_sizes(&self,size:Size) -> usize {
		//errors
		debug_assert!(self.fast_reverse);
		debug_assert!(64 >> 5 == 2);
		debug_assert!(self.sizes[45] == Size::max_value());
		debug_assert!(size >= 16);

		if size < 32 {
			return (size / 8) - 2;
		} else if size <= 1024 {
			//divide by 32 and fix first element ID as we start to indexes by 0
			// +2 for thre startpoint 16/24
			return ((size >> 5) - 1) + 2;
		} else if size > MACRO_BLOC_SIZE {
			// +2 for thre startpoint 16/24
			return 43 + 2;
		} else {
			//1024/32 :  starting offset of the exp zone
			// >> 10: ( - log2(1024)) remote the start of the exp
			// +2 for thre startpoint 16/24
			return 1024/32 + arch::fast_log_2(size >> 10) - 1 + 2;
		}
	}
}