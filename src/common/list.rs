/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module implement a double link list by using a list node stored
///into the objects we want to chain. This is to be efficient an use the
///available memory by placing the header inside the memory we want to track.

//import
use common::shared::SharedPtrBox;
use core::marker::PhantomData;
use core::iter::Iterator;

///Basic list node header to be embedded into the object to chain as a list
pub struct ListNode {
	prev: Option<SharedPtrBox<ListNode>>,
	next: Option<SharedPtrBox<ListNode>>,
}

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

pub struct ListIterator<'a,T> {
	root: &'a ListNode,
	cur: SharedPtrBox<ListNode>,
	phantom: PhantomData<T>,
}

pub struct List<T> 
	where T: Listable<T>
{
	root: ListNode,
	phantom: PhantomData<T>,
}

impl <'a,T> ListIterator<'a,T> 
	where T: Listable<T>
{
	fn new(list:&'a List<T>) -> Self {
		Self {
			root: &list.root,
			cur: list.root.next.as_ref().unwrap().clone(),
			phantom: PhantomData,
		}
	}
}


impl <'a,T> Iterator for ListIterator<'a,T> 
	where T: Listable<T> + 'a
{
	type Item = SharedPtrBox<T>;

	fn next(&mut self) -> Option<SharedPtrBox<T>> {
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

impl ListNode {
	pub fn new() -> Self {
		Self {
			prev: None,
			next: None,
		}
	}

	pub fn init_as_loop(&mut self) {
		self.prev = Some(SharedPtrBox::new_ref_mut(self));
		self.next = Some(SharedPtrBox::new_ref_mut(self));
	}

	pub fn init_as_none(&mut self) {
		self.prev = None;
		self.next = None;
	}

	pub fn is_none(&self) -> bool {
		self.prev.is_none() || self.next.is_none()
	}

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

	pub fn extract_from_list(&mut self) {
		//update prev
		self.prev.as_mut().unwrap().get_mut().next = self.next.clone();
		self.next.as_mut().unwrap().get_mut().prev = self.prev.clone();

		//loop
		self.init_as_loop();
	}
}

impl <T> List<T> 
	where T: Listable<T>
{
	pub fn new() -> Self {
		Self {
			root: ListNode::new(),
			phantom: PhantomData,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.root.is_loop() || self.root.is_none()
	}

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

	pub fn do_hard_checking(&self) {
		if !self.is_empty() {
			let mut cur = &self.root;
			loop {
				//check
				let pcur = cur as * const ListNode;
				let pnext = cur.next.as_ref().unwrap().get_ptr();
				let pprev = cur.prev.as_ref().unwrap().get_ptr();
				assert!(pprev == pcur);
				assert!(pnext == cur);

				//move
				cur = &cur.next.as_ref().unwrap().get();
				//exit loop
				if cur as * const ListNode == &self.root as * const ListNode {
					break;
				}
			}
		}
	}

	/*pub fn remove(&mut self, item: & mut T) {
		//get node of new item
		let item = item.get_list_node_mut();

		//update prev
		item.extract_from_list();
	}*/

	pub fn front(&self) -> Option<&T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_ref().unwrap().get();
			Some(<T>::get_from_list_node_ref(node))
		}
	}

	pub fn front_mut(&mut self) -> Option<& mut T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_mut().unwrap().get_mut();
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(<T>::get_from_list_node_ref_mut(node as * mut ListNode))
		}
	}

	pub fn back(&self) -> Option<&T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_ref().unwrap().get();
			Some(<T>::get_from_list_node_ref(node))
		}
	}

	pub fn back_mut(&mut self) -> Option<&mut T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_mut().unwrap().get_ptr() as * mut ListNode;
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(<T>::get_from_list_node_ref_mut(node))
		}
	}

	pub fn pop_front(&mut self) -> Option<SharedPtrBox<T>> {
		let ret = self.front_mut();
		match ret {
			Some(x) => {x.get_list_node_mut().extract_from_list(); return Some(SharedPtrBox::new_ref_mut(x));}
			None => None
		}
	}

	pub fn pop_back(&mut self) -> Option<SharedPtrBox<T>> {
		let ret = self.back_mut();
		match ret {
			Some(x) => {x.get_list_node_mut().extract_from_list(); return Some(SharedPtrBox::new_ref_mut(x));}
			None => None
		}
	}

	pub fn iter(&self)-> ListIterator<T> {
		ListIterator::new(self)
	}
	//TODO 
	//Iterator
}

#[cfg(test)]
mod tests
{
	use common::list::*;
	use common::types::*;
	use portability::osmem;

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
}