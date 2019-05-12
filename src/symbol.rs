use console::Term;
use failure::Error;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref SYMTABLE_ROW: Regex = Regex::new(r"(?m)^//\s+(\w+)\s+([0-9A-F]+)\s+$").unwrap();
}

#[derive(Clone)]
pub enum TableEntry {
    Symbol(String),
    Relative(usize),
    Unknown,
}

impl TableEntry {
    pub fn unwrap_symbol_name(&self) -> &str {
        match self {
            TableEntry::Symbol(name) => name,
            _ => panic!("Cannot unwrap symbol name on Relative variant"),
        }
    }
}

/// data로부터 심볼 위치를 파싱해 syms에 저장합니다.
pub fn parse_symbol_table(data: &str, syms: &mut [TableEntry]) {
    SYMTABLE_ROW
        .captures_iter(data)
        .map(|cap| {
            (
                cap[1].to_owned(),
                usize::from_str_radix(&cap[2], 16).unwrap(),
            )
        })
        .for_each(|(name, addr)| {
            syms[addr] = TableEntry::Symbol(name);
        });
}

/// 심볼 테이블 후처리(Relative Variant 준비)
pub fn symbol_table_postprocess(syms: &mut [TableEntry]) {
    let mut current_idx = 0;
    let mut last_symbol_idx = 0;
    while let TableEntry::Unknown = syms[current_idx] {
        current_idx += 1;
        if current_idx == 1 << 16 {
            break;
        }
    }
    while current_idx < 65536 {
        if let TableEntry::Symbol(_) = &syms[current_idx] {
            last_symbol_idx = current_idx;
        } else {
            syms[current_idx] = TableEntry::Relative(current_idx - last_symbol_idx);
        }
        current_idx += 1;
    }
}

pub fn symbol_table_view(symbol_table: &[TableEntry], term: &Term) -> Result<(), Error> {
    term.write_line("By-Address            By-Name")?;
    let (mut by_addr, mut by_name): (Vec<_>, Vec<_>) = symbol_table
        .iter()
        .enumerate()
        .filter_map(|(addr, entry)| match entry {
            TableEntry::Symbol(name) => Some(((addr, name), (name, addr))),
            _ => None,
        })
        .unzip();
    by_addr.sort_unstable();
    by_name.sort_unstable();
    for ((addr1, name1), (ref name2, addr2)) in by_addr.iter().zip(by_name) {
        term.write_line(&format!(
            "0x{:04X} {:<12} | {:<12} 0x{:04X}",
            addr1, name1, name2, addr2
        ))?;
    }
    Ok(())
}

pub fn symbol_table_query(symbol_table: &[TableEntry], addr: usize) -> String {
    match &symbol_table[addr] {
        TableEntry::Symbol(name) => name.to_string(),
        TableEntry::Relative(offset) => format!(
            "{}+0x{:X}",
            symbol_table[addr - offset].unwrap_symbol_name(),
            offset
        ),
        TableEntry::Unknown => String::from("?"),
    }
}

pub fn decorate_instruction_str(
    symbol_table: &[TableEntry],
    instruction: &str,
    addr: usize,
) -> String {
    let splits = instruction.clone().split(" ").collect::<Vec<_>>();
    let decorator = match splits[0] {
        "BR" | "BRn" | "BRz" | "BRp" | "BRnz" | "BRzp" | "BRnp" | "BRnzp" | "LD" | "LDI"
        | "LEA" | "ST" | "STI" => {
            String::from(" (")
                + &symbol_table_query(
                    symbol_table,
                    (addr as i32 + 1 + splits[splits.len() - 1][1..].parse::<i32>().unwrap())
                        as usize,
                )
                + ")"
        }
        _ => String::from(""),
    };
    decorator
}
