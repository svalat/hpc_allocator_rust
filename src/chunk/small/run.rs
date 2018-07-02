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
use common::consts::*;
use common::types::{Addr,Size,SmallSize};
use common::shared::SharedPtrBox;
use common::list::{ListNode,Listable};
use common::ops;
use chunk::small::container::SmallChunkContainerPtr;
use core::mem;
use portability::arch;

/// Define a macro entry to store up to 64 bits for bitmask
type MacroEntry = u64;
type MacroEntryPtr = SharedPtrBox<MacroEntry>;

/// consts
const SMALL_RUN_SIZE: usize = 4096;
const MACRO_ENTRY_SIZE: usize = mem::size_of::<MacroEntry>();
const MACRO_ENTRY_BITS: usize = (8 * MACRO_ENTRY_SIZE);
const STORAGE_ENTRIES: usize = SMALL_RUN_SIZE /  MACRO_ENTRY_SIZE - 6;
const STORAGE_SIZE: usize = STORAGE_ENTRIES * MACRO_ENTRY_SIZE;

/// define a run
/// Remark we put the storage first and header data at the end because
/// when placing this into macro blocs we need to skip the macro bloc
/// header which reside at begenning of the segment so we just have to maek
/// this overlapping part as allocated to ignore it in the run.
pub struct SmallChunkRun {
	data:[MacroEntry; STORAGE_ENTRIES],
    list_node: ListNode,
    container: SmallChunkContainerPtr,
    cnt_alloc: SmallSize,
    skiped_size: SmallSize,
    splitting: SmallSize,
    bitmap_entries: SmallSize,
}

/// Used to point
pub type SmallChunkRunPtr = SharedPtrBox<SmallChunkRun>;

/// Implement
impl SmallChunkRun {
    pub fn setup(addr: Addr,skiped_size: SmallSize, splitting: SmallSize, container: SmallChunkContainerPtr) -> SmallChunkRunPtr {
        let mut cur = SmallChunkRunPtr::new_addr(addr);

		cur.cnt_alloc = 0;
		cur.skiped_size = (ops::up_to_power_of_2(skiped_size as usize, MACRO_ENTRY_SIZE as usize) / MACRO_ENTRY_SIZE as usize) as u16;
		cur.splitting = splitting;
		cur.bitmap_entries = 0;
		cur.container = container;
		if splitting > 0 {
			cur.set_splitting(splitting);
		}

		cur
    }

    pub fn set_splitting(&mut self,splitting: SmallSize) {
        //errors
		debug_assert!(splitting as usize <= STORAGE_SIZE);
		if self.cnt_alloc != 0 {
			panic!("Cannot change the size of non empty SmallChunkRun.");
		}
		
		//trivial
		if splitting == 0 {
			debug_assert!(self.splitting > 0);
			self.splitting = 0;
			return;
		}
		
		//setup size
		self.splitting = splitting;
		
		//calc bitmap entries
		let bitmap_real_entries = STORAGE_SIZE / splitting as usize;
		self.bitmap_entries = ops::up_to_power_of_2(bitmap_real_entries as usize,MACRO_ENTRY_BITS) as u16;
		
		//calc skiped entries
		let skiped_entries = self.get_rounded_nb_entries(self.skiped_size * MACRO_ENTRY_SIZE as u16);
		
		//calc nb entries masked by bitmap storage
		let bitmap_size = self.bitmap_entries / 8;
		let bitmap_hidden_entries = self.get_rounded_nb_entries(bitmap_size);
		
		//check
		debug_assert!(self.bitmap_entries > bitmap_hidden_entries - skiped_entries );
		
		//clear bitmap with 1 (all free)
		for i in 0..(bitmap_size as usize / MACRO_ENTRY_SIZE) {
			self.set_macro_entry(i as u16,MacroEntry::max_value());
		}
		
		//mark skiped entries and bitmap part
		for i in 0..(skiped_entries + bitmap_hidden_entries) {
			self.set_bit_status_zero(i);
		}
		
		//mark last bits to 0
		for i in bitmap_real_entries..self.bitmap_entries as usize {
			self.set_bit_status_zero(i as u16);
		}
    }

    pub fn is_empty(&self) -> bool {
        self.cnt_alloc == 0
    }

    pub fn is_full(&self) -> bool {
        debug_assert!(self.splitting > 0);
		let macro_entries = self.bitmap_entries / MACRO_ENTRY_BITS as u16;
		for i in 0..macro_entries {
			if self.get_macro_entry(i) != 0 {
				return false;
			}
		}
		return true;
    }

    pub fn malloc(&mut self,size: Size, align: Size, zero_filled: bool) -> (Addr,bool) {
        //check size
		if size > self.splitting as usize {
			panic!("SmallChunkRun only support allocation smaller than the splitting size.");
		}
		if size < self.splitting as usize / 2 {
			//TODO this sould be warning
			panic!("Caution, you allocate chunk in SmallChunkRun with size less than halfe of the quantum size.");
		}
		debug_assert!(self.splitting as usize % align == 0);
		
		//search first bit to one (availble free bloc)
		let macro_entries = self.bitmap_entries / MACRO_ENTRY_BITS as u16;
		for i in 0..macro_entries {
			//if get one bit to 1, it contain free elements
			let entry = self.get_macro_entry(i);
			if entry != 0 {
				//search the first bit to one
				let id = arch::fast_log_2(entry as usize) as u16;
				debug_assert!(id < MACRO_ENTRY_BITS as u16);
				let id = id + i * MACRO_ENTRY_BITS as u16;
				debug_assert!(self.get_bit_status(id) == true);
				self.set_bit_status_zero(id);
				debug_assert!(self.get_bit_status(id) == false);
				self.cnt_alloc += 1;
				let base_addr = (&self.data) as * const MacroEntry as Addr;
				let addr = base_addr + self.splitting as usize * id as usize;
				return (addr,false);
			}
		}
		
		//didn't not find free memory
		return (0,zero_filled);
    }

    pub fn free(&mut self,ptr: Addr) {
        //compute bit position
		//TODO maybe assume
		debug_assert!(self.contain(ptr));

		//calc bit position
		let base_addr = (&self.data) as * const MacroEntry as Addr;
		let delta = ptr - base_addr;
		let bitpos = (delta / self.splitting as usize) as u16;

		//check current status
		debug_assert!(self.get_bit_status(bitpos) == false);

		//mark as free
		self.set_bit_status_one(bitpos);
		
		//update counter
		self.cnt_alloc -= 1;
    }

    pub fn get_inner_size(&self,ptr: Addr) -> Size {
        debug_assert!(self.contain(ptr));
		return self.splitting as Size;
    }

    pub fn get_requested_size(&self,ptr: Addr)-> Size {
		debug_assert!(self.contain(ptr));
        return UNSUPPORTED;
    }

    pub fn get_total_size(&self,ptr: Addr) -> Size {
        debug_assert!(self.contain(ptr));
		return self.splitting as Size;
    }

    pub fn get_splitting(&self) -> SmallSize {
        return self.splitting;
    }

    pub fn realloc(&self,ptr: Addr, size: Size) -> Addr {
        if size > self.splitting as usize {
			panic!("Realloc isn't supported in SmammChunkRun.");
		}
		return ptr;
    }

    pub fn contain(&self,ptr: Addr) -> bool {
        let base_addr = (&self.data) as * const MacroEntry as Addr;
		return ptr >= base_addr+self.skiped_size as usize+self.bitmap_entries as usize/MACRO_ENTRY_BITS as usize && ptr < base_addr + STORAGE_SIZE;
    }

    pub fn get_container(&self) -> SmallChunkContainerPtr {
        self.container.clone()
    }

    fn set_bit_status_one(&mut self,id: SmallSize) {
        let mid = id as usize / MACRO_ENTRY_BITS;
        let bit = id as usize % MACRO_ENTRY_BITS;
        let value = self.get_macro_entry_mut(mid as u16);
		*value |= (1 as MacroEntry) << (bit);
    }

    fn set_bit_status_zero(&mut self,id: SmallSize) {
        let mid = id as usize / MACRO_ENTRY_BITS;
        let bit = id as usize % MACRO_ENTRY_BITS;
        let value = self.get_macro_entry_mut(mid as u16);
		*value &= !((1 as MacroEntry) << (bit));
    }

    fn get_bit_status(&self,id: SmallSize) -> bool {
        let mid = id as usize / MACRO_ENTRY_BITS;
        let bit = id as usize % MACRO_ENTRY_BITS;
        let value = self.get_macro_entry(mid as u16);
		return (value & ((1 as MacroEntry) << (bit as usize))) != 0;
    }

    fn get_rounded_nb_entries(&self,size: SmallSize) -> SmallSize {
        let entries = size / self.splitting;
		if entries * self.splitting != size {
			return entries + 1;
		} else {
			return entries;
		}
    }

    fn get_macro_entry_mut(&mut self,id: SmallSize) -> &mut MacroEntry {
        &mut self.data[id as usize]
    }

	fn set_macro_entry(&mut self,id: SmallSize, value: MacroEntry) {
		self.data[id as usize] = value;
	}

	fn get_macro_entry(&self,id: SmallSize) -> MacroEntry {
        self.data[id as usize]
    }
}

impl Listable<SmallChunkRun> for SmallChunkRun {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
        return &self.list_node;
    }

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
        return &mut self.list_node;
    }

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const SmallChunkRun {
        return (elmt as Addr - mem::size_of::<MacroEntry>() * STORAGE_ENTRIES) as * const SmallChunkRun
    }

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut SmallChunkRun {
        return (elmt as Addr - mem::size_of::<MacroEntry>() * STORAGE_ENTRIES) as * mut SmallChunkRun
    }
}

#[cfg(test)]
mod tests
{
	use core::mem;
	use chunk::small::run::*;
	use portability::osmem;
    use common::list::List;

	#[test]
	fn type_check() {
		assert_eq!(mem::size_of::<u64>(), mem::size_of::<usize>());
		assert_eq!(SMALL_RUN_SIZE,mem::size_of::<SmallChunkRun>());
	}

	#[test]
	fn constructor_1() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let run = SmallChunkRun::setup(ptr, 0, 0, container);
		assert_eq!(0,run.get_splitting());
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn constructor_2() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let run = SmallChunkRun::setup(ptr, 0, 16, container);
		assert_eq!(16,run.get_splitting());
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn get_inner_size() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		let (p,_) = run.malloc(16,16,false);
		assert_eq!(16,run.get_inner_size(p));
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn get_total_size() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		let (p,_) = run.malloc(16,16,false);
		assert_eq!(16,run.get_total_size(p));
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn set_splitting() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 0, container);
		assert_eq!(0, run.get_splitting());
		run.set_splitting(16);
		assert_eq!(16, run.get_splitting());
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn set_splitting_non_multiple() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 0, container);
		assert_eq!(0, run.get_splitting());
		run.set_splitting(15);
		assert_eq!(15, run.get_splitting());
		while run.malloc(15,15,false).0 != NULL {};
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn malloc_1() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		let (p,_) = run.malloc(16,16,false);
		assert!(p != NULL);
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn malloc_2() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		let (p,_) = run.malloc(16,16,false);
		assert!(p != NULL);
		let (p2,_) = run.malloc(16,16,false);
		assert!(p != p2);
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn free() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		let (p,_) = run.malloc(16,16,false);
		run.free(p);
		assert!(p != NULL);
		let (p2,_) = run.malloc(16,16,false);
		assert!(p == p2);
		osmem::munmap(ptr, 4096);
	}

	#[test]
	fn full() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		
		let mut cnt = 0;
		while run.malloc(16,16,false).0 != NULL {cnt += 1;};
		assert_eq!(SMALL_RUN_SIZE/16 - 5,cnt);

		osmem::munmap(ptr, 4096);
	}

    #[test]
	fn full_no_overlap() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		
		let mut cnt = 0;
        let mut store: [Addr; SMALL_RUN_SIZE/16] = [0; SMALL_RUN_SIZE/16];
		loop {
            let (p,_) = run.malloc(16,16,false);
            if p == NULL {
                break;
            } else {
                store[cnt] = p;
                cnt += 1;
            }
        };
		assert_eq!(SMALL_RUN_SIZE/16 - 5,cnt);

        for i in 0..cnt {
            for j in 0..i {
                assert_ne!(store[j], store[i]);
            }
        }

		osmem::munmap(ptr, 4096);
	}

    #[test]
	fn free_middle() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		
		let mut cnt = 0;
        let mut store: [Addr; SMALL_RUN_SIZE/16] = [0; SMALL_RUN_SIZE/16];
		loop {
            let (p,_) = run.malloc(16,16,false);
            if p == NULL {
                break;
            } else {
                store[cnt] = p;
                cnt += 1;
            }
        };
		assert_eq!(SMALL_RUN_SIZE/16 - 5,cnt);

        run.free(store[32]);

        assert_eq!(store[32], run.malloc(16,16,false).0);

		osmem::munmap(ptr, 4096);
	}

    #[test]
	fn is_empty() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);

        assert_eq!(run.is_empty(), true);
        let (p,_) = run.malloc(16,16,false);
        assert_eq!(run.is_empty(), false);
        run.free(p);
        assert_eq!(run.is_empty(), true);	
		
		osmem::munmap(ptr, 4096);
	}

    #[test]
	fn is_full() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 0, 16, container);
		
        assert_eq!(run.is_full(), false);

		let mut cnt = 0;
		while run.malloc(16,16,false).0 != NULL {cnt += 1;};
		assert_eq!(SMALL_RUN_SIZE/16 - 5,cnt);

        assert_eq!(run.is_full(), true);

		osmem::munmap(ptr, 4096);
	}
    
    #[test]
	fn skiped_offset() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 32, 16, container);
		
		let mut cnt = 0;
		loop {
            let (p,_) = run.malloc(16,16,false);
            if p == NULL {
                break; 
            } else {
                assert!(p >= ptr + 32 );
                cnt += 1;
            }
        } 
		assert_eq!(SMALL_RUN_SIZE/16 - 5 - 2,cnt);

		osmem::munmap(ptr, 4096);
	}

    #[test]
	fn realloc() {
		let ptr = osmem::mmap(0, 4096);
		let container = SmallChunkContainerPtr::new_null();
		let mut run = SmallChunkRun::setup(ptr, 32, 16, container);
		
		let (p1,_) = run.malloc(16,16,false);
        let p2 = run.realloc(p1,15); 
		assert_eq!(p1,p2);

		osmem::munmap(ptr, 4096);
	}

    #[test]
    fn listable() {
        let ptr1 = osmem::mmap(0, 4096);
		let ptr2 = osmem::mmap(0, 4096);

        let container = SmallChunkContainerPtr::new_null();
		let run1 = SmallChunkRun::setup(ptr1, 32, 16, container.clone());
		let run2 = SmallChunkRun::setup(ptr2, 32, 16, container.clone());
		 
        let mut list :List<SmallChunkRun> = List::new();

        list.push_back(run1);
        list.push_back(run2);

        for mut i in list.iter() {
            i.malloc(16,16,false);
        }

		osmem::munmap(ptr1, 4096);
		osmem::munmap(ptr2, 4096);
    }
}
