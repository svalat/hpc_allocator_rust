/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module define all the basic constants to be used by
///the allocator

use core::mem;
use common::types::Size;

///Define basic alignement
pub const BASIC_ALIGN: Size = mem::size_of::<usize>();
pub const SMALL_PAGE_SIZE: Size = 4096;
//#define MAGICK_VALUE 42
//#define NB_FREE_LIST 50
//#define UNSUPPORTED 0u
//#define ALLOC_MIN_SIZE (2*BASIC_ALIGN)
//TODO setup value
//#define REALLOC_THREASHOLD 64
pub const MACRO_BLOC_SIZE: Size = 2*1024*1024;
//#define MEDIUM_MIN_INNER_SIZE 16
//#define ADDR_NULL 0
//#define ALLOC_DO_WARNING true
//#define HUGE_ALLOC_THREASHOLD (MACRO_BLOC_SIZE/2)

//about region mecanism
pub const REGION_SPLITTING: Size = MACRO_BLOC_SIZE;
pub const REGION_SIZE: Size = 1024*1024*1024*1024;
pub const REGION_ENTRIES: Size = REGION_SIZE / REGION_SPLITTING;
