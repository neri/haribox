//! Simple x86 disassembler
//!
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;
use std::{env, process};

use ir86::prelude::*;

fn usage() -> ! {
    let mut args = env::args_os();
    let arg = args.next().unwrap();
    let path = Path::new(&arg);
    let lpc = path.file_name().unwrap();
    eprintln!("{} [OPTIONS] INPUT", lpc.to_str().unwrap());
    process::exit(1);
}

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();

    let mut mode = AppMode::DisAsm;
    let mut arg_next = None;
    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            usage();
        } else if arg == "-d" || arg == "-disasm" {
            mode = AppMode::DisAsm;
        } else if arg == "-s" || arg == "-statistics" {
            mode = AppMode::Statistics(StatisticsMode::Default);
        } else if arg == "-ir" {
            mode = AppMode::Statistics(StatisticsMode::Ir);
        } else if arg == "-mnemonic" {
            mode = AppMode::Statistics(StatisticsMode::Mnemonic);
        } else if arg == "-t" || arg == "-template" {
            mode = AppMode::Statistics(StatisticsMode::Template);
        } else if arg == "--" {
            arg_next = args.next();
            break;
        } else {
            arg_next = Some(arg);
            break;
        }
    }
    if arg_next.is_none() {
        usage();
    }

    let mut stats = Statistics::new();

    for path_input in arg_next.into_iter().chain(args) {
        let mut file = match File::open(&path_input) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}: {}", path_input, e);
                continue;
            }
        };
        let mut input = Vec::new();
        if let Err(e) = file.read_to_end(&mut input) {
            eprintln!("{}: {}", path_input, e);
            continue;
        }

        if let Some(elf_type) = elf::ElfFormat::identify(&input, None, None) {
            // ELF
            if elf_type != elf::ElfFormat::Elf32 {
                eprintln!("{}: unsupported ELF type: {:?}", path_input, elf_type);
                continue;
            }
            let elf = elf::elf32::Header::from_slice(&input).expect("failed to parse ELF header");
            if elf.e_machine != elf::EM_386 {
                eprintln!("{}: unsupported ELF type", path_input);
                continue;
            }

            let entry_point = Offset32(elf.e_entry);
            match mode {
                AppMode::DisAsm => {
                    println!("{}: file-format elf32-i386", path_input);
                    if entry_point.0 > 0 {
                        println!("\nEntry point: 0x{:08x}", entry_point.0);
                    }
                }
                AppMode::Statistics(_) => {}
            }

            let entries = EntryReader::<elf::elf32::ProgramHeader>::new(
                &input[elf.e_phoff as usize..],
                elf.e_phentsize as usize,
                elf.e_phnum as usize,
            )
            .expect("invalid program header");

            match mode {
                AppMode::DisAsm => {
                    for (i, entry) in entries.enumerate() {
                        if entry.p_type == elf::PT_LOAD
                            && entry.p_memsz > 0
                            && entry.p_flags.contains(elf::SegmentFlags::EXEC)
                        {
                            let base = Offset32(entry.p_vaddr);
                            let size = entry.p_filesz as usize;
                            let data = &input[entry.p_offset as usize..][..size];
                            disasm_section(&format!("segment {}", i), data, base, entry_point);
                        }
                    }
                }
                AppMode::Statistics(_) => {
                    for entry in entries {
                        if entry.p_type == elf::PT_LOAD
                            && entry.p_memsz > 0
                            && entry.p_flags.contains(elf::SegmentFlags::EXEC)
                        {
                            let base = Offset32(entry.p_vaddr);
                            let size = entry.p_filesz as usize;
                            let data = &input[entry.p_offset as usize..][..size];
                            stats.append(data, base);
                        }
                    }
                }
            }
        } else if let Some(hrb) = hrb::HrbExecutable::identify(&input) {
            // HRB
            let entry_point = Offset32(hrb.entry_point());
            let base_of_code = size_of::<hrb::HrbExecutable>();
            let size_of_code = hrb.start_data as usize;

            match mode {
                AppMode::DisAsm => {
                    println!("{}: file-format HRB", path_input);
                    if entry_point.0 > 0 {
                        println!("\nEntry point: 0x{:08x}", entry_point.0);
                    }
                    disasm_section(
                        "code",
                        &input[base_of_code..size_of_code],
                        Offset32(base_of_code as u32),
                        entry_point,
                    );
                }
                AppMode::Statistics(_) => {
                    stats.append(
                        &input[base_of_code..size_of_code],
                        Offset32(base_of_code as u32),
                    );
                }
            }
        } else {
            eprintln!("{}: unknown format", path_input);
            continue;
        }
    }

    match mode {
        AppMode::DisAsm => {}
        AppMode::Statistics(mode) => {
            stats.print(mode);
        }
    }
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    /// Disassemble the input file(s) and print the disassembly to stdout.
    DisAsm,
    /// Collect statistics about the input file(s) and print the statistics to stdout.
    Statistics(StatisticsMode),
}

/// Statistics mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatisticsMode {
    /// Default statistics mode
    Default,
    /// IR statistics mode
    Ir,
    /// Mnemonic statistics mode
    Mnemonic,
    /// Collect statistics and print a template for the match statement for the IR enum
    Template,
}

/// Disassembles a section of code and prints the disassembly to stdout.
fn disasm_section(name: &str, data: &[u8], base: Offset32, entry: Offset32) {
    println!("\nDisassembly of {}:", name);

    let mut decoder = Decoder::with_use32();

    let mut fetcher = SimpleFetcher::new(data, base);

    let trailing_zeros = data.iter().rev().take_while(|&&b| b == 0).count();
    let trailing_nops = data.iter().rev().take_while(|&&b| b == 0x90).count();
    let fetch_size = data.len() - trailing_zeros - trailing_nops;

    let mut functions = BTreeMap::new();

    if entry.0 >= base.0 && entry.0 < base.0 + fetch_size as u32 {
        functions.insert(entry, 0);
    }
    functions.insert(base, 0);

    let mut total_opcodes_size = 0;
    let mut irs = Vec::new();
    let mut labels = BTreeMap::new();
    while fetcher.pos() < fetch_size {
        let pos = fetcher.pos();
        let Ok(ir) = decoder.decode(&mut fetcher) else {
            break;
        };
        match ir {
            IrOp::JCC_Jv(_, target) | IrOp::JMP_Jv(target) => {
                labels.insert(target, ());
            }
            IrOp::CALL_Jv(target) => {
                // labels.insert(target, ());
                functions.insert(target, 0);
            }
            _ => {}
        }
        let opcodes_size = fetcher.pos() - pos;
        total_opcodes_size += opcodes_size;
        irs.push((pos, opcodes_size, ir));
    }
    if total_opcodes_size < data.len() {
        // Add a final entry for the remaining bytes as an undefined instruction
        irs.push((
            total_opcodes_size,
            data.len() - total_opcodes_size,
            IrOp::UD,
        ));
    }

    let mut func_keys = functions.keys().cloned().collect::<Vec<_>>();
    func_keys.sort();
    for (i, key) in func_keys.into_iter().enumerate() {
        functions.insert(key, i + 1);
    }

    for (pos, opcodes_size, ir) in irs {
        let opcodes = &data[pos..][..opcodes_size];
        let eip = Offset32(base.0 + pos as u32);

        if let Some(func_no) = functions.get(&eip) {
            println!("\n<func_{}>:", func_no);
        } else if labels.contains_key(&eip) {
            println!("<label_{:08x}>:", eip.0);
        }
        print!("{:08x}:  ", eip.0);

        let max_opcodes = 8;
        for byte in opcodes.iter().take(max_opcodes) {
            print!("{:02x} ", byte);
        }
        for _ in opcodes.len()..max_opcodes {
            print!("   ");
        }
        if matches!(ir, IrOp::UD) {
            println!(" ???");
        } else {
            println!(" {:x?}", ir);
        }
        if opcodes_size > max_opcodes {
            print!("           ");
            for byte in opcodes.iter().skip(max_opcodes) {
                print!("{:02x} ", byte);
            }
            println!("");
        }
    }
}

struct Statistics {
    n_instructions: usize,
    total_opcodes_size: usize,
    irs: BTreeMap<Ir2, usize>,
    mnemonics: BTreeMap<String, usize>,
}

impl Statistics {
    fn new() -> Self {
        Self {
            n_instructions: 0,
            total_opcodes_size: 0,
            irs: BTreeMap::new(),
            mnemonics: BTreeMap::new(),
        }
    }

    fn append(&mut self, data: &[u8], base: Offset32) {
        let mut decoder = Decoder::with_use32();
        let mut fetcher = SimpleFetcher::new(data, base);

        let trailing_zeros = data.iter().rev().take_while(|&&b| b == 0).count();
        let trailing_nops = data.iter().rev().take_while(|&&b| b == 0x90).count();
        let fetch_size = data.len() - trailing_zeros - trailing_nops;

        let mut n_instructions = 0;
        let mut total_opcodes_size = 0;
        while fetcher.pos() < fetch_size {
            let pos = fetcher.pos();
            let Ok(ir) = decoder.decode(&mut fetcher) else {
                break;
            };
            let opcodes_size = fetcher.pos() - pos;
            n_instructions += 1;
            total_opcodes_size += opcodes_size;

            let mnemonic = format!("{:?}", ir.mnemonic());
            self.mnemonics
                .entry(mnemonic)
                .and_modify(|count| *count += 1)
                .or_insert(1);

            let ir2 = Ir2 {
                name: ir.name().to_string(),
                n_args: ir.n_args(),
            };
            self.irs
                .entry(ir2)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        self.n_instructions += n_instructions;
        self.total_opcodes_size += total_opcodes_size;
    }

    fn print(&self, mode: StatisticsMode) {
        match mode {
            StatisticsMode::Default => {
                self.print_ir();
                println!("");
                self.print_mnemonic();
                println!("");
                self.print_avg();
            }
            StatisticsMode::Ir => {
                self.print_ir();
            }
            StatisticsMode::Mnemonic => {
                self.print_mnemonic();
            }
            StatisticsMode::Template => {
                self.print_template();
            }
        }
    }

    fn print_ir(&self) {
        let mut irs = self.irs.clone().into_iter().collect::<Vec<_>>();
        irs.sort_by(|a, b| b.1.cmp(&a.1));
        println!("IR statistics: ({} irs)", irs.len());
        for (ir, count) in irs {
            println!("  {}: {}", ir.name, count);
        }
    }

    fn print_mnemonic(&self) {
        let mut mnemonics = self.mnemonics.clone().into_iter().collect::<Vec<_>>();
        mnemonics.sort_by(|a, b| b.1.cmp(&a.1));
        println!("Mnemonic statistics: ({} mnemonics)", mnemonics.len());
        for (mnemonic, count) in mnemonics {
            let rate = (count as f64 / self.n_instructions as f64) * 100.0;
            println!("  {:8} {:8} ({:.2}%)", mnemonic, count, rate);
        }
    }

    fn print_avg(&self) {
        let avg_opcodes_size = self.total_opcodes_size as f64 / self.n_instructions as f64;
        println!(
            "Average opcodes size: {:.2} bytes ({} instructions, {} bytes fetched)",
            avg_opcodes_size, self.n_instructions, self.total_opcodes_size
        );
    }

    fn print_template(&self) {
        let mut irs = self.irs.clone().into_iter().collect::<Vec<_>>();
        irs.sort_by(|a, b| a.0.cmp(&b.0));

        println!("match ir {{");
        for (ir, _count) in irs {
            println!(
                "    IrOp::{}{} => todo!(),",
                ir.name,
                ir.match_args_template(),
            );
        }
        println!("    _ => unreachable!(),");
        println!("}}");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Ir2 {
    name: String,
    n_args: usize,
}

impl Ir2 {
    fn match_args_template(&self) -> String {
        let args = (0..self.n_args)
            .map(|i| format!("_a{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        if args.is_empty() {
            String::new()
        } else {
            format!("({})", args)
        }
    }
}

struct EntryReader<'a, 'b, TYPE: Sized> {
    data: &'a [u8],
    entry_size: usize,
    pos: usize,
    _phantom: PhantomData<&'b TYPE>,
}

impl<'a, 'b, TYPE: Sized + 'b> EntryReader<'a, 'b, TYPE> {
    pub fn new(data: &'a [u8], entry_size: usize, max_entries: usize) -> Option<Self> {
        if entry_size < size_of::<TYPE>() || data.len() < entry_size * max_entries {
            return None;
        }
        Some(Self {
            data: &data[..entry_size * max_entries],
            entry_size,
            pos: 0,
            _phantom: PhantomData,
        })
    }
}

impl<'a, 'b, TYPE: Sized + 'b> Iterator for EntryReader<'a, 'b, TYPE> {
    type Item = &'b TYPE;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        let entry_data = &self.data[self.pos..][..self.entry_size];
        let value = unsafe { &*(entry_data.as_ptr() as *const TYPE) };
        self.pos += self.entry_size;
        Some(value)
    }
}
