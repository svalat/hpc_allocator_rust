/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module implement the allocator functionallity to be exported as
/// full posix allocator. It mostly make the glue to wrap all the other
/// base classes.

//export
pub mod uma;
pub mod local;