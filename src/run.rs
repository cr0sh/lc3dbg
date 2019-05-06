use crate::helper::TermWrapper;
use crate::undo::UndoBuffer;
use console::Term;
use failure::Error;
use lc3::vm::{MCR, VM};
use std::io::Read;

pub fn run_command(
    vm: &mut VM,
    term: &mut Term,
    body: Option<&str>,
    pre_input: &mut [u8],
    undo_buffer: Option<&mut UndoBuffer>,
) -> Result<usize, Error> {
    if let Some(limit) = body {
        match limit.parse::<usize>() {
            Ok(n) => Ok(match undo_buffer {
                None => vm.run_n_with_io(
                    n,
                    &mut pre_input.chain(TermWrapper(&mut term.clone())),
                    &mut TermWrapper(&mut term.clone()),
                ),
                Some(buffer) => {
                    let mut count = 0usize;
                    while count < n && vm.mem[MCR] >> 15 > 0 {
                        buffer.push(vm);
                        assert_eq!(
                            vm.run_n_with_io(
                                1,
                                &mut pre_input.chain(TermWrapper(&mut term.clone())),
                                &mut TermWrapper(&mut term.clone()),
                            ),
                            1
                        );
                        count += 1;
                    }
                    count
                }
            }),
            Err(err) => {
                term.write_line(&format!("잘못된 입력입니다: {}", err))?;
                Ok(0)
            }
        }
    } else {
        Ok(match undo_buffer {
            None => vm.run_with_io(
                &mut pre_input.chain(TermWrapper(&mut term.clone())),
                &mut TermWrapper(&mut term.clone()),
            ),
            Some(buffer) => {
                let mut count = 0usize;
                while vm.mem[MCR] >> 15 > 0 {
                    buffer.push(vm);
                    assert_eq!(
                        vm.run_n_with_io(
                            1,
                            &mut pre_input.chain(TermWrapper(&mut term.clone())),
                            &mut TermWrapper(&mut term.clone()),
                        ),
                        1
                    );
                    count += 1;
                }
                count
            }
        })
    }
}
