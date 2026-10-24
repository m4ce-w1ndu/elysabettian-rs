use super::compile;
use ely_core::chunk::Chunk;
use ely_core::interner::StringInterner;
use ely_core::opcode::Opcode;

fn extract_opcodes(chunk: &Chunk) -> Vec<Opcode> {
    let mut ops = Vec::new();
    let mut i = 0;
    while i < chunk.count() {
        if let Ok(op) = Opcode::try_from(chunk.code[i]) {
            ops.push(op);
            match op {
                Opcode::Constant
                | Opcode::GetLocal
                | Opcode::SetLocal
                | Opcode::GetGlobal
                | Opcode::DefineGlobal
                | Opcode::SetGlobal
                | Opcode::Call
                | Opcode::Closure
                | Opcode::Import => {
                    i += 2;
                }
                Opcode::Jump | Opcode::JumpIfFalse | Opcode::Loop => {
                    i += 3;
                }
                _ => i += 1,
            }
        } else {
            i += 1;
        }
    }
    ops
}

#[test]
fn test_compile_math_expression() {
    let source = "1 + 2 * 3;";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::Constant,
            Opcode::Constant,
            Opcode::Multiply,
            Opcode::Add,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_variables() {
    let source = "
            var a = 10;
            var b = 20;
            print a + b;
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            Opcode::Constant,
            Opcode::DefineGlobal,
            Opcode::GetGlobal,
            Opcode::GetGlobal,
            Opcode::Add,
            Opcode::Print,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_local_variables_and_scopes() {
    let source = "
            var a = 1;
            {
                var b = 2;
                print a + b;
                b = 3;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            Opcode::Constant,
            Opcode::GetGlobal,
            Opcode::GetLocal,
            Opcode::Add,
            Opcode::Print,
            Opcode::Constant,
            Opcode::SetLocal,
            Opcode::Pop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_if_statement() {
    let source = "
            if (true) {
                print 1;
            } else {
                print 2;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::True,
            Opcode::JumpIfFalse,
            Opcode::Pop,
            Opcode::Constant,
            Opcode::Print,
            Opcode::Jump,
            Opcode::Pop,
            Opcode::Constant,
            Opcode::Print,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_while_statement() {
    let source = "
            var i = 0;
            while (i < 3) {
                print i;
                i = i + 1;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Less,
            Opcode::JumpIfFalse,
            Opcode::Pop,
            Opcode::GetGlobal,
            Opcode::Print,
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetGlobal,
            Opcode::Pop,
            Opcode::Loop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_for_statement() {
    let source = "
            for (var i = 0; i < 3; i = i + 1) {
                print i;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Less,
            Opcode::JumpIfFalse,
            Opcode::Pop,
            Opcode::Jump,
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetLocal,
            Opcode::Pop,
            Opcode::Loop,
            Opcode::GetLocal,
            Opcode::Print,
            Opcode::Loop,
            Opcode::Pop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_logical_and_or() {
    let mut interner = StringInterner::new();
    let source = "false && true || 42;";
    let chunk = compile(source, &mut interner).expect("Compilation failed.");

    let expected_opcodes = vec![
        Opcode::False,
        Opcode::JumpIfFalse,
        Opcode::Pop,
        Opcode::True,
        Opcode::JumpIfFalse,
        Opcode::Jump,
        Opcode::Pop,
        Opcode::Constant,
        Opcode::Pop,
        Opcode::Nop,
        Opcode::Return,
    ];

    assert_eq!(extract_opcodes(&chunk), expected_opcodes);
}

#[test]
fn test_compile_call_expression() {
    let source = "foo(1, 2);";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Constant,
            Opcode::Call,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_func_declaration() {
    let source = "
            func add(a, b) {
                print a + b;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");

    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Closure,
            Opcode::DefineGlobal,
            Opcode::Nop,
            Opcode::Return,
        ]
    );

    assert_eq!(chunk.function_constants.len(), 1);
    let func_desc = &chunk.function_constants[0];
    assert_eq!(func_desc.name.as_deref(), Some("add"));
    assert_eq!(func_desc.arity, 2);

    let func_ops = extract_opcodes(&func_desc.chunk);
    assert_eq!(
        func_ops,
        vec![
            Opcode::GetLocal,
            Opcode::GetLocal,
            Opcode::Add,
            Opcode::Print,
            Opcode::Pop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_prefix_increment_decrement() {
    // Global variables
    let source_global = "
            var x = 10;
            ++x;
            --x;
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_global, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            // ++x
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetGlobal,
            Opcode::Pop,
            // --x
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Subtract,
            Opcode::SetGlobal,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );

    // Local variables
    let source_local = "
            {
                var y = 20;
                ++y;
                --y;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_local, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            // ++y
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetLocal,
            Opcode::Pop,
            // --y
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Subtract,
            Opcode::SetLocal,
            Opcode::Pop,
            // End scope pops y
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_compound_assignments() {
    // Global compound assignments
    let source_global = "
            var a = 10;
            a += 5;
            a -= 2;
            a *= 3;
            a /= 4;
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_global, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            // a += 5
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetGlobal,
            Opcode::Pop,
            // a -= 2
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Subtract,
            Opcode::SetGlobal,
            Opcode::Pop,
            // a *= 3
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Multiply,
            Opcode::SetGlobal,
            Opcode::Pop,
            // a /= 4
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Divide,
            Opcode::SetGlobal,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );

    // Local compound assignments
    let source_local = "
            {
                var b = 10;
                b += 5;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_local, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetLocal,
            Opcode::Pop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_invalid_assignment_targets() {
    let mut interner = StringInterner::new();
    assert!(compile("1 + 2 = 3;", &mut interner).is_none());
    assert!(compile("1 + 2 += 3;", &mut interner).is_none());
    assert!(compile("1 + 2 -= 3;", &mut interner).is_none());
    assert!(compile("1 + 2 *= 3;", &mut interner).is_none());
    assert!(compile("1 + 2 /= 3;", &mut interner).is_none());
    assert!(compile("++5;", &mut interner).is_none());
    assert!(compile("--5;", &mut interner).is_none());
}

#[test]
fn test_compile_postfix_increment_decrement() {
    // Global variables
    let source_global = "
            var x = 10;
            x++;
            x--;
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_global, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            // x++: Get, Get, Constant(1.0), Add, Set, Pop, Pop(expr statement)
            Opcode::GetGlobal,
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetGlobal,
            Opcode::Pop,
            Opcode::Pop,
            // x--: Get, Get, Constant(1.0), Subtract, Set, Pop, Pop(expr statement)
            Opcode::GetGlobal,
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Subtract,
            Opcode::SetGlobal,
            Opcode::Pop,
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );

    // Local variables
    let source_local = "
            {
                var y = 20;
                y++;
                y--;
            }
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_local, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            // y++
            Opcode::GetLocal,
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetLocal,
            Opcode::Pop,
            Opcode::Pop,
            // y--
            Opcode::GetLocal,
            Opcode::GetLocal,
            Opcode::Constant,
            Opcode::Subtract,
            Opcode::SetLocal,
            Opcode::Pop,
            Opcode::Pop,
            // End scope pops y
            Opcode::Pop,
            Opcode::Nop,
            Opcode::Return,
        ]
    );

    // Expression usage: var y = x++ * 2;
    let source_expr = "
            var a = 5;
            var b = a++ * 2;
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source_expr, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![
            Opcode::Constant,
            Opcode::DefineGlobal,
            // a++
            Opcode::GetGlobal,
            Opcode::GetGlobal,
            Opcode::Constant,
            Opcode::Add,
            Opcode::SetGlobal,
            Opcode::Pop,
            // * 2
            Opcode::Constant,
            Opcode::Multiply,
            Opcode::DefineGlobal,
            Opcode::Nop,
            Opcode::Return,
        ]
    );
}

#[test]
fn test_compile_import_statement() {
    let source = "
            import('math');
            import(\"stdio\");
        ";
    let mut interner = StringInterner::new();
    let chunk = compile(source, &mut interner).expect("Compilation failed");
    let ops = extract_opcodes(&chunk);

    assert_eq!(
        ops,
        vec![Opcode::Import, Opcode::Import, Opcode::Nop, Opcode::Return,]
    );

    assert_eq!(chunk.string_constants.len(), 2);
    assert_eq!(chunk.string_constants[0], "math");
    assert_eq!(chunk.string_constants[1], "stdio");
}
