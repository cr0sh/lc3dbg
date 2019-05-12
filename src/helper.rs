use crate::symbol::{decorate_instruction_str, TableEntry};
use console::Term;
use lc3::vm::instruction::Instruction;
use lc3::vm::{MCR, VM};
use std::io::{Read, Result as IOResult, Write};
use std::iter::Iterator;

pub fn print_register_status(vm: &VM, term: &Term) -> IOResult<()> {
    term.write_line(&format!(
        r#"PC: 0x{:04X}           CC: {}   [{}]
IR: 0x{:04X}           Supervisor: {}
r0: 0x{:04X} = {:<7} r4: 0x{:04X} = {:<7}
r1: 0x{:04X} = {:<7} r5: 0x{:04X} = {:<7}
r2: 0x{:04X} = {:<7} r6: 0x{:04X} = {:<7}
r3: 0x{:04X} = {:<7} r7: 0x{:04X} = {:<7}
IR(Current): {}
PC(Next)   : {}
"#,
        vm.pc,
        if vm.condition.n {
            "n__"
        } else if vm.condition.z {
            "_z_"
        } else {
            "__p"
        },
        if vm.mem[MCR] & 0b1000_0000_0000_0000 == 0 {
            "OFF"
        } else {
            "ON"
        },
        vm.ir,
        if vm.supervisor { "yes" } else { "no" },
        vm.register[0],
        vm.register[0] as i16,
        vm.register[4],
        vm.register[4] as i16,
        vm.register[1],
        vm.register[1] as i16,
        vm.register[5],
        vm.register[5] as i16,
        vm.register[2],
        vm.register[2] as i16,
        vm.register[6],
        vm.register[6] as i16,
        vm.register[3],
        vm.register[3] as i16,
        vm.register[7],
        vm.register[7] as i16,
        Instruction::from_u16(vm.ir),
        Instruction::from_u16(vm.mem[vm.pc as usize]),
    ))
}

/// Wrapper that re-implements `std::io::Read`
/// To perform I/O directly with VM.
pub struct TermWrapper<'a>(pub &'a mut Term);

impl<'a> Read for TermWrapper<'a> {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        let mut n: usize = 0;
        for b in buf.iter_mut() {
            *b = self.0.read_char()? as u8;
            n += 1;
        }
        Ok(n)
    }
}

impl<'a> Write for TermWrapper<'a> {
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> IOResult<()> {
        self.0.flush()
    }
}

impl<'a> Iterator for TermWrapper<'a> {
    type Item = IOResult<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.read_char() {
            Ok(ch) => {
                if !ch.is_ascii() {
                    if let Err(err) = self.0.write_line(&format!(
                        "경고: non-ASCII 입력에 대한 행동은 정의되지 않았습니다. ({})",
                        ch
                    )) {
                        return Some(Err(err));
                    }
                }

                if ch as u8 == 23 {
                    None
                } else {
                    if let Err(err) = self.0.write(ch.to_string().as_ref()) {
                        return Some(Err(err));
                    }
                    Some(Ok(ch as u8))
                }
            }
            Err(err) => Some(Err(err)),
        }
    }
}

pub fn parse_usize_with_prefix(s: &str) -> Result<usize, std::num::ParseIntError> {
    if s.len() >= 1 && &s[0..1] == "x" {
        usize::from_str_radix(&s[1..], 16)
    } else {
        s.parse::<usize>()
    }
}

pub fn view_mem_entry(
    addr: usize,
    vm: &VM,
    symbol_table: &[TableEntry],
    term: &Term,
) -> IOResult<()> {
    term.write_line(&format!(
        "x{:04X} {:016b} x{:04X}   {:5} {:6}   {:<20} {}",
        addr,
        vm.mem[addr],
        vm.mem[addr],
        vm.mem[addr],
        vm.mem[addr] as i16,
        format!("{}", Instruction::from_u16(vm.mem[addr])),
        decorate_instruction_str(
            &symbol_table,
            &format!("{}", Instruction::from_u16(vm.mem[addr])),
            addr
        ),
    ))
}
