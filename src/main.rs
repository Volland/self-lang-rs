//! Self World entry point.
//!
//! Native:  `self-world`            opens the desktop
//!          `self-world --repl`     a text read-eval-print loop
//!          `self-world -e 'code'`  evaluates one expression and prints it
//!          `self-world file.self`  runs a file of Self code
//! Web:     started from index.html on the `the_canvas_id` canvas.

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use selfworld::vm::{Event, Value, Vm};
    use std::io::{BufRead, Write};

    fn flush(vm: &mut Vm) {
        for e in vm.events.drain(..) {
            if let Event::Print(s) = e {
                print!("{s}");
            }
        }
    }

    fn report(vm: &mut Vm, r: Result<Value, selfworld::vm::SelfError>) -> bool {
        flush(vm);
        match r {
            Ok(v) => {
                let s = vm.print_string(v);
                println!("{s}");
                true
            }
            Err(e) => {
                println!("Error: {}", e.msg);
                for t in e.trace {
                    println!("    in {t}");
                }
                false
            }
        }
    }

    /// Returns true if the arguments asked for a text mode (and it ran).
    pub fn run(args: &[String]) -> bool {
        if args.is_empty() || args[0] == "--open" {
            return false;
        }
        let mut vm = Vm::new();
        let shell = Value::Ref(vm.sp.shell);
        if let Some(i) = args.iter().position(|a| a == "-e") {
            let src = args.get(i + 1).cloned().unwrap_or_default();
            let r = vm.eval_doit(&src, shell);
            if !report(&mut vm, r) {
                std::process::exit(1);
            }
            return true;
        }
        if args[0] == "--help" || args[0] == "-h" {
            println!("self-world            open the Self World desktop");
            println!("self-world --repl     interactive text shell");
            println!("self-world -e CODE    evaluate CODE and print the result");
            println!("self-world FILE.self  load and run files of Self code (add --repl to keep going)");
            println!("self-world --test     run the built-in self-test");
            println!("self-world --open FILE.self   open the desktop with files loaded");
            return true;
        }
        if args[0] == "--test" {
            let r = vm.run_self_test();
            flush(&mut vm);
            match r {
                Ok(summary) => {
                    println!("{summary}");
                    if !summary.starts_with("All ") {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    println!("Error: {}", e.msg);
                    std::process::exit(1);
                }
            }
            return true;
        }
        if args[0] != "--repl" {
            // load every file given, in order, like the original Self's file-in
            let mut ok = true;
            for path in args.iter().filter(|a| !a.starts_with("--")) {
                let src = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("cannot read {path}: {e}");
                        std::process::exit(2);
                    }
                };
                vm.source_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf());
                match vm.load_source(&src) {
                    Ok(rep) => {
                        flush(&mut vm);
                        println!(
                            "{path}: {} statements, {} failed",
                            rep.statements,
                            rep.failures.len()
                        );
                        for f in rep.failures.iter().take(20) {
                            println!("    {f}");
                        }
                        if rep.failures.len() > 20 {
                            println!("    … and {} more", rep.failures.len() - 20);
                        }
                        if let Some(v) = rep.last {
                            if rep.failures.is_empty() {
                                println!("{}", vm.print_string(v));
                            }
                        }
                        ok &= rep.failures.is_empty();
                    }
                    Err(e) => {
                        println!("{path}: {}", e.msg);
                        ok = false;
                    }
                }
            }
            if args.iter().any(|a| a == "--repl") {
                return run_repl(vm);
            }
            if !ok {
                std::process::exit(1);
            }
            return true;
        }
        run_repl(vm)
    }

    fn run_repl(mut vm: Vm) -> bool {
        let shell = Value::Ref(vm.sp.shell);
        println!("Self World — text shell. Finish an expression with an empty line. Ctrl-D quits.");
        let stdin = std::io::stdin();
        let mut buf = String::new();
        print!("self> ");
        std::io::stdout().flush().ok();
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            if line.trim().is_empty() {
                if !buf.trim().is_empty() {
                    let r = vm.eval_doit(&buf, shell);
                    report(&mut vm, r);
                    buf.clear();
                }
                print!("self> ");
            } else {
                buf.push_str(&line);
                buf.push('\n');
                print!("  ... ");
            }
            std::io::stdout().flush().ok();
        }
        println!();
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if cli::run(&args) {
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Self World")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([520.0, 380.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Self World",
        options,
        Box::new(|cc| Ok(Box::new(selfworld::app::SelfWorldApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // The web build uses the library (see src/web.rs); nothing to do here.
}
