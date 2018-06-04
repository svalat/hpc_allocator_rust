/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// Implement a memory source with behave as a cache by keeping macro blocs into memory
/// to reduce exchanges with the OS and pay less the price of first touch page
/// faults.

//import
use common::consts::*;
use common::types::{Addr,Size};
use common::list::{List,ListNode,Listable};
use common::shared::SharedPtrBox;
use common::traits::{ChunkManager,MemorySource};
use common::ops;
use registry::registry::RegionRegistry;
use registry::segment::RegionSegment;
use portability::spinlock::SpinLock;
use portability::osmem;
use core::mem;

/// Implement the header to track state of free macro blocs we keep in the cache.
struct FreeMacroBloc {
    node: ListNode,
    total_size: Size,
}

/// Alias to ease code.
type FreeMacroBlocList = List<FreeMacroBloc>;

/// Internal values which will be protected by spinlock.
struct InternalThreadProtected {
    list: FreeMacroBlocList,
    current_size: Size,
}

/// Implement the cached memory source.
pub struct CachedMMSource {
    /// Store the free macro bloc list for future reuse, protected by a spinlock.
    freelist: SpinLock<InternalThreadProtected>,
    /// Maximal authozied size for the cache.
    max_size: Size,
    /// Do not keep macro blocs larger than this.
    threashold: Size,
    /// When spitting macro blocs for reuse, do we keep the extra memory in cache if possible ?
    keep_residut: bool,
    /// Ref to registry to register the new macro blocs before giving them to the caller.
    registry: Option<SharedPtrBox<RegionRegistry>>,
}

//Implement free macro bloc
impl FreeMacroBloc {
    /// Create a new macro bloc in place.
    ///
    /// @param addr: Define the base address of the free macro bloc. Also where to write the free macro bloc header.
    /// @param total_size: Define the total size of the free macro bloc.
    pub fn new(addr: Addr, total_size: Size) -> SharedPtrBox<FreeMacroBloc> {
        let mut ptr: SharedPtrBox<FreeMacroBloc> = SharedPtrBox::new_addr(addr);
        *ptr.get_mut() = FreeMacroBloc {
            node: ListNode::new(),
            total_size: total_size,
        };
        ptr
    }

    /// Return the total size of the current free macro bloc.
    pub fn get_total_size(&self) -> Size {
        self.total_size
    }

    /// Return the base address of macro bloc to be used in mremap of munmap.
    pub fn get_root_addr(&self) -> Addr {
        self as * const FreeMacroBloc as Addr
    }
}

/// Implement listable support so free macro blocs can be placed into a link list.
impl Listable<FreeMacroBloc> for FreeMacroBloc {
    fn get_list_node<'a>(&'a self) -> &'a ListNode {
        &self.node
    }

    fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
        &mut self.node
    }

    fn get_from_list_node<'a>(elmt: * const ListNode) -> * const FreeMacroBloc {
        unsafe{&*(elmt as * const ListNode as Addr as * const FreeMacroBloc)}
    }

    fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut FreeMacroBloc {
        unsafe{&mut *(elmt as * mut ListNode as Addr as * mut FreeMacroBloc)}
    }
}

/// Implement the cached memory source member functions.
impl CachedMMSource {
    /// Create a new memory source.
    ///
    /// @param registry Define the regisitry to be used for bloc registration. Can be None to ignore.
    /// @param max_size Define the maximum size of the cache to limit memory consumption.
    /// @param threashold Define the maximum size of macro blocs to keep inside the cache.
    /// @param keep_residut Define if we keep the ending part of macro blocs when reuse (if they are big enougth to be a macro bloc.)
    pub fn new(registry:Option<SharedPtrBox<RegionRegistry>>,max_size:Size,threashold:Size,keep_residut:bool) -> Self {
        Self {
            freelist: SpinLock::new(InternalThreadProtected{
                list: FreeMacroBlocList::new(),
                current_size: 0,
            }),
            max_size: max_size,
            threashold: threashold,
            keep_residut: keep_residut,
            registry: registry,
        }
    }

    /// Same than new but with default values for configuration
    pub fn new_default(registry:Option<SharedPtrBox<RegionRegistry>>) -> Self {
        Self::new(registry,MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT)
    }

    /// Free all the memory stored into the cache.
    pub fn free_all(&mut self) {
        let mut tmp = self.freelist.lock();

        while !tmp.list.is_empty() {
            let seg = tmp.list.pop_front();
            match seg {
                Some(x) => {
                    let x = x.get();
                    let size = x.get_total_size();
                    tmp.current_size -= size;
                    osmem::munmap(x.get_root_addr(),size);
                }
                None => panic!("There is a bug !"),
            }
        }
		
		debug_assert!(tmp.current_size == 0);
    }

    /// Search a free macro bloc which can match in the cache (free list).
    /// It can remap an existing smaller or larger segment after searching the closer one in term of size.
    /// If keep_residut is set it will store the ending part of the segment after splitting the macro bloc.
    ///
    /// @param total_size Define the size we want accouting headers.
    /// @param manager Define the chunk manager to attach to the segment
    fn search_in_cache(&mut self,total_size:Size, manager: Option<* mut ChunkManager>) -> Option<RegionSegment> {
        //errors
        debug_assert!(total_size >= REGION_SPLITTING);
        debug_assert!(total_size <= self.threashold);
        debug_assert!(total_size % SMALL_PAGE_SIZE == 0);
        
        //quickly check if list is empty
        //be non exact but find, this limit contention
        let is_empty = self.freelist.nolock_safe_read().list.is_empty();
        
        //not found 
        if is_empty {
            return None;
        }
        
        //vars
        let mut best: SharedPtrBox<FreeMacroBloc> = SharedPtrBox::new_null();
        let mut best_delta = Size::max_value();

        //critical section
        {
            //take lock
            let mut tmp = self.freelist.lock();

            //search most adapted
            for bloc in tmp.list.iter() {
                let delta = (bloc.get().get_total_size() as i64 - total_size as i64).abs() as Size;
                //check if match better
                if delta < best_delta {
                    best_delta = delta;
                    best = bloc;
                }

                //stop if match exactly
                if delta == 0 {
                    break;
                }
            }

            //extract from list
            if !best.is_null() {
                tmp.list.remove(best.clone());
                tmp.current_size -= best.get().get_total_size();
            }
        }
        
        if best.is_null() {
            return None
        } else {
            //if to large, split or increase if too small
            if best_delta != 0 {
                best = self.fix_reuse_size(best,total_size);
            }

            //retu
            return Some(RegionSegment::new(best.get_root_addr(),best.get_total_size(),manager))
        }
    }


    /// When reusing segment this function is used to resize the segment (shrink of enlarge).
    /// If the residut is large enought it might keep it for latter use.
    ///
    /// @param bloc Define the bloc to resize.
    /// @param total_size Define the expected size of segment (considering header size).
    fn fix_reuse_size(&mut self, bloc: SharedPtrBox<FreeMacroBloc>, total_size: Size) -> SharedPtrBox<FreeMacroBloc> {
        //errors
        debug_assert!(!bloc.is_null());
        
        //extract size
        let size = bloc.get_total_size();
        debug_assert!(size != total_size);
        
        //if too small, mremap, otherwise split
        let ret;
        if size < total_size {
            let ptr = osmem::mremap(bloc.get_root_addr(),size,total_size,0);
            ret = FreeMacroBloc::new(ptr,total_size);
        } else {
            //split
            ret = FreeMacroBloc::new(bloc.get_root_addr(),total_size);
            
            //point next
            let next = ret.get_root_addr() + total_size;
            let next_size = size - total_size;

            //keep next for reuse of return to OS
            if self.keep_residut && next_size <= self.threashold {
                let residut = FreeMacroBloc::new(next,next_size);
                let mut tmp = self.freelist.lock();
                tmp.list.push_front(residut.clone());
                tmp.current_size += residut.get().get_total_size();
            } else {
                osmem::munmap(next,next_size);
            }
        }
        
        return ret;
    }
}

impl MemorySource for CachedMMSource {
    fn map(&mut self,inner_size: Size, _zero_filled: bool, manager: Option<* mut ChunkManager>) -> (RegionSegment, bool) {
        //errors
        debug_assert!(inner_size > 0);
        
        //compute total size
        let mut total_size = inner_size + mem::size_of::<RegionSegment>();
        
        //if to small
        if total_size < REGION_SPLITTING {
            total_size = REGION_SPLITTING;
        }
        
        //roudn to multiple of page size
        total_size = ops::up_to_power_of_2(total_size,SMALL_PAGE_SIZE);

        //manage zero status
        let mut zero: bool = false;
        let mut res: Option<RegionSegment> = None;

        //search in cache if smaller than threashold
        if total_size <= self.threashold {
            res = self.search_in_cache(total_size,manager);
            zero = false;
        }
        
        //if not found of too large, do real mmap
        if res.is_none() {
            let ptr = osmem::mmap(0,total_size);
            //TODO support again
            //allocCondWarning(ptr != NULL,"Failed to get memory from OS with mmap, maybe get OOM.");
            zero = true;
            
            //not found
            //if (ptr == NULL)
            //    return NULL;
            //else
            res = Some(RegionSegment::new(ptr,total_size,manager));
        }
        
        //register
        if self.registry.is_some() && manager.is_some() && res.is_some() {
            self.registry.as_mut().unwrap().set_segment_entry(res.unwrap());
        }

        return (res.unwrap(),zero);
    }
    
    fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: Option<* mut ChunkManager>) -> RegionSegment {
        //errors
        old_segment.sanity_check();
        
        //checkup size
        let mut total_size = new_inner_size + mem::size_of::<RegionSegment>();
        if total_size < REGION_SPLITTING {
            total_size = REGION_SPLITTING;
        }
        total_size = ops::up_to_power_of_2(total_size,SMALL_PAGE_SIZE);

        //unregister
        if self.registry.is_some(){
            match old_segment.get_manager() {
                Some(_) => self.registry.as_mut().unwrap().remove_from_segment(old_segment),
                None => {},
            }
        }

        //remap
        let ptr = osmem::mremap(old_segment.get_root_addr(),old_segment.get_total_size(),total_size,0);

        //register
        if self.registry.is_some() && manager.is_some() {
            self.registry.as_mut().unwrap().set_entry(ptr,total_size,manager.unwrap())
        } else {
            RegionSegment::new(ptr,total_size,manager)
        }
    }
	
    fn unmap(&mut self,segment: RegionSegment) {
        //errors
        segment.sanity_check();
        
        //unregister
        if self.registry.is_some() && segment.get_manager().is_some() {
            self.registry.as_mut().unwrap().remove_from_segment(segment);
        }
        
        //if small, keep, other wise unmap
        //we don't take lock to check current_size as it is fine if we are not strict on it.
        //This avoid to take twice of to have the lock kept arround syscall munmap.
        let size = segment.get_total_size();
        if size > self.threashold || size + self.freelist.nolock_safe_read().current_size > self.max_size {
            osmem::munmap(segment.get_root_addr(),size);
        } else {
            let mut tmp = self.freelist.lock();
            tmp.list.push_front(FreeMacroBloc::new(segment.get_root_addr(),size));
            tmp.current_size += size;
        }
    }
}

#[cfg(test)]
mod tests
{
	use chunk::dummy::*;
	use registry::registry::*;
    use mmsource::cached::*;

	#[test]
    fn create() {
        let registry = RegionRegistry::new();
        let _source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);
    }
    
    #[test]
    fn simple_map() {
        let registry = RegionRegistry::new();
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);

        //allocate
        let (seg,zeroed) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 4*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);

        source.unmap(seg);
    }

    #[test]
    fn simple_map_no_chunk_manager() {
        let registry = RegionRegistry::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);

        //allocate
        let (seg,zeroed) = source.map(4*1024*1024,true,None);

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 4*1024*1024);

        source.unmap(seg);
    }

    #[test]
    fn simple_map_no_registry() {
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(None,MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);

        //allocate
        let (seg,zeroed) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 4*1024*1024);

        source.unmap(seg);
    }

    #[test]
    fn simple_reuse() {
        let registry = RegionRegistry::new();
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);

        //allocate
        let (seg,zeroed) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 4*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        let addr = seg.get_root_addr();

        //free & realloc
        source.unmap(seg);

        let (seg,zeroed) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        assert_eq!(zeroed,false);
        assert!(seg.get_inner_size() >= 4*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        assert_eq!(seg.get_root_addr(),addr);

        source.unmap(seg);
    }

    #[test]
    fn simple_remap() {
        let registry = RegionRegistry::new();
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,MMSRC_KEEP_RESIDUT);

        //allocate
        let (seg,zeroed) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        let (seg2,_) = source.map(4*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 4*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        let addr = seg.get_root_addr();

        //free & realloc
        let seg = source.remap(seg,8*1024*1024,Some(&mut manager as * mut ChunkManager));

        assert!(seg.get_inner_size() >= 8*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        assert!(seg.get_root_addr() != addr);

        source.unmap(seg);
        source.unmap(seg2);
    }

    #[test]
    fn reuse_split_true() {
        let registry = RegionRegistry::new();
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,true);

        //allocate
        let (seg,zeroed) = source.map(8*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 8*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        let addr = seg.get_root_addr();

        //free & realloc
        source.unmap(seg);

        let (seg,zeroed) = source.map(2*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        assert_eq!(zeroed,false);
        assert!(seg.get_inner_size() >= 2*1024*1024 && seg.get_inner_size() <= 2*1024*1024+SMALL_PAGE_SIZE);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        assert_eq!(seg.get_root_addr(),addr);

        source.unmap(seg);

        let (seg,zeroed) = source.map(2*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        assert_eq!(zeroed,false);
        assert!(seg.get_inner_size() >= 2*1024*1024 && seg.get_inner_size() <= 2*1024*1024+SMALL_PAGE_SIZE);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);

        source.unmap(seg);

        source.free_all();
    }

    #[test]
    fn reuse_split_false() {
        let registry = RegionRegistry::new();
        let mut manager = DummyChunkManager::new();
        let mut source = CachedMMSource::new(Some(SharedPtrBox::new_ref(&registry)),MMSRC_MAX_SIZE,MMSRC_THREASHOLD,false);

        //allocate
        let (seg,zeroed) = source.map(8*1024*1024,true,Some(&mut manager as * mut ChunkManager));

        //check
        assert_eq!(zeroed,true);
        assert!(seg.get_inner_size() >= 8*1024*1024);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        let addr = seg.get_root_addr();

        //free & realloc
        source.unmap(seg);

        let (seg,zeroed) = source.map(2*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        assert_eq!(zeroed,false);
        assert!(seg.get_inner_size() >= 2*1024*1024 && seg.get_inner_size() <= 2*1024*1024+SMALL_PAGE_SIZE);
        assert_eq!(registry.get_segment(seg.get_root_addr()).is_some(),true);
        assert_eq!(seg.get_root_addr(),addr);

        let (seg2,zeroed) = source.map(2*1024*1024,true,Some(&mut manager as * mut ChunkManager));
        assert_eq!(zeroed,true);
        assert!(seg2.get_inner_size() >= 2*1024*1024 && seg2.get_inner_size() <= 2*1024*1024+SMALL_PAGE_SIZE);
        assert_eq!(registry.get_segment(seg2.get_root_addr()).is_some(),true);

        source.unmap(seg);
        source.unmap(seg2);

        source.free_all();
    }
}