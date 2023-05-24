use analysis::LoweringCx;

fn main() {
    let input_file = std::env::args().nth(1).expect("first argument");
    let src = std::fs::read_to_string(&input_file).unwrap_or_else(|err| {
        eprintln!("failed to read file {input_file}: {err}");
        std::process::exit(1);
    });

    let ast = parser::parse_file(&src);
    dbg_pls::color!(&ast);
    let Ok(ast) = ast else {
        std::process::exit(1);
    };
    let mut printer = parser::pretty::PrettyPrinter::new(std::io::stdout().lock(), false);
    println!("// START CODE  -------------------");
    printer.translation_unit(&ast).unwrap();
    println!("// END CODE    -------------------");

    let arena = bumpalo::Bump::new();
    let mut lcx = LoweringCx::new(&arena);

    let ir = analysis::lower_translation_unit(&mut lcx, &ast).unwrap_or_else(|err| {
        dbg!(err);
        std::process::exit(1);
    });

    codegen::generate(&lcx, &ir).unwrap_or_else(|err| {
        dbg!(err);
        std::process::exit(1);
    });
}
