/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module define all the OS wrappers to ease portability

//import
pub mod osmem;
pub mod spinlock;
pub mod arch;
pub mod libc;
pub mod libnuma;
//pub mod hwloc;