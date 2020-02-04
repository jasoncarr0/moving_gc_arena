
# Migrating to Gitlab: https://gitlab.com/jcarr0/moving_gc_arena

# Recent changes

## [0.2.1] - 2020-01-24

### Added
- The feature "packed-headers" can be enabled for an experimental object layout featuring reduced header size.
- Further improved documentation.

### Changed
- Deprecated the confusing MutEntry::{as_ref, as_mut_ref}; Use {get, get_mut} instead

## [0.2.0] - 2020-01-11
### Added
- Added an optional feature to enable debugging index validity at the cost of efficiency
    + Accesses to regions will be checked and give correct errors
    + Validity will be checked during GC
    + This has a dramatic increase in space cost, and a small overhead in time.
- Ix<T> supports an identifier method, which returns a usize,
    unique for the current region/generation
- Added and improved documentation in several places

### Changed
- Separated Weak and Root into two different types.
    + Weak.ix() now returns Option<Ix<T>>, which can be used to test
      if it's been collected. (Root's methods are unchanged)
- MutEntry::to_root is now MutEntry::root and no longer takes ownership

### Fixed
- Creating a weak and root pointer to the same entry would cause Weak pointers
to act like roots.

# Moving GC Arena

This is a library for indexed regions supporting efficient garbage collection and (eventually) other traversal operations such as cloning.

You should use this library if you want to keep a safe cyclic graph data structure, with simple, performant garbage collection.
This library does not read the Rust stack, instead, roots are acquired resources, which can be used like any other resource and dropped as normal. It compiles on stable 2018 Rust and contains only minimal unsafe code.

You should not use this library if you need hard real-time guarantees for allocation enough that Vec is a problem (and can't manage to pre-allocate). In the current version, only single-threaded use is possible.

Dereferencing indices uses a reference to the region, giving strong safety guarantees. Users are recommended to create wrappers for traversal if the ergonomics of this gets in the way.

## Details of features and limitations

* Members are a fixed type and size
* Regions and External indices (gc::Root and gc::Weak) use Rc, so they are not Send/Sync
* Internal indices (gc::Ix) are Copy and Send/Sync
* Access is guarded by access to the region (that is, dereferencing takes &Region and &mut Region).
* Drop implementations are called as normal (if necessary) whenever an object is collected
* Garbage collection may be performed both automatically and manually. Every resize of the buffer triggers a garbage collection for the best performance.
* Garbage collection uses Cheney's algorithm.
* Size cannot yet be tuned: We always double the size at least. Region::gc will shrink the allocation

## Example Usage

```rust
use moving_gc_arena as gc;

let mut r = gc::Region::new();

struct Adj(Vec<gc::Ix<Adj>>);
impl gc::HasIx<Adj> for Adj {
 fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
     F: FnMut(&'b mut gc::Ix<T>)
 {
     self.0.foreach_ix(f);
 }
}
impl Adj {
    fn new() -> Self {
        Adj(Vec::new())
    }
}

let mut obj1 = r.alloc(|_|{Adj::new()}).root();
let mut obj2 = r.alloc(|_|{Adj::new()}).root();
let mut obj3 = r.alloc(|_|{Adj::new()}).root();

// mutual cycle
obj1.get_mut(&mut r).0.push(obj2.ix());
obj2.get_mut(&mut r).0.push(obj1.ix());

// self-cycle
obj3.get_mut(&mut r).0.push(obj3.ix());

std::mem::drop(obj1);
std::mem::drop(obj3);

r.gc(); // manually-triggered collection
//obj3 now collected but obj1 and obj2 are live
```
