/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This module implement a double link list by using a list node stored
/// into the objects we want to chain. This is to be efficient an use the
/// available memory by placing the header inside the memory we want to track.
/// 
/// This list use a SharedPtrBox to track the pointer (which cannot be null, so we use Option).
/// This is a way to bypass all the mutability and ownership as we handle the lifetime of the chunks
/// inside the allocator itself.

//import
use common::shared::SharedPtrBox;
use core::marker::PhantomData;
use core::iter::Iterator;

/// Basic list node header to be embedded into the object to chain as a list
pub struct ListNode {
	prev: Option<SharedPtrBox<ListNode>>,
	next: Option<SharedPtrBox<ListNode>>,
}

/// Trait to be implemented by the structs which can be placed into that link list.
/// This mostly implement access to the local ListNode field to contain next/prev
/// and to come back from this to the original object.
pub trait Listable<T> {
	fn get_list_node<'a>(&'a self) -> &'a ListNode;
	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode;
	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const T;
	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut T;

	fn get_from_list_node_ref<'a>(elmt: * const ListNode) -> &'a T {
		unsafe{&*Self::get_from_list_node(elmt)}
	}

	fn get_from_list_node_ref_mut<'a>(elmt: * mut ListNode) -> &'a mut T {
		unsafe{&mut *Self::get_from_list_node_mut(elmt)}
	}
}

/// Implement iterator to loop over the list.
pub struct ListIterator<'a,T> {
	root: &'a ListNode,
	cur: SharedPtrBox<ListNode>,
	phantom: PhantomData<T>,
}

/// Define the list which mostly consist in a root element.
pub struct List<T> 
	where T: Listable<T>
{
	root: ListNode,
	phantom: PhantomData<T>,
}

/// Implement the iterator.
impl <'a,T> ListIterator<'a,T> 
	where T: Listable<T>
{
	/// Create a new iterator, argument is the list to iterate over.
	fn new(list:&'a List<T>) -> Self {
		let cur;

		match list.root.next.as_ref() {
			None    => cur = SharedPtrBox::new_null(),
			Some(x) => cur = x.clone()
		}

		Self {
			root: &list.root,
			cur: cur,
			phantom: PhantomData,
		}
	}
}

/// Implement the iterator interface so we can use the for loop.
impl <'a,T> Iterator for ListIterator<'a,T> 
	where T: Listable<T> + 'a
{
	/// Define item to return.
	type Item = SharedPtrBox<T>;

	/// Return the next element or None if at the end of list.
	fn next(&mut self) -> Option<SharedPtrBox<T>> {
		//empty list
		if self.cur.is_null() {
			return None;
		}

		//check if end
		let pcur = self.cur.get_ptr();
		let proot = self.root as * const ListNode;

		if pcur == proot {
			None
		} else {
			let cur = self.cur.clone();
			self.cur = cur.get().next.as_ref().unwrap().clone();
			Some(SharedPtrBox::new_ptr(<T>::get_from_list_node(cur.get_ptr())))
		}
	}
}

/// Implement a list node.
impl ListNode {
	/// Create a new list node which point None on both ext and prev.
	pub fn new() -> Self {
		Self {
			prev: None,
			next: None,
		}
	}

	/// Setup the element as a loop where next and prev point to self. This make the life
	/// easier for insertion/removal operation in the list.
	pub fn init_as_loop(&mut self) {
		self.prev = Some(SharedPtrBox::new_ref_mut(self));
		self.next = Some(SharedPtrBox::new_ref_mut(self));
	}

	/// Make it as none as if it is a fresh node.
	pub fn init_as_none(&mut self) {
		self.prev = None;
		self.next = None;
	}

	/// Check if is none.
	pub fn is_none(&self) -> bool {
		self.prev.is_none() || self.next.is_none()
	}

	/// Check if is a look to itself.
	pub fn is_loop(&self) -> bool {
		if self.prev.is_some() && self.next.is_some() {
			let pprev = self.prev.as_ref().unwrap().get_ptr();
			let pnext = self.next.as_ref().unwrap().get_ptr();
			let pself = self as * const ListNode;
			if pprev == pnext && pprev == pself {
				true
			} else {
				false
			}
		} else {
			false
		}
	}

	/// Remove the element from the list it belong to.
	pub fn extract_from_list(&mut self) -> Option<SharedPtrBox<ListNode>> {
		let ret;
		
		{
			//unwrap
			let prev = self.prev.as_mut().unwrap();
			let next = self.next.as_mut().unwrap();

			//Extract
			if prev.get_addr() == next.get_addr() {
				ret = Some(prev.clone());
			} else {
				ret = None;
			}

			//update prev
			prev.get_mut().next = Some(next.clone());
			next.get_mut().prev = Some(prev.clone());
		}

		//loop
		self.init_as_loop();

		ret
	}
}

/// To be able to ini an array of list for MediumFreePool
/// This can be done only on empty list
impl Copy for ListNode { }

/// To be able to ini an array of list for MediumFreePool
/// This can be done only on empty list
impl Clone for ListNode {
    fn clone(&self) -> Self {
		assert!(self.is_none());
        Self::new()
    }
}

/// To be able to ini an array of list for MediumFreePool
/// This can be done only on empty list
impl <T> Copy for List<T>
	where T: Listable<T>
{}

/// To be able to ini an array of list for MediumFreePool
/// This can be done only on empty list
impl <T> Clone for List<T> 
	where T: Listable<T>
{
    fn clone(&self) -> Self {
		assert!(self.is_empty());
        Self::new()
    }
}

/// Implement the list operations.
impl <T> List<T> 
	where T: Listable<T>
{
	/// Create an empty list.
	pub fn new() -> Self {
		Self {
			root: ListNode::new(),
			phantom: PhantomData,
		}
	}

	/// Check if the list is empty.
	pub fn is_empty(&self) -> bool {
		self.root.is_loop() || self.root.is_none()
	}

	/// Insert the given element at the end of the list.
	pub fn push_back(&mut self, item: SharedPtrBox<T>) {
		//check
		assert!(!item.is_null());

		//get node of new item
		let mut item = item.clone();
		let item = item.get_mut();
		let mut item = item.get_list_node_mut();

		//if list is empty
		if self.is_empty() {
			self.root.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
			self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		}

		//setup prev/next of new item
		item.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		item.prev = self.root.prev.clone();

		//insert
		self.root.prev.as_mut().unwrap().get_mut().next = Some(SharedPtrBox::new_ref_mut(&mut item));
		self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut item));
	}

	/// Insert the given element at beginning of the list.
	pub fn push_front(&mut self, item: SharedPtrBox<T>) {
		//check
		assert!(!item.is_null());

		//get node of new item
		let mut item = item.clone();
		let item = item.get_mut();
		let mut item = item.get_list_node_mut();

		//if list is empty
		if self.is_empty() {
			self.root.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
			self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		}

		//setup prev/next of new item
		item.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		item.next = self.root.next.clone();

		//insert
		self.root.next.as_mut().unwrap().get_mut().prev = Some(SharedPtrBox::new_ref_mut(&mut item));
		self.root.next = Some(SharedPtrBox::new_ref_mut(&mut item));
	}

	/// Used to hard check the elements of the list to detect bugs.
	pub fn hard_checking(&self) {
		if !self.is_empty() {
			let mut cur = &self.root;
			loop {
				//check
				let pcur = cur as * const ListNode;
				let pnext = cur.next.as_ref().unwrap().prev.as_ref().unwrap().get_ptr();
				let pprev = cur.prev.as_ref().unwrap().next.as_ref().unwrap().get_ptr();
				assert!(pprev == pcur);
				assert!(pnext == pcur);

				//move
				cur = &cur.next.as_ref().unwrap().get();
				//exit loop
				if cur as * const ListNode == &self.root as * const ListNode {
					break;
				}
			}
		}
	}

	/// Remove the given element from the list.
	pub fn remove(item: &mut SharedPtrBox<T>) -> Option<SharedPtrBox<Self>> {
		//get node of new item
		let tmp = item.get_mut();
		let elt = tmp.get_list_node_mut();

		//update prev
		//TODO maybe make debug_assert
		if !elt.is_none() && !elt.is_loop() {
			let node = elt.extract_from_list();
			match node {
				Some(x) => Some(SharedPtrBox::new_addr(x.get_addr())),
				None => None,
			}
		} else {
			None
		}
	}

	/// Return the first element of the list.
	pub fn front(&self) -> Option<SharedPtrBox<T>> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_ref().unwrap().get();
			Some(SharedPtrBox::new_ref(<T>::get_from_list_node_ref(node)))
		}
	}

	/// Return the first element of a mutable list.
	/// TODO this is not really usefull as we return a SharedPtrBox which bypass all mut checks.
	pub fn front_mut(&mut self) -> Option<SharedPtrBox<T>> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_mut().unwrap().get_mut();
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(SharedPtrBox::new_ref_mut(<T>::get_from_list_node_ref_mut(node)))
		}
	}

	/// Return the last element of the list.
	pub fn back(&self) -> Option<SharedPtrBox<T>> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_ref().unwrap().get();
			Some(SharedPtrBox::new_ref(<T>::get_from_list_node_ref(node)))
		}
	}

	/// Return the last elemnt of a mutable list
	/// TODO this is not really usefull as we return a SharedPtrBox which bypass all mut checks.
	pub fn back_mut(&mut self) -> Option<SharedPtrBox<T>> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_mut().unwrap().get_ptr() as * mut ListNode;
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(SharedPtrBox::new_ref_mut(<T>::get_from_list_node_ref_mut(node)))
		}
	}

	/// Extract and return the first element of a list.
	pub fn pop_front(&mut self) -> Option<SharedPtrBox<T>> {
		let ret = self.front_mut();
		match ret {
			Some(mut  x) => {T::get_list_node_mut(x.get_mut()).extract_from_list(); return Some(x);}
			None => None
		}
	}

	/// Extract and return the last element of a list.
	pub fn pop_back(&mut self) -> Option<SharedPtrBox<T>> {
		let ret = self.back_mut();
		match ret {
			Some(mut x) => {T::get_list_node_mut(x.get_mut()).extract_from_list(); return Some(x);}
			None => None
		}
	}

	/// Return an iterator to iterate over the list.
	pub fn iter(&self)-> ListIterator<T> {
		ListIterator::new(self)
	}
}

#[cfg(test)]
mod tests
{
	use common::list::*;
	use common::types::*;
	use portability::osmem;
	use core::mem;

	struct Fake {
		node: ListNode,
		pub value: i32,
	}

	impl Fake {
		fn new(value:i32) -> Self {
			Self {
				node: ListNode::new(),
				value:value,
			}
		}

		fn new_mem(addr:Addr, value:i32) -> SharedPtrBox<Fake> {
			let ptr = addr as * mut Fake;
			let fake = unsafe{ & mut *ptr };
			fake.value = value;
			SharedPtrBox::new_ptr(ptr)
		}
	}

	impl Listable<Fake> for Fake {
		fn get_list_node<'a>(&'a self) -> &'a ListNode {
			&self.node
		}

		fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
			&mut self.node
		}

		fn get_from_list_node<'a>(elmt: * const ListNode) -> * const Fake {
			unsafe{&*(elmt as * const ListNode as Addr as * const Fake)}
		}

		fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut Fake {
			unsafe{&mut *(elmt as * mut ListNode as Addr as * mut Fake)}
		}
	}
	#[test]
	fn size() {
		//TODO this should ideally be 16
		//assert_eq!(mem::size_of::<ListNode>(), 16);
		assert_eq!(mem::size_of::<ListNode>(), 32);
	}

	#[test]
	fn basic_empty_list_elmnt() {
		let mut el1 = ListNode::new();
		assert_eq!(el1.is_loop(), false);

		el1.init_as_loop();
		assert_eq!(el1.is_loop(), true);
	}

	#[test]
	fn basic_empty_list() {
		let el1: List<Fake> = List::new();
		assert_eq!(el1.is_empty(), true);
	}

	#[test]
	fn push_front() {
		let mut el1: List<Fake> = List::new();
		let v1 = Fake::new(10);
		el1.push_front(SharedPtrBox::new_ref(&v1));
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,10);

		let v2 = Fake::new(11);
		el1.push_front(SharedPtrBox::new_ref(&v2));
		assert_eq!(el1.front().unwrap().value,11);
		assert_eq!(el1.back().unwrap().value,10);
	}

	#[test]
	fn push_back() {
		let mut el1: List<Fake> = List::new();
		let v1 = Fake::new(10);
		el1.push_front(SharedPtrBox::new_ref(&v1));
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,10);

		let v2 = Fake::new(11);
		el1.push_back(SharedPtrBox::new_ref(&v2));
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,11);
	}

	#[test]
	fn pop_front() {
		let mut el1: List<Fake> = List::new();

		let v1 = Fake::new(10);
		el1.push_front(SharedPtrBox::new_ref(&v1));

		let v2 = Fake::new(11);
		el1.push_front(SharedPtrBox::new_ref(&v2));

		{
			let e3 = el1.pop_front().unwrap();
			assert_eq!(e3.value, 11);
		}

		//checl list
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,10);
	}

	#[test]
	fn iterator_empty() {
		let el1: List<Fake> = List::new();
		for _ in el1.iter() {
			//should not be called
			assert!(false);
		}
	}

	#[test]
	fn iterator() {
		let mut el1: List<Fake> = List::new();

		let v1 = Fake::new(0);
		el1.push_back(SharedPtrBox::new_ref(&v1));

		let v2 = Fake::new(1);
		el1.push_back(SharedPtrBox::new_ref(&v2));

		//loop
		for (i,v) in el1.iter().enumerate() {
			assert_eq!(v.value, i as i32);
		}
	}

	#[test]
	fn with_dynamic_alloc() {
		let mut list: List<Fake> = List::new();
		for i in 0..10 {
			let ptr = osmem::mmap(0,4096);
			let fake = Fake::new_mem(ptr, i);
			list.push_back(fake);
		}

		for i in 0..10 {
			let v = list.pop_front().unwrap();
			assert_eq!(v.value, i);
			osmem::munmap(v.get_addr(), 4096);
		}
	}
	
	#[test]
	fn remove() {
		let mut list: List<Fake> = List::new();

		let v1 = Fake::new(0);
		list.push_back(SharedPtrBox::new_ref(&v1));

		let v2 = Fake::new(1);
		list.push_back(SharedPtrBox::new_ref(&v2));

		let v3 = Fake::new(10);
		list.push_back(SharedPtrBox::new_ref(&v3));

		let mut ret = list.back().unwrap();
		let lst = List::remove(&mut ret);
		assert_eq!(lst.is_none(), true);

		//loop
		for (i,v) in list.iter().enumerate() {
			assert_eq!(v.value, i as i32);
		}

		let mut ret = list.back().unwrap();
		let lst = List::remove(&mut ret);
		assert_eq!(lst.is_none(), true);

		let mut ret = list.back().unwrap();
		let lst = List::remove(&mut ret);
		assert_eq!(lst.unwrap().get_ptr(), &list as * const List<Fake>);
	}
}