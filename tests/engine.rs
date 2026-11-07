//! Checks of the Self interpreter and its starting world.

use selfworld::vm::{Value, Vm};

fn eval(vm: &mut Vm, src: &str) -> String {
    let shell = Value::Ref(vm.sp.shell);
    match vm.eval_doit(src, shell) {
        Ok(v) => vm.print_string(v),
        Err(e) => format!("ERROR: {}", e.msg),
    }
}

fn check(cases: &[(&str, &str)]) {
    let mut vm = Vm::new();
    for (src, want) in cases {
        assert_eq!(eval(&mut vm, src), *want, "evaluating {src}");
    }
}

#[test]
fn arithmetic_and_precedence() {
    check(&[
        ("3 + 4", "7"),
        ("3 + 4 * 2", "14"),
        ("2 + 3 factorial", "8"),
        ("3 + 4 factorial max: 20", "27"),
        ("7 / 2", "3"),
        ("7 / 2.0", "3.5"),
        ("-7 % 3", "2"),
        ("10 between: 1 And: 20", "true"),
        ("16r1F", "31"),
        ("1.5e2", "150.0"),
    ]);
}

#[test]
fn big_integers() {
    check(&[
        ("30 factorial", "265252859812191058636308480000000"),
        ("(2 raisedTo: 100) - 1", "1267650600228229401496703205375"),
        ("100 factorial / 98 factorial", "9900"),
        ("(2 raisedTo: 70) < (2 raisedTo: 71)", "true"),
    ]);
}

#[test]
fn objects_slots_and_prototypes() {
    check(&[
        ("(| x <- 3. y = 4. sum = ( x + y ) |) sum", "7"),
        ("| a | a: bankAccount copy. a deposit: 100. a withdraw: 30. a balance", "70"),
        ("bankAccount balance", "0"),
        ("(3 @ 4) + (1 @ 1)", "4@5"),
        ("(3 @ 4) r", "5.0"),
        ("lobby _AddSlots: (| counter <- 0. bump = ( counter: counter + 1 ) |). bump. bump. counter", "2"),
        ("(reflect: 3 @ 4) names", "('parent', 'x', 'y')"),
        ("lobby", "lobby"),
        ("traits integer", "traits integer"),
        ("point copy", "0@0"),
        ("list copyRemoveAll", "list()"),
    ]);
}

#[test]
fn inheritance_resend_and_dynamic_parents() {
    check(&[
        (
            "lobby _AddSlots: (| animal = (| parent* = traits clonable. speak = ( 'noise' ) |) |). \
             lobby _AddSlots: (| puppy = (| parent* = animal. speak = ( resend.speak, '!' ) |) |). puppy speak",
            "'noise!'",
        ),
        (
            "lobby _AddSlots: (| quiet = (| parent* = traits clonable. talk = 'psst' |). \
             loud = (| parent* = traits clonable. talk = 'HEY' |) |). \
             lobby _AddSlots: (| parrot = (| parent* <- quiet |) |). \
             parrot parent: loud. parrot talk",
            "'HEY'",
        ),
        ("parrot parent: quiet. parrot talk", "'psst'"),
    ]);
}

#[test]
fn blocks_and_returns() {
    check(&[
        ("[| :a. :b | a * b ] value: 6 With: 7", "42"),
        ("| adder | adder: [| :n | [| :x | x + n ] ]. (adder value: 10) value: 5", "15"),
        ("lobby _AddSlots: (| firstEven: v = ( v do: [| :x | x even ifTrue: [ ^ x ] ]. nil ) |). firstEven: (3 & 7 & 8 & 9) asVector", "8"),
        ("| r | r: 0. 1 to: 100 Do: [| :i | r: r + i ]. r", "5050"),
        ("| i | i: 0. [ i < 10 ] whileTrue: [ i: i + 1 ]. i", "10"),
        ("[ 1 / 0 ] onError: [| :m | 'caught ', m ]", "'caught division by zero'"),
    ]);
}

#[test]
fn collections_and_strings() {
    check(&[
        ("(1 & 2 & 3) asVector", "(1, 2, 3)"),
        ("(5 & 1 & 4) asVector sort", "(1, 4, 5)"),
        ("(1 to: 10 Collect: [| :i | i * i ]) sum", "385"),
        ("((1 to: 6 Collect: [| :i | i ]) select: [| :x | x even ]) size", "3"),
        ("'hello', ' world'", "'hello world'"),
        ("'abc' reverse asUppercase", "'CBA'"),
        ("'a b c' words size", "3"),
        ("(dictionary copyRemoveAll at: 'a' Put: 1) at: 'a'", "1"),
        ("'n = ', 3", "'n = 3'"),
        ("(vector copySize: 2) at: 5", "ERROR: index 5 is outside 0 to 1"),
    ]);
}

#[test]
fn errors_are_reported() {
    check(&[
        ("3 foo", "ERROR: 3 does not understand 'foo'"),
        ("[ true ] whileTrue", "ERROR: stopped: this loop ran too long — is it infinite?"),
    ]);
    let mut vm = Vm::new();
    assert!(eval(&mut vm, "(3 +").starts_with("ERROR: Syntax error"));
    // deep recursion is caught rather than crashing
    eval(&mut vm, "lobby _AddSlots: (| down: n = ( down: n + 1 ) |)");
    assert!(eval(&mut vm, "down: 0").contains("stack overflow"));
}

#[test]
fn garbage_collection_keeps_the_world() {
    let mut vm = Vm::new();
    eval(&mut vm, "| s | s: ''. 1 to: 2000 Do: [| :i | s: s, 'x' ]. s size");
    let before = vm.object_count();
    let freed = vm.collect_garbage(&[]);
    assert!(freed > 1000, "freed {freed} of {before}");
    assert_eq!(eval(&mut vm, "100 factorial printString size"), "158");
    assert_eq!(eval(&mut vm, "(3 @ 4) + (1 @ 1)"), "4@5");
}

#[test]
fn every_menu_example_runs() {
    for (title, code) in selfworld::app::EXAMPLES {
        let mut vm = Vm::new();
        let r = eval(&mut vm, code);
        assert!(!r.starts_with("ERROR"), "example '{title}' failed: {r}");
    }
}

#[test]
fn the_self_test_written_in_self_passes() {
    let mut vm = Vm::new();
    assert_eq!(vm.run_self_test().unwrap(), "All 86 checks passed");
}

#[test]
fn loading_files_statement_by_statement() {
    let mut vm = Vm::new();
    // line breaks end statements, as in original Self files; a failure doesn't stop the rest
    let src = "'a file'\nlobby _AddSlots: (| greet: n = ( 'Hello, ', n ) |)\n3 foo\n(greet: 'Self') printLine\n100 factorial printString size";
    let rep = vm.load_source(src).unwrap();
    assert_eq!(rep.statements, 5);
    assert_eq!(rep.failures, vec!["line 3: 3 does not understand 'foo'".to_string()]);
    assert_eq!(vm.print_string(rep.last.unwrap()), "158");
    // statements spanning lines inside brackets still work
    let rep = vm.load_source("lobby _AddSlots: (|\n  twice: x = (\n    x * 2 )\n|).\ntwice: 21").unwrap();
    assert!(rep.failures.is_empty(), "{:?}", rep.failures);
    assert_eq!(vm.print_string(rep.last.unwrap()), "42");
    // syntax errors give a line and column
    let e = vm.load_source("3 +\n(4").unwrap_err();
    assert!(e.msg.starts_with("Syntax error at line 2"), "{}", e.msg);
}

#[test]
fn original_self_file_format() {
    let mut vm = Vm::new();
    let src = r#" 'Sun-$Revision: 1 $'

 '-- Module body'

 bootstrap addSlotsTo: bootstrap stub -> 'traits' -> () From: ( | {
         'Category: examples\x7fModuleInfo: Module: demo InitialContents: FollowSlot'

         gadget = bootstrap setObjectAnnotationOf: bootstrap stub -> 'traits' -> 'gadget' -> () From: ( |
             {} = 'ModuleInfo: Creator: traits gadget.
'.
            | ) .
        } | )

 bootstrap addSlotsTo: bootstrap stub -> 'traits' -> 'gadget' -> () From: ( | {
         'ModuleInfo: Module: demo InitialContents: FollowSlot'

         parent* = bootstrap stub -> 'traits' -> 'clonable' -> ().
        } | )

 bootstrap addSlotsTo: bootstrap stub -> 'traits' -> 'gadget' -> () From: ( | {
         'Category: doing\x7fModuleInfo: Module: demo InitialContents: FollowSlot'

         spin: n = ( |
            | n * 360).
        } | )

 bootstrap addSlotsTo: bootstrap stub -> 'globals' -> () From: ( | {
         'ModuleInfo: Module: demo InitialContents: FollowSlot'

         gadget = bootstrap setObjectAnnotationOf: ( |
             | ) From: ( | {} = 'ModuleInfo: Creator: globals gadget.' | ) .
        } | )

 bootstrap addSlotsTo: bootstrap stub -> 'globals' -> 'gadget' -> () From: ( | {
         'ModuleInfo: Module: demo InitialContents: FollowSlot'

         parent* = bootstrap stub -> 'traits' -> 'gadget' -> ().
        } | )
"#;
    let rep = vm.load_source(src).unwrap();
    assert!(rep.failures.is_empty(), "{:?}", rep.failures);
    assert_eq!(eval(&mut vm, "gadget copy spin: 2"), "720");
    assert_eq!(eval(&mut vm, "traits gadget"), "traits gadget");
    // paths only follow an object's own slots after the first step
    assert_eq!(eval(&mut vm, "(bootstrap stub -> 'globals' -> 'true' -> ()) == true"), "true");
    // a keyword message without a receiver can be an argument, as in the original sources
    assert_eq!(eval(&mut vm, "| r | r: copy: 3. r"), "ERROR: shell does not understand 'copy:'");
    assert_eq!(eval(&mut vm, "(| f: x = ( x + 1 ). g = ( 10 + f: 4 ) |) g"), "15");
    // _Restart repeats the current method
    assert_eq!(eval(&mut vm, "(| n <- 0. count = ( n: n + 1. n >= 5 ifTrue: [ ^ n ]. _Restart ) |) count"), "5");
}

#[test]
fn morphs_are_live_objects() {
    let mut vm = Vm::new();
    let r = eval(&mut vm, "| w. n. l | n: 0. l: labelMorph copyText: 'Count: 0'. w: windowMorph copyTitle: 'Counter'. \
        w addMorph: l. w addMorph: (buttonMorph copyLabel: '+' Action: [ n: n + 1. l text: 'Count: ', n printString ]). \
        w openInWorld. (w morphs at: 1) click. (w morphs at: 1) click. l text");
    assert_eq!(r, "'Count: 2'");
    assert_eq!(eval(&mut vm, "world morphs size"), "1");
    assert_eq!(eval(&mut vm, "(world morphs first title)"), "'Counter'");
    // stepping
    let r = eval(&mut vm, "| b | b: circleMorph copy. b _AddSlots: (| step = ( x: x + 5 ) |). b openInWorld. b startStepping. b step. b step. b x");
    assert_eq!(r, "50");
    assert_eq!(eval(&mut vm, "world clear. world morphs size"), "0");
}

#[test]
fn the_built_in_apps_load_and_build_their_morphs() {
    for (title, _, file, src) in selfworld::app::APPS {
        let mut vm = Vm::new();
        let rep = vm.load_source(src).unwrap_or_else(|e| panic!("{file}: {}", e.msg));
        assert!(rep.failures.is_empty(), "app '{title}' ({file}) failed: {:?}", rep.failures);
        let r = eval(&mut vm, "world morphs size");
        assert_ne!(r, "0", "app '{title}' put no morphs in the world");
    }
}

#[test]
fn saved_objects_load_into_a_fresh_world() {
    let mut vm = Vm::new();
    eval(&mut vm, "lobby _AddSlots: (| pet = (| parent* = traits clonable. name <- 'Rex'. greet = ( 'hi from ', name ) |) |)");
    eval(&mut vm, "traits integer _AddSlots: (| triple = ( self * 3 ) |)");
    let saved = vm.file_out_changes();
    let mut fresh = Vm::new();
    let rep = fresh.load_source(&saved).unwrap();
    assert!(rep.failures.is_empty(), "{:?}\n{saved}", rep.failures);
    assert_eq!(eval(&mut fresh, "pet greet"), "'hi from Rex'");
    assert_eq!(eval(&mut fresh, "7 triple"), "21");
}

/// The self-test, translated with the whole starting world to JavaScript and
/// run by Node (skipped when Node isn't installed). Only checks that need the
/// Self compiler or VM reflection at run time may fail.
#[test]
fn the_self_test_passes_when_translated_to_javascript() {
    let Ok(out) = std::process::Command::new("node").arg("--version").output() else {
        eprintln!("node not found: skipping");
        return;
    };
    if !out.status.success() {
        return;
    }
    let script = selfworld::jsgen::script("selfTest run printLine", &[selfworld::world::SELFTEST_SOURCE]).unwrap();
    let path = std::env::temp_dir().join(format!("selfworld-selftest-{}.js", std::process::id()));
    std::fs::write(&path, script).unwrap();
    let out = std::process::Command::new("node").arg(&path).output().unwrap();
    let _ = std::fs::remove_file(&path);
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let allowed = ["storeString rebuilds an object", "fileOut changes", "bootstrap paths", "eval:", "translates to JavaScript"];
    for line in text.lines().filter(|l| l.contains("FAILED")) {
        assert!(allowed.iter().any(|a| line.contains(a)), "unexpected failure in JavaScript: {line}\n{text}");
    }
    assert!(text.contains(" passed, "), "no summary from the JavaScript self-test:\n{text}\n{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn the_built_in_apps_translate_to_web_apps() {
    for (title, _, file, src) in selfworld::app::APPS {
        let html = selfworld::jsgen::web_app(src, title).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(html.starts_with("<!doctype html>") && html.contains("SelfMorphic.start()"), "{file}");
        assert!(!html.contains("</script>\n</script>"), "{file}");
    }
}

#[test]
fn js_print_script_for_debugging() {
    // writes target/selftest.js when SELF_WORLD_JS_OUT is set (used while developing the translator)
    if let Ok(path) = std::env::var("SELF_WORLD_JS_OUT") {
        let s = selfworld::jsgen::script("selfTest run printLine", &[selfworld::world::SELFTEST_SOURCE]).unwrap();
        std::fs::write(path, s).unwrap();
    }
}
