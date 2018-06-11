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
use chunk::medium::chunk::{MediumChunk,MediumChunkPtr, CHUNK_ALLOCATED, CHUNK_FREE};
use common::types::{Size,Addr};
use common::list::{List,ListNode};
use common::consts::*;
use portability::arch;
use core::mem;

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
pub enum ChunkInsertMode {
	/// Insert such a way we take it out first
	FIFO,
	/// Insert such a way we take it out last
	LIFO,
}

/// Define a chunk free list.
type ChunkFreeList = List<MediumChunk>;
type ChunkFreeListId = usize;

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
	lists: [ChunkFreeList; NB_FREE_LIST],
}

impl MediumFreePool {
	/// Compute the size of the list by search the end element containng usize::MAX.
	/// Also it ensure to have a size number mulitple of 2 to fit with dychotomic algorithm.
	fn get_nb_list_from_array(list: &[Size; NB_FREE_LIST]) -> usize {
		//check
		debug_assert!(FREE_LIST_SIZES.len() <= NB_FREE_LIST);

		//search end
		let mut size = 0;
		for i in 0..list.len() {
			if list[i] == Size::max_value() {
				size = i;
				break;
			}
		}

		//we keep last Size::Max to store all entries bigger than X
		//We need also to keep multiple of 2 for dychotomic search.
		if size % 2 == 0 {
			size + 2
		} else {
			size + 1
		}
	}

	/// Build a new free pool by using the default free list size classes.
	pub fn new() -> Self {
		Self {
			nb_list: Self::get_nb_list_from_array(&FREE_LIST_SIZES),
			sizes: FREE_LIST_SIZES,
			fast_reverse: FAST_REVERSE,
			status: [false; NB_FREE_LIST],
			lists: [ChunkFreeList::new(); NB_FREE_LIST],
		}
	}

	/// Build a new free pool by using a custom free list size classes. Notice
	/// it disable the fast reverse computation to get class from size
	/// which make fallback to dychotomic approach.
	/// 
	/// You should take care about the size of your list which should be
	/// mulitple of 2. You can fill the end with multiple usize::MAX.
	///
	/// Notice you also need at least one usize::MAX element. 
	pub fn new_cust_list(list: &[Size; NB_FREE_LIST]) -> Self {
		Self {
			nb_list: Self::get_nb_list_from_array(list),
			sizes: *list,
			fast_reverse: false,
			status: [false; NB_FREE_LIST],
			lists: [ChunkFreeList::new(); NB_FREE_LIST],
		}
	}

	/// Insert a new memory segment in the pool.
	/// 
	/// @param ptr Define the base address of the segment.
	/// @param size Define the total size of the segment. Header will be placed in it. so
	/// it should be big enough.
	/// @param mode Insert in FIFO or LIFO mode for later reuse.
	pub fn insert_addr(&mut self,ptr: Addr, size: Size, mode: ChunkInsertMode) {
		let chunk = MediumChunk::setup_size(ptr,size);
		self.insert_chunk(chunk,mode);
	}

	/// Insert a new alloated chunk in the free list. Its state will be changed to free state.
	/// @param chunk Define the chunk to insert.
	/// @param mode Define the insertion mode FIFO or LIFO.
	pub fn insert_chunk(&mut self, mut chunk: MediumChunkPtr, mode: ChunkInsertMode) {
		//get size
		chunk.check();
		let inner_size = chunk.get_inner_size();

		//errors
		debug_assert!(inner_size >= mem::size_of::<ChunkFreeList>());
		debug_assert!(chunk.get_total_size() > 0);
		debug_assert!(chunk.get_status() == CHUNK_ALLOCATED);
		
		//get the free list
		let mut flistid = self.get_free_list(inner_size);
		
		let list_class = self.get_list_class(flistid);
		if flistid != 0 && list_class != Size::max_value() && list_class != inner_size {
			flistid -= 1;
		}

		//mark free
		chunk.set_status(CHUNK_FREE);
		
		//insert
		match mode {
			ChunkInsertMode::FIFO => self.lists[flistid].push_front(chunk),
			ChunkInsertMode::LIFO => self.lists[flistid].push_back(chunk),
		}
		
		//mark non empty
		self.set_empty_status(flistid,true);
	}

	/// Return the list ID from a free list address.
	fn get_list_id(&self, list: * const ChunkFreeList) -> ChunkFreeListId {
		(list as Addr - & self.lists as * const ChunkFreeList as Addr) / mem::size_of::<ChunkFreeList>()
	}

	/// Remove a chunk from the free list it belong to.
	/// If the list become empty it is marked as empty for fast checking.
	pub fn remove(&mut self, mut chunk: &mut MediumChunkPtr) {
		//errors
		debug_assert!(!chunk.is_null());
		chunk.check();
		debug_assert!(chunk.get_status() == CHUNK_FREE);
		
		let list = ChunkFreeList::remove(&mut chunk);
		match list {
			Some(x) => {
				let id = self.get_list_id(x.get_ptr()); 
				self.set_empty_status(id,false)
			},
			None => {},
		}

		chunk.set_status(CHUNK_ALLOCATED);
	}

	/// Find a free chunk by searching in available lists.
	/// 
	/// It first search in the free list then search in the previous one
	/// which contain smaller object but can contain one of the good size so require
	/// looping on all element to find one where it is guarenti on first element of 
	/// next lists.
	/// 
	/// @param inner_size Define the size which must be contained by the chunk.
	pub fn find_chunk(&mut self, inner_size: Size) -> Option<MediumChunkPtr> {
		//vars
		let mut res;

		//errors
		debug_assert!(inner_size > mem::size_of::<ListNode>());
		
		//get the minimum valid size
		let list = self.get_free_list(inner_size);
		let start_point = list;
		
		//if empty list, go to next if available
		//otherwite, take the first of the list (oldest one -> FIFO)
		let list = self.get_first_next_non_empty_list(list);
		match list {
			Some(list) => res = self.find_adapted_chunk(list,inner_size),
			None => res = None,
		}
		
		//if not found, try our chance in the previous list (we may find some sufficient bloc, but may
		//require more steps of searching as their may be some smaller blocs in this one on the contrary
		//of our starting point which guaranty to get sufficient size
		if res.is_none() && start_point > 0 {
			let list = start_point - 1;
			res = self.find_adapted_chunk(list,inner_size);
		}
		
		//if find, remove from list
		match res {
			Some(mut x) => {
				self.remove(&mut x);
				return Some(x);
			},
			None =>  return None,
		}
	}

	/// Apply merge opertoin on the given chunk. It will merge with all free chunks on the left
	/// and on the right and return the new chunk it built.
	pub fn merge(&mut self, chunk: MediumChunkPtr) -> MediumChunkPtr {
		let mut first = chunk.clone();
		let mut last = chunk.clone();

		//error
		debug_assert!(!chunk.is_null());
		debug_assert!(chunk.get_status() == CHUNK_ALLOCATED);
		//assume_m(chunk->getStatus() == CHUNK_ALLOCATED,"The central chunk must be allocated to be merged.");
		
		//search for the first free chunk before the central one.
		let mut cur = chunk.get_prev();
		loop {
			match cur {
				Some(mut x) => {
					if x.get_status() == CHUNK_FREE {
						first = x.clone();
						//can remove current one from free list to be merged at the end of the function
						self.remove(&mut x);
						//move to next one
						cur = x.get_prev();
					} else {
						break;
					}
				},
				None => break,
			}
		}

		//search the last mergeable after the central one
		let mut cur = chunk.get_next();
		loop {
			match cur {
				Some(mut x) => {
					if x.get_status() == CHUNK_FREE {
						last = x.clone();
						//can remove current one from free list to be merged at the end of the function
						self.remove(&mut x);
						//move to next one
						cur = x.get_next();
					} else {
						break;
					}
				},
				None => break,
			}
		}
		
		//calc final bloc size
		first.merge(last);
		return first;
	}

	/// Try to look on merging the right free chunk to form a new chunk of the requested size.
	pub fn try_merge_for_size(&mut self, mut chunk: MediumChunkPtr, find_inner_size: Size) -> Option<MediumChunkPtr> {
		//errors
		debug_assert!(!chunk.is_null());
		debug_assert!( find_inner_size > 0);
		debug_assert!( find_inner_size > chunk.get_inner_size());
		
		//start to search
		let mut cur = chunk.get_next();
		let mut last = chunk.clone();
		let mut size = chunk.get_inner_size();
		let mut last_next = chunk.clone();
		
		//loop until enought
		loop {
			match cur {
				Some(x) => {
					if x.get_status() == CHUNK_FREE && size < find_inner_size {
						size += x.get_total_size();
						last = x.clone();
						//move to next one
						cur = x.get_next();
					} else {
						last_next = x.clone();
						break;
					}
				},
				None => break
			}
		}
		
		//if not enought, return NULL
		if size < find_inner_size {
			return None;
		}
		
		//free all from chunk to last
		cur = chunk.get_next();
		//loop until enought
		loop {
			match cur {
				Some(mut x) => {
					if x.get_status() == CHUNK_FREE && x.get_addr() != last_next.get_addr() {
						self.remove(&mut x);
						//move to next one
						cur = x.get_next();
					} else {
						break;
					}
				},
				None => break
			}
		}
		
		//final merge
		chunk.merge(last);
		return Some(chunk);
	}

	/// Do safe tests for debugging.
	pub fn hard_checking(&self) {
		for list in self.lists.iter() {
			list.hard_checking();
		}
	}

	/// Return the free list if containing the given size class. It uses dynchotomic or anytic approach
	/// depending on the setatus of fast_revers. Notice analytic work only for the default
	/// size class list
	fn get_free_list(&mut self, inner_size: Size) -> ChunkFreeListId {
		//errors
		debug_assert!(self.nb_list > 0);
		debug_assert!(inner_size > 0);
		
		if self.fast_reverse {
			return self.get_free_list_by_analytic( inner_size );
		} else {
			return self.get_free_list_by_dichotomy( inner_size );
		}
	}

	/// Implement the search of list id in a dynchotomic way.
	fn get_free_list_by_dichotomy(&mut self, inner_size: Size) -> ChunkFreeListId {
		//local vars
		let mut seg_size = self.nb_list;
		let mut i = seg_size >> 1;
		let mut base = 0;
		
		//errors
		debug_assert!(seg_size > 0);
		debug_assert!( inner_size > 0);
		debug_assert!(inner_size >= self.sizes[base]);
		
		if self.sizes[base] >= inner_size {
			i = 0;
		} else {
			//use dicotomic search, it's faster as we know the list sizes are sorted.
			while self.sizes[base + i-1] >= inner_size || self.sizes[base + i] < inner_size {
				if self.sizes[base + i] < inner_size {
					seg_size -= i;
					base += i;
				} else {
					seg_size = i;
				}
				i = seg_size >> 1;//divide by 2
			}
		}
		debug_assert!(base+i < self.nb_list);
		
		return base+i;
	}

	/// Make static computation to get the list if by knowing the content of the default
	/// size class list.
	fn get_free_list_by_analytic(&mut self, inner_size: Size) -> ChunkFreeListId {
		//errors
		debug_assert!( inner_size > 0);
		debug_assert!(self.nb_list > 0);
		debug_assert!(self.fast_reverse);

		//get position by reverse analytic computation.
		let mut pos = self.reverse_default_free_sizes( inner_size);

		//check size of current cell, if too small, take the next one
		if self.sizes[pos] < inner_size {
			pos += 1;
		}

		//check
		debug_assert!(pos <= self.nb_list);
		debug_assert!(pos == self.get_free_list_by_dichotomy(inner_size ));

		//return position
		return pos;
	}

	/// Return the list class from list id
	fn get_list_class(&self, list:ChunkFreeListId) -> Size {
		debug_assert!(list < NB_FREE_LIST);
		//assume_m(id >= 0 && id < NB_FREE_LIST,"The given list didn't be a member of the given thread pool.");
		return self.sizes[list];
	}

	/// Change the empty status of the given free list, false for empty, true for filled.
	fn set_empty_status(&mut self, id:ChunkFreeListId, filled: bool) {
		debug_assert!(id < NB_FREE_LIST);
		
		self.status[id] = filled;
	}

	/// Search in the given list for an adaquate chunk.
	fn find_adapted_chunk(&mut self, list:ChunkFreeListId, inner_size: Size) -> Option<MediumChunkPtr> {
		//errors
		debug_assert!( inner_size > 0);
		debug_assert!( list < self.nb_list);

		//first in the list fo oldest one -> FIFO
		let mut sel = None;
		for item in self.lists[list].iter() {
			if item.get_inner_size() >= inner_size {
				sel = Some(item.clone());
				break;
			}
		}
		
		sel
	}

	/// Start from the given list and search the next non empty list (with bigger chunks)
	fn get_first_next_non_empty_list(&mut self, id:ChunkFreeListId) -> Option<ChunkFreeListId> {
		//errors
		debug_assert!(self.nb_list > 0);
		debug_assert!(self.nb_list <= NB_FREE_LIST);
		debug_assert!(id < self.nb_list);

		//quick check all
		for i in id..self.nb_list {
			if self.status[i] {
				return Some(i);
			}
		}		

		//not found
		return None;
	}

	/// Analystical reverse function to get list id from size. Work only for the default
	/// size class list.
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

#[cfg(test)]
mod tests
{
	use common::consts::*;
	use common::types::Size;
	use chunk::medium::chunk::MediumChunk;
	use chunk::medium::pools::*;
	use portability::osmem;

	//for tests
	static TEST_SIZE_LIST: [Size; NB_FREE_LIST] = [8,16,32,64,128,1,Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),
		Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value(),Size::max_value()];

	#[test]
	fn lang_requireement() {
		debug_assert!(4 >> 1 == 2);//required property to quickly divide by 2
	}

	#[test]
	fn new() {
		let _pool = MediumFreePool::new();
	}

	#[test]
	fn new_list() {
		let _pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
	}

	#[test]
	fn find_chunk_1() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		assert_eq!(pool.find_chunk(64).is_none(), true);
	}

	#[test]
	fn find_chunk_3() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(64).unwrap();
		let mut c3 = c2.split(128).unwrap();
		c3.split(64);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c3.clone(),ChunkInsertMode::LIFO);

		assert_eq!(c1.get_root_addr(),pool.find_chunk(64).unwrap().get_root_addr());
		assert_eq!(c0.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn insert_too_small() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		pool.insert_addr(buf,128,ChunkInsertMode::FIFO);

		//chekc
		assert_eq!(pool.find_chunk(128).is_none(), true);
		
		//clear
		osmem::munmap(buf, 4096);
	}

	#[test]
	fn insert_split() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		pool.insert_addr(buf,128,ChunkInsertMode::FIFO);

		//chekc
		assert_eq!(pool.find_chunk(64).unwrap().get_root_addr(), buf);
		assert_eq!(pool.find_chunk(64).is_none(), true);
		
		//clear
		osmem::munmap(buf, 4096);
	}

	#[test]
	fn insert_lifo() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(128).unwrap();
		c2.split(128);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);

		assert_eq!(c0.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c1.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn insert_fifo() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(128).unwrap();
		c2.split(128);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::FIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::FIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::FIFO);

		assert_eq!(c2.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c1.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c0.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn insert_to_small_2() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		pool.insert_addr(buf,128,ChunkInsertMode::FIFO);

		//chekc
		assert_eq!(pool.find_chunk(100).is_none(), true);
		
		//clear
		osmem::munmap(buf, 4096);
	}

	#[test]
	fn remove() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(128).unwrap();
		c2.split(128);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		pool.remove(&mut c1);

		assert_eq!(c0.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(true,pool.find_chunk(128).is_none());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn merge_1() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let c2 = c1.split(128).unwrap();
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		let c = pool.merge(c1);

		assert_eq!(c0.get_root_addr(),c.get_root_addr());
		assert_eq!(true,pool.find_chunk(128).is_none());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn merge_2() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let _c2 = c1.split(128).unwrap();
		
		//insert none
		
		//merge 		
		let c = pool.merge(c1.clone());

		assert_eq!(c1.get_root_addr(),c.get_root_addr());
		assert_eq!(true,pool.find_chunk(128).is_none());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn try_merge_1() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(32).unwrap();
		let _c2 = c1.split(32).unwrap();
		
		//insert
		//pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		//pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		let c = pool.try_merge_for_size(c0,160);

		assert_eq!(c.is_none(),true);

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn try_merge_2() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(64).unwrap();
		let mut c2 = c1.split(64).unwrap();
		c2.split(64);
		
		//insert
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		let c = pool.try_merge_for_size(c0.clone(),72);

		assert_eq!(c.as_ref().unwrap().get_root_addr(),c0.get_root_addr());
		assert_eq!(c.as_ref().unwrap().get_inner_size(), 64 * 2 + MediumChunk::header_size());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(64).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn try_merge_3() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(64).unwrap();
		let mut c2 = c1.split(64).unwrap();
		c2.split(64);
		
		//insert
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		let c = pool.try_merge_for_size(c0.clone(),3*64);

		assert_eq!(c.as_ref().unwrap().get_root_addr(),c0.get_root_addr());
		assert_eq!(c.as_ref().unwrap().get_inner_size(), 64 * 3 + 2*MediumChunk::header_size());
		assert_eq!(true,pool.find_chunk(128).is_none());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn try_merge_4() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(64).unwrap();
		let c2 = c1.split(64).unwrap();
		//do not split to keep huge
		//c2.split(64);
		
		//insert
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);
		
		let c = pool.try_merge_for_size(c0.clone(),72);

		assert_eq!(c.as_ref().unwrap().get_root_addr(),c0.get_root_addr());
		assert_eq!(c.as_ref().unwrap().get_inner_size(), 64 * 2 + MediumChunk::header_size());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(64).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn revers() {
		let mut pool = MediumFreePool::new();
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(128).unwrap();
		c2.split(128);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);

		assert_eq!(c0.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c1.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());
		assert_eq!(c2.get_root_addr(),pool.find_chunk(128).unwrap().get_root_addr());

		osmem::munmap(buf, 4096);
	}

	#[test]
	fn hard_checking() {
		let mut pool = MediumFreePool::new_cust_list(&TEST_SIZE_LIST);
		let buf = osmem::mmap(0,4096);

		//create chunks
		let mut c0 = MediumChunk::setup_size(buf,1024);
		let mut c1 = c0.split(128).unwrap();
		let mut c2 = c1.split(128).unwrap();
		c2.split(128);
		
		//insert
		pool.insert_chunk(c0.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c1.clone(),ChunkInsertMode::LIFO);
		pool.insert_chunk(c2.clone(),ChunkInsertMode::LIFO);

		pool.hard_checking();

		osmem::munmap(buf, 4096);
	}
}
