/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// A container is there to contain many runs to fit in a macro bloc.

//import
use common::shared::SharedPtrBox;

/// Implement container
pub struct SmallChunkContainer
{

}

/// Pointer
pub type SmallChunkContainerPtr = SharedPtrBox<SmallChunkContainer>;