
use moving_gc_arena as gc;

fn main() {
    let mut r = gc::Region::new();

    #[derive(Debug)]
    struct Obj {
        ix: Vec<gc::Ix<Obj>>
    }
    impl Obj {
        pub fn new() -> Self {
            Obj {
                ix: Vec::new()
            }
        }
    }
    impl gc::HasIx<Obj> for Obj {
        fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, f: F) where
            F: FnMut(&'b mut gc::Ix<Obj>)
        {
            self.ix.iter_mut().for_each(f)
        }
    }

    let ex1 = r.alloc(|_| {Obj::new()}).to_root();
    let ex2 = r.alloc(|_| {Obj { ix: vec![ex1.ix()] }}).weak();
    ex1.get_mut(&mut r).ix = vec![ex2.ix(), ex1.ix()];

    let ex3 = r.alloc(|_| {Obj { ix: vec![ex2.ix(), ex1.ix()] }}).to_root();

    std::mem::drop(ex1);
    {
        let mut v = Vec::new();
        for _ in 1..60 {
            v.clear();
            for _ in 1..100 {
                for _ in 1..1000 {
                    let root = r.alloc(|_|{Obj { ix: vec![ex2.ix()] }}).to_root();
                    v.push(root);
                }
                let i = v.get(v.len()-1).unwrap().ix();
                for root in &mut v {
                    root.get_mut(&mut r).ix.push(i);
                }
                ex3.get_mut(&mut r).ix.push(i);

                v.drain(20..70);
                println!("{}", v.len());
            }
            r.gc();
            ex3.get_mut(&mut r).ix.pop();
            for root in &v {
                let e = root.get_mut(&mut r);
                if e.ix.len() >= 5 {
                    e.ix.pop();
                    e.ix.pop();
                    e.ix.extend(v.get(0).map(gc::Ex::ix));
                }
            }
        }
    }

    r.gc();
    println!("{:?} -> {:?} []-> {:?}", ex3, ex3.get(&r), ex3.get(&r).ix.get(0).unwrap().get(&r));

    ex2.get_mut(&mut r).ix = vec![];

    r.gc();
    println!("{:?} -> {:?} []-> {:?}", ex3, ex3.get(&r), ex3.get(&r).ix.get(0).unwrap().get(&r));
}
