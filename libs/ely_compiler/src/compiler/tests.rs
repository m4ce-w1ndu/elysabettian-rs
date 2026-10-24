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
                    | Opcode::Call => {
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
                Opcode::GetLocal,
                Opcode::GetLocal,
                Opcode::Add,
                Opcode::Print,
                Opcode::Nop,
                Opcode::Return,
                Opcode::Nop,
                Opcode::Return,
            ]
        );
    }
