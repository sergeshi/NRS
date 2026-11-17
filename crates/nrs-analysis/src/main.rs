use nrs_analysis::{
    RomAnalysis, analyze_coverage, analyze_code_data, analyze_memory_references, build_cross_references, decode_chr, disassemble_nrom, format_coverage_report, format_code_data_report, format_memory_reference_report, format_cross_references,
    format_listing_with_labels, generate_labels, write_chr_atlas_png,
};
use nrs_rom::RomImage;

fn main() {
    let mut args = std::env::args().skip(1);

    let path = args.next().unwrap_or_else(|| {
        eprintln!("Usage: nrs-analyze <path-to-rom.nes> [atlas.png] [disassembly.txt]");
        std::process::exit(2);
    });

    let atlas_path = args.next().unwrap_or_else(|| "chr_atlas.png".to_string());

    let disassembly_path = args.next();

    let data = std::fs::read(&path).unwrap_or_else(|error| {
        eprintln!("Cannot read '{}': {}", path, error);
        std::process::exit(1);
    });

    let rom = RomImage::parse(&data).unwrap_or_else(|error| {
        eprintln!("Invalid NES ROM: {:?}", error);
        std::process::exit(1);
    });

    let report = RomAnalysis::inspect(&rom);
    let tiles = decode_chr(&rom.chr_rom);

    println!("========================================");
    println!("             NRS ROM ANALYZER");
    println!("========================================");

    println!("File:             {}", path);
    println!("File size:        {} bytes", data.len());
    println!("Format:           {:?}", report.format);
    println!("Mapper:           {}", report.mapper);
    println!("Submapper:        {}", report.submapper);
    println!("PRG ROM:          {} bytes", report.prg_size);
    println!("CHR ROM:          {} bytes", report.chr_size);
    println!("CHR tiles:        {}", report.chr_tile_count);
    println!("Decoded tiles:    {}", tiles.len());
    println!("Mirroring:        {:?}", report.mirroring);
    println!("TV system:        {:?}", report.tv_system);
    println!("Trainer:          {}", report.has_trainer);
    println!("Battery:          {}", report.battery_backed);
    println!("Trailing data:    {} bytes", report.trailing_size);

    println!();
    println!("CPU INTERRUPT VECTORS:");

    match report.vectors {
        Some(v) => {
            println!("NMI:              ${:04X}", v.nmi);
            println!("RESET:            ${:04X}", v.reset);
            println!("IRQ/BRK:          ${:04X}", v.irq);
        }
        None => {
            println!("Unavailable: PRG ROM is too small");
        }
    }

    match write_chr_atlas_png(&tiles, &atlas_path) {
        Ok(()) => {
            println!();
            println!("CHR ATLAS EXPORTED:");
            println!("File:             {}", atlas_path);
            println!("Tiles:            {}", tiles.len());
            println!("Image:            576 x 1152 pixels");
        }
        Err(error) => {
            eprintln!("Cannot export CHR atlas: {}", error);
            std::process::exit(1);
        }
    }

    let disassembly = disassemble_nrom(&rom.prg_rom);
    let references = build_cross_references(&disassembly);
    let labels = generate_labels(&disassembly);
    let coverage = analyze_coverage(rom.prg_rom.len(), &disassembly);
    let code_data = analyze_code_data(rom.prg_rom.len(), &disassembly);
    let memory_references = analyze_memory_references(&disassembly);

    println!();
    println!("CPU STATIC ANALYSIS:");
    println!("Instructions:     {}", disassembly.instruction_count());
    println!("Labels:           {}", labels.len());
    println!("Cross references: {}", references.len());
    println!(
        "Calls:            {}",
        references
            .iter()
            .filter(|r| r.kind_name() == "CALL")
            .count()
    );
    println!(
        "Jumps:            {}",
        references
            .iter()
            .filter(|r| r.kind_name() == "JUMP")
            .count()
    );
    println!(
        "Branches:         {}",
        references
            .iter()
            .filter(|r| r.kind_name() == "BRANCH")
            .count()
    );
    println!(
        "Indirect JMP:     {}",
        disassembly.unresolved_indirect_jumps.len()
    );
    println!("Invalid targets:  {}", disassembly.invalid_targets.len());
    println!();
    print!("{}", format_coverage_report(&coverage, 12));
    println!();
    print!("{}", format_code_data_report(&code_data, 12));
    println!();
    print!("{}", format_memory_reference_report(&memory_references, 24));

    if let Some(output_path) = disassembly_path {
        let listing = format_listing_with_labels(&disassembly, &labels);

        std::fs::write(&output_path, listing).unwrap_or_else(|error| {
            eprintln!("Cannot write '{}': {}", output_path, error);
            std::process::exit(1);
        });

        println!();
        println!("LABELED DISASSEMBLY EXPORTED:");
        println!("File:             {}", output_path);
    }

    let xref_report = format_cross_references(&references);

    println!();
    println!("CROSS REFERENCES:");
    println!(
        "{}",
        xref_report.lines().take(12).collect::<Vec<_>>().join("\n")
    );

    println!("========================================");
}
