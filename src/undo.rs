use console::Term;
use lc3::vm::VM;
use std::io::Result as IOResult;

pub struct UndoBuffer {
    buf: Vec<VM>,
    size: usize,
    ptr: usize,
}

impl UndoBuffer {
    pub fn new(size: usize) -> UndoBuffer {
        UndoBuffer {
            buf: Vec::with_capacity(size),
            size: 0,
            ptr: 0,
        }
    }

    pub fn push(&mut self, vm: &VM) {
        if self.buf.len() == self.ptr {
            self.buf.push(vm.clone());
        } else {
            self.buf[self.ptr] = vm.clone();
        }
        self.ptr += 1;
        if self.size < self.buf.capacity() {
            self.size += 1;
        }
        if self.ptr == self.buf.capacity() {
            self.ptr = 0;
        }
    }

    fn undo(&mut self, n: usize) -> Option<&VM> {
        if self.size < n {
            return None;
        }

        self.size -= n;
        if self.ptr <= n {
            self.ptr += self.buf.len();
        }
        self.ptr -= n;
        Some(&self.buf[self.ptr])
    }

    pub fn undo_command(&mut self, vm: &mut VM, term: &Term, n: usize) -> IOResult<()> {
        if let Some(uvm) = self.undo(n) {
            *vm = uvm.clone();
            term.write_line(&format!(
                "{} instruction 전으로 VM의 상태를 되돌렸습니다.",
                n
            ))?;
        } else {
            term.write_line(&format!(
                "되돌릴 수 없습니다. 최대 {} instruction만큼만 돌릴 수 있습니다.",
                self.size
            ))?;
        }
        Ok(())
    }
}
