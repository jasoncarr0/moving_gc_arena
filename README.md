
# Recent changes

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
      if it's been collected
- MutEntry::to_root is now MutEntry::root and no longer takes ownership

### Fixed
- Creating a weak and root pointer to the same entry would cause Weak pointers
to act like roots.

# Moving GC Arena

This is a library for indexed regions supporting efficient garbage collection and (eventually) other traversal operations such as cloning.

You should use this library if you want to keep a safe cyclic graph data structure, with simple, performant garbage collection.
This library does not read the Rust stack, instead, roots are simply acquired resources, which can be used like any other resource. It compiles on stable 2018 Rust.

You should not use this library if you need hard real-time guarantees for allocation enough that Vec is a problem (and can't manage to pre-allocate).

Dereferencing indices uses a reference to the region itself, giving strong safety guarantees. Users are recommended to create
wrappers for traversal if the ergonomics of this gets in the way.

## Details of features and limitations

* Members are a fixed type and size
* Internal indices (gc::Ix) are Copy and Send/Sync
* External indices (gc::Root and gc::Weak) use Rc, so they are not Send/Sync
* Access is guarded by access to the region (that is, dereferencing takes &Region and &mut Region).
* Drop implementations are called as normal whenever an object is collected
* Garbage collection may be performed both automatically and manually. Every resize of the buffer triggers a garbage collection for the best performance.
* Garbage collection uses Cheney's algorithm.
* Size cannot yet be tuned: We always double the size at least. Region::gc will shrink the allocation
