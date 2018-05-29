/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This implement a dummy chunk manager. This is beacuse we cannot easily
///NULL a rust pointer to trait ChunkManager (size not known).
///So the clener solutions seams to push a dummy chunk manager for chunk
///waiting into the memory source.

//import
use common::traits::ChunkManager;

//decl
pub struct DummyChunkManager;

//impl
impl DummyChunkManager {
	pub fn new() -> DummyChunkManager {
		DummyChunkManager {}
	}
}

//impl trait
impl ChunkManager for DummyChunkManager {

}