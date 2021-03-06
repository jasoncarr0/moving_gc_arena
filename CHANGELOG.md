# Changelog

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
      if it's been collected
- MutEntry::to_root is now MutEntry::root and no longer takes ownership

### Fixed
- Creating a weak and root pointer to the same entry would cause Weak pointers
to act like roots.

## [0.1.1] - 2020-01-07
### Added
- Ex<T> now is Clone regardless of T
- Cargo.toml links to repository
### Fixed
- lib.rs links to docs correctly

## [0.1.0] - 2020-01-06
Initial release

