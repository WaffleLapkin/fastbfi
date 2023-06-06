use crate::compiler::{
    Addr,
    Inst::{self, *},
    ADDR_SIZE,
};

#[allow(non_snake_case)]
pub fn Interpreter<'bc, 'io>(
    bc: &'bc [u8],
    input: &'io mut dyn FnMut() -> u8,
    output: &'io mut dyn FnMut(u8),
) -> Interpreter<'bc, 'io> {
    Interpreter {
        bc,
        cursor: 0,
        ptr: 0,
        data: Vec::with_capacity(100),
        input,
        output,
    }
}

pub struct Interpreter<'bc, 'io> {
    // We could probably operate mainly on two pointers instead,
    // which might be faster.
    bc: &'bc [u8],
    cursor: usize,

    // FIXME: maybe make another datatype for this,
    //        so that programs requiring long tapes don't need to reallocate
    //
    // FIXME: check if we need to implement infinity to the left
    data: Vec<u8>,
    ptr: usize,

    input: &'io mut dyn FnMut() -> u8,
    output: &'io mut dyn FnMut(u8),
}

impl<'bc, 'io> Interpreter<'bc, 'io> {
    /// Run the program till completion.
    pub fn run(mut self) {
        // Dispatch the first instruction
        //
        // (note that instructions jump to the next one once completed,
        // so this will run the whole program)
        self.dispatch();
    }

    const DISPATCH_TABLE: [fn(&mut Interpreter<'bc, 'io>); 9] = {
        let mut tmp: [fn(&mut Interpreter<'bc, 'io>); 9] = [|_| (); 9];

        // Use indexing here, instead of just creating array with function already in place,
        // so that his fails compilation in case `Inst`'s discriminants change,
        // in an incompatible way.
        tmp[IncPtr as usize] = Interpreter::inc_ptr;
        tmp[DecPtr as usize] = Interpreter::dec_ptr;
        tmp[Inc as usize] = Interpreter::inc;
        tmp[Dec as usize] = Interpreter::dec;
        tmp[Out as usize] = Interpreter::out;
        tmp[Inp as usize] = Interpreter::inp;
        tmp[Jz as usize] = Interpreter::jz;
        tmp[Jnz as usize] = Interpreter::jnz;
        tmp[Halt as usize] = Interpreter::halt;

        tmp
    };

    /// Dispatch the current instruction.
    fn dispatch(&mut self) {
        become Interpreter::DISPATCH_TABLE[self.inst() as usize](self)
    }

    /// Run the next instruction.
    fn next(&mut self) {
        // FIXME: bound check
        self.cursor += 1;

        become self.dispatch()
    }

    /// Handle the [`IncPtr`] instruction.
    fn inc_ptr(&mut self) {
        debug_assert!(self.at(IncPtr));

        // FIXME: bound check
        self.ptr += 1;
        become self.next();
    }

    /// Handle the [`DecPtr`] instruction.
    fn dec_ptr(&mut self) {
        debug_assert!(self.at(DecPtr));

        // FIXME: bound check
        self.ptr -= 1;
        become self.next();
    }

    /// Handle the [`Inc`] instruction.
    fn inc(&mut self) {
        debug_assert!(self.at(Inc));

        let byte = self.deref_mut();
        *byte = byte.wrapping_add(1);
        become self.next();
    }

    /// Handle the [`Dec`] instruction.
    fn dec(&mut self) {
        debug_assert!(self.at(Dec));

        let byte = self.deref_mut();
        *byte = byte.wrapping_sub(1);
        become self.next();
    }

    /// Handle the [`Out`] instruction.
    fn out(&mut self) {
        debug_assert!(self.at(Out));

        let &mut byte = self.deref_mut();
        (self.output)(byte);
        //        dbg!(&self.data, self.ptr);
        become self.next();
    }

    /// Handle the [`Inp`] instruction.
    fn inp(&mut self) {
        debug_assert!(self.at(Inp));

        *self.deref_mut() = (self.input)();
        become self.next();
    }

    /// Handle the [`Jz`] instruction.
    fn jz(&mut self) {
        debug_assert!(self.at(Jz));

        let addr =
            Addr::from_le_bytes(self.bc[self.cursor..][1..][..ADDR_SIZE].try_into().unwrap());

        if self.deref() == 0 {
            self.cursor = addr.try_into().unwrap();
            become self.dispatch();
        } else {
            // account for addr encoding

            // FIXME: bounds check
            self.cursor += ADDR_SIZE;
            become self.next();
        }
    }

    /// Handle the [`Jnz`] instruction.
    fn jnz(&mut self) {
        debug_assert!(self.at(Jnz));

        let addr =
            Addr::from_le_bytes(self.bc[self.cursor..][1..][..ADDR_SIZE].try_into().unwrap());

        if self.deref() != 0 {
            self.cursor = addr.try_into().unwrap();
            become self.dispatch();
        } else {
            // account for addr encoding

            // FIXME: bounds check
            self.cursor += ADDR_SIZE;
            become self.next();
        }
    }

    /// Handle the [`Halt`] instruction.
    fn halt(&mut self) {
        debug_assert!(self.at(Halt));

        /* empty */
    }

    /// Returns the byte at the data pointer, resizing the data array if needed.
    fn deref(&mut self) -> u8 {
        if self.data.len() <= self.ptr {
            self.data.resize(self.ptr + 1, 0);
        }

        self.data[self.ptr]
    }

    /// Returns a unique reference to the byte at the data pointer, resizing the data array if
    /// needed and allowing to mutate it.
    fn deref_mut(&mut self) -> &mut u8 {
        if self.data.len() <= self.ptr {
            self.data.resize(self.ptr + 1, 0);
        }

        &mut self.data[self.ptr]
    }

    /// Returns true if the current instruction is `i`.
    fn at(&self, i: Inst) -> bool {
        self.inst() == i
    }

    /// Returns the current instruction.
    fn inst(&self) -> Inst {
        Inst::from_bc(self.bc[self.cursor]).unwrap()
    }

    #[allow(dead_code)]
    fn print(&self) {
        let mut i = 0;
        while i < self.bc.len() {
            print!("{i:02x} ");
            match Inst::from_bc(self.bc[i]).unwrap() {
                IncPtr => print!(">"),
                DecPtr => print!("<"),
                Inc => print!("+"),
                Dec => print!("-"),
                Out => print!("."),
                Inp => print!(","),
                Jz => {
                    print!(
                        "jf({:02x})",
                        u32::from_le_bytes(self.bc[i..][1..][..4].try_into().unwrap())
                    );
                    i += 4
                }
                Jnz => {
                    print!(
                        "jb({:02x})",
                        u32::from_le_bytes(self.bc[i..][1..][..4].try_into().unwrap())
                    );
                    i += 4
                }
                Halt => print!("(halt)"),
            }
            i += 1;
            println!()
        }
    }
}
