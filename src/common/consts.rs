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
use common::types;

///Define basic alignement
pub const BASIC_ALIGN: types::Size = mem::size_of::<usize>();
