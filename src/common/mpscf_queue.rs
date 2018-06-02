/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module implement a MPSCF queue (Multiple Producer, Single Consumer Flush). This is a link list
/// where many producer can push items in an atomic way. The single consumer can then flush all the list
/// in one go.
///
/// This is used inside the allocator to handle remote free. A remote free can be registrerd to the local
/// allocator by all the remote threads. Then the local allocator flush in one go all the pending allocs
/// to freed.
/// 
/// This is to store object which owned their own memory so we have to put a MPSCFItem inside the objects
/// we want to register into this list. Then it is the responsibility of the user to allocate/free the objects.
/// Which is fine as it is use by the allocator itself.

//import
use common::shared::SharedPtrBox;
use core::sync::atomic::{Ordering,AtomicPtr};
use core::ptr;
use core::mem;

/// Define the basic item to be chains in one way into the list (so next pointer).
/// It uses the ShardPtrBox to bypass ownership and mutability checking.
#[derive(Copy,Clone)]
pub struct MPSCFItem {
    next: SharedPtrBox<MPSCFItem>,
}

/// Define the queue object as two atmic pointers : head and tail.
pub struct MPSCFQueue {
    head: AtomicPtr<MPSCFItem>,
    tail: AtomicPtr<MPSCFItem>,
}

impl MPSCFItem {
    /// Create a new istem whic is init with a Null pointer into next.
    pub fn new() -> Self {
        Self {
            next: SharedPtrBox::new_null(),
        }
    }
}

impl MPSCFQueue {
    /// Create a new empty list (so head/tail are NULL).
    pub fn new() -> Self {
        Self {
            head:AtomicPtr::new(ptr::null_mut()),
            tail:AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed).is_null()
    }

    /// Insert an item into the list. This operation can be done by several threads concurently.
    /// 
    /// Notice this operation required two atomic operations, a swap and a store. There can
    /// be delay between the two due to OS scheduling. In order the fix this, the dequeue_all()
    /// method use wait_until_end_id() to check consistency and wait until it is true. Ths fix
    /// the delay issue betweeen the two atomic operations.
    pub fn insert_item(&mut self,mut item: SharedPtrBox<MPSCFItem>) {
       //errors
        debug_assert!(!item.is_null());
        debug_assert!(item.get_addr() % mem::size_of::<MPSCFItem>() == 0);
        
        //this is the new last element, so next is NULL
        item.get_mut().next.set_null();

        //update tail with swap first
        let prev = self.tail.swap(item.get_ptr_mut(),Ordering::Relaxed);
        
        //Then update head if required or update prev->next
        //This operation didn't required atomic ops as long as we are aligned in memory
        if prev.is_null() {
            //in theory atomic isn't required for this write otherwise we can do atomic write
            self.head.store(item.get_ptr_mut(),Ordering::Relaxed);
        } else {
            unsafe{(&mut *prev)}.next = item;
        }
    }

    //this is used to fix issue with insert, as we update tail, then setup tail.next
    //there is a chance that the thread was interupted inbetween the two operation
    //so this functinon wait the real tail match with expected tail so we are sure
    //operation was done.
    fn wait_until_end_id(head: * mut MPSCFItem, expected_tail: * mut MPSCFItem) {
        //vars
        let mut current = head;

        //errors
        debug_assert!(!current.is_null());
        debug_assert!(!expected_tail.is_null());

        //loop until we find tail
        while current != expected_tail {
            let cur = unsafe{&mut *current};
            if cur.next.is_null() {
                while false {};
            } else {
                current = cur.next.get_ptr_mut();
            }
        }

        //check that we have effectively the last element otherwise it's a bug.
        let cur = unsafe{&mut *current};
        debug_assert!(cur.next.is_null());
    }

    /// Flush all the elemnt into the list and return the first one, then the user has to go througth
    /// all of them to make the required operation.
    /// 
    /// This must be called by a uniq thread (or protected by lock).
    pub fn dequeue_all(&mut self) -> Option<SharedPtrBox<MPSCFItem>> {
        // read head and mark it as NULL
        let head = self.head.load(Ordering::Relaxed);

        //if has entry, need to clear the current list
        if !head.is_null() {
            /* Mark head as empty, in theory it's ok without atomic write here.
            At register time, head is write only for first element.
            as we have one, produced work only on tail.
            We will flush tail after this, so it's ok with cache coherence if the two next
            ops are not reorder.*/
            //TODO we should check if not require SeqCst or Acquire
            self.head.store(ptr::null_mut(),Ordering::Relaxed);
            //OPA_write_barrier();

            //swap tail to make it NULL
            let tail = self.tail.swap(ptr::null_mut(),Ordering::Relaxed);

            //we have head, so NULL tail is abnormal
            debug_assert!(!tail.is_null());

            /* walk on the list until last element and check that it was
            tail, otherwise, another thread is still in registering the tail entry
            but didn't finish to setup ->next, so wait unit next became tail*/
            Self::wait_until_end_id(head,tail);

           /* now we can return */
            Some(SharedPtrBox::new_ptr(head))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests
{
	extern crate std;

    use common::mpscf_queue::*;
    use portability::osmem;
    use core::sync::atomic::AtomicBool;
    use self::std::{thread, time};

    #[test]
    fn basic() {
        //setup items
        let mut items = [MPSCFItem::new(); 8];

        //insert all
        let mut queue = MPSCFQueue::new();
        assert_eq!(queue.is_empty(), true);
        for i in 0..items.len() {
            queue.insert_item(SharedPtrBox::new_ref_mut(&mut items[i]));
        }
        assert_eq!(queue.is_empty(), false);

        //dequeue
        let handler = queue.dequeue_all();
        assert_eq!(queue.is_empty(), true);

        //check all
        let mut next = handler.unwrap();
        let mut i = 0;
        while !next.is_null() {
            assert_eq!(next.get_ptr(), &items[i] as * const MPSCFItem);
            next = next.next;
            i += 1;
        }
    }

    #[test]
    fn threads() {
        let mut handlers = std::vec::Vec::new();
		let threads = 8;
        const INSERT: usize = 500;

        //shared elements
        let mut rlist = MPSCFQueue::new();
        let list = SharedPtrBox::new_ref_mut(&mut rlist);
        let mut rcnt: usize = 0;
        let cnt = SharedPtrBox::new_ref_mut(&mut rcnt);

        //to track finish
        let mut run = AtomicBool::new(true);

        //threads pushing
		for _ in 0..threads {
			let mut list = list.clone();
			let handler = std::thread::spawn(move|| {
                for _ in 0..INSERT {
                    let addr = osmem::mmap(0,4096);
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+1*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+2*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+3*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+4*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+5*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+6*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr+7*512));
                    list.get_mut().insert_item(SharedPtrBox::new_addr(addr));
                }
			});
			handlers.push(handler);
		}

        //on thread pulling
        let ccnt = cnt.clone();
        let mut clist = list.clone();
        let ccrun = SharedPtrBox::new_ref_mut(&mut run);
        let mut crun = ccrun.clone();
        let fhandler = std::thread::spawn(move|| {
            let crun = crun.get_mut();
            let mut ccnt = ccnt.clone();
            while crun.load(Ordering::Relaxed) {
                //check
                let handler = clist.get_mut().dequeue_all();
                match handler {
                    Some(handler) => {
                        let mut next = handler;
                        while !next.is_null() {
                            let mut tmp = next.next;
                            let addr = next.get_addr();
                            if addr % 4096 == 0 {
                                osmem::munmap(addr,4096);
                            }
                            next = tmp;
                            *ccnt.get_mut() += 1;
                        }
                    },
                    None => {}
                } 
            }   
        });

        //wait all
		for handler in handlers {
			let _ = handler.join();
		}

        let delay = time::Duration::from_millis(10);
        thread::sleep(delay);
        run.store(false, Ordering::Relaxed);

        let _ = fhandler.join();

        assert_eq!(rcnt, threads * INSERT * 8);
    }
}
