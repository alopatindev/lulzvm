#[cfg(test)]
mod tests {
    use byteorder::ByteOrder;
    use std::io::{BufReader, BufWriter};
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn simple() {
        {
            let executable = vec![0x00, 0x00];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,
                NOP];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // b
                PUSH, 0x0a,                    // a
                DEC,                           // a--
                EMIT, OUTPUT,
                JNE, 0x06, 0x00];              // a != b

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!([0x00, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // offset

                // label loop
                LOAD, PTR_WITH_OFFSET, 0x16, 0x00,
                                               // x = [data segment + offset]
                EMIT, OUTPUT,                  // print x
                PUSH, 0x00,                    // zero
                JE, 0x14, 0x00,                // if x == zero: goto end
                POP,                           // pop zero
                POP,                           // pop x
                INC,                           // offset++
                JMP, 0x04, 0x00,               // goto loop

                // label end
                NOP,                           // optional
                0x03, 0x02, 0x01, 0x00];

            let (output, vm) = run(&[], executable, 4);

            assert_eq!(&[0x03, 0x02, 0x01, 0x00], vm.data());
            assert_eq!(&[0x00, 0x00, 0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[0x03, 0x02, 0x01, 0x00], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // b

                // label loop
                EMIT, INPUT,                   // a
                EMIT, OUTPUT,                  // print a
                JE, 0x0f, 0x00,                // if a == b: goto end
                POP,                           // remove a
                JMP, 0x04, 0x00                // goto loop

                // label end
                // NOP                         // optional
                ];

            let input = [0x03, 0x02, 0x01, 0x00];
            let (output, vm) = run(&input, executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[0x03, 0x02, 0x01, 0x00], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // i
                                               // loop:
                LOAD, PTR, 0x15, 0x00,         // x
                JLE, 0x15, 0x00,               // if x <= i goto: end
                DEC,
                STORE, PTR, 0x15, 0x00,        // x
                POP,                           // pop x
                INC,                           // i++
                JMP, 0x04, 0x00,               // goto loop
                                               // end:
                0x05];                         // x

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x02], vm.data());
            assert_eq!(&[0x02, 0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // i
                PUSH, 0x05,                    // x
                                               // loop:
                DEC,
                SWP,
                INC,
                SWP,
                JLE, 0x10, 0,                  // if x <= i: goto end
                JMP, 0x06, 0];                 // goto loop

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x02, 0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // const zero
                                               // loop:
                EMIT, INPUT,                   //   x = read
                CALL, PTR, 0x14, 0x00,         //   x = f(x)
                EMIT, OUTPUT,                  //   print x
                JE, 0x12, 0x00,                // if x == zero: goto exit
                POP,                           // pop x
                JMP, 0x02, 0x00,               // goto loop
                EMIT, TERMINATE,               // exit:

                                               // f:
                PUSH, 0x02,
                MUL,                           // a = a * 2
                RET];

            let input = [0x03, 0x02, 0x01, 0x00];
            let (output, vm) = run(&input, executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[0x06, 0x04, 0x02, 0x00], output.as_slice());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn locals_stack() {
        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x77, 0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77,
                POP,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x88,
                PUSH, 0x99,
                SWP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x88, 0x99], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[ignore]
    #[test]
    fn locals_stack_damage() {
        {
            let executable = vec![
                0x00, 0x00,

                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable_size = WORD_SIZE + LOCALS_STACK_SIZE * 2;
            let executable_size = executable_size as usize;
            let mut executable = Vec::with_capacity(executable_size);
            executable.resize(executable_size, 0x00);

            let mut i = 3;
            while i < executable_size {
                executable[i] = PUSH;
                i += 2;
            }

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(LOCALS_STACK_SIZE, vm.locals_stack().len() as Word);
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable_size = WORD_SIZE + (LOCALS_STACK_SIZE + 1) * 2;
            let executable_size = executable_size as usize;
            let mut executable = Vec::with_capacity(executable_size);
            executable.resize(executable_size, 0x00);

            let mut i = 3;
            while i < executable_size {
                executable[i] = PUSH;
                i += 2;
            }

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(LOCALS_STACK_SIZE, vm.locals_stack().len() as Word);
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                SWP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }
    }

    #[test]
    fn event_queue() {
        let executable = vec![0x00, 0x00];

        let (output, mut vm) = run(&[], executable, 0);
        let event_queue_end = vm.memory.event_queue_end;

        assert_eq!(event_queue_end, vm.get_register(EP));
        assert_eq!(event_queue_end, vm.get_register(EE));

        vm.event_queue_push(CLOCK, 0x05);
        vm.event_queue_push(OUTPUT, 0x06);

        assert_eq!(&[OUTPUT, 0x06, CLOCK, 0x05], vm.event_queue());

        assert_lt!(vm.get_register(EP), vm.get_register(EE));
        assert_eq!(event_queue_end, vm.get_register(EE));

        let (event, argument) = vm.event_queue_pop();
        assert_eq!(CLOCK, event);
        assert_eq!(0x05, argument);
        assert_eq!(&[OUTPUT, 0x06], vm.event_queue());

        assert_lt!(vm.get_register(EP), vm.get_register(EE));
        assert_gt!(event_queue_end, vm.get_register(EE));

        let (event, argument) = vm.event_queue_pop();
        assert_eq!(OUTPUT, event);
        assert_eq!(0x06, argument);
        assert!(vm.event_queue().is_empty());

        assert_eq!(event_queue_end, vm.get_register(EP));
        assert_eq!(event_queue_end, vm.get_register(EE));

        vm.event_queue_push(CLOCK, 0x07);
        let _ = vm.event_queue_pop();

        assert!(vm.data().is_empty());
        assert!(vm.locals_stack().is_empty());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn load_store() {
        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0x06, 0x00,
                0x7b];

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x7b], vm.data());
            assert_eq!(&[0x7b], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,
                LOAD, PTR_WITH_OFFSET, 0x08, 0x00,
                0x11, 0x22];

            let (output, vm) = run(&[], executable, 2);

            assert_eq!(&[0x11, 0x22], vm.data());
            assert_eq!(&[0x22, 0x01], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                LOAD, PTR_WITH_OFFSET, 0x0e, 0x00,
                SWP,
                INC,
                LOAD, PTR_WITH_OFFSET, 0x0e, 0x00,
                0x11, 0x22];

            let (output, vm) = run(&[], executable, 2);

            assert_eq!(&[0x11, 0x22], vm.data());
            assert_eq!(&[0x22, 0x01, 0x11], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0xff, 0xff];        // access violation

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0x02, 0x00];        // try to load code segment
                                               // as data segment

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,
                                               // empty locals_stack
                LOAD, PTR_WITH_OFFSET, 0x06, 0x00,
                0x88];

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x88], vm.data());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                STORE, PTR, 0x08, 0x00,
                0x00];

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x55], vm.data());
            assert_eq!(&[0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x01,
                STORE, PTR_WITH_OFFSET, 0x0a, 0x00,
                0x00, 0x88];

            let (output, vm) = run(&[], executable, 2);

            assert_eq!(&[0x00, 0x55], vm.data());
            assert_eq!(&[0x01, 0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                STORE, PTR_WITH_OFFSET, 0x00, 0x00];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x55], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn arithmetic() {
        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x02,                    // b
                PUSH, 0x03,                    // a
                ADD];                          // pop 2 bytes,
                                               // add (a + b) and push

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x05], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0xff,
                PUSH, 0x01,
                ADD];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,                    // not enough operands
                ADD];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x02,
                PUSH, 0x03,
                SUB];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                SUB];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xff], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                MUL];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x06], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x99,
                PUSH, 0x66,
                MUL];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xf6], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x0c,
                DIV];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,
                DIV];                          // div by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x37,
                MOD];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x37,
                MOD];                          // mod by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x06], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0xff,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x04], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xff], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn arithmetic_missing_args() {
        for &opcode in &[ADD, SUB, DIV, MUL, MOD] {
            {
                let executable = vec![
                    0x00, 0x00,

                    PUSH, 0x01,
                    opcode];

                let (output, vm) = run(&[], executable, 0);

                assert!(vm.data().is_empty());
                assert_eq!(&[0x01], vm.locals_stack());
                assert!(vm.return_stack().is_empty());
                assert!(vm.event_queue().is_empty());
                assert_eq!(b"Segfault", output.as_slice());
            }
        }

        for &opcode in &[ADD, SUB, DIV, MUL, MOD, INC, DEC] {
            {
                let executable = vec![
                    0x00, 0x00,

                    opcode];

                let (output, vm) = run(&[], executable, 0);

                assert!(vm.data().is_empty());
                assert!(vm.locals_stack().is_empty());
                assert!(vm.return_stack().is_empty());
                assert!(vm.event_queue().is_empty());
                assert_eq!(b"Segfault", output.as_slice());
            }
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn bitwise() {
        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x55,
                AND];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,
                PUSH, 0x55,
                AND];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                AND];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                AND];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x00,
                OR];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x02,
                OR];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x02], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x02,
                NOT];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                NOT];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,
                SHL, 0x03];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x08], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x80,
                SHL, 0x01];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                SHL, 0x01];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x80,
                SHR, 0x01];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x40], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                SHR, 0x01];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x00,
                XOR];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x10,
                PUSH, 0x20,
                XOR];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x30], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn jumps() {
        {
            let executable = vec![
                0x00, 0x00,

                JMP, 0x55, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(0x5555, vm.get_register(PC));

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x00,

                JE, 0x55, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(0x5555, vm.get_register(PC));

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,

                JE, 0x55, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(0x09, vm.get_register(PC));

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,

                JG, 0x55, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(0x5555, vm.get_register(PC));

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01, 0x00], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,
                PUSH, 0x00,

                JG, 0x55, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(0x09, vm.get_register(PC));

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x01], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn functions() {
        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,
                CALL, 0x09, 0x00,              // call f
                EMIT, TERMINATE,

                INC,                           // f:
                EMIT, OUTPUT,
                RET];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x02], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x01,                    // main:
                CALL, 0x09, 0x00,              // call f
                EMIT, TERMINATE,

                INC,                           // f:
                EMIT, OUTPUT,
                CALL, 0x10, 0x00,              // call y
                RET,

                NOT,                           // y
                OUTPUT,
                RET];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x02], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.as_slice().is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn events() {
        // TODO: set event handler
        // TODO: implement default handlers

        {
            let executable = vec![
                0x00, 0x00,

                EMIT, TERMINATE];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                EMIT, INPUT];

            let input = [0x11];
            let (output, vm) = run(&input, executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x11], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x22,
                EMIT, OUTPUT];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x22], vm.locals_stack());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[0x22], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,
                DIV];                          // div by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.locals_stack().is_empty());
            assert!(vm.return_stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }
    }

    fn run(input: DataSlice,
           mut executable: Data,
           data_size: Word)
           -> (Data, VM<BufReader<DataSlice>, BufWriter<Data>>) {
        let _ = env_logger::init();

        let code_size = executable.len() as Word - CODE_OFFSET - data_size;
        Endian::write_u16(&mut executable, code_size);

        let input = BufReader::new(input);

        let output: Data = vec![];
        let output = BufWriter::new(output);

        let mut vm = VM::new(input, output, executable);
        vm.run();

        let output = vm.output
            .get_mut()
            .by_ref()
            .iter()
            .map(|x| *x)
            .collect::<Data>();

        (output, vm)
    }
}
