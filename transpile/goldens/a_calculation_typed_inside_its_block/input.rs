// A closure's captures are prepared in a block above it, and only from inside
// that block does the closure say what it answers: at the call, `reader` names
// nothing and the body has no type. A refusal that answered `()` made the
// calculation hold a unit, so the tail below lost its `return` and the engine
// reported its own gap as a contradiction in the program.

pub struct Source {
    pub n: u64,
}

pub struct Reader {
    pub n: u64,
}

impl Source {
    pub fn new(n: u64) -> Source {
        Source { n }
    }

    pub fn reader(&self) -> Reader {
        Reader { n: self.n }
    }
}

impl Reader {
    pub fn get(&self) -> u64 {
        self.n
    }
}

pub struct Calc<T> {
    pub compute: Box<dyn Fn() -> T>,
}

impl<T: 'static> Calc<T> {
    pub fn make<F>(compute: F) -> Calc<T>
    where F: Fn() -> T + 'static {
        Calc { compute: Box::new(compute) }
    }

    pub fn read(&self) -> T {
        (self.compute)()
    }
}

pub fn doubled() -> u64 {
    let source = Source::new(3);
    let calc = Calc::make({
        let reader = source.reader();
        move || reader.get() * 2
    });
    calc.read()
}
