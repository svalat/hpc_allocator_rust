/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file implement the region regitry which provide a gobal
///way to register allocators managing the different region segments
///which are larger than 2 MB.

//export
pub mod segments;