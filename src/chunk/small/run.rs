/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// A run is a small segment of a given size to store small allocations of
/// fixed size (always the same size in a given run). So it can use
/// a bitmap to remember the allocated or free state of each chunk and have no
/// headers attached to the chunk. This is more efficient in space and in cache
/// behavior. This come from the JeMalloc allocator.

//import
use common::types::{Addr,Size,SmallSize};
use common::shared::SharedPtrBox;
use common::list::{ListNode,Listable};
use chunk::small::container::SmallChunkContainerPtr;
use core::mem;

/// Define a macro entry to store up to 64 bits for bitmask
type MacroEntry = u64;

/// consts
const SMALL_RUN_SIZE: usize = 4096;
const MACRO_ENTRY_SIZE: usize = mem::size_of::<MacroEntry>();
const MACRO_ENTRY_BITS: usize = (8 * MACRO_ENTRY_SIZE);
const MACRO_ENTRY_MASK: usize = MACRO_ENTRY_SIZE - 1;
const STORAGE_ENTRIES: usize = SMALL_RUN_SIZE /  MACRO_ENTRY_SIZE - 4;
const STORAGE_SIZE: usize = STORAGE_ENTRIES * MACRO_ENTRY_SIZE;

/// define a run
pub struct SmallChunkRun {
    data:[MacroEntry; STORAGE_ENTRIES],
    container: SmallChunkContainerPtr,
    listNode: ListNode,
    cnt_alloc: SmallSize,
    skiped_size: SmallSize,
    splitting: SmallSize,
    bitmap_entries: SmallSize,
}

/// Used to point
pub type SmallChunkRunPtr = SharedPtrBox<SmallChunkRun>;

/// Implement
impl SmallChunkRun {
    pub fn setup(addr: Addr,skipedSize: SmallSize, splitting: SmallSize, container: SmallChunkContainerPtr) -> SmallChunkRunPtr {
        unimplemented!();
    }

    pub fn set_splitting(size: SmallSize) {
        unimplemented!();
    }

    pub fn is_empty() -> bool {
        unimplemented!();
    }

    pub fn is_full() -> bool {
        unimplemented!();
    }

    pub fn malloc(size: Size, align: Size, zero_filled: bool) -> (Addr,bool) {
        unimplemented!();
    }

    pub fn free(ptr: Addr) {
        unimplemented!();
    }

    pub fn get_inner_size(ptr: Addr) -> Size {
        unimplemented!();
    }

    pub fn get_requested_size(ptr: Addr)-> Size {
        unimplemented!();
    }

    pub fn get_total_size(ptr: Addr) -> Size {
        unimplemented!();
    }

    pub fn get_splitting() -> SmallSize {
        unimplemented!();
    }

    pub fn realloc(ptr: Addr, size: Size) -> Addr {
        unimplemented!();
    }

    pub fn contain(ptr: Addr) -> bool {
        unimplemented!();
    }

    pub fn get_container() -> SmallChunkContainerPtr {
        unimplemented!();
    }

    fn set_bit_status_one(id: SmallSize) {
        unimplemented!();
    }

    fn set_bit_status_zero(id: SmallSize) {
        unimplemented!();
    }

    fn get_bit_status(id: SmallSize) {
        unimplemented!();
    }

    fn get_rounded_nb_entry(size: SmallSize) -> SmallSize {
        unimplemented!();
    }

    fn get_macro_entry(id: SmallSize) -> * const MacroEntry {
        unimplemented!();
    }
}

impl Listable<SmallChunkRun> for SmallChunkRun {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
        unimplemented!();
    }

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
        unimplemented!();
    }

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const SmallChunkRun {
        unimplemented!();
    }

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut SmallChunkRun {
        unimplemented!();
    }
}
