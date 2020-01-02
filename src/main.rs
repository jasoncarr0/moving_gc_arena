

mod gc;

fn main() {
    println!("Reference {}, Ix: {}, Ex: {}, Entry nz: {}, Spot nz: {}, Option<Entry>: {}",
             std::mem::size_of::<&usize>(),
             std::mem::size_of::<gc::Ix<()>>(),
             std::mem::size_of::<gc::Ex<()>>(),
             std::mem::size_of::<gc::Entry<&usize>>(),
             std::mem::size_of::<gc::Spot<&usize>>(),
             std::mem::size_of::<Option<gc::Entry<&usize>>>(),
             );

    impl gc::HasIx<i64> for i64 { }
    let mut r = gc::Region::new();
    let ix = r.alloc(0).ix();
    let ex = r.alloc(1).listen();
    println!("{:?} {:?}", ix.get(&r), ex.get(&r));
}


    //let mut a = A {};
    //a.get({a.put2(); 0});//.forget());
    //a.put(a.get(0));
    //a.get(a.put(0));
    
/*
use std::marker::PhantomData;
struct A { }
struct B<'a> { _b: PhantomData<&'a ()> }
impl A {
    fn get(&self, i: i32) -> i32 { i }
    fn put(&mut self, i: i32) -> i32 { i }
    fn x<S, T>(s: S, t: T) { }
}
*/
