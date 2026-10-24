use super::*;
use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;
use ely_core::value::Value;


    fn emit_constant(chunk: &mut Chunk, val: Value, line: u32) {
        let idx = chunk.add_constant(val);
        chunk.write_opcode(Opcode::Constant, line);
        chunk.write_byte(idx as u8, line);
    }

    #[test]
    fn test_vm_interpret_simple_return() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(42.0), 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(42.0));
    }

    #[test]
    fn test_vm_interpret_binary_arithmetic() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(10.0), 1);
        emit_constant(&mut chunk, Value::number(5.0), 1);
        chunk.write_opcode(Opcode::Add, 1);
        emit_constant(&mut chunk, Value::number(2.0), 1);
        chunk.write_opcode(Opcode::Multiply, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(30.0));
    }

    #[test]
    fn test_vm_interpret_bitwise_operations() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(12.0), 1);
        emit_constant(&mut chunk, Value::number(5.0), 1);
        chunk.write_opcode(Opcode::BwAnd, 1);
        emit_constant(&mut chunk, Value::number(3.0), 1);
        chunk.write_opcode(Opcode::BwOr, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(7.0));
    }

    #[test]
    fn test_vm_interpret_array_operations() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(10.0), 1);
        emit_constant(&mut chunk, Value::number(20.0), 1);
        emit_constant(&mut chunk, Value::number(30.0), 1);
        chunk.write_opcode(Opcode::ArrayBuild, 1);
        chunk.write_byte(3, 1);

        emit_constant(&mut chunk, Value::number(1.0), 1);
        chunk.write_opcode(Opcode::ArrayIndex, 1);

        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(20.0));
    }

    #[test]
    fn test_vm_interpret_class_instance_and_properties() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        // 1. Declare class
        let class_name_const = chunk.add_constant(Value::obj(1));
        chunk.write_opcode(Opcode::Class, 1);
        chunk.write_byte(class_name_const as u8, 1);

        // 2. Instantiate class: Call with 0 args
        chunk.write_opcode(Opcode::Call, 1);
        chunk.write_byte(0, 1);

        // Store instance in local slot 0
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(0, 1);

        // Set property field 42 = 999.0: pushes instance, pushes value
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(0, 1);
        let field_name_const = chunk.add_constant(Value::obj(42));
        emit_constant(&mut chunk, Value::number(999.0), 1);
        chunk.write_opcode(Opcode::SetProperty, 1);
        chunk.write_byte(field_name_const as u8, 1);
        // Pop result of SetProperty
        chunk.write_opcode(Opcode::Pop, 1);

        // Get property field 42: push instance, GetProperty
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(0, 1);
        chunk.write_opcode(Opcode::GetProperty, 1);
        chunk.write_byte(field_name_const as u8, 1);

        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.peek(0), Value::number(999.0));
    }

    #[test]
    fn test_vm_interpret_inherit_and_invoke() {
        let mut vm = VM::new();

        // Build method function: returns 42.0 (takes 0 args, slot 0 is receiver)
        let mut method_chunk = Chunk::new();
        emit_constant(&mut method_chunk, Value::number(42.0), 1);
        method_chunk.write_opcode(Opcode::Return, 1);
        let mut method_func = crate::heap::ObjFunction::new(0, Some("foo".to_string()));
        method_func.chunk = Rc::new(method_chunk);
        let method_func_val = vm.allocate_object(HeapObject::Function(method_func));

        let mut chunk = Chunk::new();
        // 1. Superclass A
        let class_a_name = chunk.add_constant(Value::obj(1));
        chunk.write_opcode(Opcode::Class, 1);
        chunk.write_byte(class_a_name as u8, 1);

        // Add method 'foo' (name id 100) to class A
        let method_name_const = chunk.add_constant(Value::obj(100));
        let method_const = chunk.add_constant(method_func_val);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(method_const as u8, 1);
        chunk.write_opcode(Opcode::Method, 1);
        chunk.write_byte(method_name_const as u8, 1);

        // 2. Subclass B
        let class_b_name = chunk.add_constant(Value::obj(2));
        chunk.write_opcode(Opcode::Class, 1);
        chunk.write_byte(class_b_name as u8, 1);

        // Inherit: stack has [Class A, Class B]
        chunk.write_opcode(Opcode::Inherit, 1);
        // Inherit pops Class B, Class A is left on stack. Pop Class A
        chunk.write_opcode(Opcode::Pop, 1);

        // Instantiate Class B (which is on heap id 2)
        // Push Class B directly and Call(0)
        chunk.write_opcode(Opcode::Constant, 1);
        let class_b_val = Value::obj(2);
        let b_const = chunk.add_constant(class_b_val);
        chunk.write_byte(b_const as u8, 1);
        chunk.write_opcode(Opcode::Call, 1);
        chunk.write_byte(0, 1);

        // Stack has Instance of B. Invoke 'foo' with 0 args
        chunk.write_opcode(Opcode::Invoke, 1);
        chunk.write_byte(method_name_const as u8, 1);
        chunk.write_byte(0, 1);

        // Return result
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.peek(0), Value::number(42.0));
    }

    #[test]
    fn test_vm_jit_hot_path_tier_up() {
        let mut vm = VM::new();

        // Target function: func add(a, b) { return a + b; }
        let mut add_chunk = Chunk::new();
        add_chunk.write_opcode(Opcode::GetLocal, 1);
        add_chunk.write_byte(1, 1); // a
        add_chunk.write_opcode(Opcode::GetLocal, 1);
        add_chunk.write_byte(2, 1); // b
        add_chunk.write_opcode(Opcode::Add, 1);
        add_chunk.write_opcode(Opcode::Return, 1);

        let mut add_func = crate::heap::ObjFunction::new(2, Some("add".to_string()));
        add_func.chunk = Rc::new(add_chunk);
        let add_func_val = vm.allocate_object(HeapObject::Function(add_func));

        // Call the function 60 times (> JIT_HOT_THRESHOLD = 50)
        let mut chunk = Chunk::new();
        let add_const = chunk.add_constant(add_func_val);
        let c_one = chunk.add_constant(Value::number(1.0));
        let c_two = chunk.add_constant(Value::number(2.0));

        for _ in 0..60 {
            chunk.write_opcode(Opcode::Constant, 1);
            chunk.write_byte(add_const as u8, 1);
            chunk.write_opcode(Opcode::Constant, 1);
            chunk.write_byte(c_one as u8, 1);
            chunk.write_opcode(Opcode::Constant, 1);
            chunk.write_byte(c_two as u8, 1);
            chunk.write_opcode(Opcode::Call, 1);
            chunk.write_byte(2, 1);
            chunk.write_opcode(Opcode::Pop, 1);
        }

        // 61st call: keep result on stack
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(add_const as u8, 1);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_one as u8, 1);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_two as u8, 1);
        chunk.write_opcode(Opcode::Call, 1);
        chunk.write_byte(2, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.peek(0), Value::number(3.0));

        // Verify that the function was tiered-up to native JIT execution!
        let func_obj = vm.get_object(add_func_val).unwrap();
        if let HeapObject::Function(f) = func_obj {
            assert!(f.invocation_count >= 50);
            assert!(f.jit_entry.is_some(), "Function was not tiered-up to JIT!");
        } else {
            panic!("Expected function object");
        }
    }

    #[test]
    fn test_vm_interpret_prefix_inc_dec_and_compound_assignments() {
        let mut interner = ely_core::interner::StringInterner::new();
        let source = "
            func compute() {
                var x = 10;
                ++x;       // 11
                x += 5;    // 16
                --x;       // 15
                x -= 3;    // 12
                x *= 2;    // 24
                x /= 4;    // 6
                return x;
            }
            compute();
        ";
        let chunk = ely_compiler::compiler::compile(source, &mut interner).expect("Compilation failed");
        let mut vm = VM::new();
        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_postfix_increment_decrement() {
        let mut interner = ely_core::interner::StringInterner::new();
        let source = "
            func compute() {
                var x = 10;
                var old_x = x++;
                var old_x2 = x--;
                var y = x++ * 2;
                return old_x + old_x2 + y + x;
            }
            compute();
        ";
        let chunk = ely_compiler::compiler::compile(source, &mut interner).expect("Compilation failed");
        let mut vm = VM::new();
        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_import_statement() {
        let mut vm = VM::new();
        let source = "
            import('math');
            import(\"io\");
            var a = sqrt(144);
            var b = pow(2, 5);
            var t = clock();
        ";
        let chunk = ely_compiler::compiler::compile(source, &mut vm.interner).expect("Compilation failed");
        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        let a_id = vm.interner.intern("a");
        let b_id = vm.interner.intern("b");
        let t_id = vm.interner.intern("t");

        assert_eq!(vm.globals.get(&a_id).unwrap().as_number(), 12.0);
        assert_eq!(vm.globals.get(&b_id).unwrap().as_number(), 32.0);
        assert!(vm.globals.get(&t_id).unwrap().as_number() > 0.0);
    }
