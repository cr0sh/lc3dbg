use console::Term;
use failure::Error;
use lc3::vm::{MCR, VM};
use pretty_bytes::converter::convert;
use std::env::args;
use std::fs;
use std::io::{ErrorKind, Result as IOResult};
use std::path::Path;
use undo::UndoBuffer;

mod helper;
mod run;
mod symbol;
mod undo;

const HELP: &str = r#"lc3dbg - LC-3 디버거
사용법:
    lc3dbg <filename1>, <filename2>, ...: 주어진 LC-3 오브젝트 파일들을 순서대로 엽니다.
    lc3dbg --help: 이 도움말을 출력합니다.
"#;

fn print_help(term: &Term) -> IOResult<()> {
    term.write_line(HELP)
}

fn main() -> Result<(), Error> {
    let args = args().skip(1).collect::<Vec<_>>();
    let mut term = Term::stdout();
    term.write_line(&format!("lc3dbg v{}", env!("CARGO_PKG_VERSION")))?;

    if args == ["--help".to_owned()] {
        print_help(&term)?;
        return Ok(());
    }

    let mut vm = VM::new();
    let mut symbol_table = vec![symbol::TableEntry::Unknown; 65536];

    for arg in args.into_iter() {
        term.write_line(&format!("프로그램 로드: {}", arg))?;
        let path = Path::new(&arg);
        vm.load_file(path)?;
        match fs::read(path.with_extension("sym")) {
            Err(ref e) if e.kind() == ErrorKind::NotFound => continue,
            file @ _ => {
                term.move_cursor_up(1)?;
                term.clear_line()?;
                term.write_line(&format!("프로그램 로드: {}+sym", arg))?;
                symbol::parse_symbol_table(&String::from_utf8(file?)?, &mut symbol_table)
            }
        };
    }

    symbol::symbol_table_postprocess(&mut symbol_table);

    let mut pre_input: Vec<u8> = vec![];
    let mut undo_buffer: Option<UndoBuffer> = None;

    helper::print_register_status(&vm, &term)?;

    'cmdloop: loop {
        term.write_str("lc3dbg>")?;
        let line = term.read_line()?.trim().to_owned();
        let (cmd, body) = {
            let mut it = line.splitn(2, ' ');
            (it.next(), it.next())
        };

        match cmd {
            Some("help") => help_command(&mut vm, &term, body),
            Some("bye") => {
                break;
            }
            Some("run") => {
                if vm.mem[MCR] >> 15 == 0 {
                    term.write_line("VM이 꺼져 있습니다.")?;
                    term.write_line(
                        "강제로 켜려면, `turn on` 명령어를 사용하세요.",
                    )?;
                    continue;
                }
                let instructions = run::run_command(
                    &mut vm,
                    &mut term,
                    body,
                    &mut pre_input,
                    undo_buffer.as_mut(),
                )?;
                term.write_line(&format!(
                    "{}개의 instruction을 실행했습니다.",
                    instructions
                ))?;
                Ok(())
            }
            Some("stat") => {
                helper::print_register_status(&vm, &term)?;
                Ok(())
            }
            Some("input") => {
                term.write_line(
                    "미리 입력될 내용을 모두 쓴 후, Ctrl+W를 누르세요.",
                )?;
                pre_input = helper::TermWrapper(&mut term).collect::<Result<Vec<u8>, _>>()?;
                Ok(())
            }
            Some("noinput") => {
                pre_input = vec![];
                term.write_line("미리 설정된 입력을 모두 지웠습니다.")?;
                Ok(())
            }
            Some("turn") => {
                vm.mem[MCR] = match body {
                    Some("on") => {
                        term.write_line("VM을 켭니다.")?;
                        0b1000_0000_0000_0000u16
                    }
                    Some("off") => {
                        term.write_line("VM을 끕니다.")?;
                        0b0000_0000_0000_0000u16
                    }
                    _ => {
                        term.write_line("유효한 명령어가 아닙니다.")?;
                        continue;
                    }
                } + (vm.mem[MCR] & 0b0111_1111_1111_1111u16);
                Ok(())
            }
            Some("undo") => match undo_buffer {
                Some(ref mut buf) => match str::parse::<usize>(body.unwrap_or("1")) {
                    Ok(n) => {
                        buf.undo_command(&mut vm, &term, n)?;
                        Ok(())
                    }
                    Err(err) => {
                        term.write_line("유효한 명령어가 아닙니다.")?;
                        term.write_line(&format!("{}", err))?;
                        Ok(())
                    }
                },
                None => {
                    term.write_line("undo buffer가 설정되지 않았습니다.")?;
                    term.write_line(
                        "먼저 buffer <size> 명령어로 버퍼를 초기화하세요.",
                    )?;
                    Ok(())
                }
            },
            Some("buffer") | Some("buf") => match body
                .ok_or::<Box<std::error::Error>>(
                    "버퍼 크기가 명시되지 않았습니다(버퍼를 없애려면 크기를 0으로 두세요)"
                        .into(),
                )
                .and_then(|x| str::parse::<usize>(x).map_err(From::from))
            {
                Ok(size) if size == 0 => {
                    undo_buffer = None;
                    term.write_line("버퍼를 없앴습니다.")?;
                    Ok(())
                }
                Ok(size) => {
                    let expected_bytes = std::mem::size_of::<Option<VM>>() as u32 * size as u32;
                    if expected_bytes >= 100 * (1 << 20) {
                        term.write_line(&format!(
                            "경고: 100MiB 이상의 큰 메모리를 할당합니다. ({})",
                            convert(expected_bytes.into())
                        ))?;
                        loop {
                            term.write_str("계속하시겠습니까? (y/n) ")?;
                            let answer = term.read_line()?;
                            match answer.as_ref() {
                                "y" | "yes" | "d" | "dd" | "ㅇ" | "ㅇㅇ" | "예" | "네" => {
                                    break;
                                }
                                "n" | "no" | "s" | "ss" | "ㄴ" | "ㄴㄴ" | "아니오"
                                | "아뇨" => {
                                    term.write_line("취소합니다.")?;
                                    continue 'cmdloop;
                                }
                                _ => {
                                    term.write_line("잘못된 입력입니다.")?;
                                }
                            }
                        }
                    }
                    undo_buffer = Some(UndoBuffer::new(size));
                    term.write_line(&format!(
                        "기존 버퍼를 모두 지우고 크기 {}의 버퍼를 새로 초기화했습니다.",
                        size
                    ))?;
                    term.write_line(&format!(
                        "할당된 메모리: {}",
                        convert(expected_bytes.into())
                    ))?;
                    Ok(())
                }
                Err(err) => {
                    term.write_line("유효한 명령어가 아닙니다.")?;
                    term.write_line(&format!("{}", err))?;
                    Ok(())
                }
            },
            Some("sym") => match body {
                Some(addr) => {
                    let addr = match helper::parse_usize_with_prefix(addr) {
                        Ok(addr) => addr,
                        Err(err) => {
                            term.write_line("잘못된 입력입니다.")?;
                            term.write_line(&format!("{}", err))?;
                            continue;
                        }
                    };
                    term.write_line(&symbol::symbol_table_query(&symbol_table, addr))?;
                    Ok(())
                }
                None => symbol::symbol_table_view(&symbol_table, &term),
            },
            Some("mem") => match body {
                Some(body) => {
                    let mut bsplit = body.split(" ");

                    let addr_str = bsplit.next().unwrap();
                    let mut range_str = match bsplit.next() {
                        Some(rstr) => rstr.to_owned(),
                        None => String::from("+10"),
                    };
                    if range_str[0..1].parse::<u8>().is_ok() {
                        range_str = String::from("+") + &range_str;
                    } else if &range_str[1..] == "" {
                        range_str = range_str.to_owned() + "10";
                    }

                    let addr: usize;
                    if addr_str.to_ascii_lowercase() == "pc" {
                        addr = vm.pc as usize;
                    } else {
                        match helper::parse_usize_with_prefix(addr_str) {
                            Ok(a) => addr = a,
                            Err(err) => {
                                term.write_line("잘못된 입력입니다.")?;
                                term.write_line(&format!("{}", err))?;
                                continue;
                            }
                        };
                    }

                    let mut lower: usize;
                    let mut upper: usize;
                    let n = match helper::parse_usize_with_prefix(&range_str[1..]) {
                        Ok(addr) => addr,
                        Err(err) => {
                            term.write_line("잘못된 입력입니다.")?;
                            term.write_line(&format!("{}", err))?;
                            continue;
                        }
                    };

                    match &range_str[0..1] {
                        "+" => {
                            lower = addr;
                            upper = addr + n;
                        }
                        "-" => {
                            lower = addr + 1 - n;
                            upper = addr + 1;
                        }
                        "~" => {
                            lower = addr - n;
                            upper = addr + n + 1;
                        }
                        _ => {
                            term.write_line("잘못된 입력입니다.")?;
                            continue;
                        }
                    }
                    lower = std::cmp::max(0, lower);
                    upper = std::cmp::min(1 << 16, upper);
                    term.write_line("addr  binary           hex      uint    int   instruction           (symbol)")?;
                    for addr in lower..upper {
                        helper::view_mem_entry(addr, &vm, &symbol_table, &term)?;
                    }
                    Ok(())
                }
                None => {
                    term.write_line("잘못된 입력입니다.")?;
                    Ok(())
                }
            },
            _ => {
                term.write_line("유효한 명령어가 아닙니다.")?;
                continue;
            }
        }?;
    }

    term.write_line("ㅂ2")?;
    Ok(())
}

fn help_command(_: &mut VM, term: &Term, body: Option<&str>) -> Result<(), Error> {
    if body.is_none() {
        term.write_str(
            r#"LC-3 디버거 명령어
    input: 프로그램에 입력될 내용을 미리 설정합니다.
    noinput: input으로 설정된 내용을 모두 지웁니다.

    run: 프로그램이 종료될 때까지 실행합니다.
    stat: 현재 VM의 상태를 출력합니다.

    turn on: VM의 Clock Enable Bit을 1로 만듭니다(VM을 켭니다).
    turn off: VM의 Clock Enable Bit을 0으로 만듭니다(VM을 끕니다).

    buf(fer) <size>: 최대 <size> instruction만큼 VM을 되돌릴 수 있는 버퍼를 생성합니다.
                     이미 버퍼가 있었다면, 기록을 삭제하고 새로 만듭니다. (undo가 불가능해집니다.)
                     참고: 버퍼를 생성된 상태에서 프로그램을 실행하면 성능이 약간 감소할 수 있습니다.
                     (매 instruction마다 VM을 복사해야 하기 때문입니다.)
    buf(fer) 0: 버퍼를 없앱니다.
    undo <n=1>: <n> instruction만큼 VM을 되돌립니다.

    sym: 심볼 테이블을 봅니다.
    sym <addr>: 해당 위치로부터 가장 가까운 심볼을 찾습니다.
    mem <addr> <n>: 주어진 주소 주변의 메모리 값을 확인합니다.

    help: 이 도움말을 출력합니다.
    help <command>: 해당 명령어에 대한 도움말을 출력합니다.

    bye: 디버거를 종료합니다.
"#,
        )?;
        return Ok(());
    }
    let cmd = body.unwrap().trim();
    term.write_line(
        match cmd {
            "input"=>"input: 프로그램에 입력될 내용을 미리 설정합니다.
모든 입력은 ASCII 범위를 넘지 않아야 합니다. UTF-8, EUC-KR 입력에 대한 동작은 정의되지 않았습니다.
(ex: 한글, Emoji, ASCII 범위를 넘는 특수문자 등)
입력이 모두 끝나면, Ctrl+W를 누릅니다.
이후 run 명령을 통해 프로그램을 실행하면, input에 의해 설정된 입력이 자동으로 프로그램에 전달됩니다.
미리 입력된 내용을 다 전달했음에도 프로그램이 계속 입력을 요청하면, 그 때부터는 터미널의 입력을 프로그램에 전달합니다.
(noinput 상태처럼)
input 명령을 여러 번 실행하면, 가장 마지막에 설정된 입력만 작동하며 이전의 설정은 모두 지워집니다.",
            "noinput" => "input 명령으로 설정된 입력을 모두 지웁니다. 이후 run 명령을 통해 프로그램을 실행하면, 터미널의 입력을 프로그램에 전달합니다.",
            "run" => "run: 프로그램을 VM이 종료될 때까지 실행합니다.
run n: 프로그램이 종료될 때까지 최대 n개의 instruction을 실행합니다.
       서브루틴, TRAP 내부의 instruction 또한 모두 포함합니다.",
            "stat" => "stat: 현재 VM의 상태를 출력합니다.",
            "turn" => "turn on: VM의 Clock Enable Bit을 1로 만듭니다(VM을 켭니다).
turn off: VM의 Clock Enable Bit을 0으로 만듭니다(VM을 끕니다).",
            "buffer" | "buf" => "buf(fer) <size>: 최대 <size> instruction만큼 VM을 되돌릴 수 있는 버퍼를 생성합니다.
                 이미 버퍼가 있었다면, 기록을 삭제하고 새로 만듭니다. (undo가 불가능해집니다.)
                 참고: 버퍼를 생성된 상태에서 프로그램을 실행하면 성능이 약간 감소할 수 있습니다.
                 (매 instruction마다 VM을 복사해야 하기 때문입니다.)
buf(fer) 0: 버퍼를 없앱니다.",
            "undo" => "undo <n>: <n> instruction만큼 VM을 되돌립니다.
          undo를 취소할 수는 없으니 주의하세요.",
            "sym" => "sym: 심볼 테이블을 출력합니다.
sym <addr>: 해당 위치로부터 가장 가까운 심볼을 찾습니다.
            해당 위치에서 앞으로만 검색합니다.
            addr 변수는 x1234와 같이 16진수로, 또는 1234와 같이 10진수로 표현할 수 있습니다.",
            "mem" => "mem <addr> <+/-/~n>: 현재 PC 근방의 메모리 값을 확인합니다.
    addr 변수는 x1234와 같이 16진수로, 또는 1234와 같이 10진수로 표현하거나, PC(pc)를 입력해 현재 pc 주변의 값을 볼 수 있습니다.
    +n: 주어진 메모리 위치 및 다음 n개의 메모리 값을 확인합니다. (최대 n개)
    -n: 주어진 메모리 위치 및 이전 n개의 메모리 값을 확인합니다. (최대 n개)
    ~n: 주어진 메모리 위치 및 앞뒤 n개의 메모리 값을 확인합니다. (최대 2n+1개)
    n이 주어지지 않는다면 10, +/-/~가 주어지지 않는다면 +가 기본으로 주어집니다.
    n 변수는 x1234와 같이 16진수로, 또는 1234와 같이 10진수로 표현할 수 있습니다.",
            "help" => "help: 이 도움말을 출력합니다.
help <command>: 해당 명령어에 대한 도움말을 출력합니다.",
            _ => "존재하지 않는 명령어입니다.",
        })?;
    Ok(())
}
