use std::collections::BTreeMap;
use std::fs::{File, read_to_string};
use std::io::*;

fn main() {
    {
        let csv_path = "./src/ir.csv";
        let mut lines = Vec::new();
        for line in read_to_string(csv_path).unwrap().lines() {
            if !line.is_empty() && !line.starts_with("#") {
                lines.push(line.to_string());
            }
        }

        let mut os = File::create("./src/_generated/ir.rs").unwrap();

        make_ir(&mut os, lines.as_slice());

        println!("cargo:rerun-if-changed={}", csv_path);
    }
}

fn make_ir(os: &mut File, lines: &[String]) {
    let mut ir_orders = Vec::new();
    let mut ir_sources = BTreeMap::new();
    let mut mnemonics = BTreeMap::new();

    for (line_num, line) in lines.iter().enumerate().skip(1) {
        let cols = line.split(',').map(|s| s.trim()).collect::<Vec<_>>();

        let mnemonic = cols.get(0).unwrap_or(&"");
        if mnemonic.is_empty() || mnemonic.starts_with("#") {
            // Skip empty or commented mnemonic
            continue;
        }
        let mode = *cols.get(1).unwrap_or(&"");
        let sub_category = *cols.get(2).unwrap_or(&"");

        let mode = Mode::from_str(mode);
        let signature = format!("{}_{:#?}", mnemonic, mode);
        let sub_category = SubCategory::parse(sub_category);

        let ir_source = IrSource {
            signature: signature.clone(),
            mnemonic: mnemonic.to_uppercase().to_string(),
            ir_mnemonic: mnemonic.to_uppercase().replace('.', "_"),
            mode,
            sub_category,
        };

        if ir_sources.contains_key(&signature) {
            panic!(
                "Duplicate mnemonic '{}' at line {}",
                ir_source.signature,
                line_num + 1
            );
        }
        if !mnemonics.contains_key(&ir_source.mnemonic) {
            mnemonics.insert(ir_source.mnemonic.clone(), ());
        }
        ir_orders.push(signature.clone());
        ir_sources.insert(signature.clone(), ir_source);
    }
    let mut mnemonics = mnemonics
        .into_iter()
        .map(|(mnemonic, _)| mnemonic)
        .collect::<Vec<_>>();
    mnemonics.sort();

    let mut ir2s = Vec::new();
    for ir in ir_orders {
        let ir = ir_sources.get(&ir).unwrap();

        match ir.mode {
            Mode::Implied => {
                ir2s.push(Ir2 {
                    comment: format!("`{}`", ir.mnemonic),
                    variant_name: format!("{}", ir.ir_mnemonic),
                    args: "".to_string(),
                    n_args: 0,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::ModRM_R2 => {
                for (mode1, mode2) in [
                    (SubMode::Rb, SubMode::Rb),
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::Rb, SubMode::MbA16),
                    (SubMode::Rb, SubMode::MbA32),
                    (SubMode::Rw, SubMode::MwA16),
                    (SubMode::Rw, SubMode::MwA32),
                    (SubMode::Rd, SubMode::MdA16),
                    (SubMode::Rd, SubMode::MdA32),
                    (SubMode::MbA16, SubMode::Rb),
                    (SubMode::MbA32, SubMode::Rb),
                    (SubMode::MwA16, SubMode::Rw),
                    (SubMode::MwA32, SubMode::Rw),
                    (SubMode::MdA16, SubMode::Rd),
                    (SubMode::MdA32, SubMode::Rd),
                    (SubMode::Rb, SubMode::Ib),
                    (SubMode::Rw, SubMode::Iw),
                    (SubMode::Rd, SubMode::Id),
                    (SubMode::MbA16, SubMode::Ib),
                    (SubMode::MbA32, SubMode::Ib),
                    (SubMode::MwA16, SubMode::Iw),
                    (SubMode::MwA32, SubMode::Iw),
                    (SubMode::MdA16, SubMode::Id),
                    (SubMode::MdA32, SubMode::Id),
                ] {
                    if !ir
                        .sub_category
                        .test(mode1.sub_category_flags(), mode2.sub_category_flags())
                    {
                        continue;
                    }
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::ModRM_R1 => {
                for submode in [
                    SubMode::Rb,
                    SubMode::Rw,
                    SubMode::Rd,
                    SubMode::MbA16,
                    SubMode::MbA32,
                    SubMode::MwA16,
                    SubMode::MwA32,
                    SubMode::MdA16,
                    SubMode::MdA32,
                    SubMode::MpA16,
                    SubMode::MpA32,
                ] {
                    if (ir.sub_category.flags & submode.sub_category_flags())
                        != submode.sub_category_flags()
                    {
                        continue;
                    }
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}`",
                            submode.prefix(),
                            ir.mnemonic,
                            submode.comment()
                        ),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("({})", submode.type_signature()),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::ModRM_Sr => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Sr),
                    (SubMode::Rd, SubMode::Sr),
                    (SubMode::MwA16, SubMode::Sr),
                    (SubMode::MwA32, SubMode::Sr),
                    (SubMode::MdA16, SubMode::Sr),
                    (SubMode::MdA32, SubMode::Sr),
                    (SubMode::Sr, SubMode::Rw),
                    (SubMode::Sr, SubMode::Rd),
                    (SubMode::Sr, SubMode::MwA16),
                    (SubMode::Sr, SubMode::MwA32),
                    (SubMode::Sr, SubMode::MdA16),
                    (SubMode::Sr, SubMode::MdA32),
                ] {
                    if !ir
                        .sub_category
                        .test(mode1.sub_category_flags(), mode2.sub_category_flags())
                    {
                        continue;
                    }
                    let prefixes: String = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Shift => {
                for (mode1, mode2) in [
                    (SubMode::Rb, SubMode::Ib),
                    (SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Ib),
                    (SubMode::MbA16, SubMode::Ib),
                    (SubMode::MbA32, SubMode::Ib),
                    (SubMode::MwA16, SubMode::Ib),
                    (SubMode::MwA32, SubMode::Ib),
                    (SubMode::MdA16, SubMode::Ib),
                    (SubMode::MdA32, SubMode::Ib),
                    (SubMode::Rb, SubMode::Cl),
                    (SubMode::Rw, SubMode::Cl),
                    (SubMode::Rd, SubMode::Cl),
                    (SubMode::MbA16, SubMode::Cl),
                    (SubMode::MbA32, SubMode::Cl),
                    (SubMode::MwA16, SubMode::Cl),
                    (SubMode::MwA32, SubMode::Cl),
                    (SubMode::MdA16, SubMode::Cl),
                    (SubMode::MdA32, SubMode::Cl),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: if matches!(mode2, SubMode::Cl) { 1 } else { 2 },
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::MovSx => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rb),
                    (SubMode::Rd, SubMode::Rb),
                    (SubMode::Rd, SubMode::Rw),
                    (SubMode::Rw, SubMode::MbA16),
                    (SubMode::Rw, SubMode::MbA32),
                    (SubMode::Rd, SubMode::MbA16),
                    (SubMode::Rd, SubMode::MbA32),
                    (SubMode::Rd, SubMode::MwA16),
                    (SubMode::Rd, SubMode::MwA32),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Imul3 => {
                for (mode1, mode2, mode3) in [
                    (SubMode::Rw, SubMode::Rw, SubMode::Iw),
                    (SubMode::Rd, SubMode::Rd, SubMode::Id),
                    (SubMode::Rw, SubMode::MwA16, SubMode::Iw),
                    (SubMode::Rw, SubMode::MwA32, SubMode::Iw),
                    (SubMode::Rd, SubMode::MdA16, SubMode::Id),
                    (SubMode::Rd, SubMode::MdA32, SubMode::Id),
                ] {
                    let prefixes =
                        format!("{}{}{}", mode1.prefix(), mode2.prefix(), mode3.prefix());
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                            mode3.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                            mode3.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2, mode3]),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Jcc => {
                ir2s.push(Ir2 {
                    comment: format!("`{} cc, off32`", ir.mnemonic),
                    variant_name: format!("{}_Jv", ir.ir_mnemonic),
                    args: "(CC, Offset32)".to_string(),
                    n_args: 2,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Loop => {
                ir2s.push(Ir2 {
                    comment: format!("`{} off32`", ir.mnemonic),
                    variant_name: format!("{}_Jv", ir.ir_mnemonic),
                    args: "(Offset32)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Jump => {
                ir2s.push(Ir2 {
                    comment: format!("`{} off32`", ir.mnemonic),
                    variant_name: format!("{}_Jv", ir.ir_mnemonic),
                    args: "(Offset32)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
                for submode in [
                    SubMode::Rw,
                    SubMode::Rd,
                    SubMode::MwA16,
                    SubMode::MwA32,
                    SubMode::MdA16,
                    SubMode::MdA32,
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}`",
                            submode.prefix(),
                            ir.mnemonic,
                            submode.comment()
                        ),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("({})", submode.type_signature()),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::JumpF => {
                ir2s.push(Ir2 {
                    comment: format!("`{} a16:32`", ir.mnemonic),
                    variant_name: format!("{}_Ap", ir.ir_mnemonic),
                    args: "(SegmentSelector, Offset32)".to_string(),
                    n_args: 2,
                    mnemonic: ir.mnemonic.clone(),
                });
                for bits in [16, 32] {
                    for submode in [SubMode::MpA16, SubMode::MpA32] {
                        ir2s.push(Ir2 {
                            comment: format!(
                                "`{}o{}: {} {}`",
                                submode.prefix(),
                                bits,
                                ir.mnemonic,
                                submode.comment(),
                            ),
                            variant_name: format!(
                                "{}_{}O{}",
                                ir.ir_mnemonic,
                                submode.symbol(),
                                bits
                            ),
                            args: format!("({})", submode.type_signature()),
                            n_args: 1,
                            mnemonic: ir.mnemonic.clone(),
                        });
                    }
                }
            }
            Mode::Ret => {
                for bits in [16, 32] {
                    ir2s.push(Ir2 {
                        comment: format!("`d{}: {}`", bits, ir.mnemonic),
                        variant_name: format!("{}_D{}", ir.ir_mnemonic, bits),
                        args: "(u16)".to_string(),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Ib => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm8`", ir.mnemonic),
                    variant_name: format!("{}_Ib", ir.ir_mnemonic),
                    args: "(u8)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Iv => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm16`", ir.mnemonic),
                    variant_name: format!("{}_Iw", ir.ir_mnemonic),
                    args: "(u16)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
                ir2s.push(Ir2 {
                    comment: format!("`{} imm32`", ir.mnemonic),
                    variant_name: format!("{}_Id", ir.ir_mnemonic),
                    args: "(u32)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Sr => {
                ir2s.push(Ir2 {
                    comment: format!("`{} sr`", ir.mnemonic),
                    variant_name: format!("{}_Sr", ir.ir_mnemonic),
                    args: "(SrIndex)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Out => {
                for (symbol, args, match_args, comment) in [
                    ("Ib_Al", "(u8)", 1, "imm8, al"),
                    ("Dx_Al", "", 0, "dx, al"),
                    ("Ib_Aw", "(u8)", 1, "imm8, ax"),
                    ("Dx_Aw", "", 0, "dx, ax"),
                    ("Ib_Ad", "(u8)", 1, "imm8, eax"),
                    ("Dx_Ad", "", 0, "dx, eax"),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, comment),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, symbol),
                        args: args.to_string(),
                        n_args: match_args,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::In => {
                for (symbol, args, match_args, comment) in [
                    ("Al_Ib", "(u8)", 1, "al, imm8"),
                    ("Al_Dx", "", 0, "al, dx"),
                    ("Aw_Ib", "(u8)", 1, "ax, imm8"),
                    ("Aw_Dx", "", 0, "ax, dx"),
                    ("Ad_Ib", "(u8)", 1, "eax, imm8"),
                    ("Ad_Dx", "", 0, "eax, dx"),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, comment),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, symbol),
                        args: args.to_string(),
                        n_args: match_args,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::String => {
                for suffix in ["b", "w", "d"] {
                    ir2s.push(Ir2 {
                        comment: format!("`{}{} (segment, a32)`", ir.mnemonic, suffix),
                        variant_name: format!("{}{}", ir.ir_mnemonic, suffix.to_uppercase()),
                        args: "(SrIndex, bool)".to_string(),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                    if ir.sub_category.contains(SubCategory::ALLOWS_REPZ) {
                        ir2s.push(Ir2 {
                            comment: format!("`REPZ {}{} (segment, a32)`", ir.mnemonic, suffix),
                            variant_name: format!(
                                "REPZ_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: "(SrIndex, bool)".to_string(),
                            n_args: 2,
                            mnemonic: ir.mnemonic.clone(),
                        });
                        ir2s.push(Ir2 {
                            comment: format!("`REPNZ {}{} (segment, a32)`", ir.mnemonic, suffix),
                            variant_name: format!(
                                "REPNZ_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: "(SrIndex, bool)".to_string(),
                            n_args: 2,
                            mnemonic: ir.mnemonic.clone(),
                        });
                    } else {
                        ir2s.push(Ir2 {
                            comment: format!("`REP {}{} (segment, a32)`", ir.mnemonic, suffix),
                            variant_name: format!(
                                "REP_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: "(SrIndex, bool)".to_string(),
                            n_args: 2,
                            mnemonic: ir.mnemonic.clone(),
                        });
                    }
                }
            }
            Mode::Enter => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm16, imm8`", ir.mnemonic),
                    variant_name: format!("{}_Iw_Ib", ir.ir_mnemonic),
                    args: "(u16, u8)".to_string(),
                    n_args: 2,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Cr => {
                for (mode1, mode2) in [
                    (SubMode::Rd, SubMode::Cr),
                    (SubMode::Cr, SubMode::Rd),
                    (SubMode::Rd, SubMode::Dr),
                    (SubMode::Dr, SubMode::Rd),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Bt => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::MwA16, SubMode::Rw),
                    (SubMode::MwA32, SubMode::Rw),
                    (SubMode::MdA16, SubMode::Rd),
                    (SubMode::MdA32, SubMode::Rd),
                    (SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Ib),
                    (SubMode::MwA16, SubMode::Ib),
                    (SubMode::MwA32, SubMode::Ib),
                    (SubMode::MdA16, SubMode::Ib),
                    (SubMode::MdA32, SubMode::Ib),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::SetCc => {
                for submode in [SubMode::Rb, SubMode::MbA16, SubMode::MbA32] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} cc, {}`",
                            submode.prefix(),
                            ir.mnemonic,
                            submode.comment(),
                        ),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("(CC, {})", submode.type_signature()),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Shld => {
                for (mode1, mode2, mode3) in [
                    (SubMode::Rw, SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Rd, SubMode::Ib),
                    (SubMode::MwA16, SubMode::Rw, SubMode::Ib),
                    (SubMode::MwA32, SubMode::Rw, SubMode::Ib),
                    (SubMode::MdA16, SubMode::Rd, SubMode::Ib),
                    (SubMode::MdA32, SubMode::Rd, SubMode::Ib),
                    (SubMode::Rw, SubMode::Rw, SubMode::Cl),
                    (SubMode::Rd, SubMode::Rd, SubMode::Cl),
                    (SubMode::MwA16, SubMode::Rw, SubMode::Cl),
                    (SubMode::MwA32, SubMode::Rw, SubMode::Cl),
                    (SubMode::MdA16, SubMode::Rd, SubMode::Cl),
                    (SubMode::MdA32, SubMode::Rd, SubMode::Cl),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} {}, {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                            mode3.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                            mode3.symbol()
                        ),
                        args: SubMode::signatures(&[mode1, mode2, mode3]),
                        n_args: if matches!(mode3, SubMode::Cl) { 2 } else { 3 },
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Cmov => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::Rw, SubMode::MwA16),
                    (SubMode::Rw, SubMode::MwA32),
                    (SubMode::Rd, SubMode::MdA16),
                    (SubMode::Rd, SubMode::MdA32),
                ] {
                    let prefixes = format!("{}{}", mode1.prefix(), mode2.prefix(),);
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{}{} cc, {}, {}`",
                            prefixes,
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: format!(
                            "(CC, {}, {})",
                            mode1.type_signature(),
                            mode2.type_signature()
                        ),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Esc => {
                for submode in [SubMode::Rb, SubMode::MbA16, SubMode::MbA32] {
                    ir2s.push(Ir2 {
                        comment: format!("`esc op3, reg3, {}`", submode.comment(),),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("(u8, u8, {})", submode.type_signature()),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
        }
    }

    write!(
        os,
        "// This file is automatically @generated at build time. DO NOT EDIT DIRECTLY.

use crate::prelude::*;

/// Intermediate Representation for x86 opcodes
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrOp {{
"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(os, "    /// {}", ir2.comment).unwrap();
        writeln!(os, "    {}{},", ir2.variant_name, ir2.args).unwrap();
    }

    writeln!(
        os,
        "}}

impl IrOp {{
    pub fn mnemonic(&self) -> Mnemonic {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => Mnemonic::{},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.mnemonic
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}

    pub fn name(&self) -> &str {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => {:?},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.variant_name
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}

    pub fn n_args(&self) -> usize {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => {},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.n_args,
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}
}}

/// Well Known Mnemonic for x86 instructions
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {{"
    )
    .unwrap();

    for mnemonic in mnemonics {
        writeln!(os, "    {},", mnemonic.to_uppercase()).unwrap();
    }

    writeln!(os, "}}").unwrap();
}

struct IrSource {
    signature: String,
    mnemonic: String,
    ir_mnemonic: String,
    mode: Mode,
    sub_category: SubCategory,
}

struct Ir2 {
    comment: String,
    variant_name: String,
    args: String,
    n_args: usize,
    mnemonic: String,
}

impl Ir2 {
    fn match_args_anonymous(&self) -> String {
        let n_args = self.n_args;
        if n_args == 0 {
            return String::new();
        }
        let args = (0..n_args).map(|_| "_").collect::<Vec<_>>().join(", ");
        format!("({})", args)
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
    /// Implied
    Implied,
    /// BT/BTS/BTR/BTC instructions
    Bt,
    /// CMOVcc instructions
    Cmov,
    /// MOV to/from Control Register
    Cr,
    /// ENTER instruction
    Enter,
    /// opcode flows immediate byte, used for INT instruction and some instructions
    Ib,
    /// IMUL with 3 operands
    Imul3,
    /// IN instruction
    In,
    /// opcode flows immediate word
    Iv,
    /// Jcc instruction
    Jcc,
    /// JMP instruction
    Jump,
    /// JMP FAR instruction
    JumpF,
    /// Loop instruction (loop, loopz, loopnz, jcxz, jecxz)
    Loop,
    /// ModR/M with single operand
    ModRM_R1,
    /// ModR/M generic format (two operands)
    ModRM_R2,
    /// ModR/M with Segment Register
    ModRM_Sr,
    /// MOVSX/MOVZX instruction
    MovSx,
    /// OUT instruction
    Out,
    /// RET instruction
    Ret,
    /// SETcc instruction
    SetCc,
    /// Shift and rotate instructions (ROL, ROR, RCL, RCR, SHL/SAL, SHR, SAR)
    Shift,
    /// SHLD/SHRD instruction
    Shld,
    /// Segment Register
    Sr,
    /// String instructions (MOVS, CMPS, STOS, LODS, SCAS)
    String,
    /// ESC instruction
    Esc,
}

impl Mode {
    fn from_str(s: &str) -> Self {
        match s {
            "" | "implied" => Self::Implied,
            "bt" => Self::Bt,
            "cmov" => Self::Cmov,
            "cr" => Self::Cr,
            "enter" => Self::Enter,
            "esc" => Self::Esc,
            "ib" => Self::Ib,
            "imul3" => Self::Imul3,
            "in" => Self::In,
            "iv" => Self::Iv,
            "jcc" => Self::Jcc,
            "jump" => Self::Jump,
            "jumpf" => Self::JumpF,
            "loop" => Self::Loop,
            "modrm_r1" => Self::ModRM_R1,
            "modrm_r2" => Self::ModRM_R2,
            "modrm_sr" => Self::ModRM_Sr,
            "movsx" => Self::MovSx,
            "out" => Self::Out,
            "ret" => Self::Ret,
            "setcc" => Self::SetCc,
            "shift" => Self::Shift,
            "shld" => Self::Shld,
            "sr" => Self::Sr,
            "string" => Self::String,
            _ => panic!("Unknown mode '{}'", s),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SubMode {
    Rb,
    Rw,
    Rd,
    Ib,
    Iw,
    Id,
    MbA16,
    MbA32,
    MwA16,
    MwA32,
    MdA16,
    MdA32,
    MpA16,
    MpA32,
    Cl,
    Sr,
    Cr,
    Dr,
}

impl SubMode {
    fn comment(&self) -> String {
        match self {
            Self::Rb => "reg8",
            Self::Rw => "reg16",
            Self::Rd => "reg32",
            Self::Cl => "cl",
            Self::Ib => "imm8",
            Self::Iw => "imm16",
            Self::Id => "imm32",
            Self::MbA16 => "[mem8]",
            Self::MbA32 => "[mem8]",
            Self::MwA16 => "[mem16]",
            Self::MwA32 => "[mem16]",
            Self::MdA16 => "[mem32]",
            Self::MdA32 => "[mem32]",
            Self::MpA16 => "[mem]",
            Self::MpA32 => "[mem]",
            Self::Sr => "sr",
            Self::Cr => "cr",
            Self::Dr => "dr",
        }
        .to_string()
    }

    fn prefix(&self) -> String {
        match self {
            Self::Rb
            | Self::Rw
            | Self::Rd
            | Self::Ib
            | Self::Iw
            | Self::Id
            | Self::Cl
            | Self::Sr
            | Self::Cr
            | Self::Dr => "",
            Self::MbA16 | Self::MwA16 | Self::MdA16 | Self::MpA16 => "a16: ",
            Self::MbA32 | Self::MwA32 | Self::MdA32 | Self::MpA32 => "a32: ",
        }
        .to_string()
    }

    fn symbol(&self) -> String {
        match self {
            Self::Rb => "Rb",
            Self::Rw => "Rw",
            Self::Rd => "Rd",
            Self::Ib => "Ib",
            Self::Iw => "Iw",
            Self::Id => "Id",
            Self::Cl => "Cl",
            Self::Sr => "Sr",
            Self::Cr => "Cr",
            Self::Dr => "Dr",
            Self::MbA16 => "MbA16",
            Self::MbA32 => "MbA32",
            Self::MwA16 => "MwA16",
            Self::MwA32 => "MwA32",
            Self::MdA16 => "MdA16",
            Self::MdA32 => "MdA32",
            Self::MpA16 => "MpA16",
            Self::MpA32 => "MpA32",
        }
        .to_string()
    }

    fn type_signature(&self) -> String {
        match self {
            Self::Rb => "GprIndex8",
            Self::Rw => "GprIndex16",
            Self::Rd => "GprIndex32",
            Self::Ib => "u8",
            Self::Iw => "u16",
            Self::Id => "u32",
            Self::Cl => "",
            Self::Sr => "SrIndex",
            Self::Cr => "CrIndex",
            Self::Dr => "DrIndex",
            Self::MbA16 => "MemOpr16",
            Self::MbA32 => "MemOpr32",
            Self::MwA16 => "MemOpr16",
            Self::MwA32 => "MemOpr32",
            Self::MdA16 => "MemOpr16",
            Self::MdA32 => "MemOpr32",
            Self::MpA16 => "MemOpr16",
            Self::MpA32 => "MemOpr32",
        }
        .to_string()
    }

    fn sub_category_flags(&self) -> u32 {
        match self {
            Self::Ib => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_BYTE,
            Self::Iw => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_WORD,
            Self::Id => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_DWORD,
            Self::Rb => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_BYTE,
            Self::Rw => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_WORD,
            Self::Rd => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_DWORD,
            Self::MbA16 | Self::MbA32 => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_BYTE,
            Self::MwA16 | Self::MwA32 => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_WORD,
            Self::MdA16 | Self::MdA32 => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_DWORD,
            Self::MpA16 | Self::MpA32 => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_FWORD,
            Self::Cl => 0,
            Self::Sr => SubCategory::ALLOWS_SEGMENT,
            Self::Cr => SubCategory::ALLOWS_DWORD,
            Self::Dr => SubCategory::ALLOWS_DWORD,
        }
    }

    fn signatures(types: &[SubMode]) -> String {
        let mut signatures = Vec::new();
        for submode in types {
            let type_signature = submode.type_signature();
            if !type_signature.is_empty() {
                signatures.push(type_signature);
            }
        }
        if signatures.is_empty() {
            "".to_string()
        } else {
            format!("({})", signatures.join(", "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubCategory {
    flags: u32,
}

impl SubCategory {
    pub const ALLOWS_BYTE: u32 = 0b0000_0000_0000_0001;
    pub const ALLOWS_WORD: u32 = 0b0000_0000_0000_0010;
    pub const ALLOWS_DWORD: u32 = 0b0000_0000_0000_0100;
    pub const ALLOWS_FWORD: u32 = 0b0000_0000_0000_1000;

    pub const ALLOWS_REGISTER: u32 = 0b0000_0000_0001_0000;
    pub const ALLOWS_MEMORY: u32 = 0b0000_0000_0010_0000;
    pub const ALLOWS_IMMEDIATE: u32 = 0b0000_0000_0100_0000;
    pub const ALLOWS_SEGMENT: u32 = 0b0000_0000_1000_0000;

    pub const ALLOWS_LOAD: u32 = 0b0000_0001_0000_0000;
    pub const ALLOWS_STORE: u32 = 0b0000_0010_0000_0000;

    pub const ALLOWS_REPZ: u32 = 0b0001_0000_0000_0000;
    pub const RESERVED_E: u32 = 0b0010_0000_0000_0000;

    pub const ALLOWS_SIZE: u32 =
        Self::ALLOWS_BYTE | Self::ALLOWS_WORD | Self::ALLOWS_DWORD | Self::ALLOWS_FWORD;
    pub const ALLOWS_SIZE_DEFAULT: u32 = Self::ALLOWS_BYTE | Self::ALLOWS_WORD | Self::ALLOWS_DWORD;

    pub const ALLOWS_RMI: u32 =
        Self::ALLOWS_REGISTER | Self::ALLOWS_MEMORY | Self::ALLOWS_IMMEDIATE;

    pub const ALLOWS_LOAD_STORE: u32 = Self::ALLOWS_LOAD | Self::ALLOWS_STORE;

    pub fn parse(s: &str) -> Self {
        let mut flags = 0;

        for ch in s.chars() {
            match ch {
                'b' => flags |= Self::ALLOWS_BYTE,
                'w' => flags |= Self::ALLOWS_WORD,
                'd' => flags |= Self::ALLOWS_DWORD,
                'p' => flags |= Self::ALLOWS_FWORD,
                'r' => flags |= Self::ALLOWS_REGISTER,
                'm' => flags |= Self::ALLOWS_MEMORY,
                'i' => flags |= Self::ALLOWS_IMMEDIATE,
                'l' => flags |= Self::ALLOWS_LOAD,
                's' => flags |= Self::ALLOWS_STORE,
                'z' => flags |= Self::ALLOWS_REPZ,
                'e' => flags |= Self::RESERVED_E,
                _ => panic!("Unknown subcategory character '{}'", ch),
            }
        }

        if flags & Self::ALLOWS_SIZE == 0 {
            flags |= Self::ALLOWS_SIZE_DEFAULT;
        }
        if flags & Self::ALLOWS_RMI == 0 {
            flags |= Self::ALLOWS_RMI;
        }
        if flags & Self::ALLOWS_LOAD_STORE == 0 {
            flags |= Self::ALLOWS_LOAD_STORE;
        }
        Self { flags }
    }

    #[inline]
    pub fn contains(&self, flags: u32) -> bool {
        (self.flags & flags) == flags
    }

    pub fn test(&self, flags1: u32, flags2: u32) -> bool {
        let mut actual = flags1 | flags2;
        if (flags1 & Self::ALLOWS_MEMORY) != 0 {
            actual |= Self::ALLOWS_STORE;
        }
        if (flags2 & Self::ALLOWS_MEMORY) != 0 {
            actual |= Self::ALLOWS_LOAD;
        }

        if (self.flags & actual & Self::ALLOWS_SIZE) == 0 {
            return false;
        }
        if (actual & Self::ALLOWS_MEMORY) != 0
            && (self.flags & actual & Self::ALLOWS_LOAD_STORE) == 0
        {
            return false;
        }

        if !self.contains(Self::ALLOWS_REGISTER) && self.contains(Self::ALLOWS_MEMORY) {
            let expected = Self::ALLOWS_REGISTER | Self::ALLOWS_MEMORY;
            (actual & expected) == expected
        } else {
            let lhs = self.flags & Self::ALLOWS_RMI;
            let rhs = actual & Self::ALLOWS_RMI;
            (lhs & rhs) == rhs
        }
    }
}
