/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module define all the basic constants to be used by
/// the allocator

use core::mem;
use common::types::{SSize,Size};

//global values
/// Define basic alignement handled by standared chunk manager for any size larger than this one.
pub const BASIC_ALIGN: Size = mem::size_of::<usize>();
/// Define the standard page size which is 4k on most systems.
pub const SMALL_PAGE_SIZE: Size = 4096;
//#define MAGICK_VALUE 42
//#define NB_FREE_LIST 50
/// To return some unsuported value for getting requested size from chunk managers
pub const UNSUPPORTED: Size = 0;
//#define ALLOC_MIN_SIZE (2*BASIC_ALIGN)
/// Minimal size to generate a realloc in huge and medium chunk manager.
pub const REALLOC_THREASHOLD: SSize = 64;
/// Define the basic macro bloc size, used to split the region registry. This is
/// The minimum size we can allocate inside the memory source.
pub const MACRO_BLOC_SIZE: Size = 2*1024*1024;
/// Minimal inner size for medium chunks.
pub const MEDIUM_MIN_INNER_SIZE: Size = 16;
//#define ADDR_NULL 0
//#define ALLOC_DO_WARNING true
//#define HUGE_ALLOC_THREASHOLD (MACRO_BLOC_SIZE/2)

//about region mecanism
/// Define the basic size we used to split the address space and minimum size we consider
/// allocations handled by the memory source. Should ba MACRO_BLOC_SIZE in principle
pub const REGION_SPLITTING: Size = MACRO_BLOC_SIZE;
/// Each region map the macro blocs it contain and is max this size.
pub const REGION_SIZE: Size = 1024*1024*1024*1024;
/// Number of rentries for one region
pub const REGION_ENTRIES: Size = REGION_SIZE / REGION_SPLITTING;
/// Number of real bits used by processor to build the virtual address space
/// This define the number of regions we need to manage.
///
/// TODO: This should now move to 57 bits wit new intel cpus and linux kernel
/// 5 levels page table
pub const PHYS_ADDR_BITS: Size = 48;
/// From the bits used by virtual address compute the address space to handle by regions
pub const PHYS_MAX_ADDR: Size = 1 << PHYS_ADDR_BITS;
/// Max number of regions to handle in the region registry.
pub const MAX_REGIONS: Size = PHYS_MAX_ADDR / REGION_SIZE;

///Define the maximum size of total memory stored into the memory source.
pub const MMSRC_MAX_SIZE: Size = 16*1024*1024;
///Define maximum size of segments to keep in the memory source
pub const MMSRC_THREASHOLD: Size = 8*1024*1204;
///Keep non used part of segment when required less (if big enougth)
pub const MMSRC_KEEP_RESIDUT: bool = false;

///Magick number used by padded chunks.
pub const PADDED_CHUNK_MAGICK: u8 = 0x42;
