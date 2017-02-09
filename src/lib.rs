#![deny(missing_docs)]

//! A string interning data structure that was designed for minimal memory-overhead
//! and fast access to the underlying interned string contents.
//! 
//! Uses a similar interface as the string interner of the rust compiler.
//! 
//! Provides support to use all primitive types as symbols
//! 
//! Example usage:
//! 
//! ```
//! 	use string_interner::StringInterner;
//! 	let mut interner = StringInterner::<usize>::default();
//! 	let name0 = interner.get_or_intern("Elephant");
//! 	let name1 = interner.get_or_intern("Tiger");
//! 	let name2 = interner.get_or_intern("Horse");
//! 	let name3 = interner.get_or_intern("Tiger");
//! 	let name4 = interner.get_or_intern("Tiger");
//! 	let name5 = interner.get_or_intern("Mouse");
//! 	let name6 = interner.get_or_intern("Horse");
//! 	let name7 = interner.get_or_intern("Tiger");
//! 	assert_eq!(name0, 0);
//! 	assert_eq!(name1, 1);
//! 	assert_eq!(name2, 2);
//! 	assert_eq!(name3, 1);
//! 	assert_eq!(name4, 1);
//! 	assert_eq!(name5, 3);
//! 	assert_eq!(name6, 2);
//! 	assert_eq!(name7, 1);
//! ```

#![feature(conservative_impl_trait)]

#![feature(test)]
extern crate test;

extern crate num_traits;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use ::num_traits::{Unsigned, FromPrimitive, ToPrimitive};

/// Represents indices into the StringInterner.
/// 
/// Values of this type shall be lightweight as the whole purpose
/// of interning values is to be able to store them efficiently in memory.
/// 
/// This trait allows definitions of custom InternIndices besides
/// the already supported unsigned integer primitives.
pub trait Symbol: Copy + Unsigned + FromPrimitive + ToPrimitive {}
impl<T> Symbol for T where
	T: Copy + Unsigned + FromPrimitive + ToPrimitive
{} 

/// Internal reference to str used only within the StringInterner itself
/// to encapsulate the unsafe behaviour of interor references.
#[derive(Debug, Copy, Clone, Eq)]
struct InternalStrRef(*const str);

impl InternalStrRef {
	/// Creates an InternalStrRef from a str.
	/// 
	/// This just wraps the str internally.
	fn from_str(val: &str) -> Self {
		InternalStrRef(
			unsafe{ &*(val as *const str) }
		)
	}

	/// Reinterprets this InternalStrRef as a str.
	/// 
	/// This is "safe" as long as this InternalStrRef only
	/// refers to strs that outlive this instance or
	/// the instance that owns this InternalStrRef.
	/// This should hold true for StringInterner.
	/// 
	/// Does not allocate memory!
	fn as_str(&self) -> &str {
		unsafe{ &*self.0 as &str }
	}
}

impl<T> From<T> for InternalStrRef
	where T: AsRef<str>
{
	fn from(val: T) -> Self {
		InternalStrRef::from_str(val.as_ref())
	}
}

impl Hash for InternalStrRef {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_str().hash(state)
	}
}

impl PartialEq for InternalStrRef {
	fn eq(&self, other: &InternalStrRef) -> bool {
		self.as_str() == other.as_str()
	}
}

/// Defaults to using usize as the underlying and internal
/// symbol data representation within this StringInterner.
pub type DefaultStringInterner = StringInterner<usize>;

/// Provides a bidirectional mapping between String stored within
/// the interner and indices.
/// The main purpose is to store every unique String only once and
/// make it possible to reference it via lightweight indices.
/// 
/// Compilers often use this for implementing a symbol table.
/// 
/// The main goal of this StringInterner is to store String
/// with as low memory overhead as possible.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StringInterner<Sym>
	where Sym: Symbol
{
	map   : HashMap<InternalStrRef, Sym>,
	values: Vec<Box<str>>
}

impl<Sym> StringInterner<Sym>
	where Sym: Symbol
{
	/// Creates a new StringInterner with a given capacity.
	pub fn with_capacity(cap: usize) -> Self {
		StringInterner{
			map   : HashMap::with_capacity(cap),
			values: Vec::with_capacity(cap)
		}
	}

	/// Interns the given value.
	/// 
	/// Returns a symbol to access it within this interner.
	/// 
	/// This either copies the contents of the string (e.g. for str)
	/// or moves them into this interner (e.g. for String).
	pub fn get_or_intern<T>(&mut self, val: T) -> Sym
		where T: Into<String> + AsRef<str>
	{
		match self.map.get(&val.as_ref().into()) {
			Some(&idx) => idx,
			None       => self.gensym(val)
		}
	}

	/// Interns the given value and ignores collissions.
	/// 
	/// Returns a symbol to access it within this interner.
	fn gensym<T>(&mut self, new_val: T) -> Sym
		where T: Into<String> + AsRef<str>
	{
		let new_id : Sym            = self.make_symbol();
		let new_ref: InternalStrRef = new_val.as_ref().into();
		self.values.push(new_val.into().into_boxed_str());
		self.map.insert(new_ref, new_id);
		new_id
	}

	/// Creates a new symbol for the current state of the interner.
	fn make_symbol(&self) -> Sym {
		Sym::from_usize(self.len()).unwrap()
	}

	/// Returns a string slice to the string identified by the given symbol if available.
	/// Else, None is returned.
	pub fn get(&self, symbol: Sym) -> Option<&str> {
		self.values
			.get(symbol.to_usize().unwrap())
			.map(|boxed_str| boxed_str.as_ref())
	}

	/// Returns the given value's symbol for this interner if existent.
	pub fn lookup_symbol<T>(&self, val: T) -> Option<Sym>
		where T: AsRef<str>
	{
		self.map
			.get(&val.as_ref().into())
			.map(|&idx| idx)
	}

	/// Returns the number of uniquely stored Strings interned within this interner.
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// Returns an iterator over the interned strings.
	pub fn iter_values<'a>(&'a self) -> impl Iterator<Item=&'a str> {
		self.values.iter().map(|boxed_str| boxed_str.as_ref())
	}

	/// Returns an iterator over all intern indices and their associated strings.
	pub fn iter<'a>(&'a self) -> impl Iterator<Item=(Sym, &'a str)> {
		self.values
			.iter()
			.enumerate()
			.map(|(num, boxed_str)| (Sym::from_usize(num).unwrap(), boxed_str.as_ref()))
	}

	/// Removes all interned Strings from this interner.
	/// 
	/// This invalides all `Symbol` entities instantiated by it so far.
	pub fn clear(&mut self) {
		self.map.clear();
		self.values.clear()
	}
}

#[cfg(test)]
mod tests {
	use ::{DefaultStringInterner, InternalStrRef};

	fn make_dummy_interner() -> (DefaultStringInterner, [usize; 8]) {
		let mut interner = DefaultStringInterner::default();
		let name0 = interner.get_or_intern("foo");
		let name1 = interner.get_or_intern("bar");
		let name2 = interner.get_or_intern("baz");
		let name3 = interner.get_or_intern("foo");
		let name4 = interner.get_or_intern("rofl");
		let name5 = interner.get_or_intern("bar");
		let name6 = interner.get_or_intern("mao");
		let name7 = interner.get_or_intern("foo");
		(interner, [name0, name1, name2, name3, name4, name5, name6, name7])
	}

	#[test]
	fn internal_str_ref() {
		use std::mem;
		assert_eq!(mem::size_of::<InternalStrRef>(), mem::size_of::<&str>());

		let s0 = "Hello";
		let s1 = ", World!";
		let s2 = "Hello";
		let s3 = ", World!";
		let r0 = InternalStrRef::from_str(s0);
		let r1 = InternalStrRef::from_str(s1);
		let r2 = InternalStrRef::from_str(s2);
		let r3 = InternalStrRef::from_str(s3);
		assert_eq!(r0, r2);
		assert_eq!(r1, r3);
		assert_ne!(r0, r1);
		assert_ne!(r2, r3);

		use std::collections::hash_map::DefaultHasher;
		use std::hash::Hash;
		let mut sip = DefaultHasher::new();
		assert_eq!(r0.hash(&mut sip), s0.hash(&mut sip));
		assert_eq!(r1.hash(&mut sip), s1.hash(&mut sip));
		assert_eq!(r2.hash(&mut sip), s2.hash(&mut sip));
		assert_eq!(r3.hash(&mut sip), s3.hash(&mut sip));
	}

	#[test]
	fn intern_str() {
		let (_, names) = make_dummy_interner();
		assert_eq!(names[0], 0);
		assert_eq!(names[1], 1);
		assert_eq!(names[2], 2);
		assert_eq!(names[3], 0);
		assert_eq!(names[4], 3);
		assert_eq!(names[5], 1);
		assert_eq!(names[6], 4);
		assert_eq!(names[7], 0);
	}

	#[test]
	fn intern_string() {
		let mut interner = DefaultStringInterner::default();
		let name_0 = interner.get_or_intern("Hello".to_owned());
		let name_1 = interner.get_or_intern("World".to_owned());
		let name_2 = interner.get_or_intern("I am a String".to_owned());
		let name_3 = interner.get_or_intern("Foo".to_owned());
		let name_4 = interner.get_or_intern("Bar".to_owned());
		let name_5 = interner.get_or_intern("I am a String".to_owned());
		let name_6 = interner.get_or_intern("Next is empty".to_owned());
		let name_7 = interner.get_or_intern("".to_owned());
		let name_8 = interner.get_or_intern("I am a String".to_owned());
		let name_9 = interner.get_or_intern("I am a String".to_owned());
		let name10 = interner.get_or_intern("Foo".to_owned());

		assert_eq!(interner.len(), 7);

		assert_eq!(name_0, 0);
		assert_eq!(name_1, 1);
		assert_eq!(name_2, 2);
		assert_eq!(name_3, 3);
		assert_eq!(name_4, 4);
		assert_eq!(name_5, 2);
		assert_eq!(name_6, 5);
		assert_eq!(name_7, 6);
		assert_eq!(name_8, 2);
		assert_eq!(name_9, 2);
		assert_eq!(name10, 3);
	}

	#[test]
	fn len() {
		let (interner, _) = make_dummy_interner();
		assert_eq!(interner.len(), 5);	
	}

	#[test]
	fn get() {
		let (interner, _) = make_dummy_interner();
		assert_eq!(interner.get(0), Some("foo"));
		assert_eq!(interner.get(1), Some("bar"));
		assert_eq!(interner.get(2), Some("baz"));
		assert_eq!(interner.get(3), Some("rofl"));
		assert_eq!(interner.get(4), Some("mao"));
		assert_eq!(interner.get(5), None);
	}

	#[test]
	fn lookup_symbol() {
		let (interner, _) = make_dummy_interner();
		assert_eq!(interner.lookup_symbol("foo"),  Some(0));
		assert_eq!(interner.lookup_symbol("bar"),  Some(1));
		assert_eq!(interner.lookup_symbol("baz"),  Some(2));
		assert_eq!(interner.lookup_symbol("rofl"), Some(3));
		assert_eq!(interner.lookup_symbol("mao"),  Some(4));
		assert_eq!(interner.lookup_symbol("xD"),   None);
	}

	#[test]
	fn clear() {
		let (mut interner, _) = make_dummy_interner();
		assert_eq!(interner.len(), 5);
		interner.clear();
		assert_eq!(interner.len(), 0);
	}

	#[test]
	fn string_interner_iter_values() {
		let (interner, _) = make_dummy_interner();
		let mut it = interner.iter_values();
		assert_eq!(it.next(), Some("foo"));
		assert_eq!(it.next(), Some("bar"));
		assert_eq!(it.next(), Some("baz"));
		assert_eq!(it.next(), Some("rofl"));
		assert_eq!(it.next(), Some("mao"));
		assert_eq!(it.next(), None);
	}

	#[test]
	fn string_interner_iter() {
		let (interner, _) = make_dummy_interner();
		let mut it = interner.iter();
		assert_eq!(it.next(), Some((0, "foo")));
		assert_eq!(it.next(), Some((1, "bar")));
		assert_eq!(it.next(), Some((2, "baz")));
		assert_eq!(it.next(), Some((3, "rofl")));
		assert_eq!(it.next(), Some((4, "mao")));
		assert_eq!(it.next(), None);
	}

    use test::Bencher;

	#[bench]
	fn bench_intern_same(bencher: &mut Bencher) {
		let (mut interner, _) = make_dummy_interner();
		bencher.iter(|| interner.get_or_intern("foo"))
	}

	#[bench]
	fn bench_intern_list(bencher: &mut Bencher) {
		let (mut interner, _) = make_dummy_interner();
		let names = &[
			"Aaren",
			"Aarika",
			"Abagael",
			"Abagail",
			"Abbe",
			"Abbey",
			"Abbi",
			"Abbie",
			"Abby",
			"Abbye",
			"Abigael",
			"Abigail",
			"Abigale",
			"Abra",
			"Ada",
			"Adah",
			"Adaline",
			"Adan",
			"Adara",
			"Adda",
			"Addi",
			"Addia",
			"Addie",
			"Addy",
			"Adel",
			"Adela",
			"Adelaida",
			"Adelaide",
			"Adele",
			"Adelheid",
			"Adelice",
			"Adelina",
			"Adelind",
			"Adeline",
			"Adella",
			"Adelle",
			"Adena",
			"Adey",
			"Adi",
			"Adiana",
			"Adina",
			"Adora",
			"Adore",
			"Adoree",
			"Adorne",
			"Adrea",
			"Adria",
			"Adriaens",
			"Adrian",
			"Adriana",
			"Adriane",
			"Adrianna",
			"Adrianne",
			"Adriena",
			"Adrienne",
			"Aeriel",
			"Aeriela",
			"Aeriell",
			"Afton",
			"Ag",
			"Agace",
			"Agata",
			"Agatha",
			"Agathe",
			"Aggi",
			"Aggie",
			"Aggy",
			"Agna",
			"Agnella",
			"Agnes",
			"Agnese",
			"Agnesse",
			"Agneta",
			"Agnola",
			"Agretha",
			"Aida",
			"Aidan",
			"Aigneis",
			"Aila",
			"Aile",
			"Ailee",
			"Aileen",
			"Ailene",
			"Ailey",
			"Aili",
			"Ailina",
			"Ailis",
			"Ailsun",
			"Ailyn",
			"Aime",
			"Aimee",
			"Aimil",
			"Aindrea",
			"Ainslee",
			"Ainsley",
			"Ainslie",
			"Ajay",
			"Alaine",
			"Alameda",
			"Alana"];
		bencher.iter(|| {
			for &name in names.iter() {
				interner.get_or_intern(name);
			}
		})
	}
}
